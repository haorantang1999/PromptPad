//! 提示词改写。

use crate::commands::lock;
use crate::error::AppError;
use crate::models::{MemoryInjectionItem, OptimizeResponse, MAX_PROMPT_CHARS};
use crate::services::provider::{self, OptimizeRequest};
use crate::AppState;
use serde::Deserialize;
use serde_json::json;
use tauri::{AppHandle, Manager, Runtime};

/// 单次可注入的记忆条目数上限。
pub const MAX_MEMORY_IDS: usize = 20;

/// 改写请求。
#[derive(Debug, Deserialize)]
pub struct OptimizeRequestInput {
    /// 原始诉求文本。
    pub original_prompt: String,
    /// 任务域，用于记忆路由与 L1 记录归档。
    pub domain: Option<String>,
    /// 前端指定的注入记忆 ID；缺省时由后端自行检索（阶段二 BM25，空则回退 domain）。
    pub memory_ids: Option<Vec<String>>,
}

/// 调用当前配置的模型改写提示词。
///
/// 注入的记忆来自 L2 记忆层；`memory.md` 为空时会得到一条明确的「暂无历史经验」，
/// 而不是把噪声塞给模型。
#[tauri::command]
pub async fn optimize_prompt<R: Runtime>(
    app: AppHandle<R>,
    request: OptimizeRequestInput,
) -> Result<OptimizeResponse, AppError> {
    let OptimizeRequestInput {
        original_prompt,
        domain,
        memory_ids,
    } = request;

    let domain = domain
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(crate::services::record::DEFAULT_DOMAIN)
        .to_string();

    let original_prompt = original_prompt.trim();
    if original_prompt.is_empty() {
        return Err(AppError::invalid_input("原始提示词不能为空")
            .with_details(json!({ "field": "original_prompt", "reason": "empty_input" })));
    }
    if original_prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(AppError::payload_too_large(format!(
            "原始提示词超过 {MAX_PROMPT_CHARS} 字符上限"
        ))
        .with_details(json!({ "field": "original_prompt", "limit": MAX_PROMPT_CHARS })));
    }
    if let Some(ids) = memory_ids.as_ref() {
        if ids.len() > MAX_MEMORY_IDS {
            return Err(AppError::payload_too_large(format!(
                "记忆条目数超过 {MAX_MEMORY_IDS} 条上限"
            ))
            .with_details(json!({ "field": "memory_ids", "limit": MAX_MEMORY_IDS })));
        }
    }

    let state = app.state::<AppState>();
    let config = lock(&state.config)?.clone();

    // 注入集：前端指定 ID 时按 ID 取（仍强制 status=active 且 weight>0），
    // 未指定时用原文走阶段二 BM25；无命中再回退 domain 路由。
    let memories: Vec<MemoryInjectionItem> = {
        let db = lock(&state.db)?;
        match memory_ids.as_deref() {
            Some(ids) => {
                crate::services::memory::memories_by_ids(&db, ids, config.memory_token_budget)?
            }
            None => crate::services::memory::memories_for_optimize(
                &db,
                Some(domain.as_str()),
                Some(original_prompt),
                config.memory_token_budget,
            )?,
        }
    };

    let result = provider::optimize(
        &state.http,
        &config,
        &OptimizeRequest {
            original_prompt: original_prompt.to_string(),
            memories: memories.clone(),
        },
    )
    .await?;

    // 注入事件不在这里落库：此时还没有 PE 记录 ID，无法作为 usage_log.session_id。
    // 注入的条目 ID 已随响应返回给前端，由 save_pe_record 统一写入 inject 事件。
    let mut result = result;
    result.domain = domain;
    Ok(result)
}
