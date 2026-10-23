//! 配置读写。

use crate::commands::lock;
use crate::config::{self, AppConfig, ConfigFileStatus, Protocol, Provider};
use crate::error::AppError;
use crate::models::AppConfigView;
use crate::AppState;
use serde::Deserialize;
use serde_json::json;
use tauri::{AppHandle, Manager, Runtime};

/// 配置补丁。未出现的字段保持原值。
#[derive(Debug, Default, Deserialize)]
pub struct ConfigPatchInput {
    pub provider: Option<Provider>,
    pub protocol: Option<Protocol>,
    pub endpoint: Option<String>,
    pub model_name: Option<String>,
    pub embedding_model: Option<String>,
    pub optimize_timeout_ms: Option<u64>,
    /// API key 明文。空字符串表示清除已存的明文。
    pub api_key: Option<String>,
    pub api_key_env: Option<String>,
    pub memory_token_budget: Option<u32>,
    pub dot_click_through: Option<bool>,
    pub global_shortcut: Option<String>,
    pub autostart_enabled: Option<bool>,
}

/// 读取当前配置。文件缺失或损坏时仍返回内存中的默认视图，并带上状态。
#[tauri::command]
pub fn get_config<R: Runtime>(app: AppHandle<R>) -> Result<AppConfigView, AppError> {
    let state = app.state::<AppState>();
    let config = lock(&state.config)?;
    let status = *lock(&state.config_status)?;
    Ok(to_view(&config, status))
}

/// 写入配置补丁并落盘。
#[tauri::command]
pub fn set_config<R: Runtime>(
    app: AppHandle<R>,
    request: ConfigPatchInput,
) -> Result<AppConfigView, AppError> {
    let state = app.state::<AppState>();

    let mut config = lock(&state.config)?.clone();

    if let Some(provider) = request.provider {
        config.provider = provider;
    }
    if let Some(protocol) = request.protocol {
        config.protocol = protocol;
    }
    if let Some(endpoint) = request.endpoint {
        let endpoint = endpoint.trim().to_string();
        // 空串必须当场拒绝：前端用 `string | null` 区分「未改」与「清空」，
        // 空串一旦落盘，配置就废了，且要等到下次改写才报「未配置」。
        if endpoint.is_empty() {
            return Err(AppError::invalid_input("模型服务地址不能为空")
                .with_details(json!({ "field": "endpoint", "reason": "empty_input" })));
        }
        config.endpoint = endpoint;
    }
    if let Some(model_name) = request.model_name {
        let model_name = model_name.trim().to_string();
        if model_name.is_empty() {
            return Err(AppError::invalid_input("模型名不能为空")
                .with_details(json!({ "field": "model_name", "reason": "empty_input" })));
        }
        config.model_name = model_name;
    }
    if let Some(embedding_model) = request.embedding_model {
        let embedding_model = embedding_model.trim().to_string();
        config::validate_embedding_model(&embedding_model)?;
        config.embedding_model = embedding_model;
    }
    if let Some(api_key) = request.api_key {
        // 空串是合法的「清除明文」，不能当成「不修改」。
        config.api_key = api_key.trim().to_string();
    }
    if let Some(api_key_env) = request.api_key_env {
        let api_key_env = api_key_env.trim().to_string();
        if api_key_env.is_empty() {
            config.api_key_env = config::AppConfig::default().api_key_env;
        } else {
            // 这里必须挡住「把 key 贴进变量名」：那种值永远查不到环境变量，
            // 用户看到的现象是「配了 key 但提示不存在」，且明文还躺在变量名字段里。
            if !config::is_env_var_name(&api_key_env) {
                return Err(AppError::invalid_input(
                    "API key 环境变量名只能包含字母、数字和下划线，且不能以数字开头",
                )
                .with_details(json!({
                    "field": "api_key_env",
                    "reason": "malformed_env_name",
                })));
            }
            config.api_key_env = api_key_env;
        }
    }
    if let Some(timeout) = request.optimize_timeout_ms {
        let (min, max) = config::OPTIMIZE_TIMEOUT_RANGE;
        if timeout < min || timeout > max {
            return Err(AppError::invalid_type("改写调用超时超出允许区间").with_details(
                json!({ "field": "optimize_timeout_ms", "expected": format!("{min}..={max}"), "actual": timeout }),
            ));
        }
        config.optimize_timeout_ms = timeout;
    }
    if let Some(budget) = request.memory_token_budget {
        if budget == 0 || budget > config::MEMORY_TOKEN_BUDGET_MAX {
            return Err(AppError::invalid_type("记忆 token 预算超出允许区间").with_details(
                json!({ "field": "memory_token_budget", "expected": format!("1..={}", config::MEMORY_TOKEN_BUDGET_MAX), "actual": budget }),
            ));
        }
        config.memory_token_budget = budget;
    }
    if let Some(click_through) = request.dot_click_through {
        config.dot_click_through = click_through;
    }
    if let Some(shortcut) = request.global_shortcut {
        let shortcut = shortcut.trim().to_string();
        if !config::is_valid_shortcut(&shortcut) {
            return Err(
                AppError::invalid_input("全局热键格式不正确，应形如 Ctrl+Q").with_details(
                    json!({ "field": "global_shortcut", "reason": "malformed_shortcut" }),
                ),
            );
        }
        // 先换绑定再落盘：新键若被别的软件占用，注册会失败并把原因带回用户，
        // 此时配置还没写，旧的键仍然可用。
        if shortcut != config.global_shortcut {
            crate::services::shortcut::rebind(&app, &config.global_shortcut, &shortcut)?;
            config.global_shortcut = shortcut;
        }
    }
    if let Some(enabled) = request.autostart_enabled {
        crate::services::autostart::apply(&app, enabled)?;
        config.autostart_enabled = enabled;
    }

    config::save(&config, &state.config_path)?;
    *lock(&state.config)? = config.clone();
    *lock(&state.config_status)? = ConfigFileStatus::Ok;

    // 点击穿透只在收起态生效；配置改了立刻套到当前窗口。
    if let Ok(window) = crate::services::window_ctl::pad_window(&app) {
        let collapsed = lock(&state.mode)
            .map(|mode| *mode == crate::commands::panel::PanelMode::Collapsed)
            .unwrap_or(false);
        if collapsed {
            let _ = window.set_ignore_cursor_events(config.dot_click_through);
        }
    }

    Ok(to_view(&config, ConfigFileStatus::Ok))
}

/// 内部持久化结构 → 对外视图。
///
/// API key 明文永远不进这里，只回 `api_key_present` 指示位。
fn to_view(config: &AppConfig, status: ConfigFileStatus) -> AppConfigView {
    AppConfigView {
        provider: config.provider,
        protocol: config.protocol,
        endpoint: config.endpoint.clone(),
        model_name: config.model_name.clone(),
        embedding_model: config.embedding_model.clone(),
        optimize_timeout_ms: config.optimize_timeout_ms,
        api_key_present: config.api_key_present(),
        api_key_env: config.api_key_env.clone(),
        memory_token_budget: config.memory_token_budget,
        dot_click_through: config.dot_click_through,
        global_shortcut: config.global_shortcut.clone(),
        autostart_enabled: config.autostart_enabled,
        config_status: status.as_str().to_string(),
        needs_setup: config.needs_setup(),
    }
}
