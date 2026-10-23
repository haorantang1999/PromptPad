# Prompt Pad · 产品规格说明书（spec.md）

> 事实基线：`AGENTS.md`（唯一事实基线）。本文所有功能、约束、字段、阈值均来自 `AGENTS.md` 第 1/3/4/6/9 节与 `docs/01-案例调研与技术选型.md`、`docs/02-私人记忆库设计.md`。
> 未能从上述来源确认的内容一律标注「待确认」，不做推断与臆造。
> 字段命名统一 `snake_case`；TypeScript 类型与 Rust/serde 字段一一对应。

---

## 1 目标

### 1.1 产品定位

**Prompt Pad（提示词优化 Pad）** —— 常驻 Windows 系统最上层、全局热键 `Ctrl+Space` 唤起、带私人记忆的提示词优化 Pad。

### 1.2 目标用户

- 开发者本人（重度 AI 编程/写作助手的个人用户）。
- 1 名个人开发者 + AI 编码 agent 协作开发，无硬性工期。
- 单一角色：本机用户。不存在多角色与权限分级。

### 1.3 核心场景

任意应用中写好草稿提示词 → `Ctrl+Space` 唤起 Pad → 粘贴/输入 → 一键改写（自动注入匹配的记忆经验）→ 复制回原应用。**焦点必须能还给原窗口。**

### 1.4 产品原则

- **TINY WHEN UNUSED**：不用时极小，收起态仅 40–56px 的 DOT。
- **POWERFUL WHEN OPENED**：打开时强大，提供一键改写与记忆注入。
- **数据不出本机**：全部数据仅存本机 SQLite 与 Markdown 文件。
- **对话是草稿，文件是大脑**：AI 不记住任何事，AI 只记住「去哪里找」。

### 1.5 产品目标（可判定）

| 编号 | 目标 | 判定方式 |
|---|---|---|
| G-1 | 任意应用中可用全局热键 `Ctrl+Space` 唤起 Pad | 按下 `Ctrl+Space`，Pad 面板可见 |
| G-2 | 一键把草稿提示词改写为可用的提示词 | 改写请求返回非空改写文本并可复制 |
| G-3 | 越用越聪明：使用过程自动沉淀为可复用记忆 | 夜间任务结束后 `memory.md` 新增条目且带 `evidence` 来源 |
| G-4 | 不打断工作流：Pad 置顶但不抢焦点 | 收起态下原窗口键盘焦点不被夺走，改写后可回到原窗口 |
| G-5 | 数据不出本机 | 记忆与记录仅存在本机 Markdown 与 SQLite；模型密钥不写入 `memory/` 与 git |

---

## 2 功能

### 2.1 功能清单（编号即开发顺序，行为照抄 `AGENTS.md` 第 1 节）

| # | 功能 | 关键行为 | 关联验收标准 |
|---|---|---|---|
| ① | 透明无边框常驻窗口 | 八项窗口配置全部生效：`transparent`、`decorations:false`、`alwaysOnTop`、`skipTaskbar`、`resizable:false`、`shadow:false`、`focus:false`，`label:"pad"` | AC-01 ~ AC-03 |
| ② | 常驻 DOT | 40–56px；拖拽；置顶；`set_focusable(false)` 不抢焦点；位置持久化「**哪块屏 + 屏内百分比坐标**」（非绝对像素）；收起态 `set_ignore_cursor_events(true)` 点击穿透，展开态关闭 | AC-04 ~ AC-07 |
| ③ | 展开/收起 | 点击展开、点外部或 `Esc` 收起；动画 150–250ms；屏幕边缘展开方向自适应（靠上向下、靠右向左）且**面板绝不出屏**；独占全屏时临时隐藏 + `WS_EX_TOPMOST` | AC-08 ~ AC-11 |
| ④ | 最小输入框 + 一键改写 | 调用当前模型改写提示词（**先不做记忆**） | AC-12 ~ AC-14 |
| ⑤ | SQLite 落库 | 记录 `PE-YYYY-NNN`（`prompt_engineering.md` 事实层，永久、逐字不可改、只追加） | AC-15 ~ AC-18 |
| ⑥ | 全局热键 + 托盘 + 自启 | `Ctrl+Space`；托盘菜单（显示/隐藏/退出/设置）；开机自启；单实例（二次启动唤醒已有实例） | AC-19 ~ AC-22 |
| ⑦ | 检索层 | 阶段一 domain 路由（回退）→ 阶段二 FTS5 trigram BM25（有 query 即启用，列权重 title 10× / aliases 5× / content 1×）→ 阶段三 sqlite-vec KNN + RRF（未启用） | AC-23 ~ AC-27 |
| ⑧ | 夜间记忆沉淀 | 扫描当天 PE 记录 → signals 去重合并 → 相似度匹配（匹配则 weight+1 补 evidence / 无匹配且达标新建 candidate / 矛盾挂起人工裁决）→ 检查 `token_budget` → 重建/增量更新索引 | AC-28 ~ AC-32 |
| ⑨ | 记忆注入 | `domain` 匹配 + `status=active` + `weight>0`，按 weight 降序取 Top-K；只注入 `statement`+`why`+`how_to_apply`；总量 ≤800 token；**无匹配时明确告知「暂无历史经验」** | AC-33 ~ AC-36 |
| ⑩ | 权重更新 | 评分 ≥4 → `+1`；无改善 → `0`；变差 → `−2`；未命中 → 触发连续未命中计数 | AC-37 ~ AC-40 |
| ⑪ | 冷启动 | 批量回填 / AI 迁移导入 preference / **前 10 次使用豁免权重惩罚** | AC-41 ~ AC-43 |

### 2.2 实施顺序约束（必须体现，不得越过）

1. **第 4 步先不做记忆**：功能④「一键改写」落地时，模型请求上下文中不得包含任何记忆条目内容；记忆注入（⑨）在功能⑤之后才具备数据基础。
2. **第 7 步先纯 FTS5**：检索层实现顺序为「接入检索层（先纯 FTS5，确认有需要再加向量）」。在检索层未积累 20–30 条真实记录前，**不得引入向量库依赖与调用路径**；阶段一/二不得存在 sqlite-vec 加载调用。
3. **第 8 步再做夜间沉淀**：自动记忆沉淀依赖 L1 事实层的 PE 记录，功能⑧必须晚于功能⑤落地。
4. **阶段三向量库的启用判据**：按 PE 记录总数判定是否从 `multilingual-e5-small`（~117MB / 384 维）切换到 `bge-m3`（1024 维）；**换模型必须重建全库索引，且每条向量存模型名**。

### 2.3 记忆四层架构

| 层 | 载体 | 约束 |
|---|---|---|
| L0 临时 | 对话上下文 | 会话结束即弃，不落库 |
| L1 事实 | `memory/prompt_engineering.md` | 永久、逐字不可改、只追加；ID `PE-YYYY-NNN`；含 `signals` 字段 |
| L2 记忆 | `memory/memory.md` | **≤800 token 硬上限**；ID `M-<域缩写>-<序号>`；字段 `type`(rule\|preference\|pitfall\|asset) / `domain` / `status`(candidate\|active\|deprecated) / `statement` / `why` / `how_to_apply` / `signals` / `evidence` / `weight` / `hits` / `last_used` / `created` |
| L3 索引 | SQLite | 可重建，以 Markdown 为准；表 `entries` / `entries_fts` / `entry_vectors` / `usage_log` |

### 2.4 状态机

```
candidate ──(weight ≥2 且无失败)──> active ──(weight < −1 或连续 3 次未命中)──> deprecated
```

- `candidate` **不参与注入**；`deprecated` **不占 token 预算**。
- L1 → L2 的晋升门槛：一条 L1 记录产生的信号，只有在**被验证过至少一次**后才允许晋升为 L2 记忆；未验证的候选规则留在 L1 的 `signals` 字段里。

### 2.5 防污染四道防线

1. **人工裁决闸门**：矛盾条目、跨 domain 推广的条目，一律挂起不自动写入。
2. **候选不注入**：新提炼的规则先处于 `candidate`，至少被验证 2 次才升 `active`。
3. **全量可读**：`memory.md` 是纯 Markdown，随时能打开看、能手改、能 diff；记忆不得只存在于向量库。
4. **版本控制**：整个 `memory/` 目录纳入 git，每日一提交。

### 2.6 权限边界（单一角色下的两类闸门）

- **人工裁决闸门**：矛盾条目与跨 domain 推广条目必须经用户人工确认才写入 `memory.md`。
- **设置项**：API 密钥、模型三选一、开机自启开关仅用户可改。

---

## 3 技术栈

### 3.1 前端（均已对 npm registry 核验）

| 包 | 版本 |
|---|---|
| `react` | `18.3.1` |
| `react-dom` | `18.3.1` |
| `@types/react` | `18.3.31` |
| `@types/react-dom` | `18.3.7` |
| `vite` | `6.4.5` |
| `@vitejs/plugin-react` | `4.7.0` |
| `typescript` | `5.9.3` |
| `tailwindcss` | `4.3.3` |
| `@tailwindcss/vite` | `4.3.3` |
| `vitest` | `3.2.7` |
| `zustand` | `5.0.15` |
| `@tauri-apps/api` | `2.12.4` |
| `@tauri-apps/plugin-global-shortcut` | `2.4.1` |
| `@tauri-apps/plugin-autostart` | `2.7.0` |
| `@tauri-apps/plugin-clipboard-manager` | `2.5.0` |
| `@tauri-apps/cli`（devDependency） | `2.12.3` |

### 3.2 Rust 后端（均已对 crates.io 核验）

| crate | 版本 / 说明 |
|---|---|
| `tauri` | `2.12.4`（feature `tray-icon`） |
| `tauri-build` | `2.7.3` |
| `tauri-plugin-global-shortcut` | `2.4.1` |
| `tauri-plugin-single-instance` | `2.5.4` |
| `tauri-plugin-autostart` | `2.7.0` |
| `tauri-plugin-clipboard-manager` | `2.5.0` |
| `serde` | `1.0.229` |
| `serde_json` | `1.0.154` |
| `tokio` | `1.54.0` |
| `rusqlite` | `0.40.2`（features `bundled` + `load_extension` + `vtab` + `functions` + `backup`） |
| `reqwest` | `0.13.5` |
| `anyhow` | `1.0.104` |
| `windows` | `0.62.2` |

### 3.3 运行时

Windows 11 · Node.js 22.22.2 · pnpm 12.11.2 · Rust stable（MSVC 工具链）。

### 3.4 外壳与插件

- 外壳：**Tauri 2**。禁止 Electron 及任何 Node 进程常驻方案。
- 插件清单（**仅此四个**）：`global-shortcut`、`single-instance`、`autostart`、`clipboard-manager`。
- 托盘图标属 Tauri 2 核心 `tauri::tray`，**无独立托盘插件**。
- 平台：Windows 优先（可扩 macOS），**本期不做 Linux**。

### 3.5 存储与嵌入

| 项 | 选型 |
|---|---|
| 事实层 | Markdown `memory/prompt_engineering.md`（只追加） |
| 记忆层 | Markdown `memory/memory.md`（≤800 token 硬上限） |
| 索引层 | SQLite + FTS5 + sqlite-vec（sqlite-vec 为**阶段三**） |
| 嵌入推理 | ONNX 本地推理 |
| 阶段一嵌入模型 | `multilingual-e5-small`（~117MB / 384 维） |
| 阶段三嵌入模型 | `bge-m3`（1024 维）；切换后必须重建全库索引 |
| SQLite 迁移 | 只允许向前追加的幂等迁移；L3 索引可随时重建，**以 Markdown 文件内容为准** |

### 3.6 大模型（ 三选一可配）

`minimax` / `openai_compat` / `ollama`。提供方的基址、模型名、超时等由用户在设置中配置，产品不内置默认值（待确认：各提供方的默认模型名与基址取值）。

### 3.7 部署

Windows 单机安装包（Tauri 打包，NSIS）。数据仅存本机 SQLite 与 Markdown，无服务端。

### 3.8 命令契约

后端命令经 `#[tauri::command]` 暴露，共 8 个（签名以 `docs/interface-doc.md` 为准）：`toggle_panel`、`optimize_prompt`、`save_pe_record`、`submit_feedback`、`retrieve_memories`、`run_nightly_job`、`get_config`、`set_config`。每个命令独立文件于 `src-tauri/src/commands/`，统一在 `commands/mod.rs` 导出，`#[serde(rename_all = "snake_case")]`。

### 3.9 编码约定

- 前端：函数组件 + TypeScript `strict: true`；Tailwind 原子类，禁止写自定义 CSS 文件；**组件不得直接 `invoke`**，一律经 `src/api/client.ts`。
- 后端：统一 `AppError` 枚举 + `thiserror` 风格的手写 Display，区分**可恢复**（返回用户可见提示）与**不可恢复**（阻止写入）。
- 阈值、路径、URL、密钥一律经参数或配置传入，**无硬编码**；API key 存环境变量或系统钥匙串，禁止写入 `memory/` 与 git。
- 注释只写 WHY；代码注释用中文，标识符用英文。

---

## 4 用户故事

> 唯一角色：**本机用户（开发者本人）**。夜间任务与检索属系统行为，写在「系统响应」中，不另立角色。

| 编号 | 用户故事 | 优先级 | Given / When / Then | 关联验收标准 |
|---|---|---|---|---|
| US-01 | 作为常驻工具的使用者，我希望在任意应用中用 `Ctrl+Space` 唤起 Pad | P0 | Given Pad 已启动且处于收起态 / When 在任意前台应用按下 `Ctrl+Space` / Then Pad 面板展开并显示输入框 | AC-19 |
| US-02 | 作为写长提示词的开发者，我希望粘贴草稿后一键得到改写结果 | P0 | Given 面板已展开且输入框有内容 / When 点击「一键改写」 / Then 返回改写文本且可复制回原应用 | AC-13、AC-14 |
| US-03 | 作为在编辑器里连续工作的人，我希望 Pad 不抢走我的键盘焦点 | P0 | Given Pad 收起态常驻 / When 我在原编辑器中输入 / Then 焦点始终在原窗口，Pad 点击后焦点可还给原窗口 | AC-03、AC-05、AC-14 |
| US-04 | 作为多显示器用户，我希望 DOT 换屏或改分辨率后不跑出屏幕 | P1 | Given DOT 已拖到副屏 / When 重启应用或拔掉副屏 / Then DOT 出现在同一块屏的同一相对位置或安全回落到主屏 | AC-07 |
| US-05 | 作为追求桌面整洁的用户，我希望 Pad 平时只有一个小圆点 | P0 | Given 未使用 Pad / When 面板收起 / Then 屏幕上仅存在 40–56px 的 DOT，且透明区域点击穿透 | AC-04、AC-06 |
| US-06 | 作为想沉淀经验的开发者，我希望每次改写都被记录下来 | P1 | Given 改写成功 / When 系统落库 / Then 生成 `PE-YYYY-NNN` 记录并可回溯原文 | AC-15、AC-16、AC-17 |
| US-07 | 作为希望 Pad 越来越懂我的用户，我希望常用手法被自动记住并在下次注入 | P2 | Given 存在 active 且 weight>0 的记忆条目且 domain 匹配 / When 我发起改写 / Then 改写请求中带有该条目的 statement/why/how_to_apply | AC-33、AC-34、AC-35 |
| US-08 | 作为对空结果敏感的用户，我希望没有可注入经验时被明确告知 | P2 | Given 记忆库为空或无匹配条目 / When 我发起改写 / Then 界面明确显示「暂无历史经验」 | AC-36、AC-43 |
| US-09 | 作为愿意给反馈的用户，我希望评分能影响哪些记忆被优先使用 | P2 | Given 本次改写注入了记忆 / When 我提交评分 / Then 评分 ≥4 的条目 weight +1，变差的条目 weight −2 | AC-37、AC-38 |
| US-10 | 作为新装用户，我希望能把历史经验搬进来 | P3 | Given 记忆库为空 / When 我执行批量回填或导入偏好总结 / Then 生成 candidate 条目（不自动生效）待验证 | AC-41、AC-42 |
| US-11 | 作为在意桌面后台占用的人，我希望 Pad 不常驻一整套 Node 进程 | P0 | Given 应用运行中 / When 我查看进程构成 / Then 无常驻 Node/Electron 进程，仅 Tauri 应用本体 | AC-46 |

---

## 5 数据模型

> 统一 `snake_case`；主键与外键关系以注释显式标注。类型与 Rust/serde 字段一一对应，集中于 `src/api/types.ts`。

```ts
// ============================================================
// L1 事实层：PE 记录
// 载体：memory/prompt_engineering.md（永久、逐字不可改、只追加）与 memory/records/
// ============================================================

/** PE 记录：L1 事实层的一条提示词工程记录 */
export interface PeRecord {
  /** 主键。格式 `PE-YYYY-NNN`：YYYY 为年份，NNN 为当年顺序号，只追加不复用 */
  id: string;
  /** 创建时间（ISO 8601） */
  created_at: string;
  /** 领域标签，与 MemoryEntry.domain 对齐，用于检索路由 */
  domain: string;
  /** 提示词原文，逐字保存、不可改写 */
  prompt: string;
  /** 改写结果原文 */
  optimization: string;
  /** 本次采用的优化策略描述 */
  strategy: string;
  /** 从本条记录提炼出的信号标记集合，如 `S-HARD-SOFTSPLIT` */
  signals: string[];
  /** 使用结果记录 */
  result: string;
  /** 用户评分（1-5，null 表示未评分）；权重规则以 ≥4 判定为采纳 */
  feedback_score: number | null;
  /** 记录来源：`chat` = Pad 内改写产生；`import` = 冷启动批量回填导入 */
  source: 'chat' | 'import';
}

// ============================================================
// L2 记忆层：Memory 条目
// 载体：memory/memory.md（≤800 token 硬上限，纯 Markdown 全量可读可 diff）
// ============================================================

/** Memory 条目：L2 记忆层的一条可泛化规则 */
export interface MemoryEntry {
  /** 主键。格式 `M-<域缩写>-<序号>`（域缩写取值表待确认） */
  id: string;
  /** 一句话规则名，同时作为 FTS5 的 title 列 */
  title: string;
  /** 条目类型 */
  type: 'rule' | 'preference' | 'pitfall' | 'asset';
  /** 领域标签，与 PeRecord.domain 对齐 */
  domain: string;
  /** 状态机状态：candidate 不参与注入，deprecated 不占 token 预算 */
  status: 'candidate' | 'active' | 'deprecated';
  /** 规则陈述，一条只说一件事 */
  statement: string;
  /** 原因，注入时携带 */
  why: string;
  /** 可复制的应用方式，注入时携带 */
  how_to_apply: string;
  /** 信号集合，用于与 PE 记录的 signals 去重合并 */
  signals: string[];
  /** 证据来源集合，元素为 PeRecord.id（外键 → PeRecord.id） */
  evidence: string[];
  /** 权重：成功次数 − 失败次数，由系统自动维护 */
  weight: number;
  /** 被注入的次数 */
  hits: number;
  /** 最近一次被注入的时间；null 表示从未注入 */
  last_used: string | null;
  /** 创建日期 */
  created: string;
}

// ============================================================
// L3 索引层：SQLite
// 可重建，以 Markdown 文件内容为准；迁移只允许向前追加的幂等迁移
// ============================================================

/** `entries` 表：L1/L2 的统一可检索投影 */
export interface SqliteEntry {
  /** 主键。取值等于来源条目 id（PeRecord.id 或 MemoryEntry.id） */
  id: string;
  /** 投影来源类型 */
  source_type: 'pe' | 'memory';
  /** 标题/规则名，对应 FTS5 列 `title` */
  title: string;
  /** 别名集合，写入 FTS5 列 `aliases` */
  aliases: string[];
  /** 正文内容，对应 FTS5 列 `content` */
  content: string;
  /** 领域标签，供阶段一 domain 路由过滤 */
  domain: string;
  /** 状态；PE 记录为 null，记忆条目取 MemoryEntry.status */
  status: 'candidate' | 'active' | 'deprecated' | null;
  /** 权重；PE 记录为 null */
  weight: number | null;
  /** 来源 Markdown 文件相对路径，索引重建时据此回溯 */
  source_path: string;
  /** 投影更新时间（ISO 8601） */
  updated_at: string;
}

/** `entries_fts` 表：FTS5 虚拟表，列权重 title 10× / aliases 5× / content 1× */
export interface EntriesFtsRow {
  /** 主键（rowid）。与 `entries.id` 一一对应 */
  rowid: number;
  /** FTS5 列：title */
  title: string;
  /** FTS5 列：aliases */
  aliases: string;
  /** FTS5 列：content */
  content: string;
  /** 冗余列：domain，供阶段一 domain 路由直接过滤 */
  domain: string;
  /** BM25 得分，由 `bm25()` 在查询时计算，非持久化列 */
  bm25_score: number;
}

/** `entry_vectors` 表：阶段三 sqlite-vec 向量索引 */
export interface EntryVector {
  /** 主键。同时是外键 → `entries.id` */
  entry_id: string;
  /** 生成该向量的嵌入模型名（`multilingual-e5-small` / `bge-m3`）；换模型必须重建全库索引 */
  model: string;
  /** 向量维度：e5-small 为 384，bge-m3 为 1024 */
  dim: number;
  /** 向量数据 */
  vector: number[];
  /** 向量写入时间（ISO 8601） */
  created_at: string;
}

/** `usage_log` 表：注入 / 命中 / 未命中 / 评分流水 */
export interface UsageLog {
  /** 主键（自增整数） */
  id: number;
  /** 一次改写会话的关联标识 */
  session_id: string;
  /** 外键 → `entries.id`；未命中场景为 null */
  entry_id: string | null;
  /** 事件类型 */
  action: 'inject' | 'hit' | 'miss' | 'score';
  /** 用户评分（1-5），未评分时为 null */
  score: number | null;
  /** 本次事件引起的 weight 变化：+1 / 0 / −2 */
  weight_delta: number;
  /** 记录时间（ISO 8601） */
  created_at: string;
}

// ============================================================
// 配置与命令载荷
// ============================================================

/** 应用配置（后端 config.rs 持有，设置面板经 get_config / set_config 读写） */
export interface AppConfig {
  /** 选中的模型提供方标识，三选一 */
  provider: 'minimax' | 'openai_compat' | 'ollama';
  /** 嵌入模型名；阶段一为 `multilingual-e5-small` */
  embedding_model: string;
  /** 记忆注入 token 硬上限，默认 800 */
  token_budget: number;
  /** 注入时取用的 Top-K 条数（K 的具体取值待确认） */
  top_k: number;
  /** DOT 收起态直径（px），区间 40-56，默认 48 */
  dot_size_px: number;
  /** 展开/收起动画时长（ms），区间 150-250 */
  animation_ms: number;
  /** 开机自启开关 */
  autostart_enabled: boolean;
  /** 全局热键组合，默认 `Ctrl+Space` */
  global_shortcut: string;
  /** API key 存放方式：环境变量名 / 系统钥匙串 / 未配置（存放位置待确认） */
  api_key_source: 'env' | 'keychain' | 'none';
}

/** 模型提供方配置：MiniMax API / OpenAI 兼容端点 / Ollama 本地模型 */
export interface ModelProviderConfig {
  /** 提供方标识 */
  provider: 'minimax' | 'openai_compat' | 'ollama';
  /** 接口基址，由用户配置，产品不内置默认值 */
  base_url: string;
  /** 模型名称，由用户配置 */
  model: string;
  /** API key 引用名（环境变量名或钥匙串条目名），不落盘明文 */
  api_key_ref: string | null;
  /** 单次请求超时（秒），由用户配置 */
  timeout_seconds: number;
  /** 是否为本地提供方（Ollama 不需要网络与密钥） */
  is_local: boolean;
}

/** 一键改写请求 */
export interface OptimizeRequest {
  /** 请求 id，用于日志与 usage_log 关联 */
  request_id: string;
  /** 用户输入的原始提示词，不得为空 */
  prompt: string;
  /** 领域标签；为 null 时由后端按 domain 路由推断 */
  domain: string | null;
  /** 是否注入匹配的记忆经验；功能④阶段必须为 false */
  inject_memories: boolean;
  /** 实际命中的记忆条目 id 列表（后端注入时填充） */
  memory_ids: string[];
  /** 关联的 PE 记录 id（落库时填充） */
  pe_record_id: string | null;
}

/** 一键改写响应 */
export interface OptimizeResponse {
  /** 请求 id，与请求一致 */
  request_id: string;
  /** 改写后的提示词 */
  optimized_prompt: string;
  /** 本次实际注入的记忆条目子集 */
  injected_memories: RetrievedMemory[];
  /** 本次注入的 token 估算值 */
  injected_token_estimate: number;
  /** 是否命中历史经验；false 时界面须显示「暂无历史经验」 */
  has_memory_context: boolean;
  /** 落库的 PE 记录 id；落库失败时为 null */
  pe_record_id: string | null;
}

/** 注入给模型的记忆条目子集：只含 statement / why / how_to_apply */
export interface RetrievedMemory {
  /** 记忆条目 id（外键 → MemoryEntry.id） */
  id: string;
  /** 规则陈述 */
  statement: string;
  /** 原因 */
  why: string;
  /** 应用方式 */
  how_to_apply: string;
  /** 领域标签 */
  domain: string;
  /** 注入时的权重，仅用于排序与裁剪，不进入模型上下文 */
  weight: number;
}
```

**数据模型补充约定**

- 「连续未命中计数」不在 `memory.md` 中新增字段，由 L3 `usage_log` 中 `action = 'miss'` 的记录推导，保持 L2 字段集与 `AGENTS.md` 第 1 节一致。
- `entries_fts` 的 `bm25_score` 为查询时计算值，不落盘；`entries` 是唯一可重建的真相来源，索引重建后须与 Markdown 内容一致。
- `entry_vectors.model` 是换模型重建索引的判据；阶段一/二该表保持为空且不加载 sqlite-vec 扩展。

---

## 6 边界情况

> 覆盖五类：空输入、并发或重复提交、权限不足、外部依赖失败、异常中断。每条注明可恢复 / 不可恢复。

| 编号 | 类别 | 场景 | 期望行为 | 错误类型 |
|---|---|---|---|---|
| EDGE-01 | 空输入 | 输入框为空或仅含空白字符时点击「一键改写」 | 不发起模型请求，输入框给出可读提示，输入内容保持不变 | 可恢复 |
| EDGE-02 | 超量超长 | 输入提示词超出可接受长度上限 | 明确拒绝并提示超出上限，不截断后静默提交 | 可恢复 |
| EDGE-03 | 类型错误 | 后端收到结构不符合 `OptimizeRequest` 的载荷（如缺字段、字段类型不符） | 参数校验失败并返回明确错误，不进入模型调用与落库 | 不可恢复 |
| EDGE-04 | 并发或重复提交 | `Ctrl+Space` 短时间内连按多次 | 展开/收起状态切换幂等，面板不重复创建、不出现多个实例窗口 | 可恢复 |
| EDGE-05 | 并发或重复提交 | 二次启动应用触发单实例插件 | 只保留一个实例进程，第二次启动唤醒已有实例并显示 Pad | 可恢复 |
| EDGE-06 | 并发或重复提交 | 同一请求被重复提交（网络重试 / 用户连点「保存」） | `save_pe_record` 按 `PE-YYYY-NNN` 幂等，不产生重复记录 | 可恢复 |
| EDGE-07 | 权限不足 | `memory/` 目录或 Markdown 文件只读、无写权限 | 拒绝写入并明确提示权限不足；不损坏既有 `memory.md` 内容 | 不可恢复 |
| EDGE-08 | 权限不足 | SQLite 库文件被其他进程占用或无访问权限 | 返回可读错误，降级为不落库的只读路径，改写结果仍可使用 | 可恢复 |
| EDGE-09 | 权限不足 | 剪贴板不可读或不可写 | 改写结果仍展示在面板内，复制动作给出失败提示 | 可恢复 |
| EDGE-10 | 外部依赖失败 | 模型 API 不可用或请求超时 | 返回可恢复错误并提示用户重试或切换提供方；本次不落库、不改权重 | 可恢复 |
| EDGE-11 | 外部依赖失败 | 选定的 Ollama 本地服务未启动 | 明确提示本地服务不可用，不静默回退到其它提供方 | 可恢复 |
| EDGE-12 | 外部依赖失败 | ONNX 嵌入模型文件缺失或与库内 `entry_vectors.model` 不一致 | 阶段一/二退化为纯文本检索并提示；阶段三则拒绝写入向量并要求重建索引 | 可恢复 |
| EDGE-13 | 异常中断 | 夜间任务执行中途失败（进程被杀 / 断电 / 模型调用失败） | `memory.md` 保持上一次一致状态，不留半条条目；任务可整体重跑 | 可恢复 |
| EDGE-14 | 异常中断 | 写入 `memory.md` 或 SQLite 过程中崩溃 | 依赖幂等迁移与「先写 Markdown 后重建索引」的顺序，重启后可由 Markdown 重建索引 | 可恢复 |
| EDGE-15 | 数据不一致 | Markdown 内容与 SQLite 索引不一致（手工改过 `memory.md`） | 以 Markdown 为准重建索引，重建后检索结果与文件内容一致 | 可恢复 |
| EDGE-16 | 预算超限 | 沉淀后 `memory.md` 超出 800 token 硬上限 | 拒绝晋升并明确报错；从 weight 最低的 deprecated 起裁剪 | 不可恢复 |
| EDGE-17 | 状态边界 | 注入请求命中 `candidate` 条目 | 拒绝注入，候选不参与注入 | 可恢复 |
| EDGE-18 | 环境变化 | 记录的屏幕在下次启动前被拔出 | 按「哪块屏 + 屏内百分比坐标」安全回落到可用屏幕，面板不出屏 | 可恢复 |

---

## 7 非目标

1. **本期不做 Linux 支持**；平台只保证 Windows（可扩 macOS）。
2. **不做云端存储 / 云同步**；数据仅存本机 SQLite 与 Markdown。
3. **不做 Electron 版**，也不引入任何 Node 进程常驻方案；常驻内存占用是本产品卖点。
4. **不做多用户与团队协作**；单一角色、单机单用户，无账号体系、无权限分级、无共享记忆库。
5. **不做图形化记忆编辑器**；记忆维护以纯 Markdown 手改 + git diff/revert 为准（防线三、四）。
6. **不在阶段一/二引入向量库依赖与调用路径**；未积累 20–30 条真实记录前不接入 sqlite-vec。
7. **不做 IDE 插件、浏览器扩展等其他形态**；本期交付物只有 Windows 单机安装包。
8. **不把统计字段注入模型上下文**；`evidence` / `weight` / `hits` 等只供系统自用。

---

## 8 验收标准

> 每条均可判定真假。编号与第 2 节功能清单一一挂接。

### F-① 透明无边框常驻窗口

- **AC-01** `tauri.conf.json` 中窗口 `label` 为 `pad`，且 `transparent=true`、`decorations=false`、`alwaysOnTop=true`、`skipTaskbar=true`、`resizable=false`、`shadow=false`、`focus=false` 八项取值与配置一致。
- **AC-02** 应用启动后任务栏无该窗口图标，且窗口无系统标题栏与系统边框。
- **AC-03** 应用启动时，当前活动窗口的键盘焦点不变，原窗口可继续接收输入。

### F-② 常驻 DOT

- **AC-04** DOT 收起态直径取自配置且落在 40–56px 区间，默认值为 48px；代码中不存在裸魔法数。
- **AC-05** 收起态下 DOT 不夺取键盘焦点（`set_focusable(false)` 生效），原窗口焦点保持。
- **AC-06** 收起态 DOT 周围透明区域不响应鼠标事件（`set_ignore_cursor_events(true)` 生效），点击穿透到下层应用；面板展开时该穿透被关闭。
- **AC-07** 将 DOT 拖到另一块屏幕后重启应用，DOT 出现在同一块屏的同一相对位置（持久化内容为「哪块屏 + 屏内百分比坐标」，非绝对像素）。

### F-③ 展开/收起

- **AC-08** 点击 DOT 展开面板；点击面板外部或按 `Esc` 收起面板；两种收起路径行为一致。
- **AC-09** 展开与收起动画时长取自配置且落在 150–250ms 区间。
- **AC-10** DOT 位于屏幕右边缘时面板向左展开、位于上边缘时向下展开；任意位置展开后面板完全落在可视区域内（不出屏）。
- **AC-11** 检测到独占全屏应用时 DOT 临时隐藏，退出全屏后恢复可见。

### F-④ 最小输入框 + 一键改写

- **AC-12** 输入为空或仅空白字符时，点击「一键改写」不发起模型请求，且界面给出可读提示。
- **AC-13** 功能④阶段，改写请求的上下文中不包含任何记忆条目内容（不出现 `statement` / `why` / `how_to_apply` 文本）。
- **AC-14** 改写完成后结果可复制回原应用，且焦点可回到原窗口。

### F-⑤ SQLite 落库

- **AC-15** 每次成功改写生成一条 PE 记录，id 形如 `PE-YYYY-NNN`，`YYYY` 与记录日期年份一致，同一年内 `NNN` 连续递增。
- **AC-16** PE 记录以逐字原文写入 `memory/prompt_engineering.md`；既有记录内容不被改写，文件只追加。
- **AC-17** 同一条 PE 记录同时落 SQLite `entries` 表并带有 `signals` 字段。
- **AC-18** 落库失败时改写结果仍可复制，界面明确提示「记录未保存」。

### F-⑥ 全局热键 + 托盘 + 自启

- **AC-19** 在任意前台应用中按下 `Ctrl+Space` 均可切换 Pad 的展开/收起状态。
- **AC-20** 托盘菜单包含「显示/隐藏」「退出」「设置」三项入口；点击「退出」后进程结束。
- **AC-21** 开启开机自启后重启系统，Pad 自动启动且不弹出设置窗口。
- **AC-22** 第二次启动应用时不再新建实例进程，已有实例的 Pad 被唤醒或显示。

### F-⑦ 检索层

- **AC-23** 无有效 `query`、或 BM25 无命中时，检索回退 domain 路由（`stage=markdown_domain`），仍只注入 `active` 且 `weight>0` 的 L2 条目。
- **AC-24** 有有效 `query` 且 BM25 命中时启用 FTS5 trigram BM25（`stage=fts5_bm25`），列权重为 title 10× / aliases 5× / content 1×；条数不再作硬门禁。
- **AC-25** 阶段一/二的构建产物中不存在 sqlite-vec 扩展的加载调用路径（依赖与调用路径检查）。
- **AC-26** 嵌入模型切换判据按 PE 记录总数判定，与向量库规模解耦。
- **AC-27** 阶段三启用后每条向量均带 `model` 字段；更换嵌入模型后全库向量被重建，且不残留旧模型的向量行。

### F-⑧ 夜间记忆沉淀

- **AC-28** 夜间任务扫描当天新增的 PE 记录，并对其 `signals` 去重合并。
- **AC-29** 与已有条目相似度匹配成功时，该条目 `weight` +1 且 `evidence` 追加来源 PE 记录 id。
- **AC-30** 无匹配且达到阈值时新建 `status = candidate` 的条目；新建条目不进入注入集。
- **AC-31** 与已有条目矛盾时，任务不写入 `memory.md`，改为生成待人工裁决的挂起项。
- **AC-32** 沉淀后 `memory.md` 估算 token 超过 800 时拒绝晋升并明确报错；`deprecated` 条目不计入该预算。

### F-⑨ 记忆注入

- **AC-33** 注入集只包含 `domain` 匹配、`status = active` 且 `weight > 0` 的条目，并按 `weight` 降序取 Top-K。
- **AC-34** 进入模型上下文的内容只含 `statement`、`why`、`how_to_apply`，不含 `evidence`、`weight`、`hits`、`status`。
- **AC-35** 注入总量 ≤ 800 token；超预算时按 `weight` 从低到高裁剪。
- **AC-36** 无匹配条目时，界面明确告知「暂无历史经验」。

### F-⑩ 权重更新

- **AC-37** 评分 ≥4 时 `weight` +1；无改善时 `weight` 变化为 0；变差时 `weight` −2。
- **AC-38** 未命中某条目时该条目 `hits` 不变，并累加连续未命中计数。
- **AC-39** `weight ≥ 2` 且无失败时 `candidate` 升为 `active`；`weight < −1` 或连续 3 次未命中时降为 `deprecated`；`deprecated` 不注入且不占 token 预算。
- **AC-40** 前 10 次使用不产生 −2 权重惩罚（只收集数据）。

### F-⑪ 冷启动

- **AC-41** 批量回填：导入历史记录后可触发一次夜间任务，产出 `status = candidate` 的条目，不自动升为 `active`。
- **AC-42** AI 迁移：用户确认后的偏好总结以 `type = preference` 写入 `memory.md`。
- **AC-43** 记忆库为空时首次使用不报错，按「暂无历史经验」路径正常处理。

### 全局交付门禁

- **AC-44** `pnpm typecheck` 与 `pnpm build` 均通过，无 TS 报错。
- **AC-45** `cargo check`、`cargo test`、`cargo clippy` 通过且无 warning。（**待确认**：本机尚未安装 Rust stable + MSVC Build Tools，需安装后补验）
- **AC-46** 仓库中不存在清单外的 Tauri 插件依赖；托盘使用核心 `tauri::tray`；无 Electron 或常驻 Node 进程。
- **AC-47** 仓库与 `memory/` 目录中不存在 API key 明文。
- **AC-48** 空输入、类型错误、超量超长三类边界均有显式分支处理，程序不 panic。
- **AC-49** `memory/` 目录纳入 git 版本控制，可对单条记忆做 diff 与 revert。

---

### 附：待确认清单

沿用 `AGENTS.md` 第 9 节，并补充本文中同样未定的取值：

1. Rust stable + MSVC Build Tools 本机尚未安装（`cargo` / `rustc` 不在 PATH）——Rust 侧代码需安装后补 `cargo check`（影响 AC-45）。
2. `memory.md` 的 `<域缩写>` 取值表未定义——暂定 `ui-design` / `writing` / `code` / `general` 四域（影响 `MemoryEntry.id`）。
3. 最小窗口尺寸 40–56px 取 48px 为默认（区间内可配）。
4. `bge-m3` 切换阈值「记录 >200 条」按 PE 记录总数判定（影响 AC-26）。
5. 注入的 Top-K 具体取值未定义（`AppConfig.top_k`）。
6. API key 的最终存放方式未在「环境变量」与「系统钥匙串」之间定案（`AppConfig.api_key_source`）。
7. 各模型提供方的基址与默认模型名由用户配置，产品不内置默认值（`ModelProviderConfig`）。
