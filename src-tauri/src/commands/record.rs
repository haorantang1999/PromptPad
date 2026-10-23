//! PE 记录落库与用户反馈（接入层）。
//!
//! 分层约定（`docs/architecture.md` §1）：本模块只做**参数反序列化与三类边界校验**
//! （空输入 / 类型错误 / 超量超长），业务逻辑一律在服务层 `services/record.rs`，
//! 持久化访问再由服务层转发给 `storage::db` 与 `storage::markdown`。
//! 接入层不出现任何 SQL、文件路径或权重策略分支。

use crate::commands::lock;
use crate::error::AppError;
use crate::models::{FeedbackResult, SavePeRecordResponse, MAX_PROMPT_CHARS};
use crate::services::record;
use crate::AppState;
use serde::Deserialize;
use serde_json::json;
use tauri::{AppHandle, Manager, Runtime};

/// 单条记录最多携带的 signals 条数。
const MAX_SIGNALS: usize = 32;

/// 反馈备注长度上限。
const MAX_NOTE_CHARS: usize = 500;

/// 单次落库最多携带的注入条目数。
const MAX_INJECTED_IDS: usize = 20;

/// 落库请求。
#[derive(Debug, Deserialize)]
pub struct SavePeRecordInput {
    pub original_prompt: String,
    pub optimized_prompt: String,
    pub domain: Option<String>,
    /// 本次改写暴露的信号标签，供夜间任务去重合并与相似度匹配使用。
    pub signals: Option<Vec<String>>,
    /// 可选初始评分；未评分时留空，后续由 submit_feedback 回填。
    pub rating: Option<i64>,
    /// 本次改写实际注入的记忆条目 ID（来自 optimize_prompt 响应）。
    ///
    /// 后端据此写入 usage_log 的 `inject` 事件并累加 hits，
    /// 使后续 `submit_feedback` 能按 `record_id` 反查回被注入的条目。
    pub injected_memory_ids: Option<Vec<String>>,
}

/// 把本次改写落库为 `PE-YYYY-NNN` 记录。
#[tauri::command]
pub fn save_pe_record<R: Runtime>(
    app: AppHandle<R>,
    request: SavePeRecordInput,
) -> Result<SavePeRecordResponse, AppError> {
    let original = request.original_prompt.trim();
    let optimized = request.optimized_prompt.trim();

    if original.is_empty() {
        return Err(AppError::invalid_input("原始提示词不能为空")
            .with_details(json!({ "field": "original_prompt", "reason": "empty_input" })));
    }
    if optimized.is_empty() {
        return Err(AppError::invalid_input("改写结果不能为空")
            .with_details(json!({ "field": "optimized_prompt", "reason": "empty_input" })));
    }
    if original.chars().count() > MAX_PROMPT_CHARS {
        return Err(AppError::payload_too_large(format!(
            "原始提示词超过 {MAX_PROMPT_CHARS} 字符上限"
        ))
        .with_details(json!({ "field": "original_prompt", "limit": MAX_PROMPT_CHARS })));
    }
    if optimized.chars().count() > MAX_PROMPT_CHARS {
        return Err(AppError::payload_too_large(format!(
            "改写结果超过 {MAX_PROMPT_CHARS} 字符上限"
        ))
        .with_details(json!({ "field": "optimized_prompt", "limit": MAX_PROMPT_CHARS })));
    }

    let domain = request
        .domain
        .clone()
        .unwrap_or_else(|| record::DEFAULT_DOMAIN.to_string())
        .trim()
        .to_string();
    let domain = if domain.is_empty() {
        record::DEFAULT_DOMAIN.to_string()
    } else {
        domain
    };

    let mut signals = request.signals.unwrap_or_default();
    if signals.iter().all(|item| item.trim().is_empty()) {
        signals = vec![format!("S-{}", domain.replace('-', "").to_uppercase())];
    }
    if signals.len() > MAX_SIGNALS {
        return Err(
            AppError::payload_too_large(format!("signals 条数超过 {MAX_SIGNALS} 条上限"))
                .with_details(json!({ "field": "signals", "limit": MAX_SIGNALS })),
        );
    }
    let injected_memory_ids = request.injected_memory_ids.unwrap_or_default();
    if injected_memory_ids.len() > MAX_INJECTED_IDS {
        return Err(AppError::payload_too_large(format!(
            "injected_memory_ids 条数超过 {MAX_INJECTED_IDS} 条上限"
        ))
        .with_details(json!({ "field": "injected_memory_ids", "limit": MAX_INJECTED_IDS })));
    }
    if let Some(rating) = request.rating {
        if !(1..=5).contains(&rating) {
            return Err(
                AppError::invalid_type("评分必须在 1 到 5 之间").with_details(
                    json!({ "field": "rating", "expected": "1..=5", "actual": rating }),
                ),
            );
        }
    }

    let state = app.state::<AppState>();
    let db = lock(&state.db)?;

    record::save_pe_record(
        &db,
        &state.memory_dir,
        &record::SavePeRecordPayload {
            original_prompt: original.to_string(),
            optimized_prompt: optimized.to_string(),
            domain,
            signals,
            rating: request.rating,
            injected_memory_ids,
        },
    )
}

/// 反馈请求。
#[derive(Debug, Deserialize)]
pub struct FeedbackInput {
    pub record_id: String,
    /// 用户评分 1–5。
    pub rating: i64,
    /// 可选备注，仅落 usage_log 供人工复核，不注入模型上下文。
    pub note: Option<String>,
}

/// 提交评分，驱动记忆权重变化。
#[tauri::command]
pub fn submit_feedback<R: Runtime>(
    app: AppHandle<R>,
    request: FeedbackInput,
) -> Result<FeedbackResult, AppError> {
    let record_id = request.record_id.trim().to_string();
    if !is_pe_record_id(&record_id) {
        return Err(AppError::invalid_input("record_id 必须是 PE-YYYY-NNN 格式")
            .with_details(json!({ "field": "record_id", "reason": "malformed_id" })));
    }
    if !(1..=5).contains(&request.rating) {
        return Err(
            AppError::invalid_type("评分必须在 1 到 5 之间").with_details(
                json!({ "field": "rating", "expected": "1..=5", "actual": request.rating }),
            ),
        );
    }
    if let Some(note) = request.note.as_ref() {
        if note.chars().count() > MAX_NOTE_CHARS {
            return Err(
                AppError::payload_too_large(format!("备注超过 {MAX_NOTE_CHARS} 字符上限"))
                    .with_details(json!({ "field": "note", "limit": MAX_NOTE_CHARS })),
            );
        }
    }

    let state = app.state::<AppState>();
    let db = lock(&state.db)?;

    let budget = lock(&state.config)?.memory_token_budget;
    record::submit_feedback(
        &db,
        &state.memory_dir,
        budget,
        &record_id,
        request.rating,
        request.note.as_deref(),
    )
}

/// PE 记录 ID 格式校验：`PE-YYYY-NNN`（序号至少三位）。
fn is_pe_record_id(value: &str) -> bool {
    if !value.is_ascii() {
        return false;
    }
    let bytes = value.as_bytes();
    if bytes.len() < 9 || !value.starts_with("PE-") || bytes[7] != b'-' {
        return false;
    }
    let year = &value[3..7];
    let sequence = &value[8..];
    !year.is_empty()
        && year.bytes().all(|b| b.is_ascii_digit())
        && sequence.len() >= 3
        && sequence.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pe_记录_id_格式校验() {
        assert!(is_pe_record_id("PE-2026-042"));
        assert!(is_pe_record_id("PE-2026-1000"));
        assert!(!is_pe_record_id("PE-26-042"));
        assert!(!is_pe_record_id("PE-2026-42"));
        assert!(!is_pe_record_id("XX-2026-042"));
        assert!(!is_pe_record_id(""));
    }
}
