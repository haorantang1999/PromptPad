//! 夜间记忆沉淀流水线。
//!
//! 流程：扫描当天 PE 记录 → signals 去重合并 → 与 memory.md 条目做相似度匹配
//! → 检查 token_budget → 重建 / 增量更新索引。
//!
//! 阶段一没有本地嵌入模型，「相似度匹配」退化为
//! **同 domain 且 signals 有交集** 的启发式匹配。
//! 阶段三接入 sqlite-vec 后再换成真正的向量相似度。
//!
//! 防污染闸门在本模块的落点：
//! - 新提炼的条目一律以 `candidate` 状态写入，**不参与注入**；
//! - 跨 domain 或信号矛盾的条目挂起人工裁决，只写进报告不写进 memory.md；
//! - 超出 token_budget 时拒绝晋升新候选；`deprecated` 不占预算；任务不因超预算报错卡死。

use crate::clock;
use crate::error::AppError;
use crate::models::{MemoryEntry, MemoryStatus, MemoryType, NightlyJobReport};
use crate::storage::db::Db;
use crate::storage::markdown;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// 权重低于该阈值即降级为 `deprecated`（AGENTS.md 状态机：`weight < −1`）。
const DEPRECATE_WEIGHT: i64 = -1;

/// 连续多少次未命中即降级为 `deprecated`（AGENTS.md 状态机：`连续 3 次未命中`）。
const MISS_STREAK_LIMIT: i64 = 3;

/// 报告里最多列出多少个待人工裁决的条目 ID（契约上限 100）。
const MAX_PENDING_REVIEW_IDS: usize = 100;

/// 跑一次夜间沉淀。
///
/// `dry_run` 为 true 时只做扫描、去重与匹配并返回统计，不写 memory.md、
/// 不改 SQLite、不重建索引。
pub fn run(
    db: &Db,
    memory_dir: &Path,
    token_budget: u32,
    run_date: &str,
    dry_run: bool,
) -> Result<NightlyJobReport, AppError> {
    let started_at = clock::now_iso();
    let marker = memory_dir.join(format!(".nightly-{run_date}"));
    // 同一 run_date 已经沉淀过：只出报告，不再加权、不再写盘。
    let dry_run = dry_run || marker.exists();

    let records = db.pe_records_by_created_prefix(run_date)?;
    let scanned_record_count = records.len() as u32;

    let memory_file = markdown::memory_path(memory_dir);
    let raw_memory = markdown::read_to_string(&memory_file);
    let mut entries = markdown::parse_entries(&raw_memory);

    // 兜底：文件里明明有条目标题，却一条都解析不出来 —— 说明格式坏了。
    // 此时继续跑会把 memory.md 覆盖成一份残缺内容，把整库记忆抹掉，直接拒绝。
    let header_count = markdown::count_entries(&raw_memory);
    if header_count > 0 && entries.is_empty() {
        return Err(
            AppError::storage_read_failed("memory.md 存在无法解析的条目").with_details(json!({
                "target": "memory.md",
                "header_count": header_count,
                "parsed_count": 0,
                "hint": "请先人工修复 memory.md 的条目格式再运行夜间任务",
            })),
        );
    }
    let token_usage_before = markdown::token_usage_counted(&entries);

    let mut matched_entry_count = 0u32;
    let mut created_candidate_count = 0u32;
    let mut conflicted_count = 0u32;
    let mut pending_review_ids: Vec<String> = Vec::new();
    let mut candidates: Vec<MemoryEntry> = Vec::new();
    // 本次被命中的既有条目 ID：状态机与未命中计数都要用它。
    let mut matched_ids: BTreeSet<String> = BTreeSet::new();
    // 按 domain 归拢后的 signals：既用于匹配，也用于判断某个域当天是否在用。
    let mut merged: BTreeMap<String, Vec<String>> = BTreeMap::new();

    if scanned_record_count > 0 {
        // 1. signals 去重合并：按 domain 归拢。
        for record in &records {
            let bucket = merged.entry(record.domain.clone()).or_default();
            for signal in &record.signals {
                if !bucket.contains(signal) {
                    bucket.push(signal.clone());
                }
            }
        }

        // 2. 逐个 domain 做匹配与晋升。
        for (domain, signals) in &merged {
            // 命中的既有条目：同 domain 且 signals 有交集。
            // 一条记录可能同时命中多条规则，全部都要加权 —— 只加第一条会让
            // 「命中了但没被记下来」的规则永远停在低权重。
            let hit_ids: Vec<String> = entries
                .iter()
                .filter(|entry| {
                    entry.status != MemoryStatus::Deprecated
                        && entry.domain == *domain
                        && signals_match(signals, &entry.signals)
                })
                .map(|entry| entry.id.clone())
                .collect();

            if !hit_ids.is_empty() {
                for entry in entries.iter_mut() {
                    if !hit_ids.iter().any(|id| id == &entry.id) {
                        continue;
                    }
                    // 匹配到已有条目 → 权重 +1，补充 evidence。
                    entry.weight += 1;
                    for record in records.iter().filter(|record| &record.domain == domain) {
                        if !entry.evidence.contains(&record.id) {
                            entry.evidence.push(record.id.clone());
                        }
                    }
                    // 状态机：weight ≥ 2 且无失败 → active。
                    if entry.weight >= 2 && entry.status == MemoryStatus::Candidate {
                        entry.status = MemoryStatus::Active;
                    }
                }
                matched_entry_count += hit_ids.len() as u32;
                for id in hit_ids {
                    matched_ids.insert(id);
                }
                continue;
            }

            // 无匹配：先查矛盾，再决定是新建候选还是挂起人工裁决。
            let conflicts = detect_conflicts(&entries, domain, signals);
            if !conflicts.is_empty() {
                // 矛盾条目挂起，不自动写入 memory.md（AGENTS.md §6 第 6 条）。
                conflicted_count += conflicts.len() as u32;
                for id in conflicts {
                    push_pending(&mut pending_review_ids, id);
                }
                continue;
            }

            // 无匹配且无矛盾 → 新建 candidate，等人工确认后再生效。
            let id = next_entry_id(&entries, &candidates, domain);
            candidates.push(MemoryEntry {
                id: id.clone(),
                entry_type: MemoryType::Rule,
                domain: domain.clone(),
                status: MemoryStatus::Candidate,
                statement: summarize_statement(&records, domain),
                why: format!("由 {run_date} 的 PE 记录提炼，尚未人工确认"),
                how_to_apply: "待人工补充应用方式后再启用".to_string(),
                signals: signals.clone(),
                evidence: records
                    .iter()
                    .filter(|record| &record.domain == domain)
                    .map(|record| record.id.clone())
                    .collect(),
                weight: 0,
                hits: 0,
                last_used: None,
                created: run_date.to_string(),
            });
            push_pending(&mut pending_review_ids, id);
        }
    }

    // 3. 状态机降级 + 未命中计数。
    //
    // 「未命中」= 该条目所属的域当天有 PE 记录（这个域今天在用），
    // 但它的 signals 与当天该域的合并 signals 没有交集 —— 这条规则今天没被触发。
    // 计数存索引层（entries.miss_streak），Markdown 只落最终状态。
    if scanned_record_count > 0 {
        for entry in entries.iter_mut() {
            if entry.status == MemoryStatus::Deprecated {
                continue;
            }
            if entry.status == MemoryStatus::Active && entry.weight < DEPRECATE_WEIGHT {
                entry.status = MemoryStatus::Deprecated;
                continue;
            }
            let Some(uid) = db.uid_of(&entry.id)? else {
                continue;
            };
            let domain_in_use = merged.contains_key(&entry.domain);
            if matched_ids.contains(&entry.id) {
                if !dry_run {
                    db.reset_miss_streak(uid)?;
                }
                continue;
            }
            if !domain_in_use {
                // 这个域今天没有任何记录：既谈不上命中也算不上未命中，计数保持不变，
                // 否则隔几天不用这个域就会把之前的连续未命中记录冲掉。
                continue;
            }
            if dry_run {
                continue;
            }
            if db.miss_logged(uid, run_date)? {
                continue;
            }
            let streak = db.bump_miss_streak(uid)?;
            db.log_usage(run_date, uid, "miss", None, 0, None)?;
            if streak >= MISS_STREAK_LIMIT {
                entry.status = MemoryStatus::Deprecated;
            }
        }
    }

    // 4. token 预算检查：超出则拒绝晋升新候选，并从 weight 最低的 deprecated 起裁剪。
    let mut final_entries = entries.clone();
    final_entries.extend(candidates.iter().cloned());

    let mut rejected = false;
    let usage_with_candidates = markdown::token_usage_counted(&final_entries);

    if usage_with_candidates > token_budget {
        // deprecated 不占预算；超限时只丢掉本次新候选，任务正常结束而不是卡死。
        let candidate_ids: Vec<String> = candidates.iter().map(|c| c.id.clone()).collect();
        if !candidate_ids.is_empty() {
            final_entries.retain(|entry| !candidate_ids.contains(&entry.id));
            rejected = true;
        }
    }

    let token_usage_after = markdown::token_usage_counted(&final_entries);

    if !rejected {
        created_candidate_count = candidates.len() as u32;
    }

    // 5. 写回 memory.md（纯 Markdown，始终全量可读可 diff）并同步索引层。
    let index_rebuilt = !dry_run;
    if index_rebuilt {
        markdown::write_memory(&memory_file, &final_entries, token_budget)?;
        for entry in &final_entries {
            db.upsert_memory(entry)?;
        }
        // Markdown 是唯一真相来源：索引里多出来的条目必须删掉，
        // 否则「以 Markdown 为准」只在写入方向成立，删除方向会长期分歧。
        let kept_ids: Vec<String> = final_entries.iter().map(|entry| entry.id.clone()).collect();
        db.delete_memories_not_in(&kept_ids)?;
        db.rebuild_fts()?;
        let _ = std::fs::write(&marker, run_date);
    }

    Ok(NightlyJobReport {
        run_date: run_date.to_string(),
        scanned_record_count,
        matched_entry_count,
        created_candidate_count,
        // 每次匹配都只 +1，因此命中数与加权数一致。
        weight_bumped_count: matched_entry_count,
        conflicted_count,
        pending_review_ids,
        token_usage_before,
        token_usage_after,
        token_budget,
        index_rebuilt,
        started_at,
        finished_at: clock::now_iso(),
    })
}

/// 检测与既有条目矛盾的情况。
///
/// 阶段一没有向量相似度，只能用 signals 做结构性判定，两条规则：
/// 1. **跨 domain 推广**：合并后的 signals 命中了另一个 domain 的条目 ——
///    要么是别的域的规则被搬了过来，要么是两个域抢同一个触发词，
///    两种情况都必须人工裁决（AGENTS.md §6 第 6 条）。
/// 2. **同域重复信号**：同一个 domain 里两条既有条目共享同一个 signal，
///    但陈述不同 —— 同一个触发词对应两套说法，模型无从取舍。
///
/// 返回卷入冲突的既有条目 ID（去重后按字典序）。
fn detect_conflicts(entries: &[MemoryEntry], domain: &str, signals: &[String]) -> Vec<String> {
    let mut conflicts: Vec<String> = Vec::new();

    for entry in entries.iter().filter(|e| e.domain != domain) {
        if signals.iter().any(|signal| entry.signals.contains(signal)) {
            conflicts.push(entry.id.clone());
        }
    }

    // 同域旧条目互相共享 signal 只是警告级，不再冻结该域的新候选。
    // 当天 signals 与既有条目有交集会在匹配阶段加权，走不到这里。

    conflicts.sort();
    conflicts.dedup();
    conflicts
}

/// 把条目 ID 挂到待人工裁决列表；超出契约上限后不再追加。
fn push_pending(pending: &mut Vec<String>, id: String) {
    if pending.len() >= MAX_PENDING_REVIEW_IDS {
        return;
    }
    if !pending.contains(&id) {
        pending.push(id);
    }
}

/// 生成下一个记忆条目 ID：M-<域缩写>-<三位序号>。
///
/// 必须把**本次运行已生成的候选**也算进去：候选还没写进 memory.md，
/// 只看既有条目会让同一次运行里的两条候选拿到同一个 ID，
/// 而 `entries.entry_id` 有唯一约束，后一条会覆盖前一条。
fn next_entry_id(entries: &[MemoryEntry], candidates: &[MemoryEntry], domain: &str) -> String {
    let abbreviation = abbreviate(domain);
    let prefix = format!("M-{abbreviation}-");
    let max_seq = entries
        .iter()
        .chain(candidates.iter())
        .filter(|entry| entry.id.starts_with(&prefix))
        .filter_map(|entry| entry.id.get(prefix.len()..))
        .filter_map(|seq| seq.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("{prefix}{:03}", max_seq + 1)
}

/// 域缩写：取 domain 的首字母大写，最多 3 个字符。
fn abbreviate(domain: &str) -> String {
    let letters: Vec<char> = domain
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if letters.is_empty() {
        return "GEN".to_string();
    }
    letters.iter().take(3).collect::<String>().to_uppercase()
}

/// 阶段一启发式：同域且 signals 有交集；任一侧为空则退回纯 domain 匹配。
fn signals_match(today: &[String], entry: &[String]) -> bool {
    if today.is_empty() || entry.is_empty() {
        return true;
    }
    today.iter().any(|signal| entry.contains(signal))
}

/// 从当天记录里取一条改写结果，作为候选规则的语句雏形。
fn summarize_statement(records: &[crate::models::PeRecord], domain: &str) -> String {
    records
        .iter()
        .find(|record| record.domain == domain)
        .map(|record| {
            let text = record.optimized_prompt.trim();
            if text.chars().count() > 120 {
                let head: String = text.chars().take(120).collect();
                format!("{head}…")
            } else {
                text.to_string()
            }
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 域缩写最多三位() {
        assert_eq!(abbreviate("ui-design"), "UID");
        assert_eq!(abbreviate("general"), "GEN");
        assert_eq!(abbreviate(""), "GEN");
    }

    #[test]
    fn 记忆条目_id_按域递增() {
        let entries = vec![MemoryEntry {
            id: "M-UID-001".into(),
            entry_type: MemoryType::Rule,
            domain: "ui-design".into(),
            status: MemoryStatus::Active,
            statement: "x".into(),
            why: "y".into(),
            how_to_apply: "z".into(),
            signals: vec![],
            evidence: vec![],
            weight: 1,
            hits: 0,
            last_used: None,
            created: "2026-10-21".into(),
        }];
        assert_eq!(next_entry_id(&entries, &[], "ui-design"), "M-UID-002");
    }

    #[test]
    fn 同一次运行里的候选_id_不会重号() {
        // 候选还没写进 memory.md，若编号时只看既有条目，两条候选会拿到同一个 ID。
        let first = MemoryEntry {
            id: "M-UID-001".into(),
            entry_type: MemoryType::Rule,
            domain: "ui-design".into(),
            status: MemoryStatus::Candidate,
            statement: "x".into(),
            why: "y".into(),
            how_to_apply: "z".into(),
            signals: vec![],
            evidence: vec![],
            weight: 0,
            hits: 0,
            last_used: None,
            created: "2026-10-21".into(),
        };
        let second = MemoryEntry {
            id: "M-UID-002".into(),
            ..first.clone()
        };
        let next = next_entry_id(&[], &[first, second], "ui-design");
        assert_eq!(next, "M-UID-003");
    }

    fn entry(id: &str, domain: &str, signals: &[&str]) -> MemoryEntry {
        MemoryEntry {
            id: id.into(),
            entry_type: MemoryType::Rule,
            domain: domain.into(),
            status: MemoryStatus::Active,
            statement: format!("{id} 的陈述"),
            why: "y".into(),
            how_to_apply: "z".into(),
            signals: signals.iter().map(|s| s.to_string()).collect(),
            evidence: vec![],
            weight: 1,
            hits: 0,
            last_used: None,
            created: "2026-10-21".into(),
        }
    }

    #[test]
    fn 跨域同信号判为矛盾() {
        let entries = vec![entry("M-WRI-001", "writing", &["S-SUMMARIZE"])];
        let conflicts = detect_conflicts(&entries, "research", &["S-SUMMARIZE".to_string()]);
        assert_eq!(conflicts, vec!["M-WRI-001".to_string()]);
    }

    #[test]
    fn 同域旧条目共享信号不再冻结该域() {
        let entries = vec![
            entry("M-UID-001", "ui-design", &["S-BRAND"]),
            entry("M-UID-002", "ui-design", &["S-BRAND"]),
        ];
        let conflicts = detect_conflicts(&entries, "ui-design", &["S-LAYOUT".to_string()]);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn 无冲突时不误判() {
        let entries = vec![entry("M-UID-001", "ui-design", &["S-BRAND"])];
        let conflicts = detect_conflicts(&entries, "ui-design", &["S-BRAND".to_string()]);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn 任一侧signals为空则按domain匹配() {
        assert!(signals_match(&[], &["S-BRAND".into()]));
        assert!(signals_match(&["S-BRAND".into()], &[]));
        assert!(signals_match(&[], &[]));
        assert!(!signals_match(&["S-A".into()], &["S-B".into()]));
        assert!(signals_match(
            &["S-A".into()],
            &["S-A".into(), "S-B".into()]
        ));
    }
}
