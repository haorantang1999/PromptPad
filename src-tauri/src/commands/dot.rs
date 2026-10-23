//! DOT（收起态圆球）的交互监视。
//!
//! 圆球要同时满足两件事：按住左键能拖走，单击能展开面板。
//! 这两件事在前端**无法同时做到**，原因是 Tauri 的拖拽机制：
//! 元素一旦带 `data-tauri-drag-region`，按下即启动 Windows 的系统拖拽循环，
//! 鼠标被系统独占，浏览器从此收不到 `mouseup`，也不再派发 `click`。
//! 实测把 click 处理器挂在拖拽区上，单击毫无反应。
//!
//! 于是「什么时候松手」「这一下算点击还是算拖拽」只能由后端从系统层面判定：
//! 前端在 `mousedown` 时调用本命令，后端轮询左键状态直到松开。

use crate::commands::panel;
use crate::error::AppError;
use crate::services::window_ctl;
use serde::Serialize;
use tauri::{AppHandle, Runtime};

/// 判定「按下-抬起算不算点击」的位移阈值（物理像素）。
///
/// 小于等于该值算原地点击（展开面板），大于则算拖拽。
/// Windows 的 `SM_CXDRAG` 默认是 4，但圆球又是 Tauri 拖拽区：
/// 系统拖拽一起步就会有几像素抖动，再加 IPC 延迟与手抖，4px 会把
/// 绝大多数「想点开」误判成「只拖了一下」，表现为只能拖、点不开。
/// 16 物理像素约等于 125% DPI 下 13 逻辑像素，仍远小于有意拖动的行程。
pub const CLICK_MOVE_TOLERANCE_PX: i32 = 16;

/// 松手检测的采样间隔（毫秒）。
///
/// 16ms 约等于一帧，既能及时判定松手，又不至于把 CPU 跑满。
const POLL_INTERVAL_MS: u64 = 16;

/// 一次 DOT 交互的结果。
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DotDragOutcome {
    /// 原地按下又抬起：已经展开面板。
    Clicked,
    /// 按下后移动过：圆球已被拖走，不做其他动作。
    Dragged,
}

/// 开始一次 DOT 交互监视，直到左键松开。
///
/// 前端在圆球上收到 `mousedown` 时调用；命令在左键松开时返回，
/// 期间不阻塞 UI 线程（轮询跑在阻塞线程池里）。
#[tauri::command]
pub async fn begin_dot_drag<R: Runtime>(app: AppHandle<R>) -> Result<DotDragOutcome, AppError> {
    tauri::async_runtime::spawn_blocking(move || watch_until_release(app))
        .await
        .map_err(|err| AppError::internal_error("DOT 交互监视失败").with_details(err.to_string()))?
}

/// 轮询到左键松开为止，并据位移判断是点击还是拖拽。
fn watch_until_release<R: Runtime>(app: AppHandle<R>) -> Result<DotDragOutcome, AppError> {
    // 按下瞬间的光标位置。命令是在 mousedown 里发起的，与按下时刻只差一次 IPC。
    let (start_x, start_y) = window_ctl::cursor_position()?;
    // 窗口位移是更稳的「有没有真的拖走」判据：系统拖拽一起步光标相对屏幕会抖，
    // 但窗口没跟上时仍应算点击。
    let start_origin = window_ctl::pad_window(&app)
        .ok()
        .and_then(|window| window.outer_position().ok());

    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > std::time::Duration::from_secs(30) {
            return Ok(DotDragOutcome::Dragged);
        }
        std::thread::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS));

        let (x, y) = window_ctl::cursor_position()?;
        if window_ctl::left_button_pressed() {
            continue;
        }

        let cursor_moved = (x - start_x).abs() > CLICK_MOVE_TOLERANCE_PX
            || (y - start_y).abs() > CLICK_MOVE_TOLERANCE_PX;
        let window_moved = start_origin
            .and_then(|origin| {
                window_ctl::pad_window(&app)
                    .ok()
                    .and_then(|window| window.outer_position().ok())
                    .map(|now| {
                        (now.x - origin.x).abs() > CLICK_MOVE_TOLERANCE_PX
                            || (now.y - origin.y).abs() > CLICK_MOVE_TOLERANCE_PX
                    })
            })
            .unwrap_or(false);

        // 窗口真的挪了才算拖拽；仅光标抖、窗口还在原地 → 仍算点击。
        if window_moved || (cursor_moved && start_origin.is_none()) {
            return Ok(DotDragOutcome::Dragged);
        }

        // 原地点击：展开面板。
        panel::apply(&app, panel::PanelMode::Expanded)?;
        return Ok(DotDragOutcome::Clicked);
    }
}
