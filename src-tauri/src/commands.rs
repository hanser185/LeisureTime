use crate::state::{now_hm, now_ms, DailyData, Settings, Store, UserState};
use crate::storage;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// 开机自启动：写入/删除 Windows 注册表 Run 键（仅 Windows）
#[cfg(windows)]
pub fn apply_autostart(enable: bool) {
    use winreg::enums::*;
    use winreg::RegKey;
    const RUN_NAME: &str = "休息提醒助手";
    if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE)
    {
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        if enable && !exe.is_empty() {
            if let Err(e) = hkcu.set_value(RUN_NAME, &exe) {
                eprintln!("[autostart] 写入 Run 键失败（开机自启可能未生效）: {e:?}");
            }
        } else if let Err(e) = hkcu.delete_value(RUN_NAME) {
            // 值不存在属正常情况（从未开启过自启），不必告警
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("[autostart] 删除 Run 键失败: {e:?}");
            }
        }
    } else {
        eprintln!("[autostart] 打开注册表 Run 键失败（权限不足？），开机自启设置未生效");
    }
}
#[cfg(not(windows))]
pub fn apply_autostart(_enable: bool) {}

/// 根据 dev/prod 环境返回前端路由地址（Tauri 2 的窗口 URL 需为 WebviewUrl）
/// 始终携带 mode 参数，供前端区分 toast（角落轻提醒）/popup/fullscreen 渲染样式
fn rest_url(min: u64, mode: &str) -> WebviewUrl {
    let q = format!("?min={}&mode={}", min, mode);
    if cfg!(debug_assertions) {
        WebviewUrl::External(
            tauri::Url::parse(&format!("http://localhost:5173/#/rest{}", q)).unwrap(),
        )
    } else {
        WebviewUrl::External(tauri::Url::parse(&format!("tauri://localhost/#/rest{}", q)).unwrap())
    }
}
fn water_url() -> WebviewUrl {
    if cfg!(debug_assertions) {
        WebviewUrl::External(tauri::Url::parse("http://localhost:5173/#/water").unwrap())
    } else {
        WebviewUrl::External(tauri::Url::parse("tauri://localhost/#/water").unwrap())
    }
}

/// 打开/聚焦休息提醒窗口（带已连续工作分钟数）
/// reminder_mode：toast=右下角免打扰轻提醒（不抢焦点）；popup=居中弹窗；fullscreen=全屏遮罩
pub fn open_rest_window(app: &AppHandle, min: u64) {
    let store = app.state::<Arc<Store>>();
    let mode = {
        let s = store.lock();
        s.settings.reminder_mode.clone()
    };
    if let Some(w) = app.get_webview_window("rest") {
        let _ = w.show();
        let _ = w.set_focus();
        // 已存在的弹窗无法改 URL 参数，用事件把最新参数推给前端刷新显示与倒计时
        let _ = app.emit_to(
            "rest",
            "rest-params",
            serde_json::json!({ "min": min, "mode": mode }),
        );
        return;
    }
    let builder = WebviewWindowBuilder::new(app, "rest", rest_url(min, &mode))
        .title("休息提醒")
        .decorations(false)
        .always_on_top(true);
    let builder = if mode == "toast" {
        // 右下角免打扰：不抢焦点、不进任务栏；取不到显示器信息则退化为居中
        const W: f64 = 320.0;
        const H: f64 = 170.0;
        const MARGIN: f64 = 16.0;
        let b = builder
            .inner_size(320.0, 170.0)
            .focused(false)
            .skip_taskbar(true);
        match app.primary_monitor() {
            Ok(Some(monitor)) => {
                // position 使用物理像素
                let sf = monitor.scale_factor();
                let size = monitor.size();
                let x = size.width as f64 - W * sf - MARGIN * sf;
                let y = size.height as f64 - H * sf - MARGIN * sf;
                b.position(x, y)
            }
            _ => b.center(),
        }
    } else if mode == "fullscreen" {
        builder.fullscreen(true)
    } else {
        builder.center().inner_size(360.0, 210.0)
    };
    let _ = builder.build();
}

/// 打开/聚焦喝水提醒窗口
pub fn open_water_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("water") {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "water", water_url())
        .title("喝水提醒")
        .inner_size(320.0, 190.0)
        .decorations(false)
        .always_on_top(true)
        .center()
        .build();
}

pub fn open_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// 通知前端切换到设置页
pub fn goto_settings(app: &AppHandle) {
    let _ = app.emit("navigate", "settings");
}

pub fn do_toggle_pause(app: &AppHandle) -> bool {
    let store = app.state::<Arc<Store>>();
    // ponytail: 锁内只改状态，落盘放锁外
    let settings = {
        let mut s = store.lock();
        let paused = !s.settings.paused;
        s.set_paused(paused, now_ms());
        s.settings.clone()
    };
    storage::save_settings(&settings);
    settings.paused
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    app.state::<Arc<Store>>().lock().settings.clone()
}

/// 保存设置：后端 sanitize 后生效并落盘，同时把消毒后的值返回给前端回显，
/// 避免超界输入被静默钳制后 UI 与实际值脱节。
#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Settings {
    let mut settings = settings;
    settings.sanitize();

    let store = app.state::<Arc<Store>>();
    let autostart_changed = {
        let mut s = store.lock();
        let changed = s.settings.autostart != settings.autostart;
        s.settings = settings.clone();
        changed
    };
    // ponytail: 放锁后再落盘，避免阻塞 1Hz 调度线程与活动监听线程
    storage::save_settings(&settings);
    if autostart_changed {
        apply_autostart(settings.autostart);
    }
    settings
}

#[tauri::command]
pub fn get_daily(app: AppHandle) -> DailyData {
    app.state::<Arc<Store>>().lock().daily.clone()
}

#[tauri::command]
pub fn get_weekly(app: AppHandle) -> Vec<serde_json::Value> {
    use chrono::{Duration, Local};
    let today = Local::now().date_naive();
    // 今日直接用内存实时数据（磁盘要等 30s 周期落盘，读文件会滞后），历史日期才读文件
    let live_today = app.state::<Arc<Store>>().lock().daily.clone();
    let mut out = Vec::new();
    for i in 0..7 {
        let d = today - Duration::days(i);
        let s = d.format("%Y-%m-%d").to_string();
        let daily = if i == 0 {
            live_today.clone()
        } else {
            storage::load_daily(&s)
        };
        let work_min: u64 = daily
            .work_segments
            .iter()
            .map(|x| x.duration_sec)
            .sum::<u64>()
            / 60;
        let rest_min: u64 = daily
            .rest_segments
            .iter()
            .map(|x| x.duration_sec)
            .sum::<u64>()
            / 60;
        out.push(serde_json::json!({
            "date": s,
            "work_min": work_min,
            "rest_min": rest_min,
            "water": daily.water_intakes.len(),
        }));
    }
    out
}

#[tauri::command]
pub fn skip_rest(app: AppHandle) {
    let store = app.state::<Arc<Store>>();
    let mut s = store.lock();
    s.rest_fired_for_segment = true;
    s.last_rest_prompt_ms = now_ms(); // 推迟下一轮提醒一个完整工作阈值，避免“跳过”后立刻再弹
    s.water_deferred = false;
}

/// 稍后提醒：仅抑制稍后期内的重复触发，到点后沿用当前工作片段重新判定
#[tauri::command]
pub fn snooze_rest(app: AppHandle, minutes: u64) {
    let store = app.state::<Arc<Store>>();
    let mut s = store.lock();
    let now = now_ms();
    s.rest_fired_for_segment = false;
    s.last_rest_prompt_ms = now;
    s.snooze_until_ms = now + minutes * 60_000;
    s.water_deferred = false;
}

#[tauri::command]
pub fn record_water(app: AppHandle) {
    let store = app.state::<Arc<Store>>();
    // ponytail: 锁内只改状态并 clone，落盘放锁外，避免阻塞调度/活动线程
    let daily = {
        let mut s = store.lock();
        s.record_water(now_hm());
        s.daily.clone()
    };
    storage::save_daily(&daily);
}

/// 用户点“稍后”：推迟下次喝水提醒一整个间隔，避免关闭弹窗后 1 秒内立即重复弹出
#[tauri::command]
pub fn defer_water(app: AppHandle) {
    let store = app.state::<Arc<Store>>();
    // ponytail: 与 record_water 一致，锁内改状态并 clone，落盘放锁外，避免 defer 后退出丢失
    let daily = {
        let mut s = store.lock();
        s.defer_water();
        s.daily.clone()
    };
    storage::save_daily(&daily);
}

#[tauri::command]
pub fn toggle_pause(app: AppHandle) -> bool {
    do_toggle_pause(&app)
}

#[tauri::command]
pub fn clear_today(app: AppHandle) {
    let store = app.state::<Arc<Store>>();
    // ponytail: 重置状态在锁内完成，删文件放锁外
    let date = {
        let mut s = store.lock();
        let date = s.current_date.clone();
        s.reset_daily_and_tracking(now_ms());
        date
    };
    storage::clear_daily(&date);
}

#[tauri::command]
pub fn data_path() -> String {
    storage::data_dir().to_string_lossy().to_string()
}

/// 实时状态（前端每秒轮询，用于“当前状态标识”与计时显示）
#[tauri::command]
pub fn get_status(app: AppHandle) -> serde_json::Value {
    let store = app.state::<Arc<Store>>();
    let s = store.lock();
    let state = match s.user_state {
        UserState::Idle => "idle",
        UserState::Working => "working",
        UserState::Resting => "resting",
    };
    serde_json::json!({
        "state": state,
        "current_segment_sec": s.current_segment_ms(now_ms()) / 1000,
        "paused": s.settings.paused,
        "snooze_until_ms": s.snooze_until_ms,
    })
}

/// 在资源管理器中打开本地数据目录
#[tauri::command]
pub fn open_data_folder() {
    let path = storage::data_dir();
    let _ = std::process::Command::new("explorer").arg(path).spawn();
}

/// 退出前闭合当前未落盘的工作/休息片段并保存当日数据
pub fn flush_and_save(app: &AppHandle) {
    let store = app.state::<Arc<Store>>();
    let daily = {
        let mut s = store.lock();
        s.flush();
        s.daily.clone()
    };
    storage::save_daily(&daily);
}
