//! 记忆检索与注入（服务层）。
//!
//! 职责：按 FTS5 BM25（阶段二）或 domain 路由（阶段一回退）取出可注入的记忆、
//! 裁剪到 token 预算、组装注入集，以及夜间沉淀流水线的入口。
//!
//! 分层约定（`docs/architecture.md` §1）：本模块是「数据访问必须经服务层」这条
//! 硬约束的落点 —— 持久化一律转发给 `storage::db` 与 `storage::markdown`，
//! 本模块不直接写 SQL、也不直接打开 `memory/*.md`。
//! 入参边界校验（空输入 / 类型错误 / 超量超长）留在接入层 `commands/memory.rs`。

use crate::error::AppError;
use crate::models::{
    injection_item_tokens, MemoryInjection, MemoryInjectionItem, NightlyJobReport,
};
use crate::services::nightly;
use crate::storage::db::{self, Db};
use std::path::Path;

/// 阶段一：纯 domain 路由（BM25 无命中或无有效 query 时的回退）。
pub const STAGE_ONE: &str = "markdown_domain";

/// 阶段二：FTS5 trigram + BM25。
pub const STAGE_TWO: &str = "fts5_bm25";

/// 默认 Top-K。
pub const DEFAULT_TOP_K: u32 = 5;

/// Top-K 上限。
pub const MAX_TOP_K: u32 = 20;

/// 分页偏移上限。
pub const MAX_OFFSET: u32 = 1_000;

/// 命中扫描上限。
///
/// 给足余量，好让 `total_matched_count` 反映过滤后的真实命中总数，
/// 而不是被 LIMIT 截断后的数字。
pub const MATCH_SCAN_LIMIT: u32 = 10_000;

/// 检索记忆并构造注入集。
///
/// 硬性规则：
/// - 只注入 `statement` + `why` + `how_to_apply`，不含 evidence 与统计数据；
/// - 总量 ≤ memory_token_budget，超预算按 weight 从低到高裁剪；
/// - `candidate` 永不参与注入（由 SQL 层保证）。
///
/// 检索策略：有可清洗的 `query` 时走阶段二 BM25；无命中或无 query 时回退阶段一 domain 路由。
pub fn retrieve(
    db: &Db,
    budget: u32,
    domain: Option<&str>,
    query: Option<&str>,
    top_k: u32,
    offset: u32,
) -> Result<MemoryInjection, AppError> {
    let (candidates, stage) = matched_items_with_stage(db, domain, query)?;
    let total_matched_count = candidates.len() as u32;

    // 分页切片：注入场景恒用 offset = 0，保证取到的是全局 Top-K 前缀。
    let start = (offset as usize).min(candidates.len());
    let end = (start + top_k as usize).min(candidates.len());
    // has_more 按**分页切片**判定，而不是按预算裁剪后的条数判定：
    // 预算裁剪只会让本页变短、不会产生新的一页，用裁剪后的条数算会把最后一页误报成「还有更多」。
    let has_more = end < candidates.len();
    let mut injection_set = candidates[start..end].to_vec();
    let total_token_count = trim_to_budget(&mut injection_set, budget);

    let returned_count = injection_set.len() as u32;
    let matched = returned_count > 0;

    Ok(MemoryInjection {
        matched,
        stage: stage.to_string(),
        no_memory_reason: if matched {
            None
        } else {
            Some(NO_MEMORY_REASON.to_string())
        },
        no_memory_message: if matched {
            None
        } else {
            Some(NO_MEMORY_MESSAGE.to_string())
        },
        injection_set,
        returned_count,
        total_matched_count,
        offset,
        top_k,
        has_more,
        total_token_count,
        token_budget: budget,
    })
}

/// 运行夜间记忆沉淀流水线。
pub fn run_nightly(
    db: &Db,
    memory_dir: &Path,
    budget: u32,
    run_date: &str,
    dry_run: bool,
) -> Result<NightlyJobReport, AppError> {
    nightly::run(db, memory_dir, budget, run_date, dry_run)
}

/// 命中条目：优先 BM25，空则 domain 路由。
///
/// `candidate` 与 `deprecated` 在 SQL 层就被过滤掉了，
/// 它们既不参与注入也不占 token 预算。
pub fn matched_items(
    db: &Db,
    domain: Option<&str>,
    query: Option<&str>,
) -> Result<Vec<MemoryInjectionItem>, AppError> {
    let (items, _) = matched_items_with_stage(db, domain, query)?;
    Ok(items)
}

fn matched_items_with_stage(
    db: &Db,
    domain: Option<&str>,
    query: Option<&str>,
) -> Result<(Vec<MemoryInjectionItem>, &'static str), AppError> {
    if let Some(raw) = query.map(str::trim).filter(|value| !value.is_empty()) {
        if db::sanitize_fts_query(raw).is_some() {
            let entries = db.injectable_by_bm25(raw, domain, MATCH_SCAN_LIMIT)?;
            if !entries.is_empty() {
                return Ok((
                    entries.into_iter().map(to_injection_item).collect(),
                    STAGE_TWO,
                ));
            }
        }
    }

    let entries = db.injectable_memories(domain, MATCH_SCAN_LIMIT)?;
    Ok((
        entries.into_iter().map(to_injection_item).collect(),
        STAGE_ONE,
    ))
}

/// 按 query（可选）+ domain 取注入集，并按 token 预算裁剪。
///
/// 改写主路径用：有 `original_prompt` 时走 BM25，否则 / 无命中时回退 domain。
pub fn memories_for_optimize(
    db: &Db,
    domain: Option<&str>,
    query: Option<&str>,
    budget: u32,
) -> Result<Vec<MemoryInjectionItem>, AppError> {
    let mut items = matched_items(db, domain, query)?;
    trim_to_budget(&mut items, budget);
    Ok(items)
}

/// 按 domain 路由取注入集，并按 token 预算裁剪。
#[allow(dead_code)] // 保留给只做域路由的调用方；改写主路径已改走 memories_for_optimize。
pub fn memories_by_domain(
    db: &Db,
    domain: Option<&str>,
    budget: u32,
) -> Result<Vec<MemoryInjectionItem>, AppError> {
    memories_for_optimize(db, domain, None, budget)
}

/// 按指定 ID 取注入集。
///
/// 即使调用方点名了某条记忆，后端仍然强制 `status=active` 且 `weight>0`：
/// 过滤条件由后端说了算，客户端无法放宽（契约 §4.3）。
pub fn memories_by_ids(
    db: &Db,
    ids: &[String],
    budget: u32,
) -> Result<Vec<MemoryInjectionItem>, AppError> {
    let candidates = db.injectable_memories(None, MATCH_SCAN_LIMIT)?;
    let mut items: Vec<MemoryInjectionItem> = candidates
        .into_iter()
        .filter(|entry| ids.iter().any(|id| id == &entry.id))
        .map(to_injection_item)
        .collect();
    trim_to_budget(&mut items, budget);
    Ok(items)
}

/// 按 token 预算裁剪注入集，返回裁剪后的实际占用。
///
/// 候选已按检索排序（BM25 或 weight 降序），逐条累加、超预算就停，
/// 等价于「按当前排序从末尾裁剪」。
pub fn trim_to_budget(items: &mut Vec<MemoryInjectionItem>, budget: u32) -> u32 {
    let mut used = 0u32;
    let mut kept: Vec<MemoryInjectionItem> = Vec::with_capacity(items.len());
    for item in items.drain(..) {
        let cost = injection_item_tokens(&item);
        if used.saturating_add(cost) > budget {
            break;
        }
        used += cost;
        kept.push(item);
    }
    *items = kept;
    used
}

/// L2 条目 → 注入条目：只保留 statement / why / how_to_apply。
fn to_injection_item(entry: crate::models::MemoryEntry) -> MemoryInjectionItem {
    MemoryInjectionItem {
        memory_id: entry.id,
        statement: entry.statement,
        why: entry.why,
        how_to_apply: entry.how_to_apply,
    }
}

/// 无匹配时给模型的明确说明。
///
/// 这一步最容易被忽略：检索结果 Top-K 全量注入比不注入更糟，
/// 噪声会稀释强信号。
const NO_MEMORY_MESSAGE: &str = "暂无历史经验";

/// 未命中原因的机器可读标识。
const NO_MEMORY_REASON: &str = "no_matching_memory";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MemoryEntry, MemoryStatus, MemoryType};

    #[test]
    fn 预算裁剪按权重顺序保留() {
        let mut items = vec![
            MemoryInjectionItem {
                memory_id: "M-A-001".into(),
                statement: "一二三四五".into(),
                why: "六七八九十".into(),
                how_to_apply: "十一二三四".into(),
            },
            MemoryInjectionItem {
                memory_id: "M-A-002".into(),
                statement: "甲乙丙丁戊".into(),
                why: "己庚辛壬癸".into(),
                how_to_apply: "子丑寅卯辰".into(),
            },
        ];
        // 每条约 16 token：预算 20 只够留第一条，第二条被裁掉。
        let used = trim_to_budget(&mut items, 20);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].memory_id, "M-A-001");
        assert!(used > 0);
    }

    fn sample_entry(id: &str, statement: &str) -> MemoryEntry {
        MemoryEntry {
            id: id.into(),
            entry_type: MemoryType::Rule,
            domain: "ui-design".into(),
            status: MemoryStatus::Active,
            statement: statement.into(),
            why: "理由".into(),
            how_to_apply: "方式".into(),
            signals: vec![],
            evidence: vec![],
            weight: 2,
            hits: 0,
            last_used: None,
            created: "2026-10-21".into(),
        }
    }

    #[test]
    fn 有query时走阶段二bm25() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_memory(&sample_entry("M-A-001", "品牌色必须单独成章"))
            .unwrap();
        db.rebuild_fts().unwrap();

        let result = retrieve(&db, 800, Some("ui-design"), Some("品牌色"), 5, 0).unwrap();
        assert!(result.matched);
        assert_eq!(result.stage, STAGE_TWO);
        assert_eq!(result.injection_set[0].memory_id, "M-A-001");
    }

    #[test]
    fn 无query或无命中时回退阶段一() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_memory(&sample_entry("M-A-001", "品牌色必须单独成章"))
            .unwrap();
        db.rebuild_fts().unwrap();

        let no_query = retrieve(&db, 800, Some("ui-design"), None, 5, 0).unwrap();
        assert_eq!(no_query.stage, STAGE_ONE);
        assert!(no_query.matched);

        let miss = retrieve(&db, 800, Some("ui-design"), Some("完全无关的天文学术语"), 5, 0)
            .unwrap();
        // BM25 无命中 → 回退 domain，同域仍有条目。
        assert_eq!(miss.stage, STAGE_ONE);
        assert!(miss.matched);
    }
}
