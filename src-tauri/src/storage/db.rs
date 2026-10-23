//! L3 索引库：SQLite + FTS5。
//!
//! 索引层是**可重建**的 —— 以 `memory/` 下的 Markdown 文件为准。
//! 因此这里的写入永远不能反过来覆盖 Markdown 的内容。
//!
//! 表结构与 `docs/database-design.md` 严格一致（该文档的 DDL 已用内存 SQLite 实跑校验）：
//! - `entries`：`uid` 自增主键 + `entry_id` 业务唯一键；`uid` 同时是 FTS5 外置内容的行标识。
//! - `entries_fts`：外置内容虚拟表，**不使用触发器**，同步由数据层显式控制
//!   （触发器在重建期间会产生幽灵行）。
//! - `entry_vectors`：复合主键 `(entry_uid, embedding_model)`，同一条目可并存多套模型向量。
//! - `usage_log`：`event_type` 三值（inject / miss / score），`session_id` 把一次使用串成可回溯的链。

use crate::error::AppError;
use crate::models::{MemoryEntry, MemoryStatus, MemoryType, PeRecord};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

/// 迁移 001：L3 索引库基线（阶段一 / 阶段二）。
///
/// 全部语句幂等，可重复执行。
const MIGRATION_001: &str = r#"
CREATE TABLE IF NOT EXISTS entries (
    uid           INTEGER PRIMARY KEY,
    entry_id      TEXT    NOT NULL UNIQUE,
    layer         TEXT    NOT NULL CHECK (layer IN ('L1', 'L2')),
    entry_type    TEXT    NOT NULL CHECK (entry_type IN ('pe_record', 'rule', 'preference', 'pitfall', 'asset')),
    domain        TEXT    NOT NULL DEFAULT 'general',
    status        TEXT    NOT NULL DEFAULT 'active' CHECK (status IN ('candidate', 'active', 'deprecated')),
    title         TEXT    NOT NULL DEFAULT '',
    aliases       TEXT    NOT NULL DEFAULT '',
    content       TEXT    NOT NULL DEFAULT '',
    statement     TEXT,
    why           TEXT,
    how_to_apply  TEXT,
    signals       TEXT    NOT NULL DEFAULT '[]',
    evidence      TEXT    NOT NULL DEFAULT '[]',
    weight        INTEGER NOT NULL DEFAULT 0,
    hits          INTEGER NOT NULL DEFAULT 0,
    miss_streak   INTEGER NOT NULL DEFAULT 0,
    last_used     TEXT,
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    CHECK (layer <> 'L1' OR status = 'active'),
    CHECK (layer <> 'L2' OR entry_type <> 'pe_record')
);

CREATE INDEX IF NOT EXISTS idx_entries_route         ON entries(layer, domain, status, weight DESC);
CREATE INDEX IF NOT EXISTS idx_entries_layer_created ON entries(layer, created_at);
CREATE INDEX IF NOT EXISTS idx_entries_promotion     ON entries(weight DESC) WHERE layer = 'L2' AND status = 'candidate';

CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(
    title,
    aliases,
    content,
    content='entries',
    content_rowid='uid',
    tokenize='trigram'
);

CREATE TABLE IF NOT EXISTS entry_vectors (
    entry_uid       INTEGER NOT NULL REFERENCES entries(uid) ON DELETE CASCADE,
    embedding_model TEXT    NOT NULL,
    embedding_dim   INTEGER NOT NULL DEFAULT 384 CHECK (embedding_dim > 0),
    embedding       BLOB    NOT NULL,
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (entry_uid, embedding_model)
);

CREATE INDEX IF NOT EXISTS idx_entry_vectors_model ON entry_vectors(embedding_model);

CREATE TABLE IF NOT EXISTS usage_log (
    id           INTEGER PRIMARY KEY,
    session_id   TEXT    NOT NULL,
    entry_uid    INTEGER NOT NULL REFERENCES entries(uid) ON DELETE CASCADE,
    event_type   TEXT    NOT NULL CHECK (event_type IN ('inject', 'miss', 'score')),
    score        INTEGER CHECK (score IS NULL OR (score BETWEEN 1 AND 5)),
    weight_delta INTEGER NOT NULL DEFAULT 0,
    note         TEXT,
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    CHECK ((event_type = 'score' AND score IS NOT NULL) OR (event_type <> 'score' AND score IS NULL))
);

CREATE INDEX IF NOT EXISTS idx_usage_log_entry_event ON usage_log(entry_uid, event_type, created_at);
CREATE INDEX IF NOT EXISTS idx_usage_log_session   ON usage_log(session_id, created_at);
"#;

/// 索引库 schema 版本。
///
/// - `1`：基线（unicode61 FTS，历史库可能仍停在 0/1）
/// - `2`：FTS5 分词器改为 `trigram`（阶段二中文子串检索）
const SCHEMA_VERSION: i32 = 2;

/// 用 trigram 重建 `entries_fts`（换分词器必须 drop + create，不能 ALTER）。
const MIGRATE_FTS_TRIGRAM: &str = r#"
DROP TABLE IF EXISTS entries_fts;
CREATE VIRTUAL TABLE entries_fts USING fts5(
    title,
    aliases,
    content,
    content='entries',
    content_rowid='uid',
    tokenize='trigram'
);
INSERT INTO entries_fts(entries_fts) VALUES('rebuild');
"#;

/// 迁移 002：sqlite-vec 向量检索（阶段三）。
///
/// **本期不执行** —— AGENTS.md §6.9 规定检索层未积累 20–30 条真实记录前
/// 不得引入向量库调用路径。启用时把 `MIGRATION_002` 并入 `MIGRATION_001` 即可。
#[allow(dead_code)]
const MIGRATION_002: &str = r#"
-- CREATE VIRTUAL TABLE IF NOT EXISTS entry_vectors_vec USING vec0(
--     entry_uid INTEGER PRIMARY KEY,
--     embedding_model TEXT,
--     embedding FLOAT[384]
-- );
"#;

/// 回滚脚本：删除全部索引层对象。
///
/// 只清索引、不碰 `memory/` 下的 Markdown —— Markdown 才是真相来源。
const ROLLBACK_SQL: &str = r#"
DROP TABLE IF EXISTS usage_log;
DROP TABLE IF EXISTS entry_vectors;
DROP TABLE IF EXISTS entries_fts;
DROP TABLE IF EXISTS entries;
"#;

/// L1 记录里「原文」与「改写结果」之间的分隔符。
///
/// 用 ASCII 单元分隔符 U+001F 而不是空行：原文本身完全可能包含空行，
/// 只有固定控制符才能保证拆分可逆 —— 否则夜间任务提炼候选规则时，
/// 会把原文当成改写结果的一部分带进 `statement`。
const L1_FIELD_SEPARATOR: &str = "\u{1f}";

/// 索引库句柄。
pub struct Db {
    conn: Connection,
}

impl Db {
    /// 打开（或创建）索引库并执行幂等迁移。
    pub fn open(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    /// 内存库，供单元测试使用。
    pub fn open_in_memory() -> Result<Self, AppError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, AppError> {
        // foreign_keys 是连接级设置，且在事务内为 no-op —— 必须在开事务之前设。
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        conn.execute_batch(MIGRATION_001)?;
        migrate_schema(&conn)?;
        Ok(Self { conn })
    }

    /// 回滚全部索引层结构。
    ///
    /// 只清索引、不碰 `memory/` 下的 Markdown —— Markdown 才是真相来源。
    /// 目前只有单元测试在用，保留成公开 API 是为了将来做「索引损坏后一键重建」。
    #[allow(dead_code)]
    pub fn rollback(&self) -> Result<(), AppError> {
        self.conn.execute_batch(ROLLBACK_SQL)?;
        Ok(())
    }

    /// 写入（或覆盖）一条 L2 记忆条目，返回其 `uid`。
    pub fn upsert_memory(&self, entry: &MemoryEntry) -> Result<i64, AppError> {
        let now = now_iso();
        let content = build_l2_content(entry);
        let title = build_l2_title(entry);

        // FTS 同步必须先记下「这行此前是否已存在」：
        // FTS5 外置内容表的 'delete' 命令只对**已在索引里**的行有效，
        // 对尚未写入索引的新行执行会报「database disk image is malformed」，
        // 让 SQLite 直接判定索引损坏。
        let previous = self.fts_row_of(&entry.id)?;

        self.conn.execute(
            "INSERT INTO entries (
                entry_id, layer, entry_type, domain, status, title, aliases, content,
                statement, why, how_to_apply, signals, evidence, weight, hits,
                last_used, created_at, updated_at
             ) VALUES (?1, 'L2', ?2, ?3, ?4, ?5, '', ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT (entry_id) DO UPDATE SET
                entry_type   = excluded.entry_type,
                domain       = excluded.domain,
                status       = excluded.status,
                title        = excluded.title,
                content      = excluded.content,
                statement    = excluded.statement,
                why          = excluded.why,
                how_to_apply = excluded.how_to_apply,
                signals      = excluded.signals,
                evidence     = excluded.evidence,
                weight       = excluded.weight,
                hits         = excluded.hits,
                last_used    = excluded.last_used,
                updated_at   = excluded.updated_at",
            params![
                entry.id,
                entry.entry_type.as_str(),
                entry.domain,
                entry.status.as_str(),
                title,
                content,
                entry.statement,
                entry.why,
                entry.how_to_apply,
                serde_json::to_string(&entry.signals)?,
                serde_json::to_string(&entry.evidence)?,
                entry.weight,
                entry.hits,
                entry.last_used,
                entry.created,
                now,
            ],
        )?;

        let uid = self.uid_of(&entry.id)?;
        if let Some(uid) = uid {
            self.sync_fts_row(uid, previous.as_ref(), &title, &content)?;
        }
        Ok(uid.unwrap_or_default())
    }

    /// 写入一条 L1 事实层记录，返回其 `uid`。
    ///
    /// L1 行恒为 `status='active'`（不参与记忆状态机），`entry_type='pe_record'`。
    pub fn upsert_pe_record(&self, record: &PeRecord) -> Result<i64, AppError> {
        let now = now_iso();
        let content = format!(
            "{}{}{}",
            record.original_prompt, L1_FIELD_SEPARATOR, record.optimized_prompt
        );
        let previous = self.fts_row_of(&record.id)?;

        // L1 逐字不可改：冲突时 no-op，禁止用新内容覆盖既有事实层。
        self.conn.execute(
            "INSERT INTO entries (
                entry_id, layer, entry_type, domain, status, title, aliases, content,
                signals, weight, hits, created_at, updated_at
             ) VALUES (?1, 'L1', 'pe_record', ?2, 'active', ?3, '', ?4, ?5, 0, 0, ?6, ?7)
             ON CONFLICT (entry_id) DO NOTHING",
            params![
                record.id,
                record.domain,
                record.original_prompt,
                content,
                serde_json::to_string(&record.signals)?,
                record.created_at,
                now,
            ],
        )?;

        let uid = self.uid_of(&record.id)?;
        if previous.is_none() {
            if let Some(uid) = uid {
                self.sync_fts_row(uid, None, &record.original_prompt, &content)?;
            }
        }
        Ok(uid.unwrap_or_default())
    }

    /// FTS5 BM25 全文检索可注入记忆（阶段二）。
    ///
    /// 硬性过滤与 `injectable_memories` 相同：`L2` + `active` + `weight > 0`。
    /// `query` 经 `sanitize_fts_query` 清洗；清洗后为空则返回空向量（由调用方回退 domain 路由）。
    /// 有 domain 时同域条目优先，再按 BM25（越低越相关）与 weight 降序。
    ///
    /// trigram 对不足 3 个字符的查询不产生词项；此时（以及 FTS 零命中时）
    /// 回退到 `title`/`content` 的 `LIKE` 子串匹配，保证双字中文词仍可检索。
    pub fn injectable_by_bm25(
        &self,
        query: &str,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<MemoryEntry>, AppError> {
        let Some(match_query) = sanitize_fts_query(query) else {
            return Ok(Vec::new());
        };

        let fts_hits = self.bm25_match(&match_query, domain, limit)?;
        if !fts_hits.is_empty() {
            return Ok(fts_hits);
        }

        // FTS 零命中（含短中文）：LIKE 子串补一刀，规模 <300 条时可接受。
        self.substring_match(&match_query, domain, limit)
    }

    fn bm25_match(
        &self,
        match_query: &str,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<MemoryEntry>, AppError> {
        // trigram 对 <3 字符查询不会命中，直接跳过 MATCH，避免无意义扫描。
        if match_query.chars().count() < 3 {
            return Ok(Vec::new());
        }

        // bm25() 越低越相关，故 ORDER BY score ASC；同域优先用 CASE 软提升。
        let sql = match domain {
            Some(_) => {
                "SELECT e.entry_id, e.entry_type, e.domain, e.status, e.statement, e.why, e.how_to_apply,
                        e.signals, e.evidence, e.weight, e.hits, e.last_used, e.created_at
                 FROM entries_fts
                 JOIN entries e ON e.uid = entries_fts.rowid
                 WHERE entries_fts MATCH ?1
                   AND e.layer = 'L2' AND e.status = 'active' AND e.weight > 0
                 ORDER BY CASE WHEN e.domain = ?2 THEN 0 ELSE 1 END,
                          bm25(entries_fts, 10.0, 5.0, 1.0) ASC,
                          e.weight DESC,
                          e.created_at ASC
                 LIMIT ?3"
            }
            None => {
                "SELECT e.entry_id, e.entry_type, e.domain, e.status, e.statement, e.why, e.how_to_apply,
                        e.signals, e.evidence, e.weight, e.hits, e.last_used, e.created_at
                 FROM entries_fts
                 JOIN entries e ON e.uid = entries_fts.rowid
                 WHERE entries_fts MATCH ?1
                   AND e.layer = 'L2' AND e.status = 'active' AND e.weight > 0
                 ORDER BY bm25(entries_fts, 10.0, 5.0, 1.0) ASC,
                          e.weight DESC,
                          e.created_at ASC
                 LIMIT ?2"
            }
        };

        let mut stmt = self.conn.prepare(sql)?;
        let mapped = match domain {
            Some(domain) => stmt
                .query_map(params![match_query, domain, limit], map_injectable_row)?
                .collect::<Result<Vec<_>, _>>()?,
            None => stmt
                .query_map(params![match_query, limit], map_injectable_row)?
                .collect::<Result<Vec<_>, _>>()?,
        };
        Ok(mapped.into_iter().map(row_to_entry).collect())
    }

    fn substring_match(
        &self,
        needle: &str,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<MemoryEntry>, AppError> {
        // ESCAPE '\'：把用户输入里的 % / _ / \ 当字面量，避免通配符注入。
        let escaped = needle
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        let sql = match domain {
            Some(_) => {
                "SELECT entry_id, entry_type, domain, status, statement, why, how_to_apply,
                        signals, evidence, weight, hits, last_used, created_at
                 FROM entries
                 WHERE layer = 'L2' AND status = 'active' AND weight > 0
                   AND (title LIKE ?1 ESCAPE '\\' OR content LIKE ?1 ESCAPE '\\'
                        OR IFNULL(statement, '') LIKE ?1 ESCAPE '\\')
                 ORDER BY CASE WHEN domain = ?2 THEN 0 ELSE 1 END,
                          weight DESC,
                          created_at ASC
                 LIMIT ?3"
            }
            None => {
                "SELECT entry_id, entry_type, domain, status, statement, why, how_to_apply,
                        signals, evidence, weight, hits, last_used, created_at
                 FROM entries
                 WHERE layer = 'L2' AND status = 'active' AND weight > 0
                   AND (title LIKE ?1 ESCAPE '\\' OR content LIKE ?1 ESCAPE '\\'
                        OR IFNULL(statement, '') LIKE ?1 ESCAPE '\\')
                 ORDER BY weight DESC, created_at ASC
                 LIMIT ?2"
            }
        };

        let mut stmt = self.conn.prepare(sql)?;
        let mapped = match domain {
            Some(domain) => stmt
                .query_map(params![pattern, domain, limit], map_injectable_row)?
                .collect::<Result<Vec<_>, _>>()?,
            None => stmt
                .query_map(params![pattern, limit], map_injectable_row)?
                .collect::<Result<Vec<_>, _>>()?,
        };
        Ok(mapped.into_iter().map(row_to_entry).collect())
    }

    /// 取指定 domain 下可用于注入的记忆条目。
    ///
    /// 硬性过滤：`layer='L2'`、`status='active'`、`weight > 0`。
    /// `candidate` 永不参与注入 —— 这是「候选不注入」防污染防线的落点。
    pub fn injectable_memories(
        &self,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<MemoryEntry>, AppError> {
        let sql = match domain {
            Some(_) => {
                "SELECT entry_id, entry_type, domain, status, statement, why, how_to_apply,
                        signals, evidence, weight, hits, last_used, created_at
                 FROM entries
                 WHERE layer = 'L2' AND status = 'active' AND weight > 0 AND domain = ?1
                 ORDER BY weight DESC, created_at ASC
                 LIMIT ?2"
            }
            None => {
                "SELECT entry_id, entry_type, domain, status, statement, why, how_to_apply,
                        signals, evidence, weight, hits, last_used, created_at
                 FROM entries
                 WHERE layer = 'L2' AND status = 'active' AND weight > 0
                 ORDER BY weight DESC, created_at ASC
                 LIMIT ?2"
            }
        };

        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![domain.unwrap_or(""), limit], map_injectable_row)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row_to_entry(row?));
        }
        Ok(out)
    }

    /// 取全部 L2 条目（含 candidate 与 deprecated），夜间任务用。
    pub fn all_memories(&self) -> Result<Vec<MemoryEntry>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT entry_id, entry_type, domain, status, statement, why, how_to_apply,
                    signals, evidence, weight, hits, last_used, created_at
             FROM entries WHERE layer = 'L2'",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, String>(12)?,
            ))
        })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row_to_entry(row?));
        }
        Ok(out)
    }

    /// 生成下一个 PE 记录 ID：PE-<年>-<三位序号>。
    ///
    /// 用 MAX 而不是 COUNT：COUNT 在记录被删过时会复用已存在的序号，
    /// 而 L1 记录是永久不可改的，ID 冲突比缺号严重得多。
    pub fn next_pe_id(&self, year: i64) -> Result<String, AppError> {
        self.next_pe_id_after(year, 0)
    }

    /// 生成下一个 PE ID，序号不低于 `floor`（用于对齐 L1 Markdown 里已有的最大号）。
    pub fn next_pe_id_after(&self, year: i64, floor: i64) -> Result<String, AppError> {
        let prefix = format!("PE-{year}-");
        let max_seq: Option<i64> = self
            .conn
            .query_row(
                "SELECT MAX(CAST(substr(entry_id, 9) AS INTEGER))
                 FROM entries
                 WHERE layer = 'L1' AND entry_id LIKE ?1",
                params![format!("{prefix}%")],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let seq = max_seq.unwrap_or(0).max(floor) + 1;
        Ok(format!("{prefix}{seq:03}"))
    }

    /// 按业务 ID 取 `uid`。
    pub fn uid_of(&self, entry_id: &str) -> Result<Option<i64>, AppError> {
        let uid = self
            .conn
            .query_row(
                "SELECT uid FROM entries WHERE entry_id = ?1",
                params![entry_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        Ok(uid)
    }

    /// 业务 ID 是否已存在（精确匹配，不做前缀扫描）。
    pub fn entry_exists(&self, entry_id: &str) -> Result<bool, AppError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM entries WHERE entry_id = ?1",
            params![entry_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// 条目的创建时间；不存在时返回 None。
    pub fn entry_created_at(&self, entry_id: &str) -> Result<Option<String>, AppError> {
        let created = self
            .conn
            .query_row(
                "SELECT created_at FROM entries WHERE entry_id = ?1",
                params![entry_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(created)
    }

    /// 写入一条 usage_log 事件。
    ///
    /// `note` 只在评分事件上有值（用户手写的复核备注），
    /// 它落在库里供人工回看，**绝不进入模型上下文**。
    pub fn log_usage(
        &self,
        session_id: &str,
        entry_uid: i64,
        event_type: &str,
        score: Option<i64>,
        weight_delta: i64,
        note: Option<&str>,
    ) -> Result<(), AppError> {
        let now = now_iso();
        self.conn.execute(
            "INSERT INTO usage_log (session_id, entry_uid, event_type, score, weight_delta, note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![session_id, entry_uid, event_type, score, weight_delta, note, now],
        )?;
        Ok(())
    }

    /// 给记忆条目加权重。
    pub fn adjust_weight(&self, entry_uid: i64, delta: i64) -> Result<(), AppError> {
        self.conn.execute(
            "UPDATE entries SET weight = weight + ?2, updated_at = ?3 WHERE uid = ?1",
            params![entry_uid, delta, now_iso()],
        )?;
        Ok(())
    }

    /// 标记一次注入：hits +1、last_used 更新、miss_streak 归零。
    pub fn mark_injected(&self, entry_uid: i64) -> Result<(), AppError> {
        let now = now_iso();
        self.conn.execute(
            "UPDATE entries
             SET hits = hits + 1, miss_streak = 0, last_used = ?2, updated_at = ?2
             WHERE uid = ?1",
            params![entry_uid, now],
        )?;
        Ok(())
    }

    /// 连续未命中计数 +1，返回新的计数。
    ///
    /// 「未命中」= 一次夜间扫描里，该条目所属的域当天有记录，
    /// 但它的 signals 与当天该域的合并 signals 没有交集 —— 也就是这条规则没被触发。
    /// 连续 3 次未命中即降级为 `deprecated`（AGENTS.md 状态机）。
    pub fn bump_miss_streak(&self, entry_uid: i64) -> Result<i64, AppError> {
        self.conn.execute(
            "UPDATE entries SET miss_streak = miss_streak + 1, updated_at = ?2 WHERE uid = ?1",
            params![entry_uid, now_iso()],
        )?;
        self.miss_streak_of(entry_uid)
    }

    /// 连续未命中计数归零（本次被命中）。
    pub fn reset_miss_streak(&self, entry_uid: i64) -> Result<(), AppError> {
        self.conn.execute(
            "UPDATE entries SET miss_streak = 0, updated_at = ?2 WHERE uid = ?1",
            params![entry_uid, now_iso()],
        )?;
        Ok(())
    }

    /// 当前连续未命中计数。
    pub fn miss_streak_of(&self, entry_uid: i64) -> Result<i64, AppError> {
        let streak: i64 = self
            .conn
            .query_row(
                "SELECT miss_streak FROM entries WHERE uid = ?1",
                params![entry_uid],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0);
        Ok(streak)
    }

    /// 删除索引层中不在给定 ID 集合里的 L2 条目，返回删除条数。
    ///
    /// 夜间任务写回 memory.md 后必须调用它：Markdown 是唯一真相来源，
    /// 索引里残留的条目会让「以 Markdown 为准」这条约束长期失效。
    ///
    /// 删行时必须同步把 FTS 索引里的对应行移除：外置内容表的内容行一旦消失，
    /// 索引里的残留会让 FTS 模块判定「内容表与索引不一致」而报库损坏。
    pub fn delete_memories_not_in(&self, ids: &[String]) -> Result<usize, AppError> {
        let existing = self.all_memories()?;
        let mut removed = 0usize;
        for entry in existing {
            if ids.iter().any(|id| id == &entry.id) {
                continue;
            }
            if let Some(uid) = self.uid_of(&entry.id)? {
                self.conn.execute(
                    "INSERT INTO entries_fts(entries_fts, rowid, title, aliases, content)
                 VALUES ('delete', ?1, ?2, '', ?3)",
                    params![uid, build_l2_title(&entry), build_l2_content(&entry)],
                )?;
            }
            self.conn
                .execute("DELETE FROM entries WHERE entry_id = ?1", params![entry.id])?;
            removed += 1;
        }
        Ok(removed)
    }

    /// 累计的**反馈提交次数**（用于冷启动豁免判定）。
    ///
    /// 口径是「发生过评分的 (会话, 评分) 组合数」，不是 usage_log 里 score 行的条数：
    /// 一次反馈可能给多条被注入条目各写一行 score，按行数统计会让豁免提前结束；
    /// 而一次都没注入记忆的反馈同样算一次使用，按行数统计又会让豁免永远不解除。
    /// 同一条记录换评分算两次使用，因为幂等检查只拦截「同记录同评分」的重复提交。
    pub fn feedback_session_count(&self) -> Result<i64, AppError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM (
                 SELECT DISTINCT session_id, score FROM usage_log WHERE event_type = 'score'
             )",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// 该条目今天是否已经记过 miss（同一 run_date 不重复累计）。
    pub fn miss_logged(&self, entry_uid: i64, run_date: &str) -> Result<bool, AppError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM usage_log
             WHERE entry_uid = ?1 AND event_type = 'miss' AND session_id = ?2",
            params![entry_uid, run_date],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// 当前权重。
    pub fn weight_of(&self, entry_uid: i64) -> Result<i64, AppError> {
        let weight: i64 = self
            .conn
            .query_row(
                "SELECT weight FROM entries WHERE uid = ?1",
                params![entry_uid],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0);
        Ok(weight)
    }

    /// 某次改写是否已经提交过任意评分。一条 PE 只允许评一次。
    pub fn any_score_for_session(&self, session_id: &str) -> Result<bool, AppError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM usage_log WHERE session_id = ?1 AND event_type = 'score'",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// 某次会话已经落下的评分值。
    pub fn existing_score_for_session(&self, session_id: &str) -> Result<Option<i64>, AppError> {
        let score = self
            .conn
            .query_row(
                "SELECT score FROM usage_log
                 WHERE session_id = ?1 AND event_type = 'score' AND score IS NOT NULL
                 ORDER BY id ASC LIMIT 1",
                params![session_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten();
        Ok(score)
    }

    /// 某次会话是否已经以该评分提交过反馈（契约要求 submit_feedback 幂等）。
    pub fn score_event_exists(&self, session_id: &str, rating: i64) -> Result<bool, AppError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM usage_log
             WHERE session_id = ?1 AND event_type = 'score' AND score = ?2",
            params![session_id, rating],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// 某次会话某评分下，实际落到 L2 记忆条目上的权重变化。
    ///
    /// 只取 L2：会话锚点行（挂在 PE 记录上、用于标记「本次有反馈发生」）不属于记忆条目。
    pub fn applied_deltas_for_session(
        &self,
        session_id: &str,
        rating: i64,
    ) -> Result<Vec<(String, i64)>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT e.entry_id, u.weight_delta
             FROM usage_log u
             JOIN entries e ON e.uid = u.entry_uid
             WHERE u.session_id = ?1 AND u.event_type = 'score' AND u.score = ?2
               AND e.layer = 'L2'
             ORDER BY e.entry_id ASC",
        )?;
        let rows = stmt.query_map(params![session_id, rating], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 某次会话注入过的条目 `uid`。
    pub fn injected_uids_for_session(&self, session_id: &str) -> Result<Vec<i64>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT entry_uid FROM usage_log
             WHERE session_id = ?1 AND event_type = 'inject'",
        )?;
        let rows = stmt.query_map(params![session_id], |row| row.get::<_, i64>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 某会话被注入过的条目业务 ID。
    pub fn injected_entry_ids_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<String>, AppError> {
        let uids = self.injected_uids_for_session(session_id)?;
        let mut out = Vec::new();
        for uid in uids {
            let entry_id: Option<String> = self
                .conn
                .query_row(
                    "SELECT entry_id FROM entries WHERE uid = ?1",
                    params![uid],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(entry_id) = entry_id {
                out.push(entry_id);
            }
        }
        Ok(out)
    }

    /// 按创建时间前缀取 PE 记录（夜间任务与记录查重用）。
    ///
    /// 形参是**创建时间前缀**而不是日期：`2026-10-21` 命中当天全部记录，
    /// `2026-` 命中当年全部记录。索引存的是 `created_at`（UTC ISO 秒精度）。
    pub fn pe_records_by_created_prefix(
        &self,
        created_prefix: &str,
    ) -> Result<Vec<PeRecord>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT entry_id, title, content, domain, signals, created_at
             FROM entries
             WHERE layer = 'L1' AND created_at LIKE ?1
             ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![format!("{created_prefix}%")], |row| {
            let content: String = row.get(2)?;
            // content 里存的是「原文 ␟ 改写结果」，拆回来才是两个独立字段。
            let (original, optimized) = content
                .split_once(L1_FIELD_SEPARATOR)
                .map(|(head, tail)| (head.to_string(), tail.to_string()))
                .unwrap_or_else(|| (String::new(), content.clone()));

            Ok(PeRecord {
                id: row.get(0)?,
                original_prompt: original,
                optimized_prompt: optimized,
                domain: row.get(3)?,
                signals: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                rating: None,
                created_at: row.get(5)?,
            })
        })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// FTS 全量重建（幂等）。
    pub fn rebuild_fts(&self) -> Result<(), AppError> {
        self.conn
            .execute("INSERT INTO entries_fts(entries_fts) VALUES('rebuild')", [])?;
        Ok(())
    }

    /// FTS 单行增量同步：先把该 rowid 从索引里移除（若此前存在），再写入新值。
    ///
    /// FTS5 外置内容表的 `delete` 命令**只对已在索引里的行有效**：
    /// 对尚未写入索引的新行执行，SQLite 会报
    /// 「database disk image is malformed」并把索引判定为损坏。
    /// 因此只有当这行此前确实存在过时才发 `delete`。
    ///
    /// `previous` 携带的是**旧值**（旧 title / aliases / content）：
    /// `delete` 按旧值计算要移除的词项，传新值会删错词项、留下幽灵行。
    fn sync_fts_row(
        &self,
        uid: i64,
        previous: Option<&(i64, String, String, String)>,
        title: &str,
        content: &str,
    ) -> Result<(), AppError> {
        if let Some((_, old_title, old_aliases, old_content)) = previous {
            self.conn.execute(
                "INSERT INTO entries_fts(entries_fts, rowid, title, aliases, content)
             VALUES ('delete', ?1, ?2, ?3, ?4)",
                params![uid, old_title, old_aliases, old_content],
            )?;
        }
        self.conn.execute(
            "INSERT INTO entries_fts(rowid, title, aliases, content) VALUES (?1, ?2, '', ?3)",
            params![uid, title, content],
        )?;
        Ok(())
    }

    /// 取条目当前的 FTS 三元组（uid + title + aliases + content）。
    fn fts_row_of(
        &self,
        entry_id: &str,
    ) -> Result<Option<(i64, String, String, String)>, AppError> {
        let row = self
            .conn
            .query_row(
                "SELECT uid, title, aliases, content FROM entries WHERE entry_id = ?1",
                params![entry_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        Ok(row)
    }
}

/// 打开库后的 schema 升级：把旧库的 unicode61 FTS 换成 trigram 并重建。
///
/// Markdown 真相源不动；只重建可丢弃的 FTS 索引。
fn migrate_schema(conn: &Connection) -> Result<(), AppError> {
    let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version >= SCHEMA_VERSION {
        return Ok(());
    }
    if version < 2 {
        conn.execute_batch(MIGRATE_FTS_TRIGRAM)?;
    }
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

/// 清洗用户输入，得到可安全交给 FTS5 `MATCH` 的查询串。
///
/// 去掉 FTS5 运算符与引号，避免语法错误；清洗后无可检索字符则返回 `None`
///（调用方应回退到阶段一 domain 路由，而不是发空 MATCH）。
pub fn sanitize_fts_query(raw: &str) -> Option<String> {
    let mut cleaned = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            // FTS5 特殊语法字符一律当空白丢掉，避免 MATCH 解析失败。
            '"' | '\'' | '*' | '(' | ')' | ':' | '^' | '{' | '}' | '[' | ']' | '-' => {
                cleaned.push(' ');
            }
            c if c.is_whitespace() => cleaned.push(' '),
            c => cleaned.push(c),
        }
    }
    let tokens: Vec<&str> = cleaned.split_whitespace().filter(|t| !t.is_empty()).collect();
    if tokens.is_empty() {
        return None;
    }
    // 多词用空格连接：trigram 下按子串匹配；不加引号以免短中文被短语语法卡死。
    Some(tokens.join(" "))
}

type InjectableRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
    i64,
    i64,
    Option<String>,
    String,
);

fn map_injectable_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<InjectableRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
    ))
}

/// 行数据 → 记忆条目。
fn row_to_entry(row: InjectableRow) -> MemoryEntry {
    let (
        id,
        entry_type,
        domain,
        status,
        statement,
        why,
        how_to_apply,
        signals,
        evidence,
        weight,
        hits,
        last_used,
        created,
    ) = row;

    MemoryEntry {
        id,
        entry_type: MemoryType::parse(&entry_type).unwrap_or(MemoryType::Rule),
        domain,
        status: MemoryStatus::parse(&status).unwrap_or(MemoryStatus::Candidate),
        statement: statement.unwrap_or_default(),
        why: why.unwrap_or_default(),
        how_to_apply: how_to_apply.unwrap_or_default(),
        signals: serde_json::from_str(&signals).unwrap_or_default(),
        evidence: serde_json::from_str(&evidence).unwrap_or_default(),
        weight,
        hits,
        last_used,
        created,
    }
}

/// L2 条目的 FTS 标题：取 statement 的前 48 个字符。
fn build_l2_title(entry: &MemoryEntry) -> String {
    let text = entry.statement.trim();
    if text.chars().count() <= 48 {
        return text.to_string();
    }
    let head: String = text.chars().take(48).collect();
    format!("{head}…")
}

/// L2 条目的 FTS 正文：statement + why + how_to_apply。
fn build_l2_content(entry: &MemoryEntry) -> String {
    format!("{}\n{}\n{}", entry.statement, entry.why, entry.how_to_apply)
}

/// 当前时间的 ISO-8601 字符串（秒精度）。
///
/// 时间原语放在基础设施层 `crate::clock`，数据层只管取用，
/// 免得上层为了取「今天是哪天」而伸手进数据层。
use crate::clock::now_iso;

#[cfg(test)]
mod tests {
    use super::*;

    fn memory(id: &str, status: MemoryStatus, weight: i64) -> MemoryEntry {
        MemoryEntry {
            id: id.into(),
            entry_type: MemoryType::Rule,
            domain: "ui-design".into(),
            status,
            statement: format!("{id} 的陈述"),
            why: "理由".into(),
            how_to_apply: "方式".into(),
            signals: vec![],
            evidence: vec![],
            weight,
            hits: 0,
            last_used: None,
            created: "2026-10-21".into(),
        }
    }

    #[test]
    fn candidate_与零权重条目不会被注入() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_memory(&memory("M-UI-001", MemoryStatus::Candidate, 5))
            .unwrap();

        let injectable = db.injectable_memories(Some("ui-design"), 10).unwrap();
        assert!(injectable.is_empty(), "candidate 条目绝不能出现在注入集里");
    }

    #[test]
    fn 注入集按权重降序且过滤零权重() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_memory(&memory("M-A-001", MemoryStatus::Active, 3))
            .unwrap();
        db.upsert_memory(&memory("M-A-002", MemoryStatus::Active, 9))
            .unwrap();
        db.upsert_memory(&memory("M-A-003", MemoryStatus::Active, 0))
            .unwrap();

        let injectable = db.injectable_memories(Some("ui-design"), 10).unwrap();
        assert_eq!(injectable.len(), 2);
        assert_eq!(injectable[0].id, "M-A-002");
        assert_eq!(injectable[1].id, "M-A-001");
    }

    #[test]
    fn pe_id_按年份递增() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.next_pe_id(2026).unwrap(), "PE-2026-001");

        db.upsert_pe_record(&PeRecord {
            id: "PE-2026-001".into(),
            original_prompt: "a".into(),
            optimized_prompt: "b".into(),
            domain: "general".into(),
            signals: vec![],
            rating: None,
            created_at: "2026-10-21T00:00:00Z".into(),
        })
        .unwrap();
        assert_eq!(db.next_pe_id(2026).unwrap(), "PE-2026-002");
    }

    #[test]
    fn pe_记录原文与改写结果可无损拆分() {
        // 原文与改写结果本身都含空行，拆分必须仍然可逆。
        let db = Db::open_in_memory().unwrap();
        db.upsert_pe_record(&PeRecord {
            id: "PE-2026-001".into(),
            original_prompt: "第一段\n\n第二段".into(),
            optimized_prompt: "改写结果\n\n也有空行".into(),
            domain: "general".into(),
            signals: vec![],
            rating: None,
            created_at: "2026-10-21T00:00:00Z".into(),
        })
        .unwrap();

        let records = db.pe_records_by_created_prefix("2026-10-21").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].original_prompt, "第一段\n\n第二段");
        assert_eq!(records[0].optimized_prompt, "改写结果\n\n也有空行");
    }

    #[test]
    fn usage_log_按会话串起注入与评分() {
        let db = Db::open_in_memory().unwrap();
        let uid = db
            .upsert_memory(&memory("M-A-001", MemoryStatus::Active, 1))
            .unwrap();
        assert!(uid > 0);

        db.log_usage("PE-2026-001", uid, "inject", None, 0, None)
            .unwrap();
        db.log_usage("PE-2026-001", uid, "score", Some(5), 1, None)
            .unwrap();

        let injected = db.injected_uids_for_session("PE-2026-001").unwrap();
        assert_eq!(injected, vec![uid]);
        assert_eq!(db.feedback_session_count().unwrap(), 1);
    }

    #[test]
    fn 反馈次数按会话计而不是按条目计() {
        // 一次反馈给两条被注入条目各写一行 score，豁免计数只能算一次使用。
        let db = Db::open_in_memory().unwrap();
        let uid_a = db
            .upsert_memory(&memory("M-A-001", MemoryStatus::Active, 1))
            .unwrap();
        let uid_b = db
            .upsert_memory(&memory("M-A-002", MemoryStatus::Active, 1))
            .unwrap();

        db.log_usage("PE-2026-001", uid_a, "inject", None, 0, None)
            .unwrap();
        db.log_usage("PE-2026-001", uid_b, "inject", None, 0, None)
            .unwrap();
        db.log_usage("PE-2026-001", uid_a, "score", Some(5), 1, None)
            .unwrap();
        db.log_usage("PE-2026-001", uid_b, "score", Some(5), 1, None)
            .unwrap();

        assert_eq!(db.feedback_session_count().unwrap(), 1);
        assert!(db.score_event_exists("PE-2026-001", 5).unwrap());
        assert!(!db.score_event_exists("PE-2026-001", 3).unwrap());

        let deltas = db.applied_deltas_for_session("PE-2026-001", 5).unwrap();
        assert_eq!(deltas.len(), 2);
    }

    #[test]
    fn 未命中计数可累加与归零() {
        let db = Db::open_in_memory().unwrap();
        let uid = db
            .upsert_memory(&memory("M-A-001", MemoryStatus::Active, 1))
            .unwrap();

        assert_eq!(db.bump_miss_streak(uid).unwrap(), 1);
        assert_eq!(db.bump_miss_streak(uid).unwrap(), 2);
        db.reset_miss_streak(uid).unwrap();
        assert_eq!(db.miss_streak_of(uid).unwrap(), 0);
    }

    #[test]
    fn 索引层可按_markdown_裁剪掉已删除条目() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_memory(&memory("M-A-001", MemoryStatus::Active, 1))
            .unwrap();
        db.upsert_memory(&memory("M-A-002", MemoryStatus::Deprecated, 1))
            .unwrap();

        let removed = db.delete_memories_not_in(&["M-A-001".to_string()]).unwrap();
        assert_eq!(removed, 1);
        assert!(db.entry_exists("M-A-001").unwrap());
        assert!(!db.entry_exists("M-A-002").unwrap());
    }

    #[test]
    fn 注入后命中数加一() {
        let db = Db::open_in_memory().unwrap();
        let uid = db
            .upsert_memory(&memory("M-A-001", MemoryStatus::Active, 1))
            .unwrap();
        db.mark_injected(uid).unwrap();

        let hits: i64 = db
            .conn
            .query_row(
                "SELECT hits FROM entries WHERE uid = ?1",
                params![uid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(hits, 1);
    }

    /// 统计 FTS 索引中命中给定查询的行数。
    fn fts_hits(db: &Db, query: &str) -> i64 {
        db.conn
            .query_row(
                "SELECT COUNT(*) FROM entries_fts WHERE entries_fts MATCH ?1",
                params![query],
                |row| row.get(0),
            )
            .unwrap()
    }

    #[test]
    fn fts_索引可重建且能命中() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_memory(&memory("M-A-001", MemoryStatus::Active, 1))
            .unwrap();
        db.rebuild_fts().unwrap();

        // trigram 最短有效查询为 3 字；「的陈述」正好三个汉字。
        assert!(
            fts_hits(&db, "的陈述") > 0,
            "trigram 应对 ≥3 字中文子串命中 L2 正文"
        );
    }

    #[test]
    fn 同一条目重复写入不会让_fts_索引损坏() {
        // FTS5 外置内容表的 'delete' 命令只对**已在索引里**的行有效：
        // 首次写入时就发 delete，SQLite 会直接判定索引损坏（database disk image is malformed）。
        let db = Db::open_in_memory().unwrap();
        let mut entry = memory("M-A-001", MemoryStatus::Active, 1);
        db.upsert_memory(&entry).unwrap();

        entry.statement = "改写后的陈述".into();
        db.upsert_memory(&entry).unwrap();

        // 覆盖写入后必须**在重建之前**就能查到新内容：
        // 先 rebuild 会把索引整个重算一遍，从而掩盖增量同步的缺陷。
        assert!(
            fts_hits(&db, "改写后的陈述") > 0,
            "覆盖写入后 FTS 索引应能命中新内容"
        );
        // 旧 statement 含条目 ID；覆盖后正文不再含该 ID（不要对 MATCH 直接塞带 `-` 的串，
        // FTS5 会把 `-` 当运算符）。
        assert!(
            db.injectable_by_bm25("M-A-001", None, 10)
                .unwrap()
                .is_empty(),
            "旧标题片段应在覆盖写入后从索引里消失"
        );

        db.rebuild_fts().unwrap();
        assert!(
            fts_hits(&db, "改写后的陈述") > 0,
            "重建后 FTS 索引应仍能命中新内容"
        );
    }

    /// 阶段二：≥3 字走 trigram MATCH；双字中文走 LIKE 回退。
    #[test]
    fn fts_trigram_中文子串可命中() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_memory(&memory("M-A-001", MemoryStatus::Active, 1))
            .unwrap();
        db.rebuild_fts().unwrap();

        assert!(fts_hits(&db, "的陈述") > 0, "trigram 应对 ≥3 字中文命中");
        // 双字不进 trigram 词项，但 injectable_by_bm25 会 LIKE 回退。
        let short = db.injectable_by_bm25("陈述", Some("ui-design"), 10).unwrap();
        assert_eq!(short.len(), 1, "双字中文应经 LIKE 回退命中");
        assert_eq!(short[0].id, "M-A-001");
    }

    #[test]
    fn bm25_检索过滤candidate并按相关性返回() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_memory(&memory("M-A-001", MemoryStatus::Active, 1))
            .unwrap();
        db.upsert_memory(&memory("M-A-002", MemoryStatus::Candidate, 9))
            .unwrap();
        db.rebuild_fts().unwrap();

        let hits = db
            .injectable_by_bm25("的陈述", Some("ui-design"), 10)
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "M-A-001");
    }

    #[test]
    fn sanitize_fts_query_去掉运算符() {
        assert_eq!(sanitize_fts_query("品牌色 ^ 约束").as_deref(), Some("品牌色 约束"));
        assert_eq!(sanitize_fts_query("***").as_deref(), None);
        assert_eq!(sanitize_fts_query("   ").as_deref(), None);
        assert_eq!(sanitize_fts_query("陈述").as_deref(), Some("陈述"));
    }

    #[test]
    fn 空query的bm25返回空集() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_memory(&memory("M-A-001", MemoryStatus::Active, 1))
            .unwrap();
        let hits = db.injectable_by_bm25("***", None, 10).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn schema_version_为二() {
        let db = Db::open_in_memory().unwrap();
        let version: i32 = db
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn 回滚后表全部消失() {
        let db = Db::open_in_memory().unwrap();
        db.rollback().unwrap();
        let count: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='entries'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
}
