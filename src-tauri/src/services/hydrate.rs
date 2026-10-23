//! 从 Markdown 重建 L3 索引。
//!
//! 索引层可随时丢掉：换机、删库、损坏之后，必须以 `memory/*.md` 为准灌回去。
//! 否则 `next_pe_id` 从 001 重计，L1 文件里已有同 ID，保存会永远 `duplicate_record`。

use crate::error::AppError;
use crate::storage::db::Db;
use crate::storage::markdown;
use std::path::Path;

/// 用 L1 / L2 Markdown 全量灌进 SQLite。已存在的 L1 行不改写。
pub fn rebuild_from_markdown(db: &Db, memory_dir: &Path) -> Result<(), AppError> {
    let memory_file = markdown::memory_path(memory_dir);
    let record_file = markdown::record_path(memory_dir);

    let memory_raw = markdown::read_to_string(&memory_file);
    let entries = markdown::parse_entries(&memory_raw);
    for entry in &entries {
        if entry.id.trim().is_empty() {
            continue;
        }
        db.upsert_memory(entry)?;
    }
    let kept: Vec<String> = entries
        .iter()
        .filter(|entry| !entry.id.trim().is_empty())
        .map(|entry| entry.id.clone())
        .collect();
    db.delete_memories_not_in(&kept)?;

    let record_raw = markdown::read_to_string(&record_file);
    for record in markdown::parse_pe_records(&record_raw) {
        if db.entry_exists(&record.id)? {
            continue;
        }
        db.upsert_pe_record(&record)?;
    }

    db.rebuild_fts()?;
    Ok(())
}
