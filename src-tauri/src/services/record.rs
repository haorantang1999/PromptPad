//! PE 记录落库与记忆权重更新（服务层）。
//!
//! 职责：分配 `PE-YYYY-NNN` 记录 ID、只追加写入 L1 事实层、同步 L3 索引、
//! 记录本次注入的记忆条目，以及按评分驱动权重变化与冷启动豁免。
//!
//! 分层约定（`docs/architecture.md` §1）：本模块是「数据访问必须经服务层」这条
//! 硬约束的落点 —— 持久化一律转发给 `storage::db` 与 `storage::markdown`。
//! 入参边界校验（空输入 / 类型错误 / 超量超长）留在接入层 `commands/record.rs`。

use crate::clock;
use crate::error::AppError;
use crate::models::{FeedbackResult, MemoryStatus, PeRecord, SavePeRecordResponse, WeightDelta};
use crate::storage::db::Db;
use crate::storage::markdown;
use serde_json::json;
use std::path::Path;

/// 冷启动豁免：前 N 次反馈不做权重惩罚，只收集数据。
pub const COLD_START_EXEMPTION: i64 = 10;

/// 域缺省值：调用方没给 domain 时归到通用域。
pub const DEFAULT_DOMAIN: &str = "general";

/// 落库入参。
///
/// 服务层不接受一串散落的位置参数：`rating` 与 `signals` 一旦传错位置，
/// 编译器根本拦不住。打包成结构体后字段名自带含义。
pub struct SavePeRecordPayload {
    pub original_prompt: String,
    pub optimized_prompt: String,
    pub domain: String,
    pub signals: Vec<String>,
    pub rating: Option<i64>,
    pub injected_memory_ids: Vec<String>,
}

/// 把本次改写落库为 `PE-YYYY-NNN` 记录。
///
/// 写入顺序：先追加 L1 Markdown（只追加的真相来源），再写索引层。
/// 反过来写会让索引里出现一条 Markdown 中不存在的记录。
pub fn save_pe_record(
    db: &Db,
    memory_dir: &Path,
    payload: &SavePeRecordPayload,
) -> Result<SavePeRecordResponse, AppError> {
    let year = clock::current_year();
    let markdown_max = markdown::max_pe_seq(
        &markdown::read_to_string(&markdown::record_path(memory_dir)),
        year,
    );
    let record = PeRecord {
        id: db.next_pe_id_after(year, markdown_max)?,
        original_prompt: payload.original_prompt.clone(),
        optimized_prompt: payload.optimized_prompt.clone(),
        domain: payload.domain.clone(),
        signals: payload.signals.clone(),
        rating: payload.rating,
        created_at: clock::now_iso(),
    };

    // L1 只追加、不覆盖：同一 ID 已存在说明并发写撞了序号，必须拒绝而不是静默改写。
    // 索引层先查一遍是为了给出 `existing_created_at`；真正的闸门在 Markdown 追加处，
    // 因为索引可能被重建或清空，而 L1 文件才是唯一真相来源。
    if let Some(existing_created_at) = db.entry_created_at(&record.id)? {
        return Err(
            AppError::duplicate_record("该 PE 记录 ID 已存在").with_details(
                json!({ "record_id": record.id, "existing_created_at": existing_created_at }),
            ),
        );
    }

    let mut written_layers: Vec<String> = Vec::new();
    markdown::append_pe_record(&markdown::record_path(memory_dir), &record)?;
    written_layers.push("l1_markdown".to_string());
    db.upsert_pe_record(&record)?;
    written_layers.push("l3_sqlite".to_string());

    // 记录本次实际注入的记忆条目。
    //
    // `usage_log.session_id` 用 record_id，把「注入 → 评分」串成一条可回溯的链；
    // 这也是 submit_feedback 能定位到被注入条目的唯一依据。
    for entry_id in &payload.injected_memory_ids {
        let Some(uid) = db.uid_of(entry_id)? else {
            // 条目在索引层不存在（理论上不会发生）：跳过而不是让整次落库失败。
            continue;
        };
        db.log_usage(&record.id, uid, "inject", None, 0, None)?;
        db.mark_injected(uid)?;
    }

    Ok(SavePeRecordResponse {
        record_id: record.id,
        created_at: record.created_at,
        written_layers,
    })
}

/// 提交评分，驱动记忆权重变化。
///
/// 权重规则：评分 ≥4 → +1；无明显改善 → 0；变差（评分 ≤2）→ −2。
/// 惩罚大于奖励，因为坏经验的危害大于好经验的收益。
pub fn submit_feedback(
    db: &Db,
    memory_dir: &Path,
    token_budget: u32,
    record_id: &str,
    rating: i64,
    note: Option<&str>,
) -> Result<FeedbackResult, AppError> {
    if !db.entry_exists(record_id)? {
        return Err(AppError::not_found("找不到对应的 PE 记录")
            .with_details(json!({ "record_id": record_id })));
    }

    // 一条改写只允许评一次：换分数不能再叠加一次权重。
    if db.any_score_for_session(record_id)? {
        let existing = db.existing_score_for_session(record_id)?.unwrap_or(rating);
        let applied_weight_deltas = db
            .applied_deltas_for_session(record_id, existing)?
            .into_iter()
            .map(|(entry_id, delta)| WeightDelta {
                memory_id: entry_id.clone(),
                entry_id,
                delta,
                weight_before: 0,
                weight_after: 0,
            })
            .collect();
        return Ok(FeedbackResult {
            record_id: record_id.to_string(),
            rating: existing,
            applied_weight_deltas,
            updated_at: clock::now_iso(),
        });
    }

    let raw_delta = match rating {
        value if value >= 4 => 1i64,
        value if value <= 2 => -2i64,
        _ => 0i64,
    };

    // 冷启动豁免：前 10 次反馈只收集数据，不做惩罚。
    // 计数口径是「已提交过反馈的次数」，由 feedback_session_count 给出。
    let feedback_count = db.feedback_session_count()?;
    let delta = if feedback_count < COLD_START_EXEMPTION && raw_delta < 0 {
        0
    } else {
        raw_delta
    };

    // 评分是**条目级**事件：本次会话注入过的每个条目各写一条 score 行。
    // 这样外键完整、权重更新可逐条审计，无需额外的会话评分表。
    let injected = db.injected_entry_ids_for_session(record_id)?;
    let mut applied_weight_deltas: Vec<WeightDelta> = Vec::new();
    for entry_id in &injected {
        let Some(uid) = db.uid_of(entry_id)? else {
            continue;
        };
        let weight_before = db.weight_of(uid)?;
        db.adjust_weight(uid, delta)?;
        let weight_after = db.weight_of(uid)?;
        db.log_usage(record_id, uid, "score", Some(rating), delta, note)?;
        applied_weight_deltas.push(WeightDelta {
            memory_id: entry_id.clone(),
            entry_id: entry_id.clone(),
            delta,
            weight_before,
            weight_after,
        });
    }

    // 一次都没注入记忆时 usage_log 里不会有 score 行，冷启动豁免的计数就永远不前进。
    // 补一条挂在 PE 记录上的会话锚点行：它只用于标记「这次使用发生过反馈」，
    // 不参与权重计算（PE 记录没有权重语义）。
    if applied_weight_deltas.is_empty() {
        if let Some(uid) = db.uid_of(record_id)? {
            db.log_usage(record_id, uid, "score", Some(rating), 0, note)?;
        }
    }

    sync_weights_to_markdown(db, memory_dir, token_budget, &applied_weight_deltas)?;

    Ok(FeedbackResult {
        record_id: record_id.to_string(),
        rating,
        applied_weight_deltas,
        updated_at: clock::now_iso(),
    })
}

/// Markdown 是真相源：评分后的 weight / last_used / 状态必须写回 memory.md，
/// 否则夜间任务会用旧值覆盖索引，负评记忆复活。
fn sync_weights_to_markdown(
    db: &Db,
    memory_dir: &Path,
    token_budget: u32,
    deltas: &[WeightDelta],
) -> Result<(), AppError> {
    if deltas.is_empty() {
        return Ok(());
    }
    let path = markdown::memory_path(memory_dir);
    let raw = markdown::read_to_string(&path);
    let mut entries = markdown::parse_entries(&raw);
    let now = clock::now_iso();
    for delta in deltas {
        let Some(entry) = entries.iter_mut().find(|entry| entry.id == delta.memory_id) else {
            continue;
        };
        entry.weight = delta.weight_after;
        entry.last_used = Some(now.clone());
        if entry.weight < -1 && entry.status == MemoryStatus::Active {
            entry.status = MemoryStatus::Deprecated;
        }
    }
    markdown::write_memory(&path, &entries, token_budget)?;
    for entry in &entries {
        db.upsert_memory(entry)?;
    }
    Ok(())
}
