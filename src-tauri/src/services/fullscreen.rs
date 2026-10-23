//! 独占全屏检测与 WS_EX_TOPMOST。
//!
//! 只在 Windows 上生效；其他平台返回「非全屏」，保证本期只做 Windows 的前提下
//! 代码仍可编译运行（macOS 扩展时再补对应实现）。

use tauri::{Runtime, WebviewWindow};

/// 窗口当前是否处于「独占全屏」。
///
/// 取前台窗口所在显示器的监视器矩形（不是主屏 SM_CXSCREEN），
/// 并排除 `SW_SHOWMAXIMIZED` 的最大化窗口（任务栏自动隐藏时最大化会铺满屏幕）。
#[cfg(windows)]
pub fn is_exclusive_fullscreen() -> bool {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowPlacement, GetWindowRect, SW_SHOWMAXIMIZED, WINDOWPLACEMENT,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return false;
        }

        let mut placement = std::mem::zeroed::<WINDOWPLACEMENT>();
        placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
        if GetWindowPlacement(hwnd, &mut placement) != 0
            && placement.showCmd == SW_SHOWMAXIMIZED as u32
        {
            return false;
        }

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return false;
        }

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return false;
        }
        let mut info = std::mem::zeroed::<MONITORINFO>();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return false;
        }
        let screen = info.rcMonitor;
        rect.left <= screen.left
            && rect.top <= screen.top
            && rect.right >= screen.right
            && rect.bottom >= screen.bottom
    }
}

/// 非 Windows 平台恒为「非全屏」。
#[cfg(not(windows))]
pub fn is_exclusive_fullscreen() -> bool {
    false
}

/// 强制把窗口提到 TOPMOST z-order，并写上 `WS_EX_TOPMOST`。
#[cfg(windows)]
pub fn ensure_topmost<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), crate::error::AppError> {
    use windows_sys::Win32::Foundation::{GetLastError, SetLastError, HWND};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_TOPMOST,
    };

    let hwnd = window.hwnd()?.0 as HWND;

    unsafe {
        SetLastError(0);
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if current == 0 {
            let err = GetLastError();
            if err != 0 {
                return Err(
                    crate::error::AppError::storage_failure("读取窗口扩展样式失败").with_details(
                        serde_json::json!({
                            "target": "window_ex_style",
                            "win32_error": err,
                        }),
                    ),
                );
            }
        }

        let desired = current | WS_EX_TOPMOST as isize;
        if desired != current {
            SetLastError(0);
            let previous = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired);
            if previous == 0 {
                let err = GetLastError();
                if err != 0 {
                    return Err(
                        crate::error::AppError::storage_failure("设置窗口扩展样式失败")
                            .with_details(serde_json::json!({
                                "target": "window_ex_style",
                                "win32_error": err,
                            })),
                    );
                }
            }
        }

        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }

    Ok(())
}

/// 非 Windows 平台不做任何事。
#[cfg(not(windows))]
pub fn ensure_topmost<R: Runtime>(
    _window: &WebviewWindow<R>,
) -> Result<(), crate::error::AppError> {
    Ok(())
}

/// 收起态加上 `WS_EX_NOACTIVATE`，点 DOT 不会抢走前台。
#[cfg(windows)]
pub fn set_noactivate<R: Runtime>(
    window: &WebviewWindow<R>,
    enabled: bool,
) -> Result<(), crate::error::AppError> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_NOACTIVATE,
    };

    let hwnd = window.hwnd()?.0 as HWND;
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let desired = if enabled {
            current | WS_EX_NOACTIVATE as isize
        } else {
            current & !(WS_EX_NOACTIVATE as isize)
        };
        if desired != current {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired);
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn set_noactivate<R: Runtime>(
    _window: &WebviewWindow<R>,
    _enabled: bool,
) -> Result<(), crate::error::AppError> {
    Ok(())
}
