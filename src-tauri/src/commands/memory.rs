//! 记忆检索与注入（接入层）。
//!
//! 分层约定（`docs/architecture.md` §1）：本模块只做**参数反序列化与三类边界校验**
//! （空输入 / 类型错误 / 超量超长），业务逻辑一律在服务层 `services::memory.rs`，
//! 持久化访问再由服务层转发给 `storage::db` 与 `storage::markdown`。
//! 接入层不出现任何 SQL、文件路径或记忆策略分支。

use crate::commands::lock;
use crate::error::AppError;
use crate::models::{MemoryInjection, NightlyJobReport};
use crate::services::memory;
use crate::AppState;
use serde::Deserialize;
use serde_json::json;
use tauri::{AppHandle, Manager, Runtime};

/// 检索查询串长度上限。
const MAX_QUERY_CHARS: usize = 2_000;

/// 检索请求。
#[derive(Debug, Deserialize)]
pub struct RetrieveMemoriesInput {
    /// 任务域；阶段二作软优先，阶段一回退时作等值过滤。
    pub domain: Option<String>,
    /// 检索查询串。有有效内容时走 FTS5 BM25；空或清洗后为空则回退 domain 路由。
    pub query: Option<String>,
    /// Top-K 条数上限（分页步长）。
    pub top_k: Option<u32>,
    /// 分页偏移。
    pub offset: Option<u32>,
}

/// 夜间沉淀请求。
#[derive(Debug, Deserialize)]
pub struct RunNightlyJobInput {
    /// 处理日期，`YYYY-MM-DD`；缺省取昨天（已结束的一天）。
    pub run_date: Option<String>,
    /// true 时只做扫描、去重与匹配并返回统计，不写盘。
    pub dry_run: Option<bool>,
}

/// 检索记忆并构造注入集。
#[tauri::command]
pub fn retrieve_memories<R: Runtime>(
    app: AppHandle<R>,
    request: RetrieveMemoriesInput,
) -> Result<MemoryInjection, AppError> {
    let top_k = request.top_k.unwrap_or(memory::DEFAULT_TOP_K);
    if !(1..=memory::MAX_TOP_K).contains(&top_k) {
        return Err(AppError::invalid_type("top_k 超出允许区间").with_details(
            json!({ "field": "top_k", "expected": format!("1..={}", memory::MAX_TOP_K) }),
        ));
    }
    let offset = request.offset.unwrap_or(0);
    if offset > memory::MAX_OFFSET {
        return Err(AppError::invalid_type("offset 超出允许区间").with_details(
            json!({ "field": "offset", "expected": format!("0..={}", memory::MAX_OFFSET) }),
        ));
    }
    if let Some(query) = request.query.as_ref() {
        if query.chars().count() > MAX_QUERY_CHARS {
            return Err(AppError::payload_too_large(format!(
                "检索串超过 {MAX_QUERY_CHARS} 字符上限"
            ))
            .with_details(json!({ "field": "query", "limit": MAX_QUERY_CHARS })));
        }
    }

    let state = app.state::<AppState>();
    let budget = lock(&state.config)?.memory_token_budget;
    let db = lock(&state.db)?;

    memory::retrieve(
        &db,
        budget,
        request.domain.as_deref(),
        request.query.as_deref(),
        top_k,
        offset,
    )
}

/// 运行夜间记忆沉淀流水线。
#[tauri::command]
pub fn run_nightly_job<R: Runtime>(
    app: AppHandle<R>,
    request: RunNightlyJobInput,
) -> Result<NightlyJobReport, AppError> {
    let run_date = resolve_run_date(request.run_date.as_deref())?;
    let dry_run = request.dry_run.unwrap_or(false);

    let state = app.state::<AppState>();
    let budget = lock(&state.config)?.memory_token_budget;
    let db = lock(&state.db)?;

    memory::run_nightly(&db, &state.memory_dir, budget, &run_date, dry_run)
}

/// 校验运行日期：格式必须是 `YYYY-MM-DD`，且不能是未来日期。
fn resolve_run_date(raw: Option<&str>) -> Result<String, AppError> {
    let today = crate::clock::today_date();
    let value = match raw {
        Some(text) => text.trim().to_string(),
        None => crate::clock::yesterday_date(),
    };
    if !is_iso_date(&value) {
        return Err(AppError::invalid_input("运行日期格式必须为 YYYY-MM-DD")
            .with_details(json!({ "field": "run_date", "reason": "malformed_id" })));
    }
    if value > today {
        return Err(AppError::invalid_input("运行日期不能是未来日期")
            .with_details(json!({ "field": "run_date", "reason": "future_date" })));
    }
    Ok(value)
}

/// 判断字符串是否是合法的 `YYYY-MM-DD` 日期。
fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits = [0usize, 1, 2, 3, 5, 6, 8, 9];
    if !digits.iter().all(|&i| bytes[i].is_ascii_digit()) {
        return false;
    }
    let month: u32 = value[5..7].parse().unwrap_or(0);
    let day: u32 = value[8..10].parse().unwrap_or(0);
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 日期格式校验() {
        assert!(is_iso_date("2026-10-21"));
        assert!(!is_iso_date("2026-1-21"));
        assert!(!is_iso_date("2026/10/21"));
        assert!(!is_iso_date("2026-13-01"));
        assert!(!is_iso_date("2026-10-32"));
    }
}
