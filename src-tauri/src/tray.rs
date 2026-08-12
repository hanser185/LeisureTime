use crate::commands;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::App;

/// 构建系统托盘与右键菜单
pub fn build_tray(app: &App) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "暂停检测", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "打开设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出应用", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &settings, &quit])?;

    TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("休息提醒助手")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => {
                let paused = commands::do_toggle_pause(app);
                if let Some(tray) = app.tray_by_id("main-tray") {
                    let _ = tray.set_tooltip(Some(if paused {
                        "休息提醒助手（已暂停）"
                    } else {
                        "休息提醒助手"
                    }));
                }
            }
            "settings" => {
                commands::open_main(app);
                commands::goto_settings(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                commands::open_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
