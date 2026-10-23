// 用内存 SQLite 实跑 src-tauri/src/storage/db.rs 里的 DDL 与新增查询。
//
// db.rs 的 schema 与 docs/database-design.md 必须逐字一致，
// 而本机没有 Rust 工具链跑 `cargo test`，因此把 DDL 与查询搬到这里实跑一次：
// - 正向迁移（幂等执行两次）
// - 新增的反馈计数 / 幂等查询 / 未命中计数 / 索引裁剪
// - usage_log 的 CHECK 约束（score 事件必须带 score，inject/miss 必须不带）
// - 外键级联（删除 L2 条目时 usage_log 随之清理）
//
// 用法：node scripts/check-db-sql.mjs

import { readFileSync } from 'node:fs';
import { DatabaseSync } from 'node:sqlite';

const dbSource = readFileSync('src-tauri/src/storage/db.rs', 'utf8');

function extractConst(name) {
  const re = new RegExp(`const ${name}: &str = r#"(.*?)"#;`, 's');
  const m = re.exec(dbSource);
  if (!m) throw new Error(`未能在 db.rs 中找到 ${name}`);
  return m[1];
}

const MIGRATION_001 = extractConst('MIGRATION_001');
const ROLLBACK_SQL = extractConst('ROLLBACK_SQL');

let failures = 0;
function check(name, fn) {
  try {
    fn();
    console.log(`  ✓ ${name}`);
  } catch (err) {
    failures += 1;
    console.log(`  ✗ ${name} —— ${err.message}`);
  }
}
function assert(cond, message) {
  if (!cond) throw new Error(message || '断言不成立');
}
function assertEq(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(`${message || '不相等'}：期望 ${JSON.stringify(expected)}，实际 ${JSON.stringify(actual)}`);
  }
}

console.log('db.rs DDL 与查询实跑');

const db = new DatabaseSync(':memory:');
db.exec('PRAGMA foreign_keys = ON;');
db.exec('PRAGMA journal_mode = WAL;');
db.exec(MIGRATION_001);
// 幂等：迁移再执行一次不应报错
db.exec(MIGRATION_001);

const now = '2026-10-21T02:00:00Z';

function insertMemory(id, { layer = 'L2', domain = 'ui-design', status = 'active', weight = 1, miss = 0 } = {}) {
  db.prepare(
    `INSERT INTO entries (entry_id, layer, entry_type, domain, status, title, aliases, content,
       statement, why, how_to_apply, signals, evidence, weight, hits, miss_streak, created_at, updated_at)
     VALUES (?1, ?2, ?3, ?4, ?5, '', '', '', 's', 'w', 'h', '[]', '[]', ?6, 0, ?7, ?8, ?8)`,
  ).run(id, layer, layer === 'L1' ? 'pe_record' : 'rule', domain, status, weight, miss, now);
}

function insertPeRecord(id, content = 'origopt') {
  db.prepare(
    `INSERT INTO entries (entry_id, layer, entry_type, domain, status, title, aliases, content,
       signals, weight, hits, created_at, updated_at)
     VALUES (?1, 'L1', 'pe_record', 'general', 'active', 'orig', '', ?2, '[]', 0, 0, ?3, ?3)`,
  ).run(id, content, now);
}

function logUsage(sessionId, entryUid, eventType, score, weightDelta = 0, note = null) {
  db.prepare(
    `INSERT INTO usage_log (session_id, entry_uid, event_type, score, weight_delta, note, created_at, updated_at)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)`,
  ).run(sessionId, entryUid, eventType, score, weightDelta, note, now);
}

function uidOf(entryId) {
  return db.prepare('SELECT uid FROM entries WHERE entry_id = ?1').get(entryId)?.uid ?? null;
}

check('四条表与 FTS 虚表建好', () => {
  const names = db.prepare("SELECT name FROM sqlite_master WHERE type IN ('table')").all().map((r) => r.name);
  for (const expected of ['entries', 'entries_fts', 'entry_vectors', 'usage_log']) {
    assert(names.includes(expected), `缺少表 ${expected}`);
  }
});

check('feedback_session_count 按 (会话, 评分) 组合去重', () => {
  db.exec('DELETE FROM usage_log');
  db.exec('DELETE FROM entries');
  insertMemory('M-A-001');
  insertMemory('M-A-002');
  const uidA = uidOf('M-A-001');
  const uidB = uidOf('M-A-002');

  // 一次反馈写两条 score 行（两条被注入条目）
  logUsage('PE-2026-001', uidA, 'inject', null);
  logUsage('PE-2026-001', uidB, 'inject', null);
  logUsage('PE-2026-001', uidA, 'score', 5, 1);
  logUsage('PE-2026-001', uidB, 'score', 5, 1);

  const count = db.prepare(
    `SELECT COUNT(*) AS c FROM (SELECT DISTINCT session_id, score FROM usage_log WHERE event_type = 'score')`,
  ).get().c;
  assertEq(count, 1, '一次反馈应只算一次使用');

  // 同会话换评分 → 算两次使用
  logUsage('PE-2026-001', uidA, 'score', 3, 0);
  const count2 = db.prepare(
    `SELECT COUNT(*) AS c FROM (SELECT DISTINCT session_id, score FROM usage_log WHERE event_type = 'score')`,
  ).get().c;
  assertEq(count2, 2, '同会话换评分应算两次使用');
});

check('score_event_exists 精确到 (会话, 评分)', () => {
  const exists = db.prepare(
    `SELECT COUNT(*) AS c FROM usage_log WHERE session_id = ?1 AND event_type = 'score' AND score = ?2`,
  ).get('PE-2026-001', 5).c;
  assert(exists > 0, '应命中已提交的评分');
  const missing = db.prepare(
    `SELECT COUNT(*) AS c FROM usage_log WHERE session_id = ?1 AND event_type = 'score' AND score = ?2`,
  ).get('PE-2026-001', 1).c;
  assertEq(missing, 0, '未提交过的评分不应命中');
});

check('applied_deltas_for_session 只取 L2，排除会话锚点行', () => {
  insertPeRecord('PE-2026-002');
  const peUid = uidOf('PE-2026-002');
  // 会话锚点行：挂在 PE 记录上，delta 为 0
  logUsage('PE-2026-002', peUid, 'score', 4, 0);
  const rows = db.prepare(
    `SELECT e.entry_id AS id, u.weight_delta AS delta
     FROM usage_log u JOIN entries e ON e.uid = u.entry_uid
     WHERE u.session_id = ?1 AND u.event_type = 'score' AND u.score = ?2 AND e.layer = 'L2'`,
  ).all('PE-2026-002', 4);
  assertEq(rows.length, 0, '锚点行不应被当成 L2 权重变化');
});

check('miss 事件允许 score 为 NULL', () => {
  const uid = uidOf('M-A-001');
  logUsage('2026-10-21', uid, 'miss', null);
  const row = db.prepare("SELECT COUNT(*) AS c FROM usage_log WHERE event_type = 'miss'").get();
  assertEq(row.c, 1);
});

check('score 事件不允许 score 为 NULL（CHECK 约束生效）', () => {
  let rejected = false;
  try {
    logUsage('PE-2026-003', uidOf('M-A-001'), 'score', null);
  } catch {
    rejected = true;
  }
  assert(rejected, 'CHECK 约束应拒绝 score 为空的 score 事件');
});

check('inject 事件不允许带 score（CHECK 约束生效）', () => {
  let rejected = false;
  try {
    logUsage('PE-2026-003', uidOf('M-A-001'), 'inject', 5);
  } catch {
    rejected = true;
  }
  assert(rejected, 'CHECK 约束应拒绝带 score 的 inject 事件');
});

check('未命中计数可累加与归零', () => {
  const uid = uidOf('M-A-001');
  db.prepare('UPDATE entries SET miss_streak = 0 WHERE uid = ?1').run(uid);
  db.prepare('UPDATE entries SET miss_streak = miss_streak + 1 WHERE uid = ?1').run(uid);
  const streak = db.prepare('SELECT miss_streak FROM entries WHERE uid = ?1').get(uid).miss_streak;
  assertEq(streak, 1);
  db.prepare('UPDATE entries SET miss_streak = 0 WHERE uid = ?1').run(uid);
  const reset = db.prepare('SELECT miss_streak FROM entries WHERE uid = ?1').get(uid).miss_streak;
  assertEq(reset, 0);
});

check('索引裁剪只删 L2，且 usage_log 随之级联清理', () => {
  insertMemory('M-KEEP-001');
  insertMemory('M-DROP-001');
  const dropUid = uidOf('M-DROP-001');
  logUsage('PE-2026-009', dropUid, 'inject', null);

  const all = db.prepare("SELECT entry_id FROM entries WHERE layer = 'L2'").all().map((r) => r.entry_id);
  const keep = ['M-KEEP-001'];
  let removed = 0;
  for (const id of all) {
    if (!keep.includes(id)) {
      db.prepare('DELETE FROM entries WHERE entry_id = ?1').run(id);
      removed += 1;
    }
  }
  assertEq(removed, all.length - keep.length, '删除条数不符');
  assert(db.prepare("SELECT COUNT(*) AS c FROM entries WHERE layer = 'L2'").get().c === 1, 'L2 应只剩一条');
  const usage = db.prepare('SELECT COUNT(*) AS c FROM usage_log WHERE entry_uid = ?1').get(dropUid).c;
  assertEq(usage, 0, '外键级联应清掉该条目的 usage_log');
  const pe = db.prepare("SELECT COUNT(*) AS c FROM entries WHERE layer = 'L1'").get().c;
  assert(pe > 0, 'L1 记录不应被裁剪');
});

check('score 事件允许带 note 且能原样落库', () => {
  // 前一个用例的索引裁剪已删掉除 M-KEEP-001 外的全部 L2 条目，
  // 这里必须取当轮仍存在的 uid，否则 usage_log.entry_uid 的外键会失败。
  const uid = uidOf('M-KEEP-001');
  assert(uid !== null, 'M-KEEP-001 应仍在索引层');
  logUsage('PE-2026-010', uid, 'score', 4, 1, '分层结构比上次清楚');
  const row = db.prepare(
    "SELECT note FROM usage_log WHERE session_id = 'PE-2026-010' AND event_type = 'score'",
  ).get();
  assertEq(row?.note, '分层结构比上次清楚', '复核备注未原样落库');
});

check('FTS 增量同步：先 delete 再 insert 不会损坏索引', () => {
  // FTS5 外置内容表的 'delete' 命令只对**已在索引里**的行有效：
  // 首次写入时就发 delete，SQLite 会判定索引损坏（database disk image is malformed）。
  // 这里复刻 db.rs 的 upsert_memory：记旧值 → 改内容表 → 按旧值 delete → 按新值 insert。
  db.prepare(
    `INSERT INTO entries (entry_id, layer, entry_type, domain, status, title, aliases, content,
       statement, why, how_to_apply, signals, evidence, weight, hits, created_at, updated_at)
     VALUES ('M-FTS-001', 'L2', 'rule', 'ui-design', 'active', '旧标题', '', '旧正文', 's', 'w', 'h', '[]', '[]', 1, 0, ?1, ?1)`,
  ).run(now);
  const uid = uidOf('M-FTS-001');

  // 首次写入：索引里还没有这行 → 只能 insert
  db.prepare(`INSERT INTO entries_fts(rowid, title, aliases, content) VALUES (?, ?, '', ?)`)
    .run(uid, '旧标题', '旧正文');
  // 覆盖写入：内容表改成新值 → 按旧值 delete → 按新值 insert
  db.prepare(`UPDATE entries SET title = '新标题', content = '新正文' WHERE uid = ?1`).run(uid);
  db.prepare(`INSERT INTO entries_fts(entries_fts, rowid, title, aliases, content) VALUES ('delete', ?, ?, '', ?)`)
    .run(uid, '旧标题', '旧正文');
  db.prepare(`INSERT INTO entries_fts(rowid, title, aliases, content) VALUES (?, ?, '', ?)`)
    .run(uid, '新标题', '新正文');

  const hits = db.prepare(`SELECT COUNT(*) AS c FROM entries_fts WHERE entries_fts MATCH '新标题'`).get().c;
  assertEq(hits, 1, '覆盖写入后应能命中新内容');
  const stale = db.prepare(`SELECT COUNT(*) AS c FROM entries_fts WHERE entries_fts MATCH '旧标题'`).get().c;
  assertEq(stale, 0, '旧内容的词项应已从索引里移除');
});

check('FTS trigram：≥3 字中文可 MATCH', () => {
  // trigram 最短有效查询为 3 字；双字由应用层 LIKE 回退（见 db.rs injectable_by_bm25）。
  db.prepare(
    `INSERT INTO entries (entry_id, layer, entry_type, domain, status, title, aliases, content,
       statement, why, how_to_apply, signals, evidence, weight, hits, created_at, updated_at)
     VALUES ('M-CJK-001', 'L2', 'rule', 'ui-design', 'active', 'M-CJK-001 的陈述', '', 'M-CJK-001 的陈述', 's', 'w', 'h', '[]', '[]', 1, 0, ?1, ?1)`,
  ).run(now);
  db.exec("INSERT INTO entries_fts(entries_fts) VALUES('rebuild')");

  const whole = db.prepare(`SELECT COUNT(*) AS c FROM entries_fts WHERE entries_fts MATCH '的陈述'`).get().c;
  assert(whole > 0, '≥3 字中文应能 MATCH 命中');
});

check('entry_created_at 命中精确 ID', () => {
  const created = db.prepare('SELECT created_at FROM entries WHERE entry_id = ?1').get('PE-2026-002');
  assertEq(created?.created_at, now);
  const missing = db.prepare('SELECT created_at FROM entries WHERE entry_id = ?1').get('PE-9999-999');
  assertEq(missing, undefined, '不存在的 ID 应返回 undefined');
});

check('外键检查无残留', () => {
  const violations = db.prepare('PRAGMA foreign_key_check').all();
  assertEq(violations.length, 0, `外键违规：${JSON.stringify(violations)}`);
});

check('回滚脚本清空全部索引层对象', () => {
  db.exec(ROLLBACK_SQL);
  const count = db.prepare("SELECT COUNT(*) AS c FROM sqlite_master WHERE name = 'entries'").get().c;
  assertEq(count, 0);
});

console.log(failures === 0 ? '\n全部通过。' : `\n${failures} 条不通过。`);
process.exit(failures === 0 ? 0 : 1);
