# AGENTS.md · Prompt Pad 项目规约

> 本文件是 Prompt Pad 全部开发会话的**唯一事实基线**。任何子代理、任何会话开始前必须先读本文件。
> 事实来源：`docs/01-案例调研与技术选型.md`、`docs/02-私人记忆库设计.md`。
> 约束冲突时以本文件「硬性约束」为准。

---

## 1. 项目概览

**Prompt Pad（提示词优化 Pad）** —— 常驻 Windows 系统最上层、全局热键 `Ctrl+Q` 唤起、带私人记忆的提示词优化 Pad。

- 目标用户：开发者本人（重度 AI 编程/写作助手用户）。1 名个人开发者 + AI 编码 agent 协作，无硬性工期。
- 核心场景：任意应用中写好草稿提示词 → `Ctrl+Q` 唤起 Pad → 粘贴/输入 → 一键改写（自动注入匹配的记忆经验）→ 复制回原应用。**焦点必须能还给原窗口。**
- 产品原则：不用时极小（TINY WHEN UNUSED），打开时强大（POWERFUL WHEN OPENED）。数据不出本机。

### 硬性约束（不可协商）

| 类别 | 约束 |
|---|---|
| 外壳 | Tauri 2，**禁止 Electron**（除非某 Windows 能力绝对必需） |
| 前端 | React **18** + TypeScript 严格模式 + Vite + Tailwind CSS |
| 后端 | Rust（stable），命令经 `#[tauri::command]` 暴露 |
| 存储 | SQLite + FTS5 + sqlite-vec（阶段三）；Markdown 文件（`prompt_engineering.md` / `memory.md`） |
| 嵌入 | ONNX 本地推理；阶段一 `multilingual-e5-small`（~117MB / 384 维），记录 >200 条后换 `bge-m3`（1024 维，**换模型必须重建全库索引**，每条向量存模型名） |
| 模型 | 提供方四选一：`minimax` / `openai_compat` / `ollama` / `anthropic`；**协议两选一**：`openai_chat_completions` / `anthropic_messages`（提供方与协议分开配置，见 §9 第 12 条） |
| 插件 | 仅限 `global-shortcut`、`single-instance`、`autostart`、`clipboard-manager` 四个（**托盘图标属 Tauri 2 核心 `tauri::tray`，无独立插件**） |
| 平台 | Windows 优先（可扩 macOS），**本期不做 Linux** |
| 窗口 | `transparent`、`decorations:false`、`alwaysOnTop`、`skipTaskbar`、`resizable:false`、`shadow:false`、`focus:false`，`label:"pad"` |
| 记忆 | `memory.md` ≤ **800 token** 硬上限；`candidate` 不参与注入；注入只带 `statement`+`why`+`how_to_apply` |
| 版本控制 | git；`memory/` 目录每日提交 |

### 功能清单（实施路线 8 步，功能编号即开发顺序）

| # | 功能 | 关键行为 |
|---|---|---|
| ① | 透明无边框常驻窗口 | 上述八项窗口配置全部生效 |
| ② | 常驻 DOT | 40–56px；拖拽；置顶；`set_focusable(false)` 不抢焦点；位置持久化「**哪块屏 + 屏内百分比坐标**」（非绝对像素）；收起态 `set_ignore_cursor_events(true)` 点击穿透，展开态关闭 |
| ③ | 展开/收起 | 点击展开、点外部或 `Esc` 收起；动画 150–250ms；屏幕边缘展开方向自适应（靠上向下、靠右向左）且**面板绝不出屏**；独占全屏时临时隐藏 + `WS_EX_TOPMOST` |
| ④ | 最小输入框 + 一键改写 | 调用当前模型改写提示词（**先不做记忆**） |
| ⑤ | SQLite 落库 | 记录 `PE-YYYY-NNN`（`prompt_engineering.md` 事实层，永久、逐字不可改、只追加） |
| ⑥ | 全局热键 + 托盘 + 自启 | `Ctrl+Q`（配置项 `global_shortcut`，改键立即重绑）；托盘菜单（显示/隐藏/退出/设置）；开机自启（`autostart_enabled`）；单实例（二次启动唤醒已有实例） |
| ⑦ | 检索层 | 阶段一 domain 路由（BM25 无命中时回退）→ 阶段二 FTS5 trigram BM25（有 query 即启用，列权重 title 10× / aliases 5× / content 1×）→ 阶段三 sqlite-vec KNN + RRF（300+ 条，未启用） |
| ⑧ | 夜间记忆沉淀 | 扫描当天 PE 记录 → signals 去重合并 → 相似度匹配（匹配则 weight+1 补 evidence / 无匹配且达标新建 candidate / 矛盾挂起人工裁决）→ 检查 `token_budget` → 重建/增量更新索引 |
| ⑨ | 记忆注入 | `domain` 匹配 + `status=active` + `weight>0`，按 weight 降序取 Top-K；只注入 `statement`+`why`+`how_to_apply`；总量 ≤800 token；**无匹配时明确告知「暂无历史经验」** |
| ⑩ | 权重更新 | 评分 ≥4 → `+1`；无改善 → `0`；变差 → `−2`；未命中 → 触发连续未命中计数 |
| ⑪ | 冷启动 | 批量回填 / AI 迁移导入 preference / **前 10 次使用豁免权重惩罚** |

### 记忆四层架构

| 层 | 载体 | 约束 |
|---|---|---|
| L0 临时 | 对话上下文 | 会话结束即弃，不落库 |
| L1 事实 | `memory/prompt_engineering.md` | 永久、逐字不可改、只追加；ID `PE-YYYY-NNN`；含 `signals` 字段 |
| L2 记忆 | `memory/memory.md` | **≤800 token 硬上限**；ID `M-<域缩写>-<序号>`；字段 `type`(rule\|preference\|pitfall\|asset) / `domain` / `status`(candidate\|active\|deprecated) / `statement` / `why` / `how_to_apply` / `signals` / `evidence` / `weight` / `hits` / `last_used` / `created` |
| L3 索引 | SQLite | 可重建，以 Markdown 为准；表 `entries` / `entries_fts` / `entry_vectors` / `usage_log` |

**状态机**：`candidate` --(weight ≥2 且无失败)--> `active` --(weight < −1 或连续 3 次未命中)--> `deprecated`。
`candidate` 不注入；`deprecated` 不占 token 预算。

**防污染四道防线**：人工裁决闸门（矛盾/跨 domain 条目挂起不自动写入）· 候选不注入（至少验证 2 次才升 active）· `memory.md` 纯 Markdown 全量可读可 diff · `memory/` 目录纳入 git 每日提交。

---

## 2. 目录结构

```
F:/Prompt_pad_2/
├── AGENTS.md                  # 本文件（唯一事实基线）
├── package.json  vite.config.ts  tsconfig.json  tsconfig.node.json  index.html
├── eslint.config.js  pnpm-workspace.yaml  scripts/
├── src/                       # React 18 前端
│   ├── main.tsx  App.tsx  index.css
│   ├── api/                   # Tauri invoke 封装（唯一调用层，禁止组件直连 invoke）
│   │   └── client.ts  commands.ts  types.ts  client.test.ts
│   ├── components/            # Dot / Panel / ResultView / StatusBar
│   ├── hooks/                 # usePanelState
│   └── state/                 # store.ts（zustand）
├── src-tauri/                 # Rust 后端
│   ├── Cargo.toml  build.rs  tauri.conf.json  capabilities/default.json  icons/
│   └── src/
│       ├── main.rs  lib.rs
│       ├── config.rs  error.rs  geometry.rs  models.rs  clock.rs
│       ├── commands/          # panel / optimize / record / memory / config_cmd（只做参数校验与转调）
│       ├── services/          # window_ctl / position / fullscreen / provider / nightly / tray / memory / record
│       └── storage/           # db（SQLite+FTS5）/ markdown（memory.md 读写）
├── memory/                    # L1 + L2 记忆库（纳入 git，每日提交）
│   ├── memory.md  prompt_engineering.md  records/
├── docs/                      # 需求/架构/接口/数据库/任务拆解/决策文档
└── software_prompts/          # 52 个提示词模板（只读参考）
```

---

## 3. 技术栈与版本（均已对 npm registry / crates.io 核验）

**前端**：`react@18.3.1` `react-dom@18.3.1` `@types/react@18.3.31` `@types/react-dom@18.3.7`
`vite@6.4.5` `@vitejs/plugin-react@4.7.0` `typescript@5.9.3` `tailwindcss@4.3.3` `@tailwindcss/vite@4.3.3`
`vitest@3.2.7` `zustand@5.0.15` `@tauri-apps/api@2.12.4`
`@tauri-apps/plugin-global-shortcut@2.4.1` `@tauri-apps/plugin-autostart@2.7.0` `@tauri-apps/plugin-clipboard-manager@2.5.0`
（`@tauri-apps/cli@2.12.3` 为 devDependency）

**Rust**：`tauri@2.12.4`（feature `tray-icon`）`tauri-build@2.7.3`
`tauri-plugin-global-shortcut@2.4.1` `tauri-plugin-single-instance@2.5.4` `tauri-plugin-autostart@2.7.0` `tauri-plugin-clipboard-manager@2.5.0`
`serde@1.0.229` `serde_json@1.0.154` `rusqlite@0.40.2`（features `bundled`+`load_extension`+`vtab`+`functions`+`backup`）`reqwest@0.13.5`（feature `native-tls`）`windows-sys@0.61.2`（features `Win32_Foundation`+`Win32_UI_Input_KeyboardAndMouse`+`Win32_UI_WindowsAndMessaging`）
> 直接依赖只有以上这些：`tokio` / `anyhow` 已删除（前者由 Tauri 运行时提供，后者与 `AppError` 重复）；Win32 绑定用 `windows-sys` 而非 `windows`。
> `Win32_UI_Input_KeyboardAndMouse` 用于读取左键按下状态（圆球的点击/拖拽判定），`Win32_UI_WindowsAndMessaging` 用于光标位置、窗口矩形、前台窗口与 `WS_EX_TOPMOST`。

**运行时**：Windows 11 · Node.js 22.22.2 · pnpm 12.11.2 · Rust stable 1.99.1（MSVC 工具链，已装在 D 盘，见 §9 第 1 条）

> 精确版本以 `pnpm-lock.yaml` 与 `Cargo.lock` 为准；本表列出的是**约束版本**（语义化版本范围的上界）。
> 变更依赖时必须同步本表。

---

## 4. 编码与目录约定

1. **前端**：函数组件 + TypeScript `strict: true`；Tailwind 原子类，禁止写自定义 CSS 文件（仅 `index.css` 允许 Tailwind 入口）；组件不得直接 `invoke`，一律走 `src/api/client.ts`。
2. **后端**：每个 `#[tauri::command]` 独立文件于 `src-tauri/src/commands/`，并在 `commands/mod.rs` 统一导出；`#[serde(rename_all = "snake_case")]`。
3. **字段命名**：Rust/serde 与 JSON 一律 `snake_case`；TypeScript 类型与 Rust 字段一一对应，接口类型集中在 `src/api/types.ts`。
4. **命令契约**（10 个，签名以 `docs/interface-doc.md` 为准）：`toggle_panel` `optimize_prompt` `save_pe_record` `submit_feedback` `retrieve_memories` `run_nightly_job` `get_config` `set_config` `begin_dot_drag` `set_panel_view`。
   - `begin_dot_drag`：收起态圆球的「点击还是拖拽」判定。Tauri 拖拽区按下即启动系统拖拽循环并独占鼠标，浏览器收不到 `mouseup`/`click`，只能由后端轮询左键状态判定（见 `docs/interface-doc.md` §〇 修订表第 14 条）。
   - `set_panel_view`：把面板当前视图同步给后端，使「点外部收起」在设置视图下不生效（修订表第 15 条）。
   - Tauri 2 按**参数名**从 JS payload 取值：Rust 参数名必须与前端 `invoke` 的 key 逐字一致。
   - `toggle_panel` 参数为 `mode`（`toggle`/`expand`/`collapse`，缺省 `toggle`）；其余命令统一为 `request`。
   - 错误码只允许 `docs/interface-doc.md` 定义的 16 个小写蛇形值，不得自造。
5. **错误处理**：统一 `AppError` 枚举 + `thiserror` 风格的手写 Display；区分**可恢复**（返回用户可见提示，如模型超时）与**不可恢复**（阻止写入，如超 800 token 拒绝晋升）。
6. **无硬编码**：阈值、路径、URL、密钥一律经参数或配置传入；**默认服务地址只允许出现在前端预设表**（`src/components/Settings.tsx` 的 `PRESET_OPTIONS`），Rust 侧不得内置任何一家的服务地址。API key 允许存在**环境变量**、**系统钥匙串**或**本机明文**（`%APPDATA%` 的 `config.json`）三处，**禁止**写入 `memory/` 与 git。
7. **注释只写 WHY**：解释取舍与约束，禁止逐行复述代码含义。
8. **参数校验**：空输入 / 类型错误 / 超量超长三类边界必须显式处理，缺一不可。
9. **SQLite 迁移**：只允许向前追加的幂等迁移；L3 索引可随时重建，**以 Markdown 文件内容为准**。
10. **注释与文档语言**：代码注释用中文，标识符用英文。

---

## 5. 命令清单（出处：`package.json` scripts / `Cargo.toml`）

| 命令 | 作用 | 出处 |
|---|---|---|
| `pnpm install` | 安装依赖 | `package.json` |
| `pnpm dev` | 仅前端开发服务器（Vite） | `package.json` `scripts.dev` |
| `pnpm tauri dev` | 启动 Tauri 开发模式（含 Rust） | `package.json` `scripts.tauri` |
| `pnpm build` | 前端类型检查 + 生产构建 | `package.json` `scripts.build` |
| `pnpm preview` | 预览前端生产构建 | `package.json` `scripts.preview` |
| `pnpm test` | 前端 Vitest 单元测试 | `package.json` `scripts.test` |
| `pnpm typecheck` | 仅 TypeScript 类型检查 | `package.json` `scripts.typecheck` |
| `pnpm lint` | ESLint 检查 | `package.json` `scripts.lint` |
| `pnpm icons` | 无依赖生成打包图标（PNG + ICO） | `package.json` `scripts.icons` |
| `node scripts/check-rust-balance.mjs` | Rust 源码括号结构静态校验（零依赖，秒级反馈） | `scripts/check-rust-balance.mjs` |
| `node scripts/check-geometry.mjs` | geometry 落位算法等价复刻，验算 10 条单测断言 | `scripts/check-geometry.mjs` |
| `node scripts/check-db-sql.mjs` | 用内存 SQLite 实跑 `db.rs` 的 DDL 与查询 | `scripts/check-db-sql.mjs` |
| `node scripts/check-doc-json.mjs` | 校验 `docs/` 下 JSON 代码块可解析 | `scripts/check-doc-json.mjs` |
| `cargo check --manifest-path src-tauri/Cargo.toml` | Rust 编译检查 | `src-tauri/Cargo.toml` |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust 单元测试 | `src-tauri/Cargo.toml` |
| `cargo clippy --manifest-path src-tauri/Cargo.toml` | Rust lint | `src-tauri/Cargo.toml` |
| `cargo fmt --manifest-path src-tauri/Cargo.toml` | Rust 格式化 | `src-tauri/Cargo.toml` |
| `pnpm tauri build` | 生成 Windows 安装包（NSIS） | `package.json` `scripts.tauri` |

---

## 6. 禁止事项

1. **禁止引入 Electron** 及任何 Node 进程常驻方案——常驻内存占用是本产品卖点。
2. **禁止引入清单外的 Tauri 插件**；托盘用核心 `tauri::tray`，不得自建第三方托盘库。
3. **禁止做 Linux 支持**；本期只保证 Windows。
4. **禁止引入云端存储 / 云同步**；数据仅存本机 SQLite 与 Markdown。
5. **禁止把 `evidence`、`weight`、`hits` 等统计字段注入模型上下文**——只注入 `statement`+`why`+`how_to_apply`。
6. **禁止自动写入 `candidate` 以外的新记忆条目**，矛盾条目必须挂起人工裁决。
7. **禁止突破 `memory.md` 800 token 硬上限**；超限必须拒绝晋升并明确报错。
8. **禁止把 API key 硬编码或写入 `memory/` 与 git 仓库**。本机明文只允许落在 `%APPDATA%` 的 `config.json`；后端回给前端的配置视图**永不包含明文**（只回 `api_key_present` 指示位）。
9. **禁止在检索层未积累 20–30 条真实记录前引入向量库**（阶段一/二不得加 sqlite-vec 依赖调用路径）。
10. **禁止改写 L1 事实层既有记录**（`prompt_engineering.md` 只追加）。
11. **禁止组件直接调用 `invoke`**，必须经 `src/api/client.ts`。
12. **禁止未经确认修改已完成步骤的交付物**；每步改动须可回滚（`git revert` 或删除新增文件）。
13. **禁止虚构** API、字段、表名、版本号、性能指标；未确认处一律写「待确认」。

---

## 7. 交付前自检

- [ ] `pnpm typecheck` 与 `pnpm build` 均通过，无 TS 报错
- [ ] `cargo check` / `cargo test` / `cargo clippy` 通过，无 warning
- [ ] 未引入清单外依赖、插件、平台支持
- [ ] 关键常量（800 token、40–56px、150–250ms、Ctrl+Q）来自配置而非魔法数
- [ ] 敏感信息（API key）未进入仓库与 `memory/`
- [ ] 空输入 / 类型错误 / 超量超长 三类边界均有显式处理
- [ ] 新增行为在 `docs/` 对应文档中有对应描述，字段命名 `snake_case`
- [ ] 改动可回滚，且已在交付说明中给出回滚方式

---

## 8. 上下文投喂与压缩（单次会话 ≤ 12000 字符）

**常驻档**（每次必带，≤8000 字符）：本文件第 1、3、4、6 节；当前实施步骤编号；已确认的决策与约束。
**按需档**（相关才取，≤3000 字符）：`docs/spec.md`（功能条目）、`docs/interface-doc.md`（命令契约）、`docs/database-design.md`（表结构）、`docs/task-breakdown.md`（当轮任务）。
**可删档**（优先裁剪）：历史会话日志、已完成步骤的产物明细、参考案例原文、`software_prompts/` 模板正文。

**裁剪顺序**（先删可删 → 再压按需 → 最后压常驻）：历史日志 → 模板正文 → 已完成步骤明细 → 文档全文摘录为条目 → 常驻档合并同类项。
**裁剪后目标**：常驻 ≤6000 字符 + 按需 ≤4000 字符 ≈ 10000 字符，余量留给人机对话。

---

## 9. 待确认清单

1. **Rust stable + MSVC Build Tools 已装在 D 盘（非 C 盘）**：rustup 装在 `D:\Rust\cargo`（`RUSTUP_HOME=D:\Rust\rustup`，stable 1.99.1 + clippy + rustfmt），VS Build Tools 2022 17.14.42 装在 `D:\VSBuildTools`。调用前需设置 `PATH` 含 `D:\Rust\cargo\bin`。
   **但本机的 Smart App Control（`VerifiedAndReputableDesktop` 用户态代码完整性策略）会拦下未签名的新生成 exe**，cargo 的构建脚本因此可能被拒（os error 4551）。对策：① 把 `CARGO_TARGET_DIR` 指到 D 盘；② 构建脚本被拦时**重跑同一条 cargo 命令**——信誉服务扫过之后即放行，重复几次即可推进。实测结论见 §10。
2. **收起态点击穿透 与「点击 DOT 展开」在 Windows 上互斥**：`set_ignore_cursor_events(true)` 依赖 `WS_EX_TRANSPARENT`，作用于**整个窗口**，无法只让透明区域穿透。当前实现把它做成配置项 `dot_click_through`（默认 `false`，保证 DOT 可点击）；若严格按文档设为 `true`，DOT 只能通过 `Ctrl+Q` / 托盘展开。**需用户拍板。**
3. `memory.md` 的 `<域缩写>` 取值表未定义——代码暂按 domain 首字母大写取最多 3 位（`ui-design` → `UID`）。
4. 最小窗口尺寸 40–56px 取 48px 为默认（区间内可配）。
5. `bge-m3` 切换阈值「记录 >200 条」是否同时按向量库规模判定——按 PE 记录总数判定。
6. `set_config` 采用**补丁语义**（未出现的字段保持原值），与 `docs/interface-doc.md` 的「全量覆盖」表述不一致；已在该文档标注。
7. `optimize_prompt` 的 `memory_ids` 设为**可选**：缺省由后端用原文走阶段二 BM25（无命中回退 domain）；前端无需先调 `retrieve_memories`；已在该文档标注。
8. `docs/interface-doc.md` 的 `AppConfig` 未包含 `api_key_env` / `memory_token_budget` / `dot_click_through` 三个实现必需字段，已在该文档标注。
9. **reqwest 0.13 的 TLS 后端已定为 `native-tls`**（Windows 走系统 SChannel，零额外构建工具）。`default-tls` 实际是 rustls + aws-lc-rs，Windows 上从源码构建需 CMake/NASM，干净环境会直接构建失败 —— 原注释「default-tls 走 SChannel」是错的，已一并改正。
10. `ErrorResponse` 的 `recoverable` / `command` / `occurred_at` 三个字段未实现（只序列化 `code` / `message` / `details`），原因与影响面已在 `docs/interface-doc.md` §〇 修订第 8 条记录。
11. **API key 存储放宽为「本机明文」**：§4 第 6 条原文只允许「环境变量或系统钥匙串」，现为「粘贴即用」放宽为 `%APPDATA%` 的 config.json 明文字段 `api_key`（优先级高于 `api_key_env`）。若产品要求更严，下一步应改用 Windows DPAPI 加密落盘。已登记在 `docs/interface-doc.md` §〇 修订第 18 条与 §六 第 12 条。
12. **提供方与协议正交**：`provider` 只作标识与「是否需要 key」的依据，`protocol`（`openai_chat_completions` / `anthropic_messages`）决定线上报文格式。MiniMax Token Plan = Anthropic 协议 + Subscription Key。当前**不做** provider × protocol 的组合白名单校验，是否加待确认（`docs/interface-doc.md` §六 第 13 条）。
13. **`endpoint` 补全规则**：已是完整请求路径（`/v1/chat/completions`、`/v1/messages`）→ 原样使用；只到版本段（`/v1`）→ 补协议尾段；协议根 → 补协议默认路径。默认端点只在**前端预设**，Rust 侧不内置任何服务地址。
14. **DOT 窗口 64px / 可见球体 44px**：容器放大到 64×64 是为了让辉光与点击波纹有外扩空间（窗口边界会裁掉溢出部分）；可见球体 44px 仍落在 §1 功能② 的 40–56px 区间内。

---

## 10. 实现状态（截至 v0.1）

| 步骤 | 状态 | 备注 |
|---|---|---|
| ① 骨架 + 透明无边框窗口 | ✅ | `tauri.conf.json` 八项配置齐备 |
| ② 常驻 DOT | ✅ | 拖拽、置顶、`set_focusable(false)`、点击穿透开关、锚点持久化 |
| ③ 展开动画 + 边缘自适应 | ✅ | 几何计算在 `geometry.rs`，**含 10 条单元测试**；面板绝不出屏 |
| ④ 最小输入框 + 模型改写 | ✅ | **双协议**：`openai_chat_completions` / `anthropic_messages`，endpoint 由配置传入（完整路径原样使用） |
| ⑤ SQLite 落库 | ✅ | `entries` / `entries_fts` / `entry_vectors` / `usage_log`；DDL 已用内存 SQLite 实跑校验 |
| ⑥ 热键 + 托盘 + 自启 + 单实例 | ✅ | `Ctrl+Q`（可配置，改键立即重绑）、托盘菜单（显示/隐藏 · 设置 · 退出）、`autostart_enabled` 接线 `enable()/disable()`、单实例唤醒 |
| ⑦ 检索层（阶段一+二） | ✅ | trigram BM25（有 query）+ domain 回退；`status=active AND weight>0`；阶段三未启用 |
| ⑧ 夜间记忆沉淀 | ✅ | 每小时调度昨天；signals 空则纯 domain 匹配；超预算丢新候选不报错；`deprecated` 不占 800；`.nightly-{date}` 幂等 |
| ⑨ 记忆注入 | ✅ | 只注入 `statement`+`why`+`how_to_apply`；总量 ≤800 token；无匹配时明确告知「暂无历史经验」 |
| ⑩ 权重更新 | ✅ | ≥4 → +1 / 无改善 → 0 / 变差 → −2；一条 PE 只评一次；权重回写 `memory.md` |
| ⑪ 冷启动 | ✅ | 前 10 次反馈豁免权重惩罚，计数口径为「已提交过反馈的 (会话, 评分) 组合数」 |

**记忆状态机**：`candidate --(weight ≥2)--> active --(weight < −1 或连续 3 次未命中)--> deprecated`。
降级动作在夜间任务里执行；未命中计数存在索引层 `entries.miss_streak`，Markdown 只落最终状态。

**v0.2 真机修复（用户反馈驱动，全部经 `scripts/check-app-ui.mjs` 15 条断言实测）**：
1. **插件配置导致启动即 panic**：`tauri.conf.json` 的 `plugins.*` 写成 `{}` 时 Tauri 按插件配置类型反序列化失败（三个插件的配置类型是 unit），应用在插件初始化阶段崩溃。三个值改为 `null`。
2. **DOT 与面板都拖不动**：ACL 缺 `core:window:allow-start-dragging`（拖拽被静默拒绝）；标题栏缺 `data-tauri-drag-region`。
3. **拖面板会变回圆球**：Windows 在拖拽途中把面板置为非激活（鼠标划过别的应用窗口即触发），被「点外部收起」误判。判据改为「光标是否仍在面板矩形内」。
4. **DOT 单击打不开**：Tauri 拖拽区按下即独占鼠标，浏览器不再派发 `mouseup`/`click`。改为后端 `begin_dot_drag` 轮询左键状态判定点击/拖拽（4 物理像素阈值）。
5. **窗口形态变更顺序**：`set_size` 会连带移动窗口，必须 `set_size` → `set_position`；且必须**先改 `state.mode` 再动窗口**，否则 `Moved` 事件会把面板落位当成用户拖拽写回锚点。
6. **焦点归还**：展开前捕获前台窗口，收起时 `SetForegroundWindow` 归还。
7. **热键 `Ctrl+Space` → `Ctrl+Q`**：做成配置项 `global_shortcut`，改键时先注册新键、成功后再解旧键。
8. **托盘「设置」进不去**：改为广播 `pad://view`="settings" 再展开面板；面板标题栏另加齿轮入口。
9. **设置表单自动保存**：改动停手 500 ms 自动写盘、离开字段立即写盘；只有取值合法的字段才写入（中间态不入盘）。
10. **设置视图点外部不收起**：后端新增 `set_panel_view` 记录当前视图。

**v0.3 多协议接入 + DOT 视觉重做（用户明确需求驱动）**：
1. **协议与提供方分离**：新增 `Protocol` 枚举（`openai_chat_completions` / `anthropic_messages`），`Provider` 扩为四值（新增 `anthropic`）。MiniMax Token Plan = Anthropic 协议 + Subscription Key。
2. **Anthropic 兼容层**：`x-api-key` + `anthropic-version: 2023-06-01` + `Authorization: Bearer`（兼容官方与 Token Plan 两种取 key 方式）；`system` 走顶层字段，`max_tokens` 必填（4096），响应取 `content[].text`（同时兼容字符串形态）。
3. **endpoint 原样使用**：完整请求路径不再被二次拼接（修掉 `.../chat/completions/chat/completions`）；补全规则集中在 `AppConfig::request_url`。
4. **API key 可直填**：新增本地明文字段 `api_key`（`%APPDATA%` 的 config.json，优先级高于 `api_key_env`），后端永不回显明文，只回 `api_key_present`。
5. **历史密钥迁移**：旧版把明文误填进 `api_key_env`（导致永远查不到环境变量、等于没配），加载期自动搬到 `api_key` 并复位变量名，只迁移一次。
6. **前端接入方式预设**：`PRESET_OPTIONS` 5 条预设（MiniMax 开放平台 / MiniMax Token Plan / Anthropic 官方 / OpenAI 兼容 / Ollama）+「自定义」；默认端点只存在于前端，Rust 侧零内置 URL。
7. **DOT 视觉重做**：渐变球体 + 蓝色辉光 + 4.2s 呼吸 + 点击波纹 + 拖拽放大 1.12；窗口容器 64×64、可见球体 44px（辉光不被窗口裁掉）。

**已验证（v0.3）**：Rust 工具链装到 D 盘后，Rust 侧**已完成实编译验证**——
- `cargo check --all-targets` 通过，**零 error 零 warning**；
- `cargo test` **67 条全绿**（含 geometry 10 条、db 14 条、markdown 5 条、config 9 条、provider 8 条、error / 命令层用例）；
- `cargo clippy --all-targets` **零 warning**；
- `cargo fmt --check` 通过。
前端 `pnpm typecheck` / `pnpm lint` / `pnpm test`（6 用例）/ `pnpm build` 全绿；`pnpm check` 的四个零依赖脚本全绿
（27 个 Rust 文件结构校验、10 条几何断言、15 条 DDL 与查询实跑、39 个 JSON 块可解析）。
真机 UI 闸 `scripts/check-app-ui.mjs --interact` **15/15 全绿**（DOT 64×64 逻辑 / 80 物理、拖拽、单击、Ctrl+Q、面板拖拽、出屏、锚点不漂移）。
> SAC 会偶发拦截构建脚本（os error 4551），**重跑同一条 cargo 命令**即可通过（见 §9 第 1 条）。
> 真机闸必须在**只有一个** prompt-pad 实例时跑：残留的旧实例会抢走单实例锁，热键与点击都发到它那儿，闸会误判成「面板没展开」。

**缺陷台账**：`docs/defect-ledger.md` 汇总 4 路专家审查 + 真机闸的 65 条缺陷，给出分级裁决与当前状态。
专家 D（前端与契约对齐）的报告**未取得**（审查任务被取消），该缺口在台账首节登记。

**v0.4 易用性与记忆回路修复（Markdown 为准）**：
1. **开机自启接线**：`autostart_enabled` + `app.autolaunch().enable()/disable()` + 设置页开关。
2. **首次使用**：`needs_setup` / `config_status`；配置损坏不覆盖原件；设置页暴露热键与自启；空 API key 框不再清掉已存密钥。
3. **复制并收起**；一条 PE 只评一次；改写域缺省 `general` 并回传；剪贴板预填草稿。
4. **记忆回路**：启动时从 Markdown 重建索引（B2）；评分回写 `memory.md`（B1）；`deprecated` 不占 800、超预算丢新候选不报错（B3）；每小时跑昨天的夜间任务。
5. **窗口**：全屏按前台窗所在屏检测并排除最大化；展开失败还原 DOT；点外部收起不抢焦点；设置页 Esc 回主面板；工作区夹紧；锚点屏 DPI；Moved 250ms 节流；托盘左键切换。

**遗留项（下一版必须处理）**：
1. **应用层未独立成模块**：8 个 use case 的编排体量很小（每个 command 转调服务层 1–3 行），目前是「命令 → 服务层」直接调用，没有单独的「命令 → 应用层 → 服务层」一层。依赖方向仍然自上而下（`docs/architecture.md` 依赖规则第 2 条已满足），但与该文档 §1 分层职责表的「应用层 = `lib.rs`（用例编排 + 记忆策略）」有出入，已在 architecture.md 登记。
2. **`usage_log` 的会话串联**：注入事件在 `save_pe_record` 写入（`session_id = record_id`），依赖前端把 `optimize_prompt` 响应里的 `injected_memory_ids` 回传。若前端漏传，该次改写将不产生 inject 事件，后续评分也就找不到对应条目。
3. **`storage/state.rs` 尚未独立**：`services/position.rs` 目前直接写 `position.json`，未走数据层。
4. **注入事件的信任边界在客户端**：`injected_memory_ids` 由前端回传，理论上可谎报。单机单用户场景下危害有限，但下一版应改为后端在 `optimize_prompt` 侧直接落注入事件。
5. **前端动画未接入 CSS 过渡**：`animation_ms` 暂以常量 200 返回；`src/` 中尚无真实过渡动画。
6. **阶段二 FTS5 BM25 已启用（trigram）**：`entries_fts` 分词器为 `trigram`，中文子串可命中；`optimize_prompt` / `retrieve_memories` 有有效 `query` 时走 BM25，无命中回退 domain 路由。阶段三（embedding + sqlite-vec）仍按 §6.9 冻结。旧库经 `PRAGMA user_version < 2` 自动重建 FTS。
7. **设置视图的「点外部收起」是产品行为偏离**：AGENTS.md §1 功能 ③ 写的是「点外部或 `Esc` 收起」，本轮让设置视图不因失焦收起（见 `docs/interface-doc.md` §六 第 11 条）；Esc 在设置页返回主面板。点击穿透默认 `false`。
8. **参数反序列化失败不产生契约错误形状**（专家 C-02，高）：Tauri 在进入命令体前自行反序列化入参，失败时返回字符串而非 `{code,message,details}`，整类 `invalid_type` 失效。修法：命令入参收 `request: serde_json::Value`，体内 `from_value::<T>()` 并映射为 `AppError::invalid_type`。
9. **其余待修项**（窗口尺寸/动画未取自配置、落位屏与锚点屏口径不完全一致、`max_prompt_chars` 硬编码、B-M5 usage_log CASCADE、B-M6 FTS 事务）逐条列在 `docs/defect-ledger.md` §三～§五。
