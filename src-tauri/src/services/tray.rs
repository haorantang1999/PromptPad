//! 系统托盘。
//!
//! Tauri 2 的托盘属于核心能力（`tauri::tray`），**没有独立插件**，
//! 因此不需要额外引入第三方托盘库。

use crate::commands::panel;
use crate::error::AppError;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Runtime};

/// 构建托盘图标与菜单。
pub fn build<R: Runtime>(app: &AppHandle<R>, shortcut: &str) -> Result<(), AppError> {
    let toggle = MenuItem::with_id(app, "toggle", "显示 / 隐藏", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &settings, &quit])?;

    // 托盘图标直接复用打包用的 PNG；缺图时降级为「无图标托盘」，不阻断启动。
    let icon = Image::from_bytes(include_bytes!("../../icons/32x32.png")).ok();

    let mut builder = TrayIconBuilder::with_id("pad-tray")
        .menu(&menu)
        .tooltip(format!("Prompt Pad —— {shortcut} 唤起"))
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = panel::toggle_panel(tray.app_handle().clone(), None);
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => {
                let _ = panel::toggle_panel(app.clone(), None);
            }
            "settings" => {
                // 「设置」必须真的进设置界面。
                // 先记下视图再广播：后端要靠它决定「点外部收起」在设置视图下是否生效
                // —— 用户正要去别的应用复制 API key，这时收起面板会把表单弄丢。
                panel::store_view(app, crate::models::PadView::Settings);
                // 先广播视图切换再展开面板：顺序反了的话，前端会先按提示词视图
                // 把输入框聚焦，再切到设置表单，光标就被抢走又丢掉。
                let _ = app.emit(panel::EVENT_VIEW_CHANGED, "settings");
                let _ = panel::toggle_panel(app.clone(), Some(panel::PanelAction::Expand));
            }
            "quit" => app.exit(0),
            _ => {}
        });

    if let Some(icon) = icon {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}
