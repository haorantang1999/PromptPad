//! 大模型适配层。
//!
//! 支持两种公开协议，二者的请求体、鉴权头、响应结构都不同：
//! - **OpenAI chat/completions**：`POST {base}/v1/chat/completions`，
//!   鉴权 `Authorization: Bearer`，响应取 `choices[0].message.content`；
//! - **Anthropic messages**：`POST {base}/v1/messages`，
//!   鉴权 `x-api-key` + `anthropic-version`，system 是顶层字段，
//!   响应取 `content[]` 里的 `text`。
//!
//! 提供方（provider）与协议（protocol）是两件事：同一个 MiniMax 既有 OpenAI 兼容端点，
//! 也有 Anthropic 兼容端点（Token Plan 订阅用的就是后者），Claude Code 这类工具走的
//! 同样是 Anthropic 协议。因此「用哪家」与「怎么说话」分别配置，代码不内置任何一家
//! 服务的地址 —— endpoint 完全由使用者填写，路径补全规则见 `AppConfig::request_url`。
//!
//! 两种协议都同时发 `x-api-key` 与 `Authorization: Bearer`：
//! Anthropic 官方只认前者，MiniMax 的 Token Plan 走的是后者的
//! `ANTHROPIC_AUTH_TOKEN` 模式，多发一个头换来两端都能用。

use crate::config::{AppConfig, Protocol, ANTHROPIC_MAX_TOKENS, ANTHROPIC_VERSION};
use crate::error::AppError;
use crate::models::{injection_tokens, MemoryInjectionItem, OptimizeResponse, MAX_PROMPT_CHARS};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

/// 改写请求。
#[derive(Debug, Clone)]
pub struct OptimizeRequest {
    pub original_prompt: String,
    pub memories: Vec<MemoryInjectionItem>,
}

/// 鉴权方式。
enum Auth<'a> {
    None,
    /// `Authorization: Bearer <key>`。
    Bearer(&'a str),
    /// `x-api-key` + `anthropic-version` + `Authorization: Bearer`。
    Anthropic(&'a str),
}

/// chat/completions 请求体。
#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage>,
    temperature: f32,
    stream: bool,
}

/// 单条消息。
#[derive(Debug, Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

/// chat/completions 响应体。
#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageOut,
}

#[derive(Debug, Deserialize)]
struct ChatMessageOut {
    #[serde(default)]
    content: String,
}

/// Anthropic messages 请求体。
///
/// `system` 是顶层字段而不是一条 message，`max_tokens` 是必填项 ——
/// 少了任何一个，Anthropic 侧都会直接拒绝请求。
#[derive(Debug, Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: String,
    messages: Vec<AnthropicMessage>,
    temperature: f32,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: &'static str,
    content: String,
}

/// Anthropic messages 响应体。
///
/// `content` 用 `Value` 接而不是直接定义成数组：不同厂商的兼容实现有的返回
/// 文本块数组，有的直接返回一段字符串，写死一种会让另一种整个解析失败。
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    #[serde(default)]
    content: Value,
}

/// 调用模型完成一次改写。
pub async fn optimize(
    client: &reqwest::Client,
    config: &AppConfig,
    request: &OptimizeRequest,
) -> Result<OptimizeResponse, AppError> {
    config.validate_for_request()?;

    let prompt = request.original_prompt.trim();
    if prompt.is_empty() {
        return Err(AppError::invalid_input("提示词不能为空")
            .with_details(json!({ "field": "original_prompt", "reason": "empty_input" })));
    }
    if prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(
            AppError::payload_too_large(format!("提示词超过 {MAX_PROMPT_CHARS} 字符上限"))
                .with_details(json!({ "field": "original_prompt", "limit": MAX_PROMPT_CHARS })),
        );
    }

    let started = Instant::now();
    let system_prompt = build_system_prompt(&request.memories);
    let url = config.request_url();
    let api_key = config.api_key();

    let optimized = match config.protocol {
        Protocol::OpenaiChatCompletions => {
            call_openai(
                client,
                config,
                &url,
                api_key.as_deref(),
                &system_prompt,
                prompt,
            )
            .await?
        }
        Protocol::AnthropicMessages => {
            call_anthropic(
                client,
                config,
                &url,
                api_key.as_deref(),
                &system_prompt,
                prompt,
            )
            .await?
        }
    };

    if optimized.is_empty() {
        return Err(
            AppError::provider_unavailable("模型返回了空结果").with_details(json!({
                "provider": config.provider.as_str(),
                "protocol": config.protocol.as_str(),
                "status": "empty_response",
                "retryable": true,
            })),
        );
    }

    Ok(OptimizeResponse {
        optimized_prompt: optimized,
        provider: config.provider.as_str().to_string(),
        protocol: config.protocol.as_str().to_string(),
        model_name: config.model_name.clone(),
        injected_memory_ids: request
            .memories
            .iter()
            .map(|m| m.memory_id.clone())
            .collect(),
        injected_token_count: injection_tokens(&request.memories),
        elapsed_ms: started.elapsed().as_millis() as u64,
        domain: String::new(),
    })
}

/// 走 OpenAI chat/completions 协议。
async fn call_openai(
    client: &reqwest::Client,
    config: &AppConfig,
    url: &str,
    api_key: Option<&str>,
    system_prompt: &str,
    prompt: &str,
) -> Result<String, AppError> {
    let body = ChatRequest {
        model: &config.model_name,
        messages: vec![
            ChatMessage {
                role: "system",
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user",
                content: prompt.to_string(),
            },
        ],
        temperature: 0.3,
        stream: false,
    };

    let auth = match api_key {
        Some(key) => Auth::Bearer(key),
        None => Auth::None,
    };
    let raw = post_json(client, config, url, auth, &body).await?;

    let payload: ChatResponse = serde_json::from_str(&raw).map_err(|err| {
        AppError::provider_unavailable("模型响应不是合法 JSON").with_details(json!({
            "provider": config.provider.as_str(),
            "protocol": config.protocol.as_str(),
            "status": "malformed_response",
            "retryable": true,
            "reason": err.to_string(),
        }))
    })?;

    Ok(payload
        .choices
        .first()
        .map(|choice| choice.message.content.trim().to_string())
        .unwrap_or_default())
}

/// 走 Anthropic messages 协议。
async fn call_anthropic(
    client: &reqwest::Client,
    config: &AppConfig,
    url: &str,
    api_key: Option<&str>,
    system_prompt: &str,
    prompt: &str,
) -> Result<String, AppError> {
    let body = AnthropicRequest {
        model: &config.model_name,
        max_tokens: ANTHROPIC_MAX_TOKENS,
        system: system_prompt.to_string(),
        messages: vec![AnthropicMessage {
            role: "user",
            content: prompt.to_string(),
        }],
        temperature: 0.3,
        stream: false,
    };

    let auth = match api_key {
        Some(key) => Auth::Anthropic(key),
        None => Auth::None,
    };
    let raw = post_json(client, config, url, auth, &body).await?;

    let payload: AnthropicResponse = serde_json::from_str(&raw).map_err(|err| {
        AppError::provider_unavailable("模型响应不是合法 JSON").with_details(json!({
            "provider": config.provider.as_str(),
            "protocol": config.protocol.as_str(),
            "status": "malformed_response",
            "retryable": true,
            "reason": err.to_string(),
        }))
    })?;

    Ok(extract_anthropic_text(&payload.content))
}

/// 从 Anthropic 响应里取出正文。
///
/// 兼容两种形状：文本块数组（标准形态）与单个字符串（部分兼容实现）。
fn extract_anthropic_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.trim().to_string(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string(),
        _ => String::new(),
    }
}

/// 发请求并取回响应体文本。
///
/// 超时、网络失败、非 2xx 三类错误在这里统一映射成契约错误码：
/// 两种协议共用一份，避免两套实现各写一遍导致行为漂移。
async fn post_json(
    client: &reqwest::Client,
    config: &AppConfig,
    url: &str,
    auth: Auth<'_>,
    body: &impl Serialize,
) -> Result<String, AppError> {
    let mut builder = client
        .post(url)
        .timeout(Duration::from_millis(config.optimize_timeout_ms))
        .json(body);

    builder = match auth {
        Auth::None => builder,
        Auth::Bearer(key) => builder.bearer_auth(key),
        Auth::Anthropic(key) => builder
            .header("x-api-key", key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .bearer_auth(key),
    };

    let response = builder.send().await.map_err(|err| {
        if err.is_timeout() {
            return AppError::model_timeout("模型调用超时，请稍后重试或在设置中调大超时时间")
                .with_details(json!({
                    "provider": config.provider.as_str(),
                    "protocol": config.protocol.as_str(),
                    "timeout_ms": config.optimize_timeout_ms,
                    "retryable": true,
                }));
        }
        AppError::provider_unavailable("调用模型服务失败").with_details(json!({
            "provider": config.provider.as_str(),
            "protocol": config.protocol.as_str(),
            "status": "network_error",
            "retryable": true,
            "reason": err.to_string(),
        }))
    })?;

    let status = response.status();
    if !status.is_success() {
        let detail =
            match response.text().await {
                Ok(text) => text,
                Err(err) if err.is_timeout() => {
                    return Err(AppError::model_timeout("读取模型响应超时，请稍后重试")
                        .with_details(json!({
                            "provider": config.provider.as_str(),
                            "protocol": config.protocol.as_str(),
                            "timeout_ms": config.optimize_timeout_ms,
                            "retryable": true,
                        })));
                }
                Err(err) => {
                    return Err(AppError::provider_unavailable("读取模型错误响应失败")
                        .with_details(json!({
                            "provider": config.provider.as_str(),
                            "protocol": config.protocol.as_str(),
                            "status": status.as_u16(),
                            "retryable": false,
                            "reason": err.to_string(),
                        })));
                }
            };
        return Err(map_http_error(
            config,
            status.as_u16(),
            &detail,
            auth_secret(&auth),
        ));
    }

    response.text().await.map_err(|err| {
        if err.is_timeout() {
            return AppError::model_timeout("读取模型响应超时，请稍后重试").with_details(json!({
                "provider": config.provider.as_str(),
                "protocol": config.protocol.as_str(),
                "timeout_ms": config.optimize_timeout_ms,
                "retryable": true,
            }));
        }
        AppError::provider_unavailable("读取模型响应失败").with_details(json!({
            "provider": config.provider.as_str(),
            "protocol": config.protocol.as_str(),
            "status": "malformed_response",
            "retryable": true,
            "reason": err.to_string(),
        }))
    })
}

/// 从鉴权方式里取出明文 key，用于错误信息脱敏。
fn auth_secret<'a>(auth: &'a Auth<'a>) -> Option<&'a str> {
    match auth {
        Auth::None => None,
        Auth::Bearer(key) | Auth::Anthropic(key) => Some(key),
    }
}

/// 把 HTTP 状态码映射成契约错误码。
///
/// 分流规则：
/// - 401 / 403 → `provider_not_configured`：key 填错或没权限，重试多少次都不会成功，
///   必须让用户看到「去检查 API key」，而不是「稍后重试」。
/// - 其它 4xx → `model_unavailable` 且 `retryable: false`（请求本身有问题，重试无意义）。
/// - 5xx / 429 → `model_unavailable` 且 `retryable: true`（服务端或限流，可以重试）。
fn map_http_error(config: &AppConfig, status: u16, detail: &str, secret: Option<&str>) -> AppError {
    let reason = redact_secret(&truncate(detail, 500), secret);
    if status == 401 || status == 403 {
        return AppError::provider_not_configured("模型 API key 无效或无权限，请到设置页检查")
            .with_details(json!({
                "missing": "api_key",
                "provider": config.provider.as_str(),
                "status": status,
                "retryable": false,
                "reason": reason,
            }));
    }
    let retryable = status >= 500 || status == 429;
    AppError::provider_unavailable(format!("模型服务返回 HTTP {status}")).with_details(json!({
        "provider": config.provider.as_str(),
        "protocol": config.protocol.as_str(),
        "status": status,
        "retryable": retryable,
        "reason": reason,
    }))
}

/// 从错误文本里抹掉 API key 明文。
///
/// 部分兼容中转站会把令牌原文回显在错误体里（「无效令牌 sk-xxx」），
/// 而错误详情会一路送到前端与日志 —— 明文不能从这里漏出去。
fn redact_secret(text: &str, secret: Option<&str>) -> String {
    match secret {
        Some(secret) if !secret.is_empty() => text.replace(secret, "***"),
        _ => text.to_string(),
    }
}

/// 构造 system 提示词。
///
/// 无历史经验时明确写「暂无历史经验」，而不是塞入低分噪声 ——
/// 检索结果全量注入比不注入更糟。
fn build_system_prompt(memories: &[MemoryInjectionItem]) -> String {
    let mut prompt = String::from(
        "你是提示词工程助手。把用户给出的原始诉求，改写成一个更清晰、更聚焦、可直接交给 AI 执行的提示词。\n",
    );
    prompt.push_str("硬性要求：\n");
    prompt
        .push_str("1. 只输出改写后的提示词正文，不要解释、不要加引号、不要用 Markdown 代码块。\n");
    prompt.push_str("2. 保留用户的原始意图与全部关键约束，不要凭空添加需求。\n");
    prompt.push_str("3. 使用中文输出。\n");

    if memories.is_empty() {
        prompt.push_str("\n【历史经验】\n暂无历史经验。\n");
    } else {
        prompt.push_str(
            "\n【历史经验】\n以下规则来自你过去使用本 Pad 的真实记录，仅在相关时采用：\n",
        );
        for (index, memory) in memories.iter().enumerate() {
            prompt.push_str(&format!(
                "{}. 规则：{}\n   原因：{}\n   应用方式：{}\n",
                index + 1,
                memory.statement,
                memory.why,
                memory.how_to_apply
            ));
        }
    }

    prompt
}

/// 截断过长文本，避免把整个响应体塞进错误详情。
fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let head: String = text.chars().take(limit).collect();
    format!("{head}…（已截断）")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 无记忆时明确告知暂无历史经验() {
        let prompt = build_system_prompt(&[]);
        assert!(prompt.contains("暂无历史经验"));
    }

    #[test]
    fn 有记忆时只注入三个字段() {
        let memories = vec![MemoryInjectionItem {
            memory_id: "M-UI-001".into(),
            statement: "品牌色单列为硬性约束".into(),
            why: "防止主色被替换".into(),
            how_to_apply: "单开一章硬性品牌约束".into(),
        }];
        let prompt = build_system_prompt(&memories);
        assert!(prompt.contains("品牌色单列为硬性约束"));
        assert!(prompt.contains("防止主色被替换"));
        assert!(prompt.contains("单开一章硬性品牌约束"));
        assert!(!prompt.contains("暂无历史经验"));
    }

    #[test]
    fn 长文本被截断() {
        let long = "x".repeat(1000);
        let truncated = truncate(&long, 10);
        // 截断后的内容是「前 10 字 + 省略号 + （已截断）」，共 16 个字符。
        assert_eq!(truncated, format!("{}…（已截断）", "x".repeat(10)));
        assert_eq!(truncated.chars().count(), 16);
        assert_eq!(truncate("short", 10), "short");
    }

    #[test]
    fn anthropic响应能取出文本块() {
        let value = json!([
            { "type": "text", "text": "第一段" },
            { "type": "text", "text": "第二段" },
        ]);
        assert_eq!(extract_anthropic_text(&value), "第一段\n第二段");
    }

    #[test]
    fn anthropic响应兼容字符串形态() {
        let value = json!("直接一段文本");
        assert_eq!(extract_anthropic_text(&value), "直接一段文本");
        assert_eq!(extract_anthropic_text(&json!(null)), "");
    }

    #[test]
    fn anthropic请求体带必填的max_tokens() {
        let body = AnthropicRequest {
            model: "m",
            max_tokens: ANTHROPIC_MAX_TOKENS,
            system: "s".into(),
            messages: vec![AnthropicMessage {
                role: "user",
                content: "c".into(),
            }],
            temperature: 0.3,
            stream: false,
        };
        let value = serde_json::to_value(&body).unwrap();
        assert_eq!(value["max_tokens"], ANTHROPIC_MAX_TOKENS);
        assert_eq!(value["system"], "s");
        assert_eq!(value["messages"][0]["role"], "user");
    }

    /// 401/403 必须落到「去检查 key」而不是「稍后重试」——
    /// 重试再多次也不会成功，用户会一直等错的指引。
    #[test]
    fn 鉴权失败映射为未配置key() {
        let config = AppConfig::default();
        for status in [401u16, 403] {
            let err = map_http_error(&config, status, "invalid token", None);
            assert_eq!(err.code, "provider_not_configured");
            assert_eq!(err.details["missing"], "api_key");
            assert_eq!(err.details["retryable"], false);
        }
    }

    #[test]
    fn 客户端错误不可重试而服务端错误可重试() {
        let config = AppConfig::default();
        let client_err = map_http_error(&config, 400, "bad request", None);
        assert_eq!(client_err.code, "model_unavailable");
        assert_eq!(client_err.details["retryable"], false);

        let server_err = map_http_error(&config, 500, "boom", None);
        assert_eq!(server_err.details["retryable"], true);

        let throttled = map_http_error(&config, 429, "slow down", None);
        assert_eq!(throttled.details["retryable"], true);
    }

    #[test]
    fn 错误体里的key明文被抹掉() {
        let config = AppConfig::default();
        let err = map_http_error(
            &config,
            401,
            "invalid token sk-cp-secret-value rejected",
            Some("sk-cp-secret-value"),
        );
        let reason = err.details["reason"].as_str().unwrap();
        assert!(!reason.contains("sk-cp-secret-value"));
        assert!(reason.contains("***"));
    }
}
