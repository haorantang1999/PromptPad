# Prompt Pad · 任务拆解与实现顺序

> 来源：`AGENTS.md`（唯一事实基线）· `docs/01-案例调研与技术选型.md` · `docs/02-私人记忆库设计.md` · `software_prompts/03-implementation.md` IMP-01
> 拆解口径：把 AGENTS.md 第 1 节功能清单①–⑪ 拆到**函数 / 组件 / 模块级**可执行单元，编号 T01–T20 连续。
> 本文件只做任务规划，**不含任何实现代码**。

---

## 0. 粒度口径与总览

### 0.1 任务单元定义

「一个函数或一个组件」的原子单元，在本项目中按 AGENTS.md 第 4 节第 2 条的目录约定落为四种载体：

| 单元类型 | 载体 | 例子 |
|---|---|---|
| 组件 | 一个 `.tsx` 文件 | `src/components/Dot.tsx` |
| Hook | 一个 `.ts` 文件 | `src/hooks/useDrag.ts` |
| Rust 函数 | 一个服务/命令文件内的**单一职责函数** | `services/position.rs` 的百分比坐标换算函数 |
| 配置/装配 | 一个配置文件或一个模块装配 | `src-tauri/tauri.conf.json` |

> 允许同一文件内的**同族函数**合并为一个任务（例如「点击穿透切换」与「展开收起动画」同属 `window_ctl.rs` 的窗口状态切换族），但**不允许**把两个不相关关注点塞进同一任务。凡合并处均在「粒度说明」列写明合并理由。

### 0.2 总览

- 任务总数：**20 个**（符合 IMP-01「8–20」约束）
- 功能覆盖：AGENTS.md 第 1 节①–⑪ **全部覆盖**，逐条对照见 1.3
- 依赖图：**无环**（关键路径 T03 → T04 → T05 → T06 → T07 → T09 → T11）
- 并行分组：**5 组**（W0–W4），见第 3 节

### 0.3 硬性约束对拆解的约束（每条落到具体任务）

| 约束 | 落点 |
|---|---|
| 窗口八项配置全部生效 | T02 |
| 40–56px / 150–250ms / Ctrl+Space / 800 token 不写魔法数 | T04（进配置）、T09 / T10 / T11 / T19（只读取配置） |
| 组件禁止直连 `invoke` | T14（唯一通道）、T09 / T11 / T15（验收含全仓 grep 断言） |
| `candidate` 不参与注入、注入只带 statement + why + how_to_apply | T19 / T20 |
| 阶段一 / 二不得引入 sqlite-vec 调用路径 | T16 |
| L1 事实层只追加不可改写 | T17 |
| API key 不硬编码、不进 `memory/` 与 git | T04 / T13 / T20 |

---

## 1. 任务拆解表

### 1.1 主表（7 列）

| 任务号 | 任务名 | 粒度说明 | 依赖任务 | 是否可并行 | 验收条件 | 预计改动文件 |
|---|---|---|---|---|---|---|
| T01 | 工程骨架与依赖装配 | 装配前端脚手架 + Rust crate 骨架 + 入口挂载；不含任何业务逻辑 | 无 | 是（与 T02 / T03 同组） | ① `pnpm typecheck` 与 `pnpm build` 均返回 0，`dist/index.html` 存在；② `src/main.tsx` 以 React 18 `createRoot` 挂载且全文无 `invoke(`；③ `src-tauri/Cargo.toml` 的依赖清单不含清单外插件，`tauri` 特性含 `tray-icon`；④ `cargo check --manifest-path src-tauri/Cargo.toml` 返回 0（工具链未安装时见 4.3 注记） | `package.json`、`vite.config.ts`、`tsconfig.json`、`tsconfig.node.json`、`index.html`、`src/main.tsx`、`src/App.tsx`、`src/index.css`、`src-tauri/Cargo.toml`、`src-tauri/build.rs`、`src-tauri/src/main.rs` |
| T02 | pad 窗口八项配置与能力白名单 | 一个窗口配置单元：把 AGENTS.md 第 1 节八项窗口配置写进 `tauri.conf.json`，并把 capabilities 限定在四个允许插件 | 无 | 是（与 T01 / T03 同组） | ① `tauri.conf.json` 中 `app.windows[0]` 的 `label` 为 `pad`，且 `transparent` / `decorations` / `alwaysOnTop` / `skipTaskbar` / `resizable` / `shadow` / `focus` 七项布尔值逐项可核对，共 8 项全部落位；② `capabilities/default.json` 中出现的插件标识仅限 `global-shortcut` / `single-instance` / `autostart` / `clipboard-manager` 四个；③ `pnpm tauri dev` 启动后窗口无系统边框、不占任务栏、启动不抢焦点 | `src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json` |
| T03 | AppError 错误类型与错误码 | 一个错误类型单元：统一错误枚举 + 手写 Display + 可恢复/不可恢复二分 | 无 | 是（与 T01 / T02 同组） | ① `error.rs` 至少含 2 个可恢复变体（模型超时、上游 5xx）与 2 个不可恢复变体（越界写入、迁移失败）；② 每个变体的 `Display` 输出同时含错误码与定位参数名（如参数名、文件名）；③ 序列化为 JSON 后含 snake_case 错误码字段；④ 「空输入 / 类型错误 / 超量超长」三类边界各有独立变体 | `src-tauri/src/error.rs` |
| T04 | 配置模型与配置命令 | 一个配置模块：配置结构体（读）+ `get_config` / `set_config` 两个成对命令（写）；合并理由是读写闭环拆开即产生半截契约 | T03 | 是（W1 组内） | ① 配置结构体至少覆盖：模型提供方（三选一）、端点、模型名、API key 来源的环境变量名、热键、DOT 尺寸、展开动画时长、token 预算八类项；② 序列化输出中不出现 API key 明文，仓库内 grep `api_key` 无明文值；③ 配置文件不存在时返回内置默认值且不报错；④ `set_config` 后再 `get_config`，全部字段值相等；⑤ 字段名与 `docs/interface-doc.md` 一致（该文件由他人并行产出，本任务**不改动**；未定稿时先按本表占位并在契约定稿后对齐） | `src-tauri/src/config.rs`、`src-tauri/src/commands/config_cmd.rs`、`src-tauri/src/commands/mod.rs` |
| T05 | 屏幕枚举、百分比坐标换算与 DOT 位置持久化 | `position.rs` 内的同族函数组：① 枚举可用屏幕与主屏；② 「屏序号 + 屏内百分比」↔ 像素坐标换算；③ 该坐标的持久化读写。合并理由是三者是同一坐标模型的三个阶段 | T02、T03 | 是（W1 组内） | ① 持久化载荷只含 `monitor_index` 与 `x_ratio` / `y_ratio` 三项，断言中不含任何像素字段；② 持久化的屏幕被拔出时，读取回退到主屏并返回「已降级」标记为真；③ 换算使用该屏的物理尺寸与 `scale_factor`，源码中不出现固定 DPI 常量；④ 同一百分比在两种不同物理尺寸的屏上换算，结果均落在该屏可用区矩形内 | `src-tauri/src/services/position.rs` |
| T06 | 窗口状态切换：点击穿透、展开收起动画、面板显隐命令 | `window_ctl.rs` 的窗口状态切换函数族：① 收起态 `set_ignore_cursor_events(true)` / 展开态关闭；② 展开收起动画（时长取自配置）；③ `toggle_panel` 命令装配。合并理由是三者共享同一份窗口状态 | T02、T03、T05 | 是（W2 组内） | ① 收起态调用后 `ignore_cursor_events` 为 true，展开态为 false；② 同一状态连续调用两次结果不变（幂等）；③ 动画时长取自配置且落在 150–250ms 区间，收起后窗口尺寸回到 40–56px 区间；④ 无窗口句柄时返回错误码而非 panic（断言不崩溃）；⑤ `toggle_panel` 切换后能通过 `get_focused_window` 观察到焦点回到原窗口 | `src-tauri/src/services/window_ctl.rs`、`src-tauri/src/commands/panel.rs` |
| T07 | 展开态几何计算（边缘方向自适应 + 绝不出屏）【高风险 R1】 | 一个纯函数：输入 DOT 位置与面板尺寸，输出面板矩形与展开方向 | T05、T06 | 是（W3 组内，与 T08 / T09 同组） | ① 目标矩形四边全部落在目标屏可用区矩形内（对屏左上 / 右上 / 左下 / 右下 / 中部 5 个位置断言全部通过）；② 靠上时展开方向为下、靠右时为左、靠左时为右、靠下时为上；③ 面板尺寸大于可用区时返回错误码而非返回越界矩形；④ 该函数不读写全局状态，可被单元测试直接调用 | `src-tauri/src/services/window_ctl.rs` |
| T08 | 独占全屏检测与 WS_EX_TOPMOST 置顶/临时隐藏【高风险 R2】 | 一个检测函数 + 一个置顶处理函数：检测独占全屏，命中时临时隐藏窗口并置 `WS_EX_TOPMOST`，退出后恢复 | T06 | 是（W3 组内） | ① 未检测到独占全屏时返回 false 且窗口可见性不变；② 检测到独占全屏时窗口可见性为 false，且窗口扩展样式含 `WS_EX_TOPMOST`；③ 退出全屏后可见性恢复为 true、样式复原；④ 检测逻辑不依赖硬编码进程名清单（源码内无具体游戏/播放器进程名） | `src-tauri/src/services/fullscreen.rs` |
| T09 | DOT 组件与拖拽 Hook | 一个组件 + 专属于它的拖拽 Hook：渲染 40–56px 圆点、响应拖拽、结束回写百分比坐标 | T02、T06 | 是（W3 组内） | ① DOT 渲染尺寸在 40–56px 闭区间（默认 48px，来自配置）；② 点击 DOT 触发一次展开事件；③ 组件与 Hook 源码中不出现 `invoke(`；④ 拖拽结束写入的载荷只含 `monitor_index` + `x_ratio` / `y_ratio`；⑤ DOT 在收起态不拦截鼠标（由 T06 的穿透状态可观测） | `src/components/Dot.tsx`、`src/hooks/useDrag.ts` |
| T10 | 面板状态 Store 与 `usePanelState` Hook | 一个状态模块 + 消费它的 Hook：collapsed / expanded 两态、provider、最近一次改写结果的唯一状态源 | T03、T14 | 是（W2 组内） | ① store 暴露 collapsed / expanded 两态与 provider、last_result 三项，组件不各自维护副本；② `usePanelState` 在 Esc 事件下把状态置为 collapsed；③ 组件树中不出现模块级可变变量充当状态；④ 状态切换全部经 `src/api/client.ts` 调用后端 | `src/state/store.ts`、`src/hooks/usePanelState.ts` |
| T11 | Panel 组件 | 一个组件：渲染展开面板、承接点外部与 Esc 收起、挂载输入/结果/状态子组件 | T07、T09、T10 | 是（W4 组内，与 T18 / T20 并行） | ① 展开与收起动画时长取自配置且落在 150–250ms；② 点击面板外部或按 Esc 触发收起；③ 面板容器不依赖系统标题栏、边框、阴影（`shadow:false` 下自绘）；④ 组件内不出现 `invoke(` | `src/components/Panel.tsx` |
| T12 | 模型提供方适配层（三选一适配） | 一个适配层单元：一个统一 provider 抽象 + `minimax` / `openai_compat` / `ollama` 三个实现，共享同一请求体、同一响应结构、同一错误类型 | T03、T04 | 是（W1 组内） | ① 适配层暴露三个实现，对同一份输入分别产出各自合法的请求体；② 三者返回同一响应结构，字段名统一 snake_case；③ 三者共用 `AppError`，超时归为可恢复；④ 配置中的 provider 取未知值时返回明确错误码而非 panic；⑤ 端点、模型名、超时时长均来自配置，源码中无硬编码 URL | `src-tauri/src/services/provider.rs` |
| T13 | `optimize_prompt` 命令 | 一个命令函数：校验入参 → 调适配层 → 返回改写结果 | T03、T04、T12 | 是（W2 组内） | ① 空输入返回「空输入」类错误且不发起请求；② 超长输入返回「超长」类错误并给出上限值来源（配置）；③ provider 超时返回可恢复错误且错误信息含 provider 名；④ 成功时返回改写文本，JSON 字段名 snake_case；⑤ 本命令**不引入任何记忆注入逻辑**（记忆注入在 T20） | `src-tauri/src/commands/optimize.rs` |
| T14 | 前端 API 调用层（唯一 invoke 通道） | 一个调用层单元：`types.ts` 接口类型 + `commands.ts` 八个命令函数 + `client.ts` 封装 invoke 与错误归一化 | T03 | 是（W1 组内） | ① 全仓 grep `invoke(` 仅命中 `src/api/client.ts`；② 八个命令（`toggle_panel` / `optimize_prompt` / `save_pe_record` / `submit_feedback` / `retrieve_memories` / `run_nightly_job` / `get_config` / `set_config`）各有对应导出函数；③ 类型字段与 Rust 字段逐一 snake_case 对齐；④ 后端错误码被归一化为带错误码的异常，组件侧可区分可恢复与不可恢复；⑤ 与后端实现的逐字段对齐在 IMP-03 联调阶段验证，不作为本任务前置依赖 | `src/api/types.ts`、`src/api/commands.ts`、`src/api/client.ts` |
| T15 | 改写交互三组件 | 三个各司其职的组件：输入区只做输入与触发、结果区只做展示与复制回原应用、状态区只做状态与错误提示 | T10、T14 | 是（W3 组内） | ① 三个组件互不调用彼此的内部状态；② 空输入时提交按钮为禁用态；③ 状态区文案区分可恢复与不可恢复两类；④ 复制动作经 `clipboard-manager` 插件写回系统剪贴板，写入内容与结果区展示文本逐字相同；⑤ 三个组件内均不出现 `invoke(` | `src/components/PromptInput.tsx`、`src/components/ResultView.tsx`、`src/components/StatusBar.tsx` |
| T16 | SQLite 存储层：建表、幂等迁移与分级检索 | 一个存储模块单元：① 幂等迁移与建表；② 阶段一纯 domain 路由召回；③ 阶段二 FTS5 BM25 召回（列权重 title 10 / aliases 5 / content 1） | T03、T04 | 是（W1 组内） | ① 连续执行两次迁移后表结构与迁移版本号不变（幂等）；② `entries` / `entries_fts` / `usage_log` 三表存在，`entry_vectors` 仅建空表待阶段三启用；③ 阶段一按 domain 路由能召回该域条目且不召回异域条目；④ 阶段二 BM25 的列权重为 title 10 / aliases 5 / content 1（以查询排序结果断言）；⑤ 阶段一 / 二代码路径中不出现 sqlite-vec 调用与依赖；⑥ 全部 SQL 参数化，无字符串拼接 | `src-tauri/src/storage/db.rs` |
| T17 | 事实层落库：Markdown 只追加写与 PE 记录落库命令 | 一个落库单元：① `prompt_engineering.md` 只追加写；② `save_pe_record` 生成 `PE-YYYY-NNN` 并同步落 SQLite 与 L1 | T03、T16 | 是（W2 组内） | ① 记录 ID 形如 `PE-YYYY-NNN`，同年序号从 001 递增且不重复；② 同一记录重复提交不产生新行（幂等）；③ 写 `prompt_engineering.md` 后，文件既有行内容逐字不变（以写前写后行内容比对断言）；④ 写入失败时 SQLite 与 Markdown 均不残留半条记录；⑤ `records/` 目录按既定规则拆分（记录数 > 30 时启用，规则见 4.3 待确认） | `src-tauri/src/storage/markdown.rs`、`src-tauri/src/commands/record.rs`、`src-tauri/src/commands/mod.rs` |
| T18 | 系统集成初始化：全局热键、托盘、自启、单实例 | 一个 `lib.rs` 初始化单元：注册 Ctrl+Space、构建托盘菜单、接入自启开关、接入单实例唤醒 | T02、T04、T08 | 是（W4 组内） | ① 注册的全局热键为 Ctrl+Space，按下可切换面板显隐；② 托盘菜单项恰为「显示 / 隐藏 / 退出 / 设置」四项，逐项点击均产生可观测效果；③ 二次启动不新建窗口，而是唤醒已有实例并显示面板；④ 自启开关状态与配置一致，开关后读回一致；⑤ 独占全屏期间（T08）窗口不遮挡全屏应用 | `src-tauri/src/lib.rs` |
| T19 | 夜间沉淀流水线与 800 token 预算闸门 | `nightly.rs` 流水线单元：扫描当天 PE 记录 → signals 去重合并 → 与现有条目相似度匹配 → 新建 candidate 或补 evidence → 过 token 闸门 | T03、T16、T17 | 是（W3 组内） | ① 只处理当天新增的 PE 记录（给定跨天夹具，输出条数等于当天记录数）；② 两条含相同 signal 的记录合并为 1 条 signal；③ 匹配到已有条目时该条目 weight +1 且 evidence 追加原记录 ID；④ 无匹配时新建条目且 `status` 为 `candidate`；⑤ 与既有条目矛盾时挂起待人工裁决，**不写入任何新条目**；⑥ 写入后总量超 800 token 时拒绝晋升并返回明确错误码，原文件不被改写；⑦ `candidate` 条目不出现在任何注入结果中 | `src-tauri/src/services/nightly.rs` |
| T20 | 记忆命令层：检索注入、权重更新、运行夜间任务、冷启动导入 | `memory.rs` 命令单元：① `retrieve_memories`（Top-K 注入）；② `submit_feedback`（权重更新 + 冷启动豁免）；③ `run_nightly_job`（调度 T19，含冷启动批量回填）；④ AI 迁移导入入口 | T03、T12、T16、T19 | 是（W4 组内） | ① 注入结果只含 `statement` / `why` / `how_to_apply` 三个键，断言无 `evidence` / `weight` / `hits`；② 候选条件为 domain 匹配 + `status=active` + `weight>0`，按 weight 降序取 Top-K；③ 无匹配时返回显式的「暂无历史经验」标识，不塞入低分噪声；④ 评分 ≥4 时 weight +1、评分 ≤2 时 weight −2、其余不变；⑤ 连续 3 次未命中后 `status` 变 `deprecated`；⑥ 前 10 次使用不施加 −2 惩罚（计数可观测）；⑦ 冷启动：批量回填复用 T19 流水线、AI 迁移导入产出 `preference` 类条目；⑧ 注入总量 ≤800 token | `src-tauri/src/commands/memory.rs`、`src-tauri/src/commands/mod.rs` |

### 1.2 逐任务完成定义（DoD，一句可判定）

| 任务号 | 完成定义 |
|---|---|
| T01 | `pnpm typecheck`、`pnpm build` 与 `cargo check` 三条命令均返回 0，且 `src/main.tsx` 不含 `invoke(`。 |
| T02 | `tauri.conf.json` 的窗口配置项被逐条核对为 8/8 命中，capabilities 中插件标识数 ≤4 且全在白名单内。 |
| T03 | `error.rs` 中「空输入 / 类型错误 / 超量超长 / 模型超时 / 写入越界」五类各有独立变体，且每类有可断言的错误码。 |
| T04 | 配置往返读写后 8 类配置项全部相等，且序列化输出中不含 API key 明文。 |
| T05 | 持久化文件内容可用 `monitor_index` + 两个比例字段完整描述 DOT 位置，文件中无任何像素坐标键。 |
| T06 | 收起态与展开态的鼠标穿透标志可分别被观测为 true / false，且 `toggle_panel` 后焦点回到原窗口。 |
| T07 | 5 个边界位置的展开矩形断言全部落在屏幕可用区内，且展开方向与 DOT 所在象限一致。 |
| T08 | 独占全屏进入时窗口不可见且带 `WS_EX_TOPMOST`，退出后两项均复原。 |
| T09 | DOT 在屏幕上量得 40–56px，拖拽结束后持久化文件内容仍只含比例坐标。 |
| T10 | 面板显隐状态在 store 中只有一个来源，`Esc` 能把其从 expanded 切到 collapsed。 |
| T11 | 面板展开与收起的实际动画时长均落在 150–250ms，点外部与 Esc 两条路径都能收起。 |
| T12 | 三个 provider 实现对同一输入产出三种合法请求体，并共用同一响应结构与同一错误类型。 |
| T13 | 空输入、超长、provider 超时三条路径分别返回三种可区分错误，其中超时为可恢复类。 |
| T14 | 全仓 `invoke(` 命中数等于 1，且八个命令在 `commands.ts` 中各有导出。 |
| T15 | 输入区、结果区、状态区三个组件的职责互斥，空输入时提交按钮禁用。 |
| T16 | 迁移连跑两次结构不变，阶段一与阶段二各自召回正确，源码中无 sqlite-vec 调用。 |
| T17 | 同一 PE 记录重复落库不新增行，写入后 `prompt_engineering.md` 既有行逐字未变。 |
| T18 | Ctrl+Space、托盘四项菜单、二次启动唤醒、自启开关四件事各自可被观察验证。 |
| T19 | 夜间任务对跨天夹具只处理当天记录，新条目一律 `candidate`，超预算时拒绝晋升且原文件未改。 |
| T20 | 注入结果键集合恰为三个字段，权重三档变化与连续未命中降级、前 10 次豁免均可断言。 |

### 1.3 功能覆盖对照（AGENTS.md 第 1 节①–⑪）

| 功能 | 覆盖任务 | 说明 |
|---|---|---|
| ① 透明无边框常驻窗口 | T01、T02 | 八项窗口配置在 T02 落位，T01 提供可运行外壳 |
| ② 常驻 DOT | T05、T06、T09、T10 | 尺寸 T09、拖拽 T09、置顶 T06、不抢焦点 T06、百分比位置持久化 T05、点击穿透 T06 |
| ③ 展开/收起 | T06、T07、T08、T10、T11 | 动画 T06/T11、边缘自适应 T07、绝不出屏 T07、独占全屏 T08、点外部与 Esc T11 |
| ④ 最小输入框 + 一键改写 | T12、T13、T14、T15 | 三选一适配 T12、命令 T13、调用层 T14、交互 T15（先不做记忆） |
| ⑤ SQLite 落库 | T16、T17 | 建表与迁移 T16、PE-YYYY-NNN 落库与 L1 只追加 T17 |
| ⑥ 全局热键 + 托盘 + 自启 | T18 | Ctrl+Space、托盘四项菜单、自启、单实例唤醒 |
| ⑦ 检索层 | T05（读取侧）、T16 | 阶段一 domain 路由 + 阶段二 FTS5 BM25；阶段三 sqlite-vec 延后 |
| ⑧ 夜间记忆沉淀 | T19 | 扫描、signals 去重合并、相似度匹配、candidate / 人工裁决、预算闸门 |
| ⑨ 记忆注入 | T20 | domain + active + weight>0、Top-K、只注入三字段、无匹配时明确告知 |
| ⑩ 权重更新 | T20 | +1 / 0 / −2、连续未命中计数与降级 |
| ⑪ 冷启动 | T19、T20 | 批量回填走 T19 流水线、AI 迁移导入与前 10 次豁免在 T20 |

### 1.4 技术风险最高的 2 个任务

**R1 = T07 展开态几何计算（多显示器 + DPI + 边缘方向自适应 + 绝不出屏）**
风险来源：坐标模型要在「哪块屏 + 屏内百分比」与「物理像素 + scale_factor」之间来回换算，且要在四象限边界上保证矩形不出屏；多屏不同缩放比例时误差会被放大，一旦算错就是「面板跑到屏幕外 / 展开方向反了」这类直接可见的故障。

**R2 = T08 独占全屏检测 + WS_EX_TOPMOST 置顶与临时隐藏**
风险来源：独占全屏没有跨应用的统一查询接口，只能靠窗口/进程启发式判断；`WS_EX_TOPMOST` 属于原生扩展样式操作，Tauri 无直接 API，需要触碰 Windows 原生调用；该场景在普通桌面下无法稳定复现，验证成本远高于 T07。

**先做 T07（几何计算），后做 T08（全屏检测）。** 理由：

1. **可测性差异**：T07 是纯函数，能用表驱动用例在 5 个边界位置上断言「矩形不出屏」；T08 依赖真实独占全屏应用，自动化验证手段缺失，只能人工构造场景。
2. **依赖方向**：T08 在退出全屏后需要把窗口恢复到原位置，这一步直接复用 T07 产出的目标屏可用区矩形；先定 T07 可避免 T08 返工。
3. **触发面差异**：T07 每次展开都生效，是③的主干；T08 只在游戏 / 全屏视频下生效，触发面窄、可以用降级策略兜底（检测不到就保持普通置顶）。
4. **验证环境现实**：本机 Rust 工具链尚未安装（AGENTS.md 第 9 节第 1 条），先把可脱离系统状态验证的纯函数逻辑定稿，能在工具链到位后第一时间拿到确定性通过的结果。

> 次高风险（仅备注，不列入前二）：T12 provider 适配层（三套外部协议签名差异大）与 T19 的 800 token 预算闸门（token 估算口径需与模型分词器对齐）。这两项排在 T07 / T08 之后处理。

---

## 2. 依赖顺序

### 2.1 执行序列（波次内可并行，波次间严格串行）

（下列为依赖波次图，非代码；箭头表示「前置 → 后继」。）

- **W0 基础外壳**　T01、T02、T03
  - T01 / T02 互不依赖；T03 无前置
  - ↓
- **W1 基础层**　T04、T05、T12、T14、T16
  - T04←T03；T05←T02,T03；T12←T04；T14←T03；T16←T03,T04
  - ↓
- **W2 窗口与数据**　T06、T10、T13、T17
  - T06←T02,T03,T05；T10←T03,T14；T13←T03,T04,T12；T17←T03,T16
  - ↓
- **W3 交互与沉淀**　T07、T08、T09、T15、T19
  - T07←T05,T06；T08←T06；T09←T02,T06；T15←T10,T14；T19←T03,T16,T17
  - ↓
- **W4 集成与记忆**　T11、T18、T20
  - T11←T07,T09,T10；T18←T02,T04,T08；T20←T03,T12,T16,T19

### 2.2 关键路径

`T03 → T04 → T05 → T06 → T07 → T11`（共 6 跳）——最短可用路径是「一条改写链路 + 一个能展开且不出屏的面板」。
第二条关键路径 `T03 → T04 → T16 → T17 → T19 → T20`（共 6 跳）——最短可用路径是「记录 → 夜间沉淀 → 记忆注入」。

### 2.3 依赖无环说明

- 每条依赖边都指向更早的波次，跨波次回指不存在；同一波次内的任务互不依赖。
- T14（前端调用层）只依赖 T03（错误码）而**不依赖**任一后端实现任务：前后端字段对齐在 IMP-03 联调阶段验证，否则 T10 / T15 会被整条后端链阻塞。
- T20 依赖 T19（夜间流水线）而非反向，记忆注入的候选状态由 T19 产出。

---

## 3. 并行分组

> 团队现实：1 名个人开发者 + AI 编码 agent，以串行为主。本节「并行」指**同一波次内可派给不同 agent / 不同上下文窗口**独立完成，不代表同时开工。
>
> **写冲突检查**：同一波次内任意两个任务的「预计改动文件」集合互不相交（W1 的 T04 独占 `commands/mod.rs`；W4 的 T20 独占 `commands/mod.rs`），因此并行派发不产生同文件同时写入的冲突。跨波次的同文件改动（如 T04 与 T20 都涉及 `commands/mod.rs`）由波次串行天然规避。

### P0 · 基础外壳（3 个任务）

| 任务 | 负责面 |
|---|---|
| T01 | 前端 + Rust 骨架 |
| T02 | 窗口与能力配置 |
| T03 | 错误类型 |

**启动条件（全部满足）**：
1. 本文件已定稿且 `AGENTS.md` 第 2 节目录结构未被改动；
2. 仓库中除 `AGENTS.md`、`docs/`、`software_prompts/` 外无既有构建产物（避免覆盖未知交付物）；
3. 前端依赖安装命令可执行（`pnpm install` 返回 0）。

**完成判据**：三条任务各自的验收条件全部通过，且 `pnpm tauri dev` 能拉起一个符合 T02 描述的窗口。

### P1 · 基础层（5 个任务）

| 任务 | 负责面 |
|---|---|
| T04 | 配置模型与配置命令 |
| T05 | 屏幕与坐标持久化 |
| T12 | provider 适配层 |
| T14 | 前端调用层 |
| T16 | 存储层与检索 |

**启动条件**：
1. T03 的错误枚举与错误码已定稿（`error.rs` 中五类错误变体可被逐个引用）；
2. T02 的窗口配置已可被 Tauri 加载（T05 需要真实窗口上下文取 `available_monitors`）；
3. T04 的配置字段名已在 `config.rs` 出现（供 T12 / T16 读取）；
4. `docs/interface-doc.md` 若已定稿，T14 按其字段名对齐；未定稿则 T14 先按本文件占位并登记待对齐项。

**完成判据**：五个任务各自的验收条件通过，且 `get_config` / `set_config` 往返一致、存储层迁移连跑两次结构不变。

### P2 · 窗口与数据（4 个任务）

| 任务 | 负责面 |
|---|---|
| T06 | 窗口状态切换与面板显隐命令 |
| T10 | 状态 store 与面板状态 Hook |
| T13 | `optimize_prompt` 命令 |
| T17 | 事实层落库与 PE 记录命令 |

**启动条件**：
1. T05 已交付可用的坐标换算与持久化接口（T06 需要目标屏可用区）；
2. T12 的请求/响应结构已定稿（T13 直接消费）；
3. T14 的 `client.ts` 已导出 `toggle_panel` 与 `optimize_prompt`（T10 依赖其调用）；
4. T16 的 `entries` 表结构已定稿（T17 依赖其列定义）。

**完成判据**：穿透状态两态可观测、`toggle_panel` 后焦点回到原窗口、空输入与超长分别返回不同错误、PE 记录重复提交不新增行。

### P3 · 交互与沉淀（5 个任务）

| 任务 | 负责面 |
|---|---|
| T07 | 展开态几何计算（**R1**） |
| T08 | 独占全屏检测（**R2**） |
| T09 | DOT 组件与拖拽 Hook |
| T15 | 改写交互三组件 |
| T19 | 夜间沉淀流水线与预算闸门 |

**启动条件**：
1. T06 的窗口状态切换函数已交付（T07 / T08 均以其为入口）；
2. T10 的 store 已提供 collapsed / expanded 两态与 provider 字段（T15 消费）；
3. T17 的 PE 记录已可写入且可按天查询（T19 的输入依赖）；
4. T19 可先用固定夹具（跨天记录 + 预置 memory.md）本地验证，不依赖真实积累数据。

**完成判据**：5 个边界位置的几何断言全过、全屏进出时可见性与扩展样式同步翻转、DOT 量得 40–56px、夜间任务对跨天夹具只处理当天记录。

### P4 · 集成与记忆（3 个任务）

| 任务 | 负责面 |
|---|---|
| T11 | Panel 组件 |
| T18 | 热键 / 托盘 / 自启 / 单实例 |
| T20 | 记忆命令层（注入 / 权重 / 夜间调度 / 冷启动） |

**启动条件**：
1. T07 的几何计算通过全部边界断言（T11 依赖其保证不出屏）；
2. T08 的全屏检测返回可判定布尔值（T18 依赖其隐藏与恢复时机）；
3. T19 的夜间流水线可在固定夹具上产出 `candidate` 条目（T20 依赖其产出物）；
4. T12 适配层可用（T20 的 AI 迁移导入需要调用模型生成偏好总结）。

**完成判据**：面板动画时长落在 150–250ms、Ctrl+Space 与托盘四项各自生效、注入结果键集合恰为 `statement` / `why` / `how_to_apply` 三项且权重三档变化可断言。

---

## 4. 备注与待确认

### 4.1 阶段三（sqlite-vec + ONNX 嵌入）明确延后

- T16 只建空表 `entry_vectors`，阶段一 / 二代码路径中不出现 sqlite-vec 调用与依赖（AGENTS.md 第 6 节第 9 条）。
- 触发条件：PE 记录总数 > 200 条（按 AGENTS.md 第 9 节第 4 条判定），或阶段二 BM25 召回率经实际记录验证不足。
- 启用时必须**重建全库索引**，每条向量随记录存模型名（`multilingual-e5-small` 384 维 → `bge-m3` 1024 维）。

### 4.2 外部契约依赖（本文件不改动，由他人并行产出）

- `docs/interface-doc.md`：八个命令的签名与字段名。T04 / T13 / T14 / T20 的字段名以此为准；未定稿处按本文件占位并在定稿后对齐，登记为待对齐项。
- `docs/database-design.md`：表结构。T16 / T17 的列定义以此为准。
- 本任务**未读取、未修改**上述两个文件。

### 4.3 待确认项（沿用 AGENTS.md 第 9 节）

1. **Rust 工具链未安装**：本机 `cargo` / `rustc` 不在 PATH，所有 Rust 侧任务的「`cargo check` / `cargo test` / `cargo clippy` 返回 0」验收条件**当前无法本地执行**，须在安装 MSVC Build Tools 后补跑。在补跑前，上述任务的完成状态应标注为「逻辑完成、编译未验证」，不得声称通过。
2. **DOT 位置持久化文件的落盘位置**：拟放应用配置目录，确切路径与文件名待定。
3. **`memory.md` 的域缩写取值表**未定义，暂定 `ui-design` / `writing` / `code` / `general` 四域（影响 T16 阶段一路由与 T19 域匹配）。
4. **`records/` 目录拆分规则**：记录数 > 30 时启用拆分，命名规则待定（影响 T17）。

### 4.4 边界与回滚

- 每个任务只新增或修改「预计改动文件」列内列出的文件；跨列修改视为越界，需先回到本文件修订。
- 回滚方式：单任务以 `git revert` 对应提交回滚；新增文件直接删除。所有任务在 `memory/` 目录内只做追加，不产生需回滚的覆盖写。
- 交付前自检沿用 AGENTS.md 第 7 节清单，由承载任务的会话逐条勾选。
