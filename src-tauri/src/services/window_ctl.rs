//! 窗口控制：DOT 与面板之间的形态切换。
//!
//! 所有坐标计算都委托给 `crate::geometry` 的纯函数，
//! 这里只负责把纯函数的结论翻译成 Tauri 的窗口调用。

use crate::config::AppConfig;
use crate::error::AppError;
use crate::geometry::{
    anchor_from_position, resolve_panel_placement, resolve_restore_position, Anchor, Rect,
};
use crate::services::fullscreen;
use tauri::{AppHandle, LogicalSize, Manager, Monitor, PhysicalPosition, Runtime, WebviewWindow};

/// 主窗口 label，与 tauri.conf.json 中的 windows[0].label 一致。
pub const PAD_WINDOW_LABEL: &str = "pad";

/// DOT 的逻辑边长（像素）。实际物理尺寸随显示器 DPI 缩放。
///
/// 这是**透明容器**的边长，不是可见球体的边长：可见球体由前端画成 44px 并居中，
/// 多出来的 20px 透明余量专门留给辉光（box-shadow）与点击波纹外扩，
/// 否则外扩部分会被窗口边界裁掉。44px 本身落在 AGENTS.md §1 功能② 的
/// 40–56px 区间内，所以「容器 64 / 球体 44」并不违反该约束。
pub const DOT_SIZE: i32 = 64;

/// 展开面板的逻辑尺寸（像素）。
///
/// 窗口本身不可缩放（`resizable:false` 是硬约束）；高度给足是为了
/// 改写结果区能看完整段提示词，而不是被输入框挤成一条缝。
pub const PANEL_WIDTH: i32 = 460;
pub const PANEL_HEIGHT: i32 = 520;

/// 展开后允许多长时间的过渡性失焦（毫秒）。
///
/// 展开时我们先 `set_focusable(true)` 再 `set_focus()`，这期间 Windows 可能补发一次
/// `Focused(false)`；若不加保护，面板刚展开就会被立刻收起。
/// 该窗口必须长于一次焦点往返的耗时，又不能长到让「点外部收起」变得迟钝，
/// 因此取一个明显大于展开动画时长（200ms）的值。
pub const FOCUS_GRACE_MS: u64 = 600;

/// 与 DPI 相关的物理尺寸。
#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub dot_width: i32,
    pub dot_height: i32,
    pub panel_width: i32,
    pub panel_height: i32,
}

/// 取主窗口。
pub fn pad_window<R: Runtime>(app: &AppHandle<R>) -> Result<WebviewWindow<R>, AppError> {
    app.get_webview_window(PAD_WINDOW_LABEL).ok_or_else(|| {
        AppError::storage_failure("找不到主窗口").with_details(format!("label={PAD_WINDOW_LABEL}"))
    })
}

/// 按指定缩放比换算物理尺寸（锚点屏的 DPI，不是窗口当前所在屏）。
pub fn metrics_for_scale(scale: f64) -> Metrics {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };

    Metrics {
        dot_width: (DOT_SIZE as f64 * scale).round() as i32,
        dot_height: (DOT_SIZE as f64 * scale).round() as i32,
        panel_width: (PANEL_WIDTH as f64 * scale).round() as i32,
        panel_height: (PANEL_HEIGHT as f64 * scale).round() as i32,
    }
}

/// 显示器矩形（物理像素）。
fn monitor_rect(monitor: &Monitor) -> Rect {
    Rect::new(
        monitor.position().x,
        monitor.position().y,
        monitor.size().width as i32,
        monitor.size().height as i32,
    )
}

/// 工作区矩形（排除任务栏）。
fn monitor_work_rect(monitor: &Monitor) -> Rect {
    let area = monitor.work_area();
    Rect::new(
        area.position.x,
        area.position.y,
        area.size.width as i32,
        area.size.height as i32,
    )
}

/// 找到锚点对应的显示器序号。
///
/// 先按显示器名称匹配（拔插扩展屏后名称最稳定），
/// 名称对不上再退回序号，最后退回 0（主屏）。
fn resolve_monitor_index(monitors: &[Monitor], anchor: &Anchor) -> usize {
    if let Some(name) = &anchor.monitor_name {
        if let Some(index) = monitors
            .iter()
            .position(|m| m.name().map(|n| n == name).unwrap_or(false))
        {
            return index;
        }
    }
    if anchor.monitor_index < monitors.len() {
        return anchor.monitor_index;
    }
    0
}

/// 找到某个坐标落在哪块显示器上。
fn monitor_index_for_point(monitors: &[Monitor], x: i32, y: i32) -> usize {
    monitors
        .iter()
        .position(|m| {
            let rect = monitor_rect(m);
            x >= rect.x && x < rect.right() && y >= rect.y && y < rect.bottom()
        })
        .unwrap_or(0)
}

/// 把窗口恢复到锚点位置（收起态）。
pub fn restore_position<R: Runtime>(
    window: &WebviewWindow<R>,
    anchor: &Anchor,
) -> Result<(), AppError> {
    let monitors = window.available_monitors()?;
    if monitors.is_empty() {
        return Err(AppError::storage_failure("系统未报告任何显示器"));
    }

    let index = resolve_monitor_index(&monitors, anchor);
    let metrics = metrics_for_scale(monitors[index].scale_factor());
    let rect = monitor_work_rect(&monitors[index]);
    let (x, y) = resolve_restore_position(anchor, &rect, metrics.dot_width, metrics.dot_height);

    window.set_position(PhysicalPosition::new(x, y))?;
    Ok(())
}

/// 记住展开面板之前的前台窗口，收起时还回去。
///
/// 收起态窗口带 `WS_EX_NOACTIVATE`，点它不会抢走前台；但展开面板时我们主动
/// `set_focus()`，前台就变成了 Pad。若不还回去，用户收起后继续打字会全部进 Pad，
/// 原窗口一个字符都收不到 —— 「焦点必须能还给原窗口」是本产品的核心承诺。
#[cfg(windows)]
fn capture_foreground(prev_focus: &mut Option<isize>) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        // 已经是自己的窗口就别记：收起后把焦点还给自己等于没还。
        if pid != std::process::id() {
            *prev_focus = Some(hwnd as isize);
        }
    }
}

#[cfg(not(windows))]
fn capture_foreground(_prev_focus: &mut Option<isize>) {}

/// 把前台焦点还给展开前的那个窗口。
#[cfg(windows)]
fn restore_foreground(prev_focus: Option<isize>) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow;

    let Some(hwnd) = prev_focus else {
        return;
    };
    unsafe {
        SetForegroundWindow(hwnd as HWND);
    }
}

#[cfg(not(windows))]
fn restore_foreground(_prev_focus: Option<isize>) {}

/// 光标当前的位置（物理像素，屏幕坐标）。
#[cfg(windows)]
pub fn cursor_position() -> Result<(i32, i32), AppError> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    unsafe {
        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point) == 0 {
            return Err(AppError::storage_failure("读取光标位置失败"));
        }
        Ok((point.x, point.y))
    }
}

#[cfg(not(windows))]
pub fn cursor_position() -> Result<(i32, i32), AppError> {
    Err(AppError::storage_failure("当前平台不支持读取光标位置"))
}

/// 左键当前是否处于按下状态。
///
/// 用途：判定 Tauri 拖拽何时结束。
/// 拖拽区一旦被按下，Windows 会启动系统拖拽循环并独占鼠标，
/// 浏览器因此收不到 mouseup / click —— 只能从系统层面问「键还按着吗」。
#[cfg(windows)]
pub fn left_button_pressed() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};

    // GetAsyncKeyState 返回 i16，按下时最高位为 1；先转成 u16 再取高位，避免字面量越界。
    unsafe { (GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000) != 0 }
}

#[cfg(not(windows))]
pub fn left_button_pressed() -> bool {
    false
}

/// 光标是否落在窗口矩形内。
///
/// 用途是区分两件长得一样但必须区别对待的事：
/// - 「用户点了窗口外面」→ 该收起面板；
/// - 「Windows 在我们拖拽面板时把面板置为非激活」→ 不该收起。
///
/// 后者发生在拖拽途中鼠标划过别的应用窗口时：Windows 会给被拖窗口发
/// WM_ACTIVATE(WA_INACTIVE)，于是一次「拖标题栏」被误判成「点了外面」，
/// 面板当场缩回圆球（用户可见症状：面板拖不动，一碰就变球）。
///
/// 判据是光标位置：拖拽时鼠标被捕获、始终压在窗口上；
/// 真的点了外面，光标必然落在别的应用那边。
#[cfg(windows)]
pub fn cursor_inside<R: Runtime>(window: &WebviewWindow<R>) -> bool {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};

    let Ok(hwnd) = window.hwnd() else {
        return false;
    };
    unsafe {
        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point) == 0 {
            return false;
        }
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetWindowRect(hwnd.0 as _, &mut rect) == 0 {
            return false;
        }
        point.x >= rect.left && point.x < rect.right && point.y >= rect.top && point.y < rect.bottom
    }
}

#[cfg(not(windows))]
pub fn cursor_inside<R: Runtime>(_window: &WebviewWindow<R>) -> bool {
    false
}

/// 切到收起态（DOT）。
///
/// `restore_focus`：Esc / 关闭 / 热键应收起并还焦点；点外部收起则不要抢回前台。
pub fn collapse<R: Runtime>(
    window: &WebviewWindow<R>,
    config: &AppConfig,
    anchor: &Anchor,
    prev_focus: &mut Option<isize>,
    restore_focus: bool,
) -> Result<(), AppError> {
    // **顺序：先改尺寸，再改位置。**
    // Windows 上 `set_size` 会连带移动窗口（tao 为保持客户区原点会补齐不可见边框），
    // 若先定位后改尺寸，算好的锚点会被尺寸变更顶掉：实测 DOT 会漂到屏幕边缘，
    // 且漂移后的坐标被 Moved 事件写回锚点，造成位置持续漂移。
    // 反过来先 `set_size` 后 `set_position`，最后一次定位才是最终位置。
    window.set_size(LogicalSize::new(DOT_SIZE as f64, DOT_SIZE as f64))?;
    // 收起态必须恢复到锚点位置：展开时窗口被挪到了面板位置。
    restore_position(window, anchor)?;

    // 不抢焦点：点完 Pad 就能直接回到原窗口继续打字。
    window.set_focusable(false)?;
    fullscreen::set_noactivate(window, true)?;
    // 点击穿透：透明区域不拦截鼠标。
    window.set_ignore_cursor_events(config.dot_click_through)?;
    window.set_always_on_top(true)?;
    fullscreen::ensure_topmost(window)?;
    // 必须显式显示：展开时若因独占全屏被 `hide()` 过，收起后窗口仍是隐藏的，
    // 用户会看到「DOT 消失且再也回不来」。
    window.show()?;
    if restore_focus {
        restore_foreground(prev_focus.take());
    } else {
        prev_focus.take();
    }

    Ok(())
}

/// 切到展开态（面板）。
pub fn expand<R: Runtime>(
    window: &WebviewWindow<R>,
    _config: &AppConfig,
    prev_focus: &mut Option<isize>,
) -> Result<(), AppError> {
    capture_foreground(prev_focus);

    let origin = window.outer_position()?;
    let monitors = window.available_monitors()?;
    if monitors.is_empty() {
        return Err(AppError::storage_failure("系统未报告任何显示器"));
    }
    let index = monitor_index_for_point(&monitors, origin.x, origin.y);
    let metrics = metrics_for_scale(monitors[index].scale_factor());
    let screen = monitor_work_rect(&monitors[index]);

    // 以 DOT 当前所在矩形为锚，算出面板该朝哪边展开、落在哪。
    // 位置必须在改尺寸**之前**读取：Windows 上 set_size 会连带移动窗口。
    let dot_rect = Rect::new(origin.x, origin.y, metrics.dot_width, metrics.dot_height);
    let placement = resolve_panel_placement(
        &dot_rect,
        metrics.panel_width,
        metrics.panel_height,
        &screen,
    );

    // **顺序：先改尺寸，再改位置。** 与 collapse 同一个理由：
    // 若先定位后改尺寸，Windows 会把窗口挪动，算好的落位被顶掉，
    // 面板就会跑出屏幕（实测右缘超出工作区 59px），「绝不出屏」直接失效。
    window.set_size(LogicalSize::new(PANEL_WIDTH as f64, PANEL_HEIGHT as f64))?;
    window.set_position(PhysicalPosition::new(placement.x, placement.y))?;

    // 展开态必须关掉点击穿透，否则面板点不动。
    window.set_ignore_cursor_events(false)?;
    window.set_focusable(true)?;
    fullscreen::set_noactivate(window, false)?;
    window.set_always_on_top(true)?;
    fullscreen::ensure_topmost(window)?;

    window.show()?;
    window.set_focus()?;

    Ok(())
}

/// 重新计算并返回当前 DOT 的锚点（用于拖动后持久化）。
pub fn current_anchor<R: Runtime>(window: &WebviewWindow<R>) -> Result<Anchor, AppError> {
    let monitors = window.available_monitors()?;
    if monitors.is_empty() {
        return Err(AppError::storage_failure("系统未报告任何显示器"));
    }

    let origin = window.outer_position()?;
    let index = monitor_index_for_point(&monitors, origin.x, origin.y);
    let metrics = metrics_for_scale(monitors[index].scale_factor());
    let rect = monitor_work_rect(&monitors[index]);

    Ok(anchor_from_position(
        monitors[index].name().cloned(),
        index,
        origin.x,
        origin.y,
        &rect,
        metrics.dot_width,
        metrics.dot_height,
    ))
}
