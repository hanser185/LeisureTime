use crate::commands;
use crate::state::Store;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{App, Manager};

/// 构建系统托盘与右键菜单
pub fn build_tray(app: &App) -> tauri::Result<()> {
    let store = app.state::<Arc<Store>>();
    // 通过锁获取最新的暂停状态，确保初始文案与后端状态完全一致
    let paused = store.lock().settings.paused;
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

    // toggle_item 克隆供菜单事件回调里实时更新文案
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
            // 仅响应“左键单击抬起”打开主窗口；
            // 若不区分按键，右键也会被此处理器拦截，导致右键菜单无法弹出
            if let TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                commands::open_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
