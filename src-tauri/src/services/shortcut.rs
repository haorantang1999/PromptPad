//! 全局热键绑定。
//!
//! 热键是**配置项**而不是写死的字符串：用户要在「设置」里改键，
//! 改完必须立刻生效，因此「解除旧绑定 + 绑定新键」要能被反复调用。
//!
//! 插件的 handler 在构建期就注册好了，这里只改「键 → 动作」的绑定关系，
//! 所以重新绑定不会丢 handler。

use crate::error::AppError;
use tauri::{AppHandle, Runtime};

/// 绑定全局热键：先解除全部旧绑定，再注册新的那一个。
///
/// 用于冷启动：此时还没有任何绑定，直接注册即可。
pub fn bind<R: Runtime>(app: &AppHandle<R>, shortcut: &str) -> Result<(), AppError> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let global_shortcut = app.global_shortcut();
    global_shortcut.unregister_all().map_err(|err| {
        AppError::provider_unavailable("解除旧的全局热键绑定失败")
            .with_details(serde_json::json!({ "reason": err.to_string() }))
    })?;
    global_shortcut.register(shortcut).map_err(|err| {
        AppError::provider_unavailable(format!("注册全局热键 {shortcut} 失败，可能被其他软件占用"))
            .with_details(serde_json::json!({
                "shortcut": shortcut,
                "reason": err.to_string(),
            }))
    })?;
    Ok(())
}

/// 把热键从 `old` 换成 `new`。
///
/// **先注册新的、成功后再解除旧的**：反过来做的话，一旦新键被别的软件占用，
/// 旧绑定已经被解掉，用户就彻底失去唤起 Pad 的方式了 —— 而这类占用恰恰是
/// 「按了没反应」的主要成因，不能再让他同时失去兜底。
pub fn rebind<R: Runtime>(app: &AppHandle<R>, old: &str, new: &str) -> Result<(), AppError> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    if old == new {
        return Ok(());
    }

    let global_shortcut = app.global_shortcut();
    global_shortcut.register(new).map_err(|err| {
        AppError::provider_unavailable(format!("注册全局热键 {new} 失败，可能被其他软件占用"))
            .with_details(serde_json::json!({
                "shortcut": new,
                "reason": err.to_string(),
            }))
    })?;
    let _ = global_shortcut.unregister(old);
    Ok(())
}
