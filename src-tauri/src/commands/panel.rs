//! 面板形态控制。

use crate::commands::lock;
use crate::error::AppError;
use crate::models::{TogglePanelResponse, WindowAnchor};
use crate::services::fullscreen;
use crate::services::window_ctl;
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// 面板形态变化事件名。前端监听它把 Rust 侧状态镜像到 UI。
pub const EVENT_MODE_CHANGED: &str = "pad://mode";

/// 面板视图切换事件名。载荷是 `"prompt"` 或 `"settings"`。
///
/// 「设置」是面板里的**另一个视图**，不是另一个窗口：托盘点「设置」时
/// 广播这个事件，前端把内容换成设置表单。曾经这里直接强制展开提示词面板，
/// 结果用户点「设置」看到的还是粘贴词的窗口 —— 入口和行为必须对上。
pub const EVENT_VIEW_CHANGED: &str = "pad://view";

/// 切换面板当前显示的视图。
///
/// 后端必须知道当前视图，才能区分「点面板外部收起」该不该生效：
/// 在设置界面里，用户要去别的应用复制 API key，点外部是**正常工作流**，
/// 把面板收起来等于把用户正在填的表单弄丢。因此设置视图不因失焦而收起。
#[tauri::command]
pub fn set_panel_view<R: Runtime>(
    app: AppHandle<R>,
    request: crate::models::PanelViewRequest,
) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    *lock(&state.view)? = request.view;
    Ok(())
}

/// 记下当前视图（供托盘等后端发起的切换同步到状态里）。
pub fn store_view<R: Runtime>(app: &AppHandle<R>, view: crate::models::PadView) {
    let state = app.state::<AppState>();
    let mut slot = match state.view.lock() {
        Ok(slot) => slot,
        Err(_) => return,
    };
    *slot = view;
}

/// 展开 / 收起动画时长（毫秒）。
const PANEL_ANIMATION_MS: u16 = 200;

/// 面板形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelMode {
    Collapsed,
    Expanded,
}

/// 切换动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelAction {
    Toggle,
    Expand,
    Collapse,
}

/// 展开 / 收起面板。
#[tauri::command]
pub fn toggle_panel<R: Runtime>(
    app: AppHandle<R>,
    mode: Option<PanelAction>,
) -> Result<TogglePanelResponse, AppError> {
    let state = app.state::<AppState>();
    let current = *lock(&state.mode)?;

    let target = match mode.unwrap_or(PanelAction::Toggle) {
        PanelAction::Toggle => {
            if current == PanelMode::Expanded {
                PanelMode::Collapsed
            } else {
                PanelMode::Expanded
            }
        }
        PanelAction::Expand => PanelMode::Expanded,
        PanelAction::Collapse => PanelMode::Collapsed,
    };

    apply_inner(&app, target, true)?;

    let anchor = lock(&state.anchor)?.clone();
    let expanded = *lock(&state.mode)? == PanelMode::Expanded;
    Ok(TogglePanelResponse {
        expanded,
        animation_ms: PANEL_ANIMATION_MS,
        position: WindowAnchor {
            monitor_index: anchor.monitor_index,
            x_ratio: anchor.x_pct,
            y_ratio: anchor.y_pct,
        },
    })
}

/// 应用目标形态：调整窗口、落盘状态、广播事件。
pub fn apply<R: Runtime>(app: &AppHandle<R>, target: PanelMode) -> Result<(), AppError> {
    apply_inner(app, target, true)
}

fn apply_inner<R: Runtime>(
    app: &AppHandle<R>,
    target: PanelMode,
    restore_focus: bool,
) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    let config = lock(&state.config)?.clone();
    let anchor = lock(&state.anchor)?.clone();
    let window = window_ctl::pad_window(app)?;

    if target == PanelMode::Expanded && fullscreen::is_exclusive_fullscreen() {
        eprintln!("[panel] skip expand: exclusive fullscreen");
        return Ok(());
    }

    let previous = {
        let mut mode = lock(&state.mode)?;
        let previous = *mode;
        if previous == target {
            return Ok(());
        }
        *mode = target;
        previous
    };
    eprintln!("[panel] {previous:?} -> {target:?}");

    if target == PanelMode::Expanded {
        if let Ok(mut slot) = state.expanding_until.lock() {
            *slot = Some(
                std::time::Instant::now()
                    + std::time::Duration::from_millis(window_ctl::FOCUS_GRACE_MS),
            );
        }
        if let Ok(mut slot) = state.focused_at.lock() {
            *slot = Some(std::time::Instant::now());
        }
    }

    let outcome = {
        let mut prev_focus = lock(&state.prev_focus)?;
        match target {
            PanelMode::Expanded => window_ctl::expand(&window, &config, &mut prev_focus),
            PanelMode::Collapsed => {
                window_ctl::collapse(&window, &config, &anchor, &mut prev_focus, restore_focus)
            }
        }
    };
    if let Err(err) = outcome {
        if let Ok(mut mode) = lock(&state.mode) {
            *mode = previous;
        }
        if target == PanelMode::Expanded {
            let mut prev_focus = lock(&state.prev_focus).ok();
            if let Some(ref mut slot) = prev_focus {
                let _ = window_ctl::collapse(&window, &config, &anchor, slot, false);
            }
        }
        return Err(err);
    }

    app.emit(EVENT_MODE_CHANGED, target)?;
    if target == PanelMode::Collapsed {
        store_view(app, crate::models::PadView::Prompt);
        app.emit(EVENT_VIEW_CHANGED, "prompt")?;
    }
    Ok(())
}

/// 供窗口事件回调使用：点外部收起，不把焦点拽回展开前的窗口。
pub fn collapse<R: Runtime>(app: &AppHandle<R>) {
    if let Err(err) = apply_inner(app, PanelMode::Collapsed, false) {
        eprintln!("[panel] 收起失败: {err}");
    }
}
