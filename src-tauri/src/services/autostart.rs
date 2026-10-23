//! 开机自启。
//!
//! 插件只负责注册能力；真正的 enable/disable 必须跟 `AppConfig.autostart_enabled`
//! 走，否则功能 ⑥ 在设置里是空的。

use crate::error::AppError;
use serde_json::json;
use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::ManagerExt;

/// 按配置打开或关闭开机自启。
pub fn apply<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> Result<(), AppError> {
    let launcher = app.autolaunch();
    let outcome = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    outcome.map_err(|err| {
        AppError::storage_failure("写入开机自启失败").with_details(json!({
            "target": "autostart",
            "retryable": true,
            "reason": err.to_string(),
        }))
    })
}
