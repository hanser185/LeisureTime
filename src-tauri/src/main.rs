#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod activity;
mod commands;
mod scheduler;
mod state;
mod storage;
mod tray;

use state::{Settings, Store};
use std::sync::Arc;
use tauri::Manager;

fn main() {
    let settings: Settings = storage::load_settings();
    let today = state::today_string();
    let daily = storage::load_daily(&today);

    let store: Arc<Store> = Arc::new(Store::new(settings, daily, today));
    let store_setup = Arc::clone(&store);

    tauri::Builder::default()
        .setup(move |app| {
            tray::build_tray(app)?;
            // 若设置了开机自启，启动时确保注册表项存在
            let autostart = store_setup.0.lock().unwrap().settings.autostart;
            if autostart {
                commands::apply_autostart(true);
            }
            activity::start_listener(Arc::clone(&store_setup));
            scheduler::run_scheduler(app.handle().clone(), Arc::clone(&store_setup));
            Ok(())
        })
        .manage(store)
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_daily,
            commands::get_weekly,
            commands::skip_rest,
            commands::snooze_rest,
            commands::record_water,
            commands::defer_water,
            commands::toggle_pause,
            commands::clear_today,
            commands::data_path,
            commands::get_status,
            commands::open_data_folder,
        ])
        .on_window_event(|window, event| {
            // 关闭主窗口 -> 仅最小化到托盘，不退出
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            // 休息弹窗无论以何种方式关闭（手动跳过/稍后、倒计时自动关闭、点 X），
            // 都解除“喝水提醒让位”标记，否则当天喝水提醒会永久失效
            if window.label() == "rest" && matches!(event, tauri::WindowEvent::Destroyed) {
                if let Ok(mut s) = window.state::<Arc<Store>>().0.lock() {
                    s.water_deferred = false;
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("启动失败");
}
