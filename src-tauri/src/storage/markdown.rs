//! L1 / L2 的 Markdown 读写。
//!
//! 设计底线：`memory.md` 必须是**纯 Markdown、全量可读、可 diff、可手改**的。
//! 绝不让记忆只存在于向量库 —— 那样你无法知道 AI「记住了什么」。
//! 因此这里的解析与渲染必须与《02-私人记忆库设计》定义的格式严格一致。

use crate::error::AppError;
use crate::models::{estimate_tokens, MemoryEntry, MemoryStatus, MemoryType, PeRecord};
use std::fs;
use std::path::{Path, PathBuf};

/// L2 记忆层文件名。
pub const MEMORY_FILE: &str = "memory.md";
/// L1 事实层文件名。
pub const RECORD_FILE: &str = "prompt_engineering.md";

pub fn memory_path(dir: &Path) -> PathBuf {
    dir.join(MEMORY_FILE)
}

pub fn record_path(dir: &Path) -> PathBuf {
    dir.join(RECORD_FILE)
}

/// 读取文件；不存在时返回空串（首次启动的正常路径）。
pub fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// 解析 `memory.md` 中的全部条目。
///
/// HTML 注释块整块跳过：模板里的「格式说明 + 示例条目」写在 `<!-- -->` 里，
/// 不是真实记忆，解析进去会让夜间任务把示例当条目写回文件。
pub fn parse_entries(content: &str) -> Vec<MemoryEntry> {
    let mut entries: Vec<MemoryEntry> = Vec::new();
    let mut current: Option<MemoryEntry> = None;
    let mut in_comment = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if in_comment {
            if trimmed.contains("-->") {
                in_comment = false;
            }
            continue;
        }
        if trimmed.starts_with("<!--") {
            in_comment = !trimmed.contains("-->");
            continue;
        }

        if let Some(header) = trimmed.strip_prefix("## ") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            // 标题形如 `## M-UI-001 · 一句话规则名`，ID 取分隔符前的内容。
            let id = header
                .split('·')
                .next()
                .unwrap_or(header)
                .trim()
                .to_string();
            if entries.iter().any(|entry| entry.id == id) {
                // 重复 ID 只保留第一条，避免夜间任务把后一条覆盖前一条。
                current = None;
                continue;
            }
            current = Some(MemoryEntry {
                id,
                entry_type: MemoryType::Rule,
                domain: "general".to_string(),
                status: MemoryStatus::Candidate,
                statement: String::new(),
                why: String::new(),
                how_to_apply: String::new(),
                signals: Vec::new(),
                evidence: Vec::new(),
                weight: 0,
                hits: 0,
                last_used: None,
                created: String::new(),
            });
            continue;
        }

        let Some(entry) = current.as_mut() else {
            continue;
        };
        let Some((key, value)) = parse_field(line) else {
            continue;
        };

        match key {
            "type" => entry.entry_type = MemoryType::parse(value).unwrap_or(MemoryType::Rule),
            "domain" => entry.domain = value.to_string(),
            "status" => {
                entry.status = MemoryStatus::parse(value).unwrap_or(MemoryStatus::Candidate)
            }
            "statement" => entry.statement = value.to_string(),
            "why" => entry.why = value.to_string(),
            "how_to_apply" => entry.how_to_apply = value.to_string(),
            "signals" => entry.signals = parse_list(value),
            "evidence" => entry.evidence = parse_list(value),
            "weight" => entry.weight = value.parse().unwrap_or(0),
            "hits" => entry.hits = value.parse().unwrap_or(0),
            "last_used" => {
                entry.last_used = if value == "null" || value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            }
            "created" => entry.created = value.to_string(),
            _ => {}
        }
    }

    if let Some(entry) = current {
        entries.push(entry);
    }
    entries
}

/// 统计文件里真正的条目标题数（与 `parse_entries` 同一套注释跳过规则）。
///
/// 用来做「文件里明明有标题、却一条都解析不出来」的兜底判断：
/// 那种情况说明文件格式坏了，再往下写就会把整库记忆覆盖掉。
pub fn count_entries(content: &str) -> usize {
    let mut count = 0usize;
    let mut in_comment = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if in_comment {
            if trimmed.contains("-->") {
                in_comment = false;
            }
            continue;
        }
        if trimmed.starts_with("<!--") {
            in_comment = !trimmed.contains("-->");
            continue;
        }
        if trimmed.starts_with("## ") {
            count += 1;
        }
    }
    count
}

/// 解析 `- **key**: value` 形式的字段行。
///
/// 手改时常见的变体也认：`*` 项目符号、全角冒号。认不下来的行会被跳过，
/// 再被夜间任务全量写回就会把字段丢掉，所以这里宁可多认几种。
fn parse_field(line: &str) -> Option<(&str, &str)> {
    let rest = line.trim();
    let rest = rest
        .strip_prefix("- ")
        .or_else(|| rest.strip_prefix("* "))
        .or_else(|| rest.strip_prefix("-"))?;
    let rest = rest.trim().strip_prefix("**")?;
    if let Some((key, value)) = rest.split_once("**:") {
        return Some((key.trim(), value.trim()));
    }
    if let Some((key, value)) = rest.split_once("**：") {
        return Some((key.trim(), value.trim()));
    }
    None
}

/// 解析 L1 事实层里的 PE 记录，供索引重建使用。
pub fn parse_pe_records(content: &str) -> Vec<PeRecord> {
    let mut records: Vec<PeRecord> = Vec::new();
    let mut current: Option<PeRecord> = None;
    let mut section = PeSection::Meta;
    let mut original = String::new();
    let mut optimized = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(header) = trimmed.strip_prefix("## ") {
            if let Some(mut record) = current.take() {
                record.original_prompt = original.trim().to_string();
                record.optimized_prompt = optimized.trim().to_string();
                records.push(record);
            }
            original.clear();
            optimized.clear();
            section = PeSection::Meta;
            let id = header
                .split('·')
                .next()
                .unwrap_or(header)
                .trim()
                .to_string();
            current = Some(PeRecord {
                id,
                original_prompt: String::new(),
                optimized_prompt: String::new(),
                domain: "general".to_string(),
                signals: Vec::new(),
                rating: None,
                created_at: String::new(),
            });
            continue;
        }

        let Some(record) = current.as_mut() else {
            continue;
        };
        if trimmed == "### 原文" {
            section = PeSection::Original;
            continue;
        }
        if trimmed == "### 改写结果" {
            section = PeSection::Optimized;
            continue;
        }
        match section {
            PeSection::Meta => {
                let Some((key, value)) = parse_field(line) else {
                    continue;
                };
                match key {
                    "domain" => record.domain = value.to_string(),
                    "signals" => record.signals = parse_list(value),
                    "rating" => {
                        record.rating = if value == "null" || value.is_empty() {
                            None
                        } else {
                            value.parse().ok()
                        };
                    }
                    "created_at" => record.created_at = value.to_string(),
                    _ => {}
                }
            }
            PeSection::Original => {
                if !original.is_empty() {
                    original.push('\n');
                }
                original.push_str(line);
            }
            PeSection::Optimized => {
                if !optimized.is_empty() {
                    optimized.push('\n');
                }
                optimized.push_str(line);
            }
        }
    }

    if let Some(mut record) = current {
        record.original_prompt = original.trim().to_string();
        record.optimized_prompt = optimized.trim().to_string();
        records.push(record);
    }
    records
}

enum PeSection {
    Meta,
    Original,
    Optimized,
}

/// L1 文件里指定年份已出现的最大序号。
pub fn max_pe_seq(content: &str, year: i64) -> i64 {
    let prefix = format!("PE-{year}-");
    parse_pe_records(content)
        .iter()
        .filter(|record| record.id.starts_with(&prefix))
        .filter_map(|record| record.id.get(prefix.len()..)?.parse::<i64>().ok())
        .max()
        .unwrap_or(0)
}

/// 解析 `[A, B]` 形式的列表；`null` 或空串解析为空列表。
fn parse_list(value: &str) -> Vec<String> {
    let trimmed = value.trim().trim_start_matches('[').trim_end_matches(']');
    if trimmed.is_empty() || trimmed == "null" {
        return Vec::new();
    }
    trimmed
        .split(',')
        .map(|item| {
            item.trim()
                .trim_matches(['[', ']', '`', '"', '\''].as_ref())
                .to_string()
        })
        .filter(|item| !item.is_empty())
        .collect()
}

/// 渲染单条记忆条目。
pub fn render_entry(entry: &MemoryEntry) -> String {
    let title = summarize(&entry.statement, 28);
    format!(
        "## {id} · {title}\n\n\
         - **type**: {entry_type}\n\
         - **domain**: {domain}\n\
         - **status**: {status}\n\
         - **statement**: {statement}\n\
         - **why**: {why}\n\
         - **how_to_apply**: {how_to_apply}\n\
         - **signals**: [{signals}]\n\
         - **evidence**: [{evidence}]\n\
         - **weight**: {weight}\n\
         - **hits**: {hits}\n\
         - **last_used**: {last_used}\n\
         - **created**: {created}\n",
        id = entry.id,
        title = title,
        entry_type = entry.entry_type.as_str(),
        domain = entry.domain,
        status = entry.status.as_str(),
        statement = entry.statement,
        why = entry.why,
        how_to_apply = entry.how_to_apply,
        signals = entry.signals.join(", "),
        evidence = entry.evidence.join(", "),
        weight = entry.weight,
        hits = entry.hits,
        last_used = entry
            .last_used
            .clone()
            .unwrap_or_else(|| "null".to_string()),
        created = entry.created,
    )
}

/// 渲染完整的 `memory.md`（含 YAML 文件头）。
pub fn render_document(entries: &[MemoryEntry], token_budget: u32, updated: &str) -> String {
    let active_count = entries
        .iter()
        .filter(|e| e.status == MemoryStatus::Active)
        .count();
    let deprecated_count = entries
        .iter()
        .filter(|e| e.status == MemoryStatus::Deprecated)
        .count();

    let mut out = String::from("---\n");
    out.push_str("format_version: 1\n");
    out.push_str(&format!("token_budget: {token_budget}\n"));
    out.push_str("domain: <global>\n");
    out.push_str(&format!("updated: {updated}\n"));
    out.push_str(&format!("entry_count: {}\n", entries.len()));
    out.push_str(&format!("active_count: {active_count}\n"));
    out.push_str(&format!("deprecated_count: {deprecated_count}\n"));
    out.push_str("---\n\n");

    for entry in entries {
        out.push_str(&render_entry(entry));
        out.push('\n');
    }
    out
}

/// 估算当前条目集合占用的 token（含 deprecated，全量渲染口径）。
#[cfg_attr(not(test), allow(dead_code))]
pub fn token_usage(entries: &[MemoryEntry]) -> u32 {
    let body: String = entries.iter().map(render_entry).collect();
    estimate_tokens(&body)
}

/// 文件预算口径：`deprecated` 不占 800 token（AGENTS.md §1）。
pub fn token_usage_counted(entries: &[MemoryEntry]) -> u32 {
    let kept: Vec<&MemoryEntry> = entries
        .iter()
        .filter(|entry| entry.status != MemoryStatus::Deprecated)
        .collect();
    let body: String = kept.iter().copied().map(render_entry).collect();
    estimate_tokens(&body)
}

/// 写入 `memory.md`（原子写）。
pub fn write_memory(
    path: &Path,
    entries: &[MemoryEntry],
    token_budget: u32,
) -> Result<(), AppError> {
    let updated = today();
    let body = render_document(entries, token_budget, &updated);
    write_atomic(path, &body)
}

/// 追加一条 PE 记录到 L1 文件。
///
/// L1 事实层**只允许追加**，绝不改写既有内容。
/// 追加前先确认 ID 不存在：L1 文件是唯一真相来源，索引层可能被清空或重建，
/// 只查索引会漏掉「文件里有、索引里没有」的记录，从而写出两条同 ID 的记录。
pub fn append_pe_record(path: &Path, record: &PeRecord) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut existing = read_to_string(path);
    if existing.contains(&format!("## {} ", record.id)) {
        return Err(
            crate::error::AppError::duplicate_record("该 PE 记录 ID 已存在")
                .with_details(serde_json::json!({ "record_id": record.id })),
        );
    }

    let mut block = String::new();
    block.push_str(&format!("\n## {} · {}\n\n", record.id, record.created_at));
    block.push_str(&format!("- **domain**: {}\n", record.domain));
    block.push_str(&format!("- **signals**: [{}]\n", record.signals.join(", ")));
    block.push_str(&format!(
        "- **rating**: {}\n",
        record
            .rating
            .map(|r| r.to_string())
            .unwrap_or_else(|| "null".to_string())
    ));
    block.push_str(&format!("- **created_at**: {}\n", record.created_at));
    block.push_str("\n### 原文\n\n");
    block.push_str(&record.original_prompt);
    block.push_str("\n\n### 改写结果\n\n");
    block.push_str(&record.optimized_prompt);
    block.push('\n');

    if !existing.is_empty() && !existing.ends_with('\n') {
        existing.push('\n');
    }
    existing.push_str(&block);

    write_atomic(path, &existing)
}

/// 原子写：委托给共用的 `fsutil`，避免 Windows 上固定 `*.tmp` 名残留卡住写盘。
fn write_atomic(path: &Path, body: &str) -> Result<(), AppError> {
    crate::fsutil::write_atomic(path, body)
}

/// 取一句话规则名：取 statement 的前 N 个字符。
fn summarize(text: &str, limit: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= limit {
        return trimmed.to_string();
    }
    let head: String = trimmed.chars().take(limit).collect();
    format!("{head}…")
}

/// 今天的日期（UTC），形如 `2026-10-21`。
pub fn today() -> String {
    crate::clock::today_date()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"---
format_version: 1
token_budget: 800
domain: <global>
updated: 2026-10-14
entry_count: 1
active_count: 0
deprecated_count: 0
---

## M-ui-design-001 · 品牌色必须单列为硬性约束

- **type**: rule
- **domain**: ui-design
- **status**: candidate
- **statement**: 当任务涉及品牌色时，必须把不可替换的品牌色单列为硬性约束。
- **why**: 防止模型用生成的色彩替换品牌主色。
- **how_to_apply**: 在提示词中单开一章硬性品牌约束。
- **signals**: [S-HARD-SOFTSPLIT]
- **evidence**: [PE-2026-001]
- **weight**: 0
- **hits**: 0
- **last_used**: null
- **created**: 2026-10-14
"#;

    #[test]
    fn 解析条目字段完整() {
        let entries = parse_entries(SAMPLE);
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.id, "M-ui-design-001");
        assert_eq!(entry.entry_type, MemoryType::Rule);
        assert_eq!(entry.domain, "ui-design");
        assert_eq!(entry.status, MemoryStatus::Candidate);
        assert_eq!(entry.weight, 0);
        assert_eq!(entry.hits, 0);
        assert!(entry.last_used.is_none());
        assert_eq!(entry.signals, vec!["S-HARD-SOFTSPLIT"]);
        assert_eq!(entry.evidence, vec!["PE-2026-001"]);
    }

    #[test]
    fn 解析后重新渲染能往返() {
        let entries = parse_entries(SAMPLE);
        let rendered = render_document(&entries, 800, "2026-10-14");
        let reparsed = parse_entries(&rendered);
        assert_eq!(reparsed.len(), 1);
        assert_eq!(reparsed[0].statement, entries[0].statement);
        assert_eq!(reparsed[0].why, entries[0].why);
        assert_eq!(reparsed[0].how_to_apply, entries[0].how_to_apply);
        assert_eq!(reparsed[0].status, entries[0].status);
    }

    #[test]
    fn 文件头统计随条目变化() {
        let entries = parse_entries(SAMPLE);
        let rendered = render_document(&entries, 800, "2026-10-21");
        assert!(rendered.contains("entry_count: 1"));
        assert!(rendered.contains("token_budget: 800"));
        assert!(rendered.contains("updated: 2026-10-21"));
    }

    #[test]
    fn 空文件解析出空列表() {
        assert!(parse_entries("").is_empty());
    }

    #[test]
    fn 全角冒号与星号项目符号也能解析() {
        let raw = r#"
## M-GEN-001 · 手改格式

* **type**：rule
* **domain**：general
* **status**：active
* **statement**：陈述
* **why**：原因
* **how_to_apply**：用法
* **signals**：[S-GENERAL]
* **evidence**：[]
* **weight**：2
* **hits**：0
* **last_used**：null
* **created**：2026-10-22
"#;
        let entries = parse_entries(raw);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, MemoryStatus::Active);
        assert_eq!(entries[0].weight, 2);
        assert_eq!(entries[0].domain, "general");
    }

    #[test]
    fn 重复_id_只保留第一条() {
        let raw = format!("{SAMPLE}\n\n## M-ui-design-001 · 重复\n\n- **statement**: 后一条\n");
        let entries = parse_entries(&raw);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].statement.contains("品牌色"));
    }

    #[test]
    fn html_注释里的示例条目不会被解析成真实记忆() {
        // 模板里的格式说明写在注释块中，解析进去会被夜间任务当成记忆写回文件。
        let template = r#"---
format_version: 1
---

<!--
条目格式示例：

## M-<域缩写>-<序号> · 一句话规则名

- **type**: rule
- **domain**: ui-design
- **status**: candidate
- **statement**: 示例陈述
-->
"#;
        assert!(parse_entries(template).is_empty());
        assert_eq!(count_entries(template), 0);
    }

    #[test]
    fn deprecated不计入文件预算() {
        let mut entries = parse_entries(SAMPLE);
        let before = token_usage_counted(&entries);
        entries[0].status = MemoryStatus::Deprecated;
        assert_eq!(token_usage_counted(&entries), 0);
        assert!(token_usage(&entries) > 0);
        assert!(before > 0);
    }

    #[test]
    fn 真实条目在注释块之后仍能解析() {
        let content = format!("{SAMPLE}\n\n<!--\n## M-XXX-999 · 注释里的假条目\n-->\n");
        let entries = parse_entries(&content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "M-ui-design-001");
        assert_eq!(count_entries(&content), 1);
    }
}
