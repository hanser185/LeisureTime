use crate::state::{now_hm, now_ms, DailyData, Settings, Store, UserState};
use crate::storage;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// 开机自启动：写入/删除 Windows 注册表 Run 键（仅 Windows）
#[cfg(windows)]
pub fn apply_autostart(enable: bool) {
    use winreg::enums::*;
    use winreg::RegKey;
    if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE)
    {
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        if enable && !exe.is_empty() {
            let _ = hkcu.set_value("休息提醒助手", &exe);
        } else {
            let _ = hkcu.delete_value("休息提醒助手");
        }
    }
}
#[cfg(not(windows))]
pub fn apply_autostart(_enable: bool) {}

/// 根据 dev/prod 环境返回前端路由地址（Tauri 2 的窗口 URL 需为 WebviewUrl）
/// mode="fullscreen" 时追加 &mode=fullscreen，供前端渲染全屏遮罩样式
fn rest_url(min: u64, mode: &str) -> WebviewUrl {
    let q = if mode == "fullscreen" {
        format!("?min={}&mode=fullscreen", min)
    } else {
        format!("?min={}", min)
    };
    if cfg!(debug_assertions) {
        WebviewUrl::External(
            tauri::Url::parse(&format!("http://localhost:5173/#/rest{}", q)).unwrap(),
        )
    } else {
        WebviewUrl::External(
            tauri::Url::parse(&format!("tauri://localhost/#/rest{}", q)).unwrap(),
        )
    }
}
fn water_url() -> WebviewUrl {
    if cfg!(debug_assertions) {
        WebviewUrl::External(tauri::Url::parse("http://localhost:5173/#/water").unwrap())
    } else {
        WebviewUrl::External(tauri::Url::parse("tauri://localhost/#/water").unwrap())
    }
}

/// 打开/聚焦休息提醒窗口（带已工作分钟数）
/// reminder_mode=fullscreen 时创建覆盖全屏的遮罩窗口；否则为居中小窗。
pub fn open_rest_window(app: &AppHandle, min: u64) {
    let mode = {
        let s = app.state::<Arc<Store>>().0.lock().unwrap();
        s.settings.reminder_mode.clone()
    };
    if let Some(w) = app.get_webview_window("rest") {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let builder = WebviewWindowBuilder::new(app, "rest", rest_url(min, &mode))
        .title("休息提醒")
        .decorations(false)
        .always_on_top(true)
        .center();
    // fullscreen 为桌面端可用 API；否则使用固定小窗尺寸
    let builder = if mode == "fullscreen" {
        builder.fullscreen(true)
    } else {
        builder.inner_size(360.0, 210.0)
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
        .inner_size(320.0, 170.0)
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
    let mut s = store.0.lock().unwrap();
    s.settings.paused = !s.settings.paused;
    let p = s.settings.paused;
    storage::save_settings(&s.settings);
    p
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    app.state::<Arc<Store>>().0.lock().unwrap().settings.clone()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) {
    let store = app.state::<Arc<Store>>();
    let autostart_changed = {
        let mut s = store.0.lock().unwrap();
        let changed = s.settings.autostart != settings.autostart;
        s.settings = settings.clone();
        storage::save_settings(&settings);
        changed
    };
    if autostart_changed {
        apply_autostart(settings.autostart);
    }
}

#[tauri::command]
pub fn get_daily(app: AppHandle) -> DailyData {
    app.state::<Arc<Store>>().0.lock().unwrap().daily.clone()
}

#[tauri::command]
pub fn get_weekly(_app: AppHandle) -> Vec<serde_json::Value> {
    use chrono::{Duration, Local};
    let today = Local::now().date_naive();
    let mut out = Vec::new();
    for i in 0..7 {
        let d = today - Duration::days(i);
        let s = d.format("%Y-%m-%d").to_string();
        let daily = storage::load_daily(&s);
        let work_min: u64 = daily.work_segments.iter().map(|x| x.duration_sec).sum::<u64>() / 60;
        let rest_min: u64 = daily.rest_segments.iter().map(|x| x.duration_sec).sum::<u64>() / 60;
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
    let mut s = store.0.lock().unwrap();
    s.rest_fired_for_segment = true;
    s.water_deferred = false;
}

/// 稍后提醒：重置工作计时（从当前重新算阈值），并抑制稍后期内的重复触发
#[tauri::command]
pub fn snooze_rest(app: AppHandle, minutes: u64) {
    let store = app.state::<Arc<Store>>();
    let mut s = store.0.lock().unwrap();
    let now = now_ms();
    s.segment_start_ms = now;
    s.rest_fired_for_segment = false;
    s.snooze_until_ms = now + minutes * 60_000;
    s.water_deferred = false;
}

#[tauri::command]
pub fn record_water(app: AppHandle) {
    let store = app.state::<Arc<Store>>();
    let mut s = store.0.lock().unwrap();
    s.daily.water_intakes.push(now_hm());
    storage::save_daily(&s.daily);
}

#[tauri::command]
pub fn toggle_pause(app: AppHandle) -> bool {
    do_toggle_pause(&app)
}

#[tauri::command]
pub fn clear_today(app: AppHandle) {
    let store = app.state::<Arc<Store>>();
    let mut s = store.0.lock().unwrap();
    let date = s.current_date.clone();
    s.daily = DailyData {
        date: date.clone(),
        ..Default::default()
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
    let s = store.0.lock().unwrap();
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
