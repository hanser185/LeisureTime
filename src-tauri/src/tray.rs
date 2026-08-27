use crate::commands;
use crate::state::Store;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{App, Manager};

/// 构建系统托盘与右键菜单
pub fn build_tray(app: &App) -> tauri::Result<()> {
    // 初始文案与当前暂停态一致，之后每次切换动态更新
    let paused = app.state::<Arc<Store>>().lock().settings.paused;
    let toggle = MenuItem::with_id(
        app,
        "toggle",
        if paused {
            "恢复检测"
        } else {
            "暂停检测"
        },
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "settings", "打开设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出应用", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &settings, &quit])?;

    // 克隆一份句柄供菜单事件回调里更新文案
    let toggle_item = toggle.clone();

    let mut builder = TrayIconBuilder::with_id("main-tray");
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("休息提醒助手")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "toggle" => {
                let paused = commands::do_toggle_pause(app);
                let _ = toggle_item.set_text(if paused {
                    "恢复检测"
                } else {
                    "暂停检测"
                });
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
            "quit" => {
                commands::flush_and_save(app);
                app.exit(0);
            }
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
