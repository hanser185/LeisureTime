use crate::commands::{open_rest_window, open_water_window};
use crate::state::{now_ms, Store, UserState};
use crate::storage;
use std::sync::Arc;
use tauri::AppHandle;

/// 启动调度循环：每秒检查阈值，触发休息/喝水提醒
pub fn run_scheduler(app: AppHandle, store: Arc<Store>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let now = now_ms();

        let mut s = store.lock();
        // 跨日归档
        if let Some(old) = s.tick(now) {
            storage::save_daily(&old);
        }
        if s.settings.paused {
            continue;
        }

        let work_th = s.settings.work_threshold_min * 60_000;
        let reached = s.current_segment_ms(now) >= work_th;
        let in_work = s.user_state == UserState::Working;
        let not_snoozed = now >= s.snooze_until_ms;
        let in_wh = s.in_work_hours();

        // 按工作阈值循环再提醒：距上次提醒已过去一个完整阈值，解除本片段抑制，
        // 使连续长时间工作可多次提醒（修复“休息提醒只弹一次”）
        s.rearm_rest_if_due(now, work_th);

        // 休息提醒：连续工作达阈值且本片段未提醒、未处于稍后期、且处于工作时段
        if in_work && reached && !s.rest_fired_for_segment && not_snoozed && in_wh {
            s.rest_fired_for_segment = true;
            s.daily.rest_reminders += 1;
            s.daily.rest_count += 1; // 每次弹出休息提醒计为一次休息（修复“休息次数恒为 0”）
            s.last_rest_prompt_ms = now;
            s.water_deferred = true;
            // 弹窗展示真实连续分钟数（阈值只是触发下限，实际可能已超出）
            let min = (s.current_segment_ms(now) / 60_000).max(1);
            drop(s);
            open_rest_window(&app, min);
            continue;
        }

        // 喝水提醒：开启且活跃，且距上次提示超过间隔；休息提醒在场时让位；且处于工作时段
        if s.settings.water_enabled
            && in_work
            && !s.water_deferred
            && in_wh
            && now.saturating_sub(s.daily.last_water_prompt_ms)
                >= s.settings.water_interval_min * 60_000
        {
            s.daily.last_water_prompt_ms = now;
            drop(s);
            open_water_window(&app);
            continue;
        }

        // 周期落盘（每 30s），防止异常退出丢数据
        if now.saturating_sub(s.last_save_ms) >= 30_000 {
            s.last_save_ms = now;
            let d = s.daily.clone();
            drop(s);
            storage::save_daily(&d);
        }
    });
}
