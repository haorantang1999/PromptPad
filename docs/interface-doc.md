# Prompt Pad · 接口契约文档（interface-doc）

> 模板：ARC-04 · API 接口设计与接口文档
> 事实基线：`AGENTS.md`（唯一事实基线）｜`docs/01-案例调研与技术选型.md`｜`docs/02-私人记忆库设计.md`
> 形态：Prompt Pad 为**单机桌面应用，无 HTTP 服务**。全部对外接口形态为 **Tauri command**——前端 React 通过 `@tauri-apps/api` 的 `invoke` 调用 Rust 后端的 `#[tauri::command]`（AGENTS.md §4 第 2、3 条；§6 第 11 条：组件禁止直连 `invoke`，一律经 `src/api/client.ts`）。
> 命令清单严格等于 AGENTS.md §4 第 4 条的 8 个：`toggle_panel` / `optimize_prompt` / `save_pe_record` / `submit_feedback` / `retrieve_memories` / `run_nightly_job` / `get_config` / `set_config`。
> **v0.2 新增 2 个**（见 §〇·补 修订表第 14–16 条）：`begin_dot_drag`（DOT 的点击/拖拽判定）、`set_panel_view`（面板当前视图同步）。前者是 Tauri 拖拽机制的硬约束所致，后者用于让设置视图不被「点外部收起」误伤。
> 字段命名：Rust / serde 与 JSON 一律 **snake_case**（AGENTS.md §4 第 3 条），不混用 camelCase。

---

## 〇·补 · 实现对齐修订（v0.1，2026-10-21）

本文档是 v0.1 的**定稿契约**。首轮实现完成后，逐字段比对发现下列 19 处需要说明的实现口径。
**契约本身未被削弱**；每一处都记录了原因与影响面，便于后续版本收敛。

| # | 契约原文 | 实现口径 | 原因 | 影响面 |
|---|---|---|---|---|
| 1 | §五·1 签名表写 `req: XxxRequest` | Rust 参数名为 `mode`（`toggle_panel`）与 `request`（其余 7 个），前端 `invoke` 的 key 与之逐字一致 | Tauri 2 按**参数名**从 payload 取值，参数名即 payload key；本文档 §五·1 的 `req:` 是同义表述 | 无。契约中的每个 Schema 仍是唯一的入参形状 |
| 2 | §2.8「全量覆盖写入」 | **补丁语义**：未出现的字段保持原值 | 全量覆盖要求前端每次回传全部字段，任一字段传错即整体丢失配置；补丁语义更安全 | `SetConfigRequest` 的每个字段均可选 |
| 3 | §2.2 `memory_ids` 为「由 retrieve_memories 返回、准备注入的记忆 ID」 | **可选**。缺省时由后端用 `original_prompt` 走阶段二 BM25（无命中回退 domain）；提供时后端按这些 ID 取条目 | 前端先查后注入会多一次 IPC 往返，且把注入决策放到客户端有泄漏 `candidate` 的风险；后端强制 `status=active AND weight>0` 并按 800 token 裁剪 | 前端可只调 `optimize_prompt` 一次；契约中 `memory_ids` 降级为可选 |
| 4 | §2.7 `AppConfig` 仅 6 个字段（`additionalProperties: false`） | 额外含 `api_key_env`、`memory_token_budget`、`dot_click_through` 三个字段 | ①契约的「环境变量」表述未指明**变量名**，无 key 来源就无法实现；②800 token 上限是硬约束，但契约的 `token_budget` 出现在响应里，暗示其为动态值；③点击穿透是「点击穿透 vs 点击 DOT 展开」冲突的裁决开关 | `AppConfig` 增加 3 个属性，`additionalProperties` 仍为 `false` |
| 5 | §2.1 `animation_ms`「本次**实际播放**的动画时长」 | 固定返回 `200`（150–250ms 区间中值） | 前端尚未接入 CSS 过渡，实际播放时长目前无从测得；先返回满足区间约束的常量，前端接入过渡后改为实测值 | 该字段暂时不可信，`position` 与 `expanded` 准确 |
| 6 | §2.5 `RetrieveMemoriesRequest.required` 含 `domain` | `domain` 可选，缺省表示不限定域 | 阶段一 domain 路由在缺省域时仍可返回全部 active 且 `weight>0` 的条目（按 weight 降序）；前端在不知道域时也能拿到 Top-K | `required` 降为 `[]`，其余字段不变 |
| 7 | §2.4 `SavePeRecordRequest` 未含注入条目 | 增加**可选** `injected_memory_ids: string[]` | §2.2 的 `optimize_prompt` 响应已返回 `injected_memory_ids`；把它回传给 `save_pe_record` 后，后端可用 `record_id` 作为 `usage_log.session_id` 写入 `inject` 事件，评分时才能定位到本次注入的条目。**不加这个字段，后端就无法把「注入」与「评分」串成一条链**（`usage_log` 按设计没有 `record_id` 列） | `SavePeRecordRequest` 增加 1 个可选属性；`submit_feedback` 的行为不变（按 `record_id` 反查会话） |
| 8 | §五·2 `ErrorResponse` 含 `code` / `message` / `details` / `recoverable` / `command` / `occurred_at` 六个必填字段 | 实际只序列化 `{ code, message, details }` 三个字段 | ①`recoverable` 可由 `code` 唯一推导，前端 `src/api/client.ts` 已维护 `UNRECOVERABLE_CODES` 集合，冗余下发反而有两处真相；②`command` 在错误构造点拿不到调用方命令名（错误在服务层/数据层就地产生），要拿到就得给 8 个命令各加一层包装；③`occurred_at` 由前端自己打点更准（IPC 往返耗时只有前端能量） | 前端只依赖 `code` / `message` / `details`；错误响应的其余三个字段前端不得读取 |
| 9 | §2.5 / §4.2「`has_more = offset + returned_count < total_matched_count`」 | 改为**按分页切片判定**：本页切片末尾未覆盖全部命中时为 true | `returned_count` 是**预算裁剪后**的条数；预算裁剪只会让本页变短、不会产生新的一页，用裁剪后的条数算会把最后一页误报成「还有更多」，客户端因此多翻一页空页 | `returned_count` / `total_matched_count` / `offset` / `top_k` 语义与取值均不变，仅 `has_more` 更准确 |
| 10 | §2.3「同一 `record_id` + 同一 `rating` 重复提交不累加 weight」 | 落实为 **(record_id, rating) 维度的去重**：命中已有评分事件时回放上次结果，既不重复改权重，也不重复计入冷启动豁免次数 | 契约的幂等承诺此前没有落点：`usage_log` 里 score 行是条目级的，直接按行判断会把「同一会话的 N 条注入」误判成 N 次提交 | `submit_feedback` 的响应形状不变；重复提交时 `applied_weight_deltas` 回放上次的结果 |
| 11 | §2.3 冷启动豁免「前 10 次使用」 | 计数口径 = **已提交过反馈的 (会话, 评分) 组合数**，不是 `usage_log` 里 score 行的条数 | 一次反馈会给多条被注入条目各写一行 score，按行数统计会让豁免提前结束；而一次都没注入记忆的反馈同样是一次使用，按行数统计又会让豁免永远不解除 | 权重规则本身（≥4 → +1 / ≤2 → −2 / 其余 0）不变 |
| 12 | §2.6 夜间任务：`conflicted_count`「检测到矛盾、需要人工裁决的条目数」/ `pending_review_ids`「挂起待人工裁决的记忆 ID」 | **两者语义拆开**：新建 candidate 进 `pending_review_ids` 但**不**计入 `conflicted_count`；`conflicted_count` 只计「检测到矛盾」的**既有条目**（跨 domain 同信号、同域重复信号），且这类候选**不写入** memory.md | 此前实现把每个新建 candidate 都计成「矛盾条目」，报告里分不清「待确认的新规则」与「真冲突」；`docs/02-私人记忆库设计.md` §4.3 防线 1 要求矛盾条目**挂起不自动写入** | 13 个响应字段与取值域不变；另新增一条降级动作：`weight < −1` 或连续 3 次未命中的条目状态降为 `deprecated`（AGENTS.md 状态机） |
| 13 | §2.5 `retrieve_memories`「阶段二 FTS5 BM25（50–300 条）」 | **阶段二已可用**：分词器改为 `trigram`；有有效 `query` 时 `stage=fts5_bm25`，无命中或无 query 回退 `markdown_domain`。条数门槛不再作运行时闸门（800 token 预算下很难攒到 50 条 active） | `memory.md` ≤800 token 使「≥50 条才开 FTS」几乎永不触发；换 trigram 后立刻启用更贴产品「越用越聪明」目标。阶段三（`vec_knn_rrf`）仍冻结 | 响应形状不变；`stage` 可为 `fts5_bm25` 或 `markdown_domain`；旧库 `user_version<2` 自动重建 FTS |
| 14 | §一 命令清单共 8 个 | **新增第 9 个命令 `begin_dot_drag`**（无入参，返回 `DotDragOutcome`） | 圆球既要「按住左键拖走」又要「单击展开」。Tauri 的 `data-tauri-drag-region` 在 mousedown 时即启动 Windows 系统拖拽循环并独占鼠标，浏览器从此收不到 `mouseup`、也不再派发 `click` —— 实测把 click 处理器挂在拖拽区上，单击毫无反应。因此「算点击还是算拖拽」只能由后端从系统层面（轮询左键状态 + 光标位移）判定 | 前端 `Dot.tsx` 在 `mousedown` 时调用本命令；判定为点击时**后端自行展开面板**（返回 `clicked`），判定为拖拽时返回 `dragged` 且不做其他动作 |
| 15 | §一 命令清单共 8 个 | **新增第 10 个命令 `set_panel_view`**（入参 `{ view: "prompt" \| "settings" }`） | 「点面板外部收起」在**设置视图**下是误伤：用户正要去别的应用复制 API key，点外部收起面板等于把填了一半的表单弄丢。后端必须知道当前视图，才能区分这两种「失焦」 | 前端视图切换时调用本命令同步给后端；托盘「设置」由后端先行记下视图、再广播 `pad://view` |
| 16 | §2.8「全量覆盖写入」 | 设置表单**自动保存**：字段改动停手 500 ms 即写盘，离开字段立即写盘；只有取值合法的字段才被写入 | 用户填一半配置就要切到别的窗口复制内容，手动点「保存」必然丢数据。而「边打字边存」若不逐字段校验，会把 `https://` 这种中间态写进配置 | `set_config` 的入参/出参形状不变；变化的是**调用频率与字段子集**（每次只提交改动过且合法的字段） |
| 17 | §2.7 / §2.8 配置只有 `provider` / `endpoint` / `model_name` / `embedding_model` | **新增 `protocol` 字段**（`openai_chat_completions` / `anthropic_messages`）；`provider` 枚举扩为四值（新增 `anthropic`） | 提供方与协议是**两件事**：同一个 MiniMax 同时提供 OpenAI 兼容端点与 Anthropic 兼容端点（Token Plan 订阅走的是后者），Claude Code 一类工具同样走 Anthropic 协议。只有 `provider` 表达不了「用 MiniMax、但说 Anthropic 的话」 | `AppConfig` / `SetConfigRequest` 增加 `protocol` 属性；`Provider` 增加 `anthropic`；`optimize_prompt` 响应增加 `protocol` 回显 |
| 18 | §4.5「API key 只存系统钥匙串或环境变量」 | 新增**本地明文字段 `api_key`**（存于 `%APPDATA%` 下的 config.json），优先级高于 `api_key_env`；`api_key_env` 保留为备选来源 | 「粘贴即用」是接入新服务最短的路径：只支持环境变量时，用户必须先开一个 PowerShell 窗口设变量，才能用一次改写。AGENTS.md §4 第 6 条原文只允许「环境变量或系统钥匙串」，此处放宽为**本机明文**（不进 git、不进 `memory/`） | `SetConfigRequest` 增加 `api_key`（只写入）；`AppConfig` **不回显**明文，仍只回 `api_key_present` |
| 19 | §2.7 `endpoint`「模型端点，由用户在设置页填写」 | 路径补全规则明确化：已是完整请求路径（`/v1/chat/completions`、`/v1/messages`）→ **原样使用**；只到版本段（`/v1`）→ 补协议尾段；协议根 → 补协议默认路径 | 原实现无条件拼 `"{endpoint}/chat/completions"`，用户粘贴完整 URL 会得到 `.../chat/completions/chat/completions`，请求必然 404 | `endpoint` 字段语义不变，只是解析规则明确；**默认端点只存在于前端预设**，Rust 侧不内置任何服务地址 |

**未受影响的契约要素**（逐条核对一致）：8 个命令名、全部错误码（16 个，小写蛇形）、`retrieve_memories` 的分页语义与切片规则、`injection_set` 只含 `statement`/`why`/`how_to_apply`/`memory_id` 四字段、`candidate` 永不返回、`save_pe_record` 的 `written_layers` 取值域、`submit_feedback` 的权重规则（≥4 → +1 / ≤2 → −2 / 其余 0）、夜间任务的 13 个响应字段。

---

## 〇、场景适配声明

ARC-04 原文要求：「必须输出可被 OpenAPI 3.1 校验的 paths 与 components.schemas 片段」一项，在本项目以**「Tauri command 签名表（invoke 名 + serde 参数/返回类型）+ 每参数 JSON Schema」等价替代**，其余各节照常执行。本文档的对应关系：

| ARC-04 要求 | 本文档落点 | 等价性说明 |
|---|---|---|
| OpenAPI `paths` 段 | §五·1「Tauri command 签名表」 | 每个 invoke 名映射到唯一 request/response Schema 引用，与 `paths` 的 operation 一一对应 |
| OpenAPI `components.schemas` 段 | §五·2「components.schemas 片段」 | 使用标准 JSON Schema 2020-12 语法，可被 OpenAPI 3.1 校验器直接校验 |
| HTTP 方法 / 路径 | §一「接口清单表」的 方法 / 路径 两列 | 方法一律 `invoke`；路径列是**逻辑标识符** `tauri://command/<invoke_name>`，**不是 HTTP 路由**，前端不发起网络请求 |
| HTTP 状态码 | §三 错误码表的「HTTP 状态」列 | Tauri command 场景一律写 `N/A`，业务码与触发条件照常 |
| 鉴权请求头 | §四·5「API key 注入方式约定」 | 场景改写：无请求头；API key 仅存系统钥匙串 / 环境变量，由 Rust 侧读取 |

---

## 一、接口清单表

| 方法 | 路径（逻辑标识符） | 用途 | 鉴权级别 | 幂等性 | 超时 |
|---|---|---|---|---|---|
| `invoke` | `tauri://command/toggle_panel` | 展开/收起 Pad 面板（DOT 点击、全局热键 `Ctrl+Space`、托盘菜单共用） | L0 本机 IPC | **非幂等**：状态翻转，重复调用结果不同 | 500 ms（含 150–250 ms 展开/收起动画） |
| `invoke` | `tauri://command/optimize_prompt` | 提交原始诉求文本，调用当前配置的模型改写提示词 | L0 + secret | **非幂等**：每次调用产生新输出并产生 API 调用费 | **可配** `optimize_timeout_ms`，默认 30 000 ms，上限 300 000 ms |
| `invoke` | `tauri://command/save_pe_record` | 把本次改写落库为 `PE-YYYY-NNN` 记录（写入 L1 `prompt_engineering.md` + L3 SQLite） | L0 + write | **非幂等**：L1 事实层只追加，重复调用追加两条 | 3 000 ms |
| `invoke` | `tauri://command/submit_feedback` | 用户对改写结果评分 1–5，驱动记忆权重 +1 / 0 / −2 与连续未命中计数 | L0 + write | **幂等**：同一 `record_id` + 同一 `rating` 重复提交不累加 weight | 3 000 ms |
| `invoke` | `tauri://command/retrieve_memories` | 按当前任务 domain 检索记忆注入集（阶段一 domain 路由 → 阶段二 FTS5 BM25 → 阶段三向量 KNN + RRF） | L0 本机 IPC | **幂等**：只读，不写库 | 5 000 ms（阶段一/二）／30 000 ms（阶段三，含本地嵌入推理） |
| `invoke` | `tauri://command/run_nightly_job` | 夜间记忆沉淀流水线：扫描当天 PE 记录 → signals 去重合并 → 相似度匹配 → token_budget 检查 → 索引重建 | L0 + write | **条件幂等**：同一 `run_date` 不重复沉淀；重跑需人工确认 | 1 800 000 ms（30 min） |
| `invoke` | `tauri://command/get_config` | 读取模型配置（`provider` / `endpoint` / `model_name` / `embedding_model`）与超时配置 | L0 本机 IPC | **幂等**：只读 | 500 ms |
| `invoke` | `tauri://command/set_config` | 全量覆盖模型配置与超时配置 | L0 + write | **幂等**：全量覆盖语义，同参数重复设置结果一致 | 1 000 ms |
| `invoke` | `tauri://command/begin_dot_drag` | 监视收起态圆球上的一次按下-抬起，判定「点击」还是「拖拽」 | L0 本机 IPC | **幂等**：只观测，不写库；点击判定为点击时后端自行展开面板 | 与用户按住左键的时长同长（松手即返回） |
| `invoke` | `tauri://command/set_panel_view` | 同步面板当前显示的视图（提示词 / 设置） | L0 本机 IPC | **幂等**：写同一份状态 | 500 ms |

**鉴权级别定义**（本机单机应用，无用户认证与多租户）：

- **L0 本机 IPC**：同进程前端调用，无需任何凭据。
- **L0 + secret**：命令执行时需从**系统钥匙串或环境变量**读取模型 API key；凭据只在 Rust 侧读取，明文不下发前端。
- **L0 + write**：命令会写本机存储（`memory/` 目录或 SQLite），调用前调用方须确认写意图。

---

## 二、逐接口 request / response JSON Schema 与示例

> Schema 语法：JSON Schema 2020-12（与 OpenAPI 3.1 的 `components.schemas` 同源）。
> 长度/条数上限均来自**配置项**，不在代码内硬编码（AGENTS.md §4 第 6 条）；本文给出的默认值是可配置的初始值，标「待实测」。

### 2.1 `toggle_panel`

**用途**：切换 Pad 面板的展开/收起状态。DOT 点击、`Ctrl+Space` 全局热键、托盘菜单均调用此命令（前端只负责转发意图，状态真值在 Rust 侧）。

**Request Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "TogglePanelRequest",
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "action": {
      "type": "string",
      "enum": ["toggle", "expand", "collapse"],
      "default": "toggle",
      "description": "展开/收起动作。DOT 点击与 Ctrl+Space 热键均传 toggle；expand/collapse 供托盘菜单「显示/隐藏」使用。"
    }
  },
  "required": []
}
```

**Request 示例**

```json
{
  "action": "toggle"
}
```

**Response Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "TogglePanelResponse",
  "type": "object",
  "additionalProperties": false,
  "required": ["expanded", "animation_ms", "position"],
  "properties": {
    "expanded": {
      "type": "boolean",
      "description": "切换后的面板状态：true=已展开，false=已收起（收起态点击穿透、40-56px DOT）。"
    },
    "animation_ms": {
      "type": "integer",
      "minimum": 150,
      "maximum": 250,
      "description": "本次实际播放的展开/收起动画时长，落在 150-250ms 约束区间内。"
    },
    "position": {
      "$ref": "#/components/schemas/WindowAnchor",
      "description": "面板锚点位置，持久化口径为「哪块屏 + 屏内百分比坐标」，非绝对像素。"
    }
  }
}
```

**Response 示例**

```json
{
  "expanded": true,
  "animation_ms": 200,
  "position": {
    "monitor_index": 0,
    "x_ratio": 0.92,
    "y_ratio": 0.08
  }
}
```

**边界与错误**：`action` 非法取值 → `invalid_type`；写入锚点位置失败（磁盘只读）→ `storage_write_failed`。

---

### 2.2 `optimize_prompt`

**用途**：提交原始诉求文本，调用当前配置的 provider（`minimax` / `openai_compat` / `ollama` / `anthropic`）与协议（`openai_chat_completions` / `anthropic_messages`）改写提示词。前端先调 `retrieve_memories` 取得注入集，再把命中条目的 `memory_id` 经 `memory_ids` 传入；Rust 侧只把 `statement` + `why` + `how_to_apply` 拼进模型上下文。

**Request Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "OptimizePromptRequest",
  "type": "object",
  "additionalProperties": false,
  "required": ["original_prompt"],
  "properties": {
    "original_prompt": {
      "type": "string",
      "minLength": 1,
      "maxLength": 20000,
      "description": "原始诉求文本。空串或纯空白按空输入处理，返回 invalid_input；超过配置项 max_prompt_chars（默认 20000，待实测）返回 payload_too_large。"
    },
    "domain": {
      "$ref": "#/components/schemas/Domain",
      "description": "任务域，用于记忆路由与 L1 记录归档。"
    },
    "memory_ids": {
      "type": "array",
      "maxItems": 20,
      "items": {
        "type": "string",
        "pattern": "^M-[A-Z]+-[0-9]{3,}$"
      },
      "description": "由 retrieve_memories 返回、准备注入的记忆 ID。后端只接受 status=active 且 weight>0 的条目，并按 weight 降序裁剪至总量 ≤800 token。"
    }
  }
}
```

**Request 示例**

```json
{
  "original_prompt": "帮我写一个带 JWT 登录校验的 Express 中间件",
  "domain": "code",
  "memory_ids": ["M-CODE-001", "M-CODE-004"]
}
```

**Response Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "OptimizePromptResponse",
  "type": "object",
  "additionalProperties": false,
  "required": ["optimized_prompt", "provider", "protocol", "model_name", "injected_memory_ids", "injected_token_count", "elapsed_ms", "domain"],
  "properties": {
    "optimized_prompt": {
      "type": "string",
      "minLength": 1,
      "description": "改写后的提示词正文，可直接复制回原应用。"
    },
    "provider": {
      "$ref": "#/components/schemas/Provider"
    },
    "protocol": {
      "$ref": "#/components/schemas/Protocol",
      "description": "本次实际使用的报文协议（回显，便于排障）。"
    },
    "model_name": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128,
      "description": "实际生效的模型名（回显当前配置，便于排障）。"
    },
    "injected_memory_ids": {
      "type": "array",
      "maxItems": 20,
      "items": {
        "type": "string",
        "pattern": "^M-[A-Z]+-[0-9]{3,}$"
      },
      "description": "实际进入模型上下文的记忆 ID（已按 800 token 预算裁剪后的真实集合）。"
    },
    "injected_token_count": {
      "type": "integer",
      "minimum": 0,
      "maximum": 800,
      "description": "本次注入上下文的 token 估算值，硬上限 800。tokenizer 选择待确认。"
    },
    "elapsed_ms": {
      "type": "integer",
      "minimum": 0,
      "description": "端到端耗时（含 provider 网络/推理时间）。"
    },
    "domain": {
      "type": "string",
      "minLength": 1,
      "description": "本次实际用于路由与落库的 domain；缺省为 general。"
    }
  }
}
```

**Response 示例**

```json
{
  "optimized_prompt": "## 角色\n你是一名熟悉 Express 与 JWT 的后端工程师。\n\n## 硬性约束\n- 令牌签名算法固定 HS256，密钥从环境变量读取，不得硬编码。\n- 中间件必须返回 401 并带 WWW-Authenticate 头。\n\n## 输入\n- 需保护的路由路径列表\n\n## 输出\n- 完整中间件代码 + 一段最小可用测试\n\n## 验收\n- 缺失 / 过期 / 篡改三类令牌均返回 401",
  "provider": "minimax",
  "protocol": "anthropic_messages",
  "model_name": "current-configured-model",
  "injected_memory_ids": ["M-CODE-001"],
  "injected_token_count": 186,
  "elapsed_ms": 4210,
  "domain": "code"
}
```

**边界与错误**：空输入 → `invalid_input`；超长 → `payload_too_large`；未配置 provider/endpoint/model_name 或 API key 缺失 → `provider_not_configured`；调用超时 → `model_timeout`（**可恢复**，提示用户重试或调大 `optimize_timeout_ms`）；provider 5xx / 网络不可达 → `model_unavailable`；请求传入不存在的 `memory_ids` → `not_found`（忽略该条并继续，或整体报错，见 §三 `not_found` 触发条件）。

---

### 2.3 `save_pe_record`

**用途**：把一次改写落库为 `PE-YYYY-NNN` 记录。**同时**写入 L1 事实层 `memory/prompt_engineering.md`（永久、逐字不可改、只追加）与 L3 索引层 SQLite。

**Request Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "SavePeRecordRequest",
  "type": "object",
  "additionalProperties": false,
  "required": ["original_prompt", "optimized_prompt"],
  "properties": {
    "original_prompt": {
      "type": "string",
      "minLength": 1,
      "maxLength": 20000,
      "description": "提示词原文。空串按空输入处理；超长返回 payload_too_large。"
    },
    "optimized_prompt": {
      "type": "string",
      "minLength": 1,
      "maxLength": 20000,
      "description": "改写结果。空串按空输入处理；超长返回 payload_too_large。"
    },
    "domain": {
      "$ref": "#/components/schemas/Domain"
    },
    "signals": {
      "type": "array",
      "maxItems": 32,
      "items": {
        "type": "string",
        "minLength": 1,
        "maxLength": 64
      },
      "description": "本次改写暴露的信号标签，供夜间任务去重合并与相似度匹配使用；L1 记录含 signals 字段。"
    },
    "injected_memory_ids": {
      "type": "array",
      "maxItems": 20,
      "items": {
        "type": "string",
        "pattern": "^M-[A-Z]+-[0-9]{3,}$"
      },
      "description": "本次改写实际注入的记忆 ID（来自 optimize_prompt 响应的 injected_memory_ids）。后端据此写入 usage_log 的 inject 事件并累加 hits，使后续 submit_feedback 能定位到被注入的条目。不传表示本次未注入任何条目。"
    },
    "rating": {
      "type": "integer",
      "minimum": 1,
      "maximum": 5,
      "default": null,
      "description": "可选初始评分；未评分时 PE 记录该字段为 null，后续由 submit_feedback 回填。"
    }
  }
}
```

**Request 示例**

```json
{
  "original_prompt": "帮我写一个带 JWT 登录校验的 Express 中间件",
  "optimized_prompt": "## 角色\n你是一名熟悉 Express 与 JWT 的后端工程师。…",
  "domain": "code",
  "signals": ["S-STRUCTURE-ROLE", "S-HARD-RETURN-CODE"],
  "rating": 4
}
```

**Response Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "SavePeRecordResponse",
  "type": "object",
  "additionalProperties": false,
  "required": ["record_id", "created_at", "written_layers"],
  "properties": {
    "record_id": {
      "type": "string",
      "pattern": "^PE-[0-9]{4}-[0-9]{3,}$",
      "description": "L1 事实层记录 ID，格式 PE-YYYY-NNN，NNN 为该年自增序号。"
    },
    "created_at": {
      "type": "string",
      "format": "date-time",
      "description": "UTC RFC 3339 时间戳。"
    },
    "written_layers": {
      "type": "array",
      "minItems": 1,
      "maxItems": 2,
      "items": {
        "type": "string",
        "enum": ["l1_markdown", "l3_sqlite"]
      },
      "description": "本次实际落盘到的层。L1 只追加、逐字不可改；L3 可重建，以 Markdown 为准。"
    }
  }
}
```

**Response 示例**

```json
{
  "record_id": "PE-2026-042",
  "created_at": "2026-10-21T10:32:07Z",
  "written_layers": ["l1_markdown", "l3_sqlite"]
}
```

**边界与错误**：空输入 → `invalid_input`；类型错误（`rating` 越界、`signals` 非数组）→ `invalid_type`；超量超长（字段长度或 `signals` 条数超限）→ `payload_too_large`；写入 `memory/` 失败 → `storage_write_failed`；序号冲突（并发写入导致同 ID）→ `duplicate_record`。

---

### 2.4 `submit_feedback`

**用途**：用户对某次改写结果评分 1–5，驱动记忆权重更新（评分 ≥4 → `+1`；无改善 → `0`；变差 → `−2`）与连续未命中计数；并可能触发状态机迁移（`candidate` → `active` / `active` → `deprecated`）。

**Request Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "SubmitFeedbackRequest",
  "type": "object",
  "additionalProperties": false,
  "required": ["record_id", "rating"],
  "properties": {
    "record_id": {
      "type": "string",
      "pattern": "^PE-[0-9]{4}-[0-9]{3,}$",
      "description": "被评分的那次改写对应的 PE 记录 ID。后端据此回查 usage_log 中该次任务的注入集合。"
    },
    "rating": {
      "type": "integer",
      "minimum": 1,
      "maximum": 5,
      "description": "1–5 分。映射规则：≥4 → 权重 +1；=3（无明显改善）→ 权重 0；≤2（变差/跑偏）→ 权重 −2。"
    },
    "note": {
      "type": "string",
      "maxLength": 500,
      "description": "可选备注，仅落 usage_log 供人工复核，不注入模型上下文。"
    }
  }
}
```

**Request 示例**

```json
{
  "record_id": "PE-2026-042",
  "rating": 5,
  "note": "分层结构比上次清楚"
}
```

**Response Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "SubmitFeedbackResponse",
  "type": "object",
  "additionalProperties": false,
  "required": ["record_id", "rating", "applied_weight_deltas", "updated_at"],
  "properties": {
    "record_id": {
      "type": "string",
      "pattern": "^PE-[0-9]{4}-[0-9]{3,}$"
    },
    "rating": {
      "type": "integer",
      "minimum": 1,
      "maximum": 5
    },
    "applied_weight_deltas": {
      "type": "array",
      "maxItems": 20,
      "items": {
        "$ref": "#/components/schemas/WeightDelta"
      },
      "description": "本次评分实际产生的权重变更；无注入命中时为空数组。"
    },
    "status_changes": {
      "type": "array",
      "maxItems": 20,
      "items": {
        "$ref": "#/components/schemas/StatusChange"
      },
      "description": "本次评分触发的记忆状态机迁移。"
    },
    "updated_at": {
      "type": "string",
      "format": "date-time"
    }
  }
}
```

**Response 示例**

```json
{
  "record_id": "PE-2026-042",
  "rating": 5,
  "applied_weight_deltas": [
    {
      "memory_id": "M-CODE-001",
      "delta": 1,
      "weight_before": 1,
      "weight_after": 2
    },
    {
      "memory_id": "M-CODE-004",
      "delta": 1,
      "weight_before": 0,
      "weight_after": 1
    }
  ],
  "status_changes": [
    {
      "memory_id": "M-CODE-001",
      "from": "candidate",
      "to": "active",
      "reason": "validated"
    }
  ],
  "updated_at": "2026-10-21T10:35:02Z"
}
```

**边界与错误**：`record_id` 不存在 → `not_found`（**可恢复**：提示用户重新选择记录）；`rating` 越界或非整数 → `invalid_type`；`record_id` 格式不符 → `invalid_input`；并发写库与夜间任务冲突 → `lock_conflict`（**可恢复**：稍后重试）。

---

### 2.5 `retrieve_memories`

**用途**：按当前任务 domain 检索记忆注入集。阶段一纯 Markdown domain 路由（<50 条）→ 阶段二 FTS5 BM25（50–300 条）→ 阶段三向量 KNN + RRF（300+ 条）。

**强约束**（本命令的核心验收点）：

1. 过滤条件固定为 `domain` 匹配 + `status=active` + `weight>0`，按 `weight` 降序取 Top-K。
2. **只返回** `statement` + `why` + `how_to_apply`（外加 `memory_id` 用于回链）；**绝不返回** `evidence` / `weight` / `hits` / `status` 等统计与治理字段（AGENTS.md §6 第 5 条）。
3. **`candidate` 绝不返回**；`deprecated` 不返回且不占 token 预算。
4. 注入总量 **≤ 800 token** 硬上限，超出按 weight 从低到高裁剪。
5. 无匹配时返回明确的「暂无历史经验」标识（`matched=false` + `no_memory_message="暂无历史经验"`），不得塞入低分噪声。

**Request Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "RetrieveMemoriesRequest",
  "type": "object",
  "additionalProperties": false,
  "required": ["domain"],
  "properties": {
    "domain": {
      "$ref": "#/components/schemas/Domain",
      "description": "任务域，阶段一路由主键；阶段二/三在此基础上叠加全文/向量召回。"
    },
    "query": {
      "type": "string",
      "maxLength": 2000,
      "description": "检索查询串。阶段一 domain 路由可为空；阶段二用于 FTS5 BM25；阶段三用于向量 KNN。"
    },
    "top_k": {
      "type": "integer",
      "minimum": 1,
      "maximum": 20,
      "default": 5,
      "description": "Top-K 条数上限（分页步长）。默认 5，上限 20。实际返回条数还会被 800 token 预算二次裁剪。"
    },
    "offset": {
      "type": "integer",
      "minimum": 0,
      "maximum": 1000,
      "default": 0,
      "description": "分页偏移，默认 0。注入场景必须使用默认 0，保证注入的始终是 weight 降序的全局 Top-K；offset>0 仅供记忆库浏览翻页。"
    }
  }
}
```

**Request 示例**

```json
{
  "domain": "code",
  "query": "JWT 中间件 401 处理",
  "top_k": 5,
  "offset": 0
}
```

**Response Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "RetrieveMemoriesResponse",
  "type": "object",
  "additionalProperties": false,
  "required": ["matched", "stage", "injection_set", "returned_count", "total_matched_count", "offset", "top_k", "has_more", "total_token_count", "token_budget"],
  "properties": {
    "matched": {
      "type": "boolean",
      "description": "是否命中可用记忆。false 表示本次任务无历史经验，调用方不得注入任何条目。"
    },
    "stage": {
      "type": "string",
      "enum": ["markdown_domain", "fts5_bm25", "vec_knn_rrf"],
      "description": "实际生效的检索阶段。阶段由记忆条目规模决定（<50 / 50-300 / 300+），不由调用方指定。"
    },
    "no_memory_reason": {
      "type": ["string", "null"],
      "enum": ["no_matching_memory", null],
      "description": "未命中原因；命中时为 null。"
    },
    "no_memory_message": {
      "type": ["string", "null"],
      "maxLength": 64,
      "description": "未命中时的固定提示文案「暂无历史经验」；命中时为 null。"
    },
    "injection_set": {
      "type": "array",
      "maxItems": 20,
      "items": {
        "$ref": "#/components/schemas/MemoryInjectionItem"
      },
      "description": "注入集条目，只含 statement / why / how_to_apply 与回链用的 memory_id。"
    },
    "returned_count": {
      "type": "integer",
      "minimum": 0,
      "description": "本次实际返回条数（预算裁剪后）。"
    },
    "total_matched_count": {
      "type": "integer",
      "minimum": 0,
      "description": "过滤后命中的总条数，供分页判断。"
    },
    "offset": {
      "type": "integer",
      "minimum": 0
    },
    "top_k": {
      "type": "integer",
      "minimum": 1,
      "maximum": 20
    },
    "has_more": {
      "type": "boolean",
      "description": "本页切片未覆盖全部命中结果时为 true（按分页切片判定，不受 token 预算裁剪影响；见 §〇 修订第 9 条）。"
    },
    "total_token_count": {
      "type": "integer",
      "minimum": 0,
      "maximum": 800,
      "description": "注入集 token 估算总量，硬上限 800。"
    },
    "token_budget": {
      "type": "integer",
      "minimum": 0,
      "maximum": 800,
      "default": 800,
      "description": "本次生效的注入预算，来自 memory.md 文件头的 token_budget。"
    }
  }
}
```

**Response 示例（命中）**

```json
{
  "matched": true,
  "stage": "fts5_bm25",
  "no_memory_reason": null,
  "no_memory_message": null,
  "injection_set": [
    {
      "memory_id": "M-CODE-001",
      "statement": "涉及鉴权时，必须把「未通过校验时的返回码与响应头」单列为硬性约束。",
      "why": "模型默认只写成功路径，遗漏鉴权失败分支会让调用方线上报错。",
      "how_to_apply": "在提示词中单开一章「鉴权失败约定」，逐条列出 401/403 与必需响应头。"
    },
    {
      "memory_id": "M-CODE-004",
      "statement": "涉及密钥时，必须要求从环境变量读取，不得在示例代码中硬编码。",
      "why": "示例代码常被直接复制，硬编码密钥会造成泄露。",
      "how_to_apply": "在提示词中追加一句「所有密钥用环境变量占位符表示」。"
    }
  ],
  "returned_count": 2,
  "total_matched_count": 2,
  "offset": 0,
  "top_k": 5,
  "has_more": false,
  "total_token_count": 186,
  "token_budget": 800
}
```

**Response 示例（未命中）**

```json
{
  "matched": false,
  "stage": "markdown_domain",
  "no_memory_reason": "no_matching_memory",
  "no_memory_message": "暂无历史经验",
  "injection_set": [],
  "returned_count": 0,
  "total_matched_count": 0,
  "offset": 0,
  "top_k": 5,
  "has_more": false,
  "total_token_count": 0,
  "token_budget": 800
}
```

**边界与错误**：`domain` 缺失或非法 → `invalid_input`；`query` 超长 → `payload_too_large`；`top_k` / `offset` 越界 → `invalid_type`；请求阶段三向量检索但尚未建立向量索引 → `stage_not_reached`；向量库中记录模型名与当前 `embedding_model` 不一致 → `embedding_model_mismatch`（**不可恢复**：须重建全库索引）。

---

### 2.6 `run_nightly_job`

**用途**：夜间记忆沉淀流水线。扫描当天 PE 记录 → signals 去重合并 → 与 `memory.md` 条目做相似度匹配（匹配则 `weight+1` 并补 `evidence`；无匹配且达标则新建 `candidate`；与已有条目矛盾则挂起人工裁决）→ 检查 `token_budget` → 重建/增量更新索引。

**Request Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "RunNightlyJobRequest",
  "type": "object",
  "additionalProperties": false,
  "required": [],
  "properties": {
    "run_date": {
      "type": "string",
      "pattern": "^\\d{4}-\\d{2}-\\d{2}$",
      "description": "处理日期，本机本地时区当日；省略时取当日。仅允许处理已结束的日期（不处理今天，以免与当天写入竞争）。"
    },
    "dry_run": {
      "type": "boolean",
      "default": false,
      "description": "true 时只做扫描、去重与匹配并返回统计，不写 memory.md、不改 SQLite、不重建索引。"
    }
  }
}
```

**Request 示例**

```json
{
  "run_date": "2026-10-20",
  "dry_run": false
}
```

**Response Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "RunNightlyJobResponse",
  "type": "object",
  "additionalProperties": false,
  "required": ["run_date", "scanned_record_count", "matched_entry_count", "created_candidate_count", "weight_bumped_count", "conflicted_count", "pending_review_ids", "token_usage_before", "token_usage_after", "token_budget", "index_rebuilt", "started_at", "finished_at"],
  "properties": {
    "run_date": {
      "type": "string",
      "pattern": "^\\d{4}-\\d{2}-\\d{2}$"
    },
    "scanned_record_count": {
      "type": "integer",
      "minimum": 0,
      "description": "扫描到的当天 PE 记录条数。"
    },
    "matched_entry_count": {
      "type": "integer",
      "minimum": 0,
      "description": "与既有记忆条目相似度匹配成功的条数。"
    },
    "created_candidate_count": {
      "type": "integer",
      "minimum": 0,
      "description": "新建的 candidate 条目数。系统只允许自动创建 candidate，不得自动写入 active。"
    },
    "weight_bumped_count": {
      "type": "integer",
      "minimum": 0,
      "description": "因匹配到既有条目而 weight +1 的条目数。"
    },
    "conflicted_count": {
      "type": "integer",
      "minimum": 0,
      "description": "检测到矛盾、需要人工裁决的既有条目数；新建 candidate 不计入此数（见 §〇 修订第 12 条）。"
    },
    "pending_review_ids": {
      "type": "array",
      "maxItems": 100,
      "description": "挂起待人工裁决的记忆 ID：本次新建的 candidate 与检测到矛盾的既有条目都在其中；矛盾条目不会被自动写入 memory.md。",
      "items": {
        "type": "string",
        "pattern": "^M-[A-Z]+-[0-9]{3,}$"
      }
    },
    "token_usage_before": {
      "type": "integer",
      "minimum": 0,
      "maximum": 800
    },
    "token_usage_after": {
      "type": "integer",
      "minimum": 0,
      "maximum": 800,
      "description": "写入后的 memory.md token 占用，硬上限 800。"
    },
    "token_budget": {
      "type": "integer",
      "minimum": 0,
      "maximum": 800,
      "default": 800
    },
    "index_rebuilt": {
      "type": "boolean",
      "description": "是否重建/增量更新了 L3 索引。"
    },
    "started_at": {
      "type": "string",
      "format": "date-time"
    },
    "finished_at": {
      "type": "string",
      "format": "date-time"
    }
  }
}
```

**Response 示例**

```json
{
  "run_date": "2026-10-20",
  "scanned_record_count": 6,
  "matched_entry_count": 2,
  "created_candidate_count": 1,
  "weight_bumped_count": 2,
  "conflicted_count": 1,
  "pending_review_ids": ["M-CODE-003"],
  "token_usage_before": 612,
  "token_usage_after": 668,
  "token_budget": 800,
  "index_rebuilt": true,
  "started_at": "2026-10-21T02:00:00Z",
  "finished_at": "2026-10-21T02:00:06Z"
}
```

**边界与错误**：`run_date` 格式非法或为未来日期 → `invalid_input`；与 `submit_feedback` 并发写库 → `lock_conflict`（**可恢复**：稍后重试）；写入后 `token_usage_after > 800` → `memory_budget_exceeded`（**不可恢复**：拒绝晋升并回滚本次写入）；检测到矛盾条目 → **不报错**，改为在 `pending_review_ids` 中挂起并计入 `conflicted_count`。

---

### 2.7 `get_config`

**用途**：读取当前模型配置。**只返回** `provider` / `endpoint` / `model_name` / `embedding_model` 与超时配置，并回显 API key 是否存在（**绝不返回 key 明文**）。

**Request Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "EmptyRequest",
  "type": "object",
  "additionalProperties": false,
  "properties": {}
}
```

**Request 示例**

```json
{}
```

**Response Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AppConfig",
  "type": "object",
  "additionalProperties": false,
  "required": ["provider", "protocol", "endpoint", "model_name", "embedding_model"],
  "properties": {
    "provider": {
      "$ref": "#/components/schemas/Provider"
    },
    "protocol": {
      "$ref": "#/components/schemas/Protocol"
    },
    "endpoint": {
      "type": "string",
      "minLength": 1,
      "maxLength": 512,
      "description": "模型端点，由用户在设置页填写。本文不预设任何厂商 URL（AGENTS.md §4 第 6 条：URL 一律经参数或配置传入）。补全规则见 §〇·补 修订表第 19 条。"
    },
    "model_name": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128
    },
    "embedding_model": {
      "type": "string",
      "enum": ["multilingual-e5-small", "bge-m3"],
      "description": "当前嵌入模型名。每条向量存储当时使用的模型名；更换模型必须重建全库索引。"
    },
    "optimize_timeout_ms": {
      "type": "integer",
      "minimum": 1000,
      "maximum": 300000,
      "default": 30000,
      "description": "改写调用超时，可配。上限 300000 ms。"
    },
    "api_key_present": {
      "type": "boolean",
      "readOnly": true,
      "default": false,
      "description": "只读指示位：本地明文 api_key 或环境变量中是否存在可用 API key。绝不返回 key 明文。provider=ollama 时恒为 false。"
    }
  }
}
```

**Response 示例**

```json
{
  "provider": "minimax",
  "protocol": "anthropic_messages",
  "endpoint": "https://<provider-host>/anthropic",
  "model_name": "current-configured-model",
  "embedding_model": "multilingual-e5-small",
  "optimize_timeout_ms": 30000,
  "api_key_present": true
}
```

> 示例中的 `endpoint` 为占位符，实际值由用户在设置页填写。

**边界与错误**：配置文件损坏或缺失 → `provider_not_configured`（**可恢复**：引导用户到设置页填写）；配置文件不可读 → `storage_read_failed`。

---

### 2.8 `set_config`

**用途**：全量覆盖写入模型配置与超时配置。

**Request Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "SetConfigRequest",
  "type": "object",
  "additionalProperties": false,
  "required": ["provider", "protocol", "endpoint", "model_name", "embedding_model"],
  "properties": {
    "provider": {
      "$ref": "#/components/schemas/Provider"
    },
    "protocol": {
      "$ref": "#/components/schemas/Protocol"
    },
    "endpoint": {
      "type": "string",
      "minLength": 1,
      "maxLength": 512,
      "description": "模型端点，由用户填写；本接口不内置任何厂商默认 URL。补全规则见 §〇·补 修订表第 19 条。"
    },
    "model_name": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128
    },
    "embedding_model": {
      "type": "string",
      "enum": ["multilingual-e5-small", "bge-m3"],
      "description": "切换嵌入模型时必须重建全库索引；本命令只切换配置，重建由后台索引流程完成。"
    },
    "optimize_timeout_ms": {
      "type": "integer",
      "minimum": 1000,
      "maximum": 300000,
      "default": 30000,
      "description": "改写调用超时，可配。"
    },
    "api_key": {
      "type": "string",
      "maxLength": 512,
      "writeOnly": true,
      "description": "API key 明文（只写入，绝不回显）。空字符串表示清除已存的本地明文。见 §〇·补 修订表第 18 条。"
    }
  }
}
```

> `api_key` 是 `writeOnly`：**只进不出**。响应里永远只有 `api_key_present` 指示位。
> `api_key_env`（环境变量名）作为备选来源保留，语义与优先级见 §4.5。

**Request 示例**

```json
{
  "provider": "ollama",
  "protocol": "openai_chat_completions",
  "endpoint": "http://<host>:<port>/v1/chat/completions",
  "model_name": "current-configured-model",
  "embedding_model": "multilingual-e5-small",
  "optimize_timeout_ms": 60000
}
```

**Response Schema**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "SetConfigResponse",
  "type": "object",
  "additionalProperties": false,
  "required": ["config", "updated_at"],
  "properties": {
    "config": {
      "$ref": "#/components/schemas/AppConfig",
      "description": "写入后生效的完整配置回显。"
    },
    "updated_at": {
      "type": "string",
      "format": "date-time"
    }
  }
}
```

**Response 示例**

```json
{
  "config": {
    "provider": "ollama",
    "endpoint": "http://<host>:<port>/v1/chat/completions",
    "model_name": "current-configured-model",
    "embedding_model": "multilingual-e5-small",
    "optimize_timeout_ms": 60000,
    "api_key_present": false
  },
  "updated_at": "2026-10-21T09:12:44Z"
}
```

**边界与错误**：`provider` / `protocol` 不在枚举内 → `invalid_type`；`endpoint` / `model_name` 为空串 → `invalid_input`；`embedding_model` 非法 → `invalid_type`；`optimize_timeout_ms` 越界 → `invalid_type`；`api_key_env` 不是合法环境变量名 → `invalid_input`；配置文件写入失败 → `storage_write_failed`。

### 2.9 `begin_dot_drag`

> v0.2 新增。契约清单 8 → 9 个命令，理由见 §〇·补 修订表第 14 条。

**用途**：收起态圆球既要能按住左键拖走，又要单击就展开面板。Tauri 拖拽区按下即启动系统拖拽循环并独占鼠标，浏览器收不到 `mouseup`、也不再派发 `click`，因此判定必须由后端从系统层面完成。

**request**：无入参（前端 `invoke("begin_dot_drag")`）。

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "EmptyRequest",
  "type": "object",
  "additionalProperties": false,
  "properties": {}
}
```

**response**：

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "DotDragOutcome",
  "type": "string",
  "enum": ["clicked", "dragged"],
  "description": "clicked = 原地按下又抬起（后端已展开面板）；dragged = 按下后移动过（圆球已被拖走）"
}
```

**行为约定**：

- 命令在左键松开时返回；等待期间不阻塞 UI 线程（轮询跑在阻塞线程池）。
- 位移阈值 4 物理像素：小于等于阈值算点击，大于算拖拽。阈值取 4 是因为 Windows 拖拽循环 1 像素抖动就会移动窗口，阈值太小会把用户手抖当成拖拽，表现为「点圆球没反应」。
- 判定为 `clicked` 时**后端自行展开面板**（调用 `apply(Expanded)`），前端不再重复调 `toggle_panel`，避免双重切换。

### 2.10 `set_panel_view`

> v0.2 新增。契约清单 8 → 10 个命令，理由见 §〇·补 修订表第 15 条。

**用途**：把「面板当前显示的是提示词还是设置」同步给后端。后端据此决定「点面板外部收起」在设置视图下是否生效 —— 设置界面里用户要去别的应用复制 API key，收起面板等于把填了一半的表单弄丢。

**request**：

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "PanelViewRequest",
  "type": "object",
  "additionalProperties": false,
  "required": ["view"],
  "properties": {
    "view": {
      "type": "string",
      "enum": ["prompt", "settings"]
    }
  }
}
```

**response**：无内容（`()`，序列化为 `null`）。

**边界与错误**：`view` 不在枚举内 → `invalid_type`；视图状态锁中毒 → `lock_conflict`。

---

## 三、统一错误响应结构与错误码表

### 3.1 统一错误响应结构

Rust 侧统一 `AppError` 枚举 + 手写 `Display`（AGENTS.md §4 第 5 条），经 serde 序列化为下列结构；前端 `src/api/client.ts` 统一捕获并转成可展示文案。

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ErrorResponse",
  "type": "object",
  "additionalProperties": false,
  "required": ["code", "message", "details", "recoverable", "command", "occurred_at"],
  "properties": {
    "code": {
      "type": "string",
      "enum": [
        "invalid_input",
        "invalid_type",
        "payload_too_large",
        "model_timeout",
        "model_unavailable",
        "provider_not_configured",
        "memory_budget_exceeded",
        "conflict_pending_review",
        "embedding_model_mismatch",
        "not_found",
        "duplicate_record",
        "storage_write_failed",
        "storage_read_failed",
        "lock_conflict",
        "stage_not_reached",
        "internal_error"
      ],
      "description": "业务错误码（snake_case），稳定不变，前端据此分支处理。"
    },
    "message": {
      "type": "string",
      "minLength": 1,
      "maxLength": 500,
      "description": "面向用户的中文提示文案，不含堆栈与密钥。"
    },
    "details": {
      "type": "object",
      "description": "结构化补充信息，形状随 code 变化；无补充时为空对象。可恢复错误至少包含 retryable 布尔位。"
    },
    "recoverable": {
      "type": "boolean",
      "description": "true=可恢复（用户可重试/改配置/换输入）；false=不可恢复（阻止写入或需人工裁决）。"
    },
    "command": {
      "type": "string",
      "enum": [
        "toggle_panel",
        "optimize_prompt",
        "save_pe_record",
        "submit_feedback",
        "retrieve_memories",
        "run_nightly_job",
        "get_config",
        "set_config"
      ],
      "description": "出错时正在执行的 invoke 名。"
    },
    "occurred_at": {
      "type": "string",
      "format": "date-time"
    }
  }
}
```

**错误响应示例（可恢复）**

```json
{
  "code": "model_timeout",
  "message": "模型调用超时，请稍后重试或在设置中调大超时时间",
  "details": {
    "provider": "minimax",
    "timeout_ms": 30000,
    "retryable": true
  },
  "recoverable": true,
  "command": "optimize_prompt",
  "occurred_at": "2026-10-21T10:32:30Z"
}
```

**错误响应示例（不可恢复，阻止写入）**

```json
{
  "code": "memory_budget_exceeded",
  "message": "记忆库已达 800 token 上限，本次晋升被拒绝，请先裁剪或废弃低权重条目",
  "details": {
    "token_usage_after": 838,
    "token_budget": 800,
    "retryable": false,
    "rejected_entry_ids": ["M-CODE-011"]
  },
  "recoverable": false,
  "command": "run_nightly_job",
  "occurred_at": "2026-10-21T02:00:06Z"
}
```

### 3.2 错误码表

> HTTP 状态列一律 `N/A`：Tauri command 是本机进程内调用，不存在 HTTP 状态码。

| 业务码 | HTTP 状态 | 可恢复性 | message（默认文案） | details 含义 | 触发条件 |
|---|---|---|---|---|---|
| `invalid_input` | N/A | 可恢复 | 「请求内容为空或缺少必填字段」 | `field`（缺失字段名）、`reason`（empty_input / missing_field / malformed_id） | 必填参数为空字符串或纯空白（空输入边界）；`record_id` 不匹配 `PE-YYYY-NNN`；`run_date` 格式非法或为未来日期 |
| `invalid_type` | N/A | 可恢复 | 「参数类型或取值范围不正确」 | `field`、`expected`、`actual` | serde 反序列化类型不匹配（类型错误边界）；`rating` 非 1–5 整数；`top_k`/`offset` 越界；`provider` 不在 `minimax\|openai_compat\|ollama\|anthropic` 内；`protocol` 不在 `openai_chat_completions\|anthropic_messages` 内；`embedding_model` 非法 |
| `payload_too_large` | N/A | 可恢复 | 「内容超出长度上限」 | `field`、`limit`、`actual` | 超量超长边界：`original_prompt`/`optimized_prompt` 超过 `max_prompt_chars`（默认 20000）；`query` 超过 2000；`signals` 条数超过 32；`memory_ids` 条数超过 20 |
| `model_timeout` | N/A | **可恢复** | 「模型调用超时，请稍后重试或在设置中调大超时时间」 | `provider`、`timeout_ms`、`retryable: true` | provider 调用超过可配的 `optimize_timeout_ms`（默认 30 000 ms）；本地 provider 推理超时同码 |
| `model_unavailable` | N/A | **可恢复** | 「模型服务暂时不可用，请稍后重试」 | `provider`、`status`（HTTP 状态或传输层错误）、`retryable: true` | provider 返回 5xx、连接被拒、DNS/网络不可达 |
| `provider_not_configured` | N/A | 可恢复 | 「尚未配置模型接入信息，请到设置页填写」 | `missing`（provider/endpoint/model_name/api_key 之一）、`provider` | `provider`/`endpoint`/`model_name` 缺失或为空；`provider` ≠ `ollama` 而系统钥匙串与环境变量中均无 API key；配置文件不存在或损坏 |
| `memory_budget_exceeded` | N/A | **不可恢复** | 「记忆库已达 800 token 上限，本次晋升被拒绝」 | `token_usage_after`、`token_budget`、`rejected_entry_ids` | 夜间任务写入后 `memory.md` token 占用超过 800 硬上限；本次晋升被拒绝并回滚（AGENTS.md §6 第 7 条） |
| `conflict_pending_review` | N/A | **不可恢复** | 「检测到与既有记忆矛盾的条目，已挂起等待人工裁决」 | `candidate_signals`、`conflicted_entry_ids`、`pending_review_ids` | signals 合并结果与既有条目矛盾或跨 domain 推广；按 AGENTS.md §6 第 6 条不得自动写入，必须人工裁决（夜间任务以 `pending_review_ids` 形式返回，不中断流水线时另见 §2.6 说明） |
| `embedding_model_mismatch` | N/A | **不可恢复** | 「嵌入模型已变更，需重建索引后才能使用向量检索」 | `current_model`、`stored_models`、`stale_count` | 向量库中记录存储的模型名与当前 `embedding_model` 不一致（换模型必须重建全库索引） |
| `not_found` | N/A | 可恢复 | 「未找到对应的记录或记忆条目」 | `record_id` 或 `memory_ids` | `submit_feedback` 引用不存在的 `record_id`；`optimize_prompt` 传入不存在的 `memory_ids` |
| `duplicate_record` | N/A | 可恢复 | 「该 PE 记录 ID 已存在」 | `record_id`、`existing_created_at` | 并发写入导致 `PE-YYYY-NNN` 序号冲突（L1 只追加，不覆盖既有记录） |
| `storage_write_failed` | N/A | 可恢复 | 「写入本机存储失败」 | `target`（`memory.md` / `prompt_engineering.md` / `sqlite`）、`retryable: true` | `memory/` 目录或 SQLite 只读、磁盘满、文件被占用 |
| `storage_read_failed` | N/A | 可恢复 | 「读取本机存储失败」 | `target`、`retryable: true` | 配置文件、`memory.md`、SQLite 文件读取失败或格式非法 |
| `lock_conflict` | N/A | **可恢复** | 「正在执行另一个写操作，请稍后重试」 | `holder`（`run_nightly_job` / `submit_feedback` / `save_pe_record`）、`retryable: true` | 权重更新与夜间批量任务并发写库，或同一 `run_date` 的夜间任务正在运行（AGENTS.md：不得并发写库） |
| `stage_not_reached` | N/A | 可恢复 | 「当前数据规模尚未进入该检索阶段」 | `current_stage`、`required_stage`、`entry_count` | 请求阶段二/三检索能力，但记忆条目规模未达阈值（阶段一 <50 条）；或阶段三所需依赖/索引尚未就绪 |
| `internal_error` | N/A | 不可恢复 | 「内部错误，请查看日志」 | `context`（脱敏后的上下文）、`retryable: false` | 未归类的内部异常兜底；必须同时写入本地日志 |

---

## 四、通用约定

### 4.1 版本前缀与版本策略

- 本项目**无 HTTP 路由**，因此不存在 URL 版本前缀（如 `/api/v1`）。`invoke` 名即接口标识，不带版本号。
- 接口契约版本**跟随应用版本**（`src-tauri/tauri.conf.json` 的 `version` 字段），随安装包一起升级。
- 兼容性判定：新增可选字段、新增枚举值属兼容变更；删除字段、语义变更、枚举值移除属破坏性变更，必须同步更新本文档与 `src/api/types.ts`（AGENTS.md §4 第 3 条：TS 类型与 Rust 字段一一对应，接口类型集中在 `src/api/types.ts`）。
- 8 个 command 名本身是稳定契约：改名即视为破坏性变更。

### 4.2 分页参数与默认值

| 参数 | 适用命令 | 类型 | 默认值 | 下限 | 上限 | 说明 |
|---|---|---|---|---|---|---|
| `offset` | `retrieve_memories` | integer | `0` | 0 | 1000 | 分页偏移，跳过前 N 条命中结果 |
| `top_k` | `retrieve_memories` | integer | `5` | 1 | 20 | Top-K 条数（分页步长）；实际返回还要受 800 token 预算二次裁剪 |

- **唯一分页接口**：`retrieve_memories`。当前 8 个 command 中**没有 PE 记录列表/详情接口**，因此没有第二个分页接口（见 §六 待确认第 1 条）。
- 分页语义：`injection_set` 为按 `weight` 降序排序的命中结果切片 `[offset, offset + top_k)`；`total_matched_count` 为过滤后总命中数；`has_more` 表示**本页切片之后还有未返回的命中**（按切片判定，不受 token 预算裁剪影响，见 §〇 修订第 9 条）。
- 注入场景必须使用 `offset = 0`（默认值），保证注入的始终是全局 Top-K；`offset > 0` 仅供记忆库浏览 UI 翻页，**不得**用于模型注入。
- 排序不接受客户端参数：排序键固定为 `weight` 降序，过滤条件固定为 `domain` 匹配 + `status=active` + `weight>0`。

### 4.3 排序与过滤参数

| 维度 | 约定 |
|---|---|
| 排序 | 固定 `weight` 降序（后端决定），不接受 `sort` / `order` 参数 |
| 过滤 | `domain`（必填）、`status=active`、`weight>0` 三项后端强制；客户端无法放宽 |
| 阶段选择 | 有有效 `query` 且 BM25 命中 → `fts5_bm25`；否则回退 `markdown_domain`。`vec_knn_rrf`（阶段三）未启用。不接受客户端指定 |

### 4.4 Content-Type 与序列化

- 无 HTTP Content-Type。`invoke` 通道传 **UTF-8 JSON**，由 `serde` / `serde_json` 序列化与反序列化。
- 前端调用必须经 `src/api/client.ts`（AGENTS.md §6 第 11 条）；组件禁止直接 `invoke`。
- 所有字符串按 UTF-8 处理；所有枚举值以 snake_case 字符串传输（与 Rust `#[serde(rename_all = "snake_case")]` 一致）。

### 4.5 API key 注入方式约定

1. **唯一敏感物**是模型 API key。它允许存放于**系统钥匙串 / 环境变量 / 本机明文配置**三处（AGENTS.md §4 第 6 条），但**禁止**硬编码、**禁止**写入 `memory/` 目录与 git 仓库（AGENTS.md §6 第 8 条）。
2. **本机明文**落在 `%APPDATA%` 下的 `config.json`（`api_key` 字段），与「把 key 放进 git 仓库」是两件事：该目录不在仓库内，仓库也从不读取它。放宽原因见 §〇·补 修订表第 18 条。
3. **取值优先级**：本地明文 `api_key` > 环境变量 `api_key_env`。两者都为空才视为「未配置」。
4. **契约面**：`SetConfigRequest.api_key` 为 `writeOnly`（只进不出）；`get_config` 只回 `api_key_present` 布尔指示位，**永不回显明文**。
5. 注入时机：Rust 侧在发起 provider 调用前读取，明文不进入前端、不进入日志、不进入 SQLite。
6. `provider = ollama` 时本地推理不需要 API key，`api_key_present` 恒为 `false`。
7. 缺 key 时的统一报错：`provider_not_configured`（`details.missing = "api_key"`）。
8. **历史遗留迁移**：旧版本曾把明文误填进 `api_key_env`（该字段本应只存变量名）。加载配置时若发现 `api_key_env` 不是合法环境变量名，会把它搬到 `api_key` 字段并把变量名复位为默认值，且只迁移一次。

### 4.6 时间与日期格式

| 类型 | 格式 | 示例 | 适用字段 |
|---|---|---|---|
| 时间戳 | RFC 3339 / ISO 8601，UTC，带 `Z` | `2026-10-21T10:32:07Z` | `created_at`、`updated_at`、`occurred_at`、`started_at`、`finished_at` |
| 日期 | `YYYY-MM-DD` | `2026-10-21` | `run_date`、L2 记忆条目的 `created` / `last_used` |
| 时区 | 一律以 UTC 存储与传输；本地时区只在 `run_date` 的「当日」判定上使用 | — | — |

### 4.7 字段命名与类型约定

- 一律 **snake_case**（Rust / serde / JSON / TypeScript 同名同形），不混用 camelCase。
- 布尔用 `is_`/`has_` 前缀（如 `has_more`）；集合用复数名词（如 `signals`、`injection_set`）；时间戳用 `_at` 后缀，日期用无后缀或 `_date`。
- ID 格式：`PE-YYYY-NNN`（PE 记录）、`M-<域缩写>-<序号>`（记忆条目）。
- 数值单位：`*_ms` 毫秒、`*_count` 个数、`*_token_count` token 数。

---

## 五、Tauri command 签名表与 components.schemas 片段

### 5.1 Tauri command 签名表（等价替代 OpenAPI 3.1 `paths`）

| invoke 名 | Rust 命令签名 | serde 参数类型 | serde 返回类型 | 幂等 | 超时 | 请求 Schema | 响应 Schema |
|---|---|---|---|---|---|---|---|
| `toggle_panel` | `fn toggle_panel(app: AppHandle, state: State<'_, AppState>, req: TogglePanelRequest) -> Result<TogglePanelResponse, AppError>` | `TogglePanelRequest` | `TogglePanelResponse` | 非幂等 | 500 ms | `TogglePanelRequest` | `TogglePanelResponse` |
| `optimize_prompt` | `fn optimize_prompt(app: AppHandle, state: State<'_, AppState>, req: OptimizePromptRequest) -> Result<OptimizePromptResponse, AppError>` | `OptimizePromptRequest` | `OptimizePromptResponse` | 非幂等 | 可配 `optimize_timeout_ms`（默认 30 000 ms） | `OptimizePromptRequest` | `OptimizePromptResponse` |
| `save_pe_record` | `fn save_pe_record(app: AppHandle, state: State<'_, AppState>, req: SavePeRecordRequest) -> Result<SavePeRecordResponse, AppError>` | `SavePeRecordRequest` | `SavePeRecordResponse` | 非幂等（L1 只追加） | 3 000 ms | `SavePeRecordRequest` | `SavePeRecordResponse` |
| `submit_feedback` | `fn submit_feedback(app: AppHandle, state: State<'_, AppState>, req: SubmitFeedbackRequest) -> Result<SubmitFeedbackResponse, AppError>` | `SubmitFeedbackRequest` | `SubmitFeedbackResponse` | 幂等 | 3 000 ms | `SubmitFeedbackRequest` | `SubmitFeedbackResponse` |
| `retrieve_memories` | `fn retrieve_memories(app: AppHandle, state: State<'_, AppState>, req: RetrieveMemoriesRequest) -> Result<RetrieveMemoriesResponse, AppError>` | `RetrieveMemoriesRequest` | `RetrieveMemoriesResponse` | 幂等（只读） | 5 000 ms / 30 000 ms（阶段三） | `RetrieveMemoriesRequest` | `RetrieveMemoriesResponse` |
| `run_nightly_job` | `fn run_nightly_job(app: AppHandle, state: State<'_, AppState>, req: RunNightlyJobRequest) -> Result<RunNightlyJobResponse, AppError>` | `RunNightlyJobRequest` | `RunNightlyJobResponse` | 条件幂等 | 1 800 000 ms | `RunNightlyJobRequest` | `RunNightlyJobResponse` |
| `get_config` | `fn get_config(app: AppHandle, state: State<'_, AppState>) -> Result<AppConfig, AppError>` | 无（空对象） | `AppConfig` | 幂等（只读） | 500 ms | `EmptyRequest` | `AppConfig` |
| `set_config` | `fn set_config(app: AppHandle, state: State<'_, AppState>, req: SetConfigRequest) -> Result<SetConfigResponse, AppError>` | `SetConfigRequest` | `SetConfigResponse` | 幂等（全量覆盖） | 1 000 ms | `SetConfigRequest` | `SetConfigResponse` |
| `begin_dot_drag` | `fn begin_dot_drag(app: AppHandle) -> Result<DotDragOutcome, AppError>` | 无（空对象） | `DotDragOutcome` | 幂等（只观测） | 与按住左键时长同长 | `EmptyRequest` | `DotDragOutcome` |
| `set_panel_view` | `fn set_panel_view(app: AppHandle, req: PanelViewRequest) -> Result<(), AppError>` | `PanelViewRequest` | `()` | 幂等 | 500 ms | `PanelViewRequest` | `EmptyResponse` |

> Rust 侧所有请求/响应结构体统一 `#[serde(rename_all = "snake_case")]`；每个命令独立文件于 `src-tauri/src/commands/`，并在 `commands/mod.rs` 统一导出（AGENTS.md §4 第 2 条）。

### 5.2 `components.schemas` 片段（OpenAPI 3.1 / JSON Schema 2020-12）

```yaml
openapi: 3.1.0
info:
  title: Prompt Pad Tauri Command Contract
  version: 0.1.0
  description: >-
    Prompt Pad 为单机桌面应用，无 HTTP 接口。本片段以 x-tauri-commands 扩展
    等价替代 OpenAPI 3.1 的 paths 段；components.schemas 段使用标准
    JSON Schema 2020-12 语法，可被 OpenAPI 3.1 校验器直接校验。
paths: {}
x-tauri-commands:
  - invoke: toggle_panel
    request_schema: { $ref: "#/components/schemas/TogglePanelRequest" }
    response_schema: { $ref: "#/components/schemas/TogglePanelResponse" }
    error_response: { $ref: "#/components/schemas/ErrorResponse" }
    idempotency: non_idempotent
    timeout_ms: 500
  - invoke: optimize_prompt
    request_schema: { $ref: "#/components/schemas/OptimizePromptRequest" }
    response_schema: { $ref: "#/components/schemas/OptimizePromptResponse" }
    error_response: { $ref: "#/components/schemas/ErrorResponse" }
    idempotency: non_idempotent
    timeout_ms: 30000
    timeout_configurable: optimize_timeout_ms
  - invoke: save_pe_record
    request_schema: { $ref: "#/components/schemas/SavePeRecordRequest" }
    response_schema: { $ref: "#/components/schemas/SavePeRecordResponse" }
    error_response: { $ref: "#/components/schemas/ErrorResponse" }
    idempotency: non_idempotent
    timeout_ms: 3000
  - invoke: submit_feedback
    request_schema: { $ref: "#/components/schemas/SubmitFeedbackRequest" }
    response_schema: { $ref: "#/components/schemas/SubmitFeedbackResponse" }
    error_response: { $ref: "#/components/schemas/ErrorResponse" }
    idempotency: idempotent
    timeout_ms: 3000
  - invoke: retrieve_memories
    request_schema: { $ref: "#/components/schemas/RetrieveMemoriesRequest" }
    response_schema: { $ref: "#/components/schemas/RetrieveMemoriesResponse" }
    error_response: { $ref: "#/components/schemas/ErrorResponse" }
    idempotency: idempotent
    timeout_ms: 30000
  - invoke: run_nightly_job
    request_schema: { $ref: "#/components/schemas/RunNightlyJobRequest" }
    response_schema: { $ref: "#/components/schemas/RunNightlyJobResponse" }
    error_response: { $ref: "#/components/schemas/ErrorResponse" }
    idempotency: conditionally_idempotent
    timeout_ms: 1800000
  - invoke: get_config
    request_schema: { $ref: "#/components/schemas/EmptyRequest" }
    response_schema: { $ref: "#/components/schemas/AppConfig" }
    error_response: { $ref: "#/components/schemas/ErrorResponse" }
    idempotency: idempotent
    timeout_ms: 500
  - invoke: set_config
    request_schema: { $ref: "#/components/schemas/SetConfigRequest" }
    response_schema: { $ref: "#/components/schemas/SetConfigResponse" }
    error_response: { $ref: "#/components/schemas/ErrorResponse" }
    idempotency: idempotent
    timeout_ms: 1000
  - invoke: begin_dot_drag
    request_schema: { $ref: "#/components/schemas/EmptyRequest" }
    response_schema: { $ref: "#/components/schemas/DotDragOutcome" }
    error_response: { $ref: "#/components/schemas/ErrorResponse" }
    idempotency: idempotent
    timeout_ms: 0
    timeout_note: 与用户按住左键的时长同长，松手即返回
  - invoke: set_panel_view
    request_schema: { $ref: "#/components/schemas/PanelViewRequest" }
    response_schema: { $ref: "#/components/schemas/EmptyResponse" }
    error_response: { $ref: "#/components/schemas/ErrorResponse" }
    idempotency: idempotent
    timeout_ms: 500
components:
  schemas:
    Provider:
      type: string
      enum: [minimax, openai_compat, ollama, anthropic]
      description: 模型提供方。anthropic 为 v0.2 新增（见 §〇·补 修订表第 17 条）。

    Protocol:
      type: string
      enum: [openai_chat_completions, anthropic_messages]
      description: >-
        线上报文协议，与 Provider 正交：
        openai_chat_completions = POST {base}/v1/chat/completions，鉴权 Authorization: Bearer；
        anthropic_messages = POST {base}/v1/messages，鉴权 x-api-key + anthropic-version。

    Domain:
      type: string
      enum: [ui-design, writing, code, general]
      description: 任务域，取值暂定（域缩写表待确认）。

    MemoryStatus:
      type: string
      enum: [candidate, active, deprecated]
      description: 记忆条目状态机状态。candidate 不参与注入；deprecated 不占 token 预算。

    EmptyRequest:
      type: object
      additionalProperties: false
      properties: {}

    WindowAnchor:
      type: object
      additionalProperties: false
      required: [monitor_index, x_ratio, y_ratio]
      properties:
        monitor_index:
          type: integer
          minimum: 0
          description: 所在显示器序号。
        x_ratio:
          type: number
          minimum: 0
          maximum: 1
          description: 屏内水平百分比坐标。
        y_ratio:
          type: number
          minimum: 0
          maximum: 1
          description: 屏内垂直百分比坐标。

    TogglePanelRequest:
      type: object
      additionalProperties: false
      properties:
        action:
          type: string
          enum: [toggle, expand, collapse]
          default: toggle

    TogglePanelResponse:
      type: object
      additionalProperties: false
      required: [expanded, animation_ms, position]
      properties:
        expanded:
          type: boolean
        animation_ms:
          type: integer
          minimum: 150
          maximum: 250
        position:
          $ref: "#/components/schemas/WindowAnchor"

    OptimizePromptRequest:
      type: object
      additionalProperties: false
      required: [original_prompt]
      properties:
        original_prompt:
          type: string
          minLength: 1
          maxLength: 20000
        domain:
          $ref: "#/components/schemas/Domain"
        memory_ids:
          type: array
          maxItems: 20
          items:
            type: string
            pattern: "^M-[A-Z]+-[0-9]{3,}$"

    OptimizePromptResponse:
      type: object
      additionalProperties: false
      required: [optimized_prompt, provider, protocol, model_name, injected_memory_ids, injected_token_count, elapsed_ms, domain]
      properties:
        optimized_prompt:
          type: string
          minLength: 1
        provider:
          $ref: "#/components/schemas/Provider"
        protocol:
          $ref: "#/components/schemas/Protocol"
        model_name:
          type: string
          minLength: 1
          maxLength: 128
        injected_memory_ids:
          type: array
          maxItems: 20
          items:
            type: string
            pattern: "^M-[A-Z]+-[0-9]{3,}$"
        injected_token_count:
          type: integer
          minimum: 0
          maximum: 800
        elapsed_ms:
          type: integer
          minimum: 0
        domain:
          type: string
          minLength: 1

    SavePeRecordRequest:
      type: object
      additionalProperties: false
      required: [original_prompt, optimized_prompt]
      properties:
        original_prompt:
          type: string
          minLength: 1
          maxLength: 20000
        optimized_prompt:
          type: string
          minLength: 1
          maxLength: 20000
        domain:
          $ref: "#/components/schemas/Domain"
        signals:
          type: array
          maxItems: 32
          items:
            type: string
            minLength: 1
            maxLength: 64
        rating:
          type: integer
          minimum: 1
          maximum: 5
          default: null

    SavePeRecordResponse:
      type: object
      additionalProperties: false
      required: [record_id, created_at, written_layers]
      properties:
        record_id:
          type: string
          pattern: "^PE-[0-9]{4}-[0-9]{3,}$"
        created_at:
          type: string
          format: date-time
        written_layers:
          type: array
          minItems: 1
          maxItems: 2
          items:
            type: string
            enum: [l1_markdown, l3_sqlite]

    PeRecord:
      type: object
      additionalProperties: false
      required: [id, original_prompt, optimized_prompt, domain, signals, rating, created_at]
      description: >-
        L1 事实层 PE 记录字段口径。当前 8 个 command 中没有「读取 PE 记录」的
        列表/详情接口，本 schema 仅固定落库与文档口径，不对应任何 invoke。
      properties:
        id:
          type: string
          pattern: "^PE-[0-9]{4}-[0-9]{3,}$"
        original_prompt:
          type: string
          minLength: 1
        optimized_prompt:
          type: string
          minLength: 1
        domain:
          $ref: "#/components/schemas/Domain"
        signals:
          type: array
          items:
            type: string
        rating:
          type: [integer, "null"]
          minimum: 1
          maximum: 5
        created_at:
          type: string
          format: date-time

    SubmitFeedbackRequest:
      type: object
      additionalProperties: false
      required: [record_id, rating]
      properties:
        record_id:
          type: string
          pattern: "^PE-[0-9]{4}-[0-9]{3,}$"
        rating:
          type: integer
          minimum: 1
          maximum: 5
        note:
          type: string
          maxLength: 500

    WeightDelta:
      type: object
      additionalProperties: false
      required: [memory_id, entry_id, delta, weight_before, weight_after]
      properties:
        memory_id:
          type: string
          pattern: "^M-[A-Z]+-[0-9]{3,}$"
        entry_id:
          type: string
          pattern: "^M-[A-Z]+-[0-9]{3,}$"
          description: 与 memory_id 同值，兼容旧前端。
        delta:
          type: integer
          enum: [-2, 0, 1]
          description: 评分 ≥4 → +1；无明显改善 → 0；变差或跑偏 → -2。
        weight_before:
          type: integer
        weight_after:
          type: integer

    StatusChange:
      type: object
      additionalProperties: false
      required: [memory_id, from, to, reason]
      properties:
        memory_id:
          type: string
          pattern: "^M-[A-Z]+-[0-9]{3,}$"
        from:
          $ref: "#/components/schemas/MemoryStatus"
        to:
          $ref: "#/components/schemas/MemoryStatus"
        reason:
          type: string
          enum: [validated, weight_below_threshold, consecutive_miss_limit, superseded]

    SubmitFeedbackResponse:
      type: object
      additionalProperties: false
      required: [record_id, rating, applied_weight_deltas, updated_at]
      properties:
        record_id:
          type: string
          pattern: "^PE-[0-9]{4}-[0-9]{3,}$"
        rating:
          type: integer
          minimum: 1
          maximum: 5
        applied_weight_deltas:
          type: array
          maxItems: 20
          items:
            $ref: "#/components/schemas/WeightDelta"
        status_changes:
          type: array
          maxItems: 20
          items:
            $ref: "#/components/schemas/StatusChange"
        updated_at:
          type: string
          format: date-time

    RetrieveMemoriesRequest:
      type: object
      additionalProperties: false
      required: [domain]
      properties:
        domain:
          $ref: "#/components/schemas/Domain"
        query:
          type: string
          maxLength: 2000
        top_k:
          type: integer
          minimum: 1
          maximum: 20
          default: 5
        offset:
          type: integer
          minimum: 0
          maximum: 1000
          default: 0

    MemoryInjectionItem:
      type: object
      additionalProperties: false
      required: [memory_id, statement, why, how_to_apply]
      description: >-
        注入条目。只含 statement + why + how_to_apply 与回链用的 memory_id；
        不含 evidence / weight / hits / status 等统计与治理字段。
      properties:
        memory_id:
          type: string
          pattern: "^M-[A-Z]+-[0-9]{3,}$"
        statement:
          type: string
          minLength: 1
        why:
          type: string
          minLength: 1
        how_to_apply:
          type: string
          minLength: 1

    RetrieveMemoriesResponse:
      type: object
      additionalProperties: false
      required:
        [matched, stage, injection_set, returned_count, total_matched_count,
         offset, top_k, has_more, total_token_count, token_budget]
      properties:
        matched:
          type: boolean
        stage:
          type: string
          enum: [markdown_domain, fts5_bm25, vec_knn_rrf]
        no_memory_reason:
          type: [string, "null"]
          enum: [no_matching_memory, null]
        no_memory_message:
          type: [string, "null"]
          maxLength: 64
          description: 未命中时固定为「暂无历史经验」。
        injection_set:
          type: array
          maxItems: 20
          items:
            $ref: "#/components/schemas/MemoryInjectionItem"
        returned_count:
          type: integer
          minimum: 0
        total_matched_count:
          type: integer
          minimum: 0
        offset:
          type: integer
          minimum: 0
        top_k:
          type: integer
          minimum: 1
          maximum: 20
        has_more:
          type: boolean
        total_token_count:
          type: integer
          minimum: 0
          maximum: 800
        token_budget:
          type: integer
          minimum: 0
          maximum: 800
          default: 800

    RunNightlyJobRequest:
      type: object
      additionalProperties: false
      properties:
        run_date:
          type: string
          pattern: '^\d{4}-\d{2}-\d{2}$'
        dry_run:
          type: boolean
          default: false

    RunNightlyJobResponse:
      type: object
      additionalProperties: false
      required:
        [run_date, scanned_record_count, matched_entry_count, created_candidate_count,
         weight_bumped_count, conflicted_count, pending_review_ids, token_usage_before,
         token_usage_after, token_budget, index_rebuilt, started_at, finished_at]
      properties:
        run_date:
          type: string
          pattern: '^\d{4}-\d{2}-\d{2}$'
        scanned_record_count:
          type: integer
          minimum: 0
        matched_entry_count:
          type: integer
          minimum: 0
        created_candidate_count:
          type: integer
          minimum: 0
        weight_bumped_count:
          type: integer
          minimum: 0
        conflicted_count:
          type: integer
          minimum: 0
        pending_review_ids:
          type: array
          maxItems: 100
          items:
            type: string
            pattern: "^M-[A-Z]+-[0-9]{3,}$"
        token_usage_before:
          type: integer
          minimum: 0
          maximum: 800
        token_usage_after:
          type: integer
          minimum: 0
          maximum: 800
        token_budget:
          type: integer
          minimum: 0
          maximum: 800
          default: 800
        index_rebuilt:
          type: boolean
        started_at:
          type: string
          format: date-time
        finished_at:
          type: string
          format: date-time

    AppConfig:
      type: object
      additionalProperties: false
      required: [provider, protocol, endpoint, model_name, embedding_model]
      properties:
        provider:
          $ref: "#/components/schemas/Provider"
        protocol:
          $ref: "#/components/schemas/Protocol"
        endpoint:
          type: string
          minLength: 1
          maxLength: 512
        model_name:
          type: string
          minLength: 1
          maxLength: 128
        embedding_model:
          type: string
          enum: [multilingual-e5-small, bge-m3]
        optimize_timeout_ms:
          type: integer
          minimum: 1000
          maximum: 300000
          default: 30000
        api_key_present:
          type: boolean
          readOnly: true
          default: false
        api_key_env:
          type: string
        memory_token_budget:
          type: integer
          minimum: 1
          maximum: 800
        dot_click_through:
          type: boolean
        global_shortcut:
          type: string
        autostart_enabled:
          type: boolean
        config_status:
          type: string
          enum: [ok, missing, malformed]
        needs_setup:
          type: boolean

    SetConfigRequest:
      type: object
      additionalProperties: false
      required: [provider, protocol, endpoint, model_name, embedding_model]
      properties:
        provider:
          $ref: "#/components/schemas/Provider"
        protocol:
          $ref: "#/components/schemas/Protocol"
        endpoint:
          type: string
          minLength: 1
          maxLength: 512
        model_name:
          type: string
          minLength: 1
          maxLength: 128
        embedding_model:
          type: string
          enum: [multilingual-e5-small, bge-m3]
        optimize_timeout_ms:
          type: integer
          minimum: 1000
          maximum: 300000
          default: 30000
        api_key:
          type: string
          maxLength: 512
          writeOnly: true
          description: API key 明文，只写入不回显；空串表示清除已存明文。
        api_key_env:
          type: string
        memory_token_budget:
          type: integer
          minimum: 1
          maximum: 800
        dot_click_through:
          type: boolean
        global_shortcut:
          type: string
        autostart_enabled:
          type: boolean

    SetConfigResponse:
      type: object
      additionalProperties: false
      required: [config, updated_at]
      properties:
        config:
          $ref: "#/components/schemas/AppConfig"
        updated_at:
          type: string
          format: date-time

    DotDragOutcome:
      type: string
      enum: [clicked, dragged]
      description: >-
        clicked = 原地按下又抬起（后端已自行展开面板）；
        dragged = 按下后移动过（圆球已被拖走）

    PanelViewRequest:
      type: object
      additionalProperties: false
      required: [view]
      properties:
        view:
          type: string
          enum: [prompt, settings]

    EmptyResponse:
      type: "null"
      description: 无内容响应（序列化为 null）

    ErrorResponse:
      type: object
      additionalProperties: false
      required: [code, message, details, recoverable, command, occurred_at]
      properties:
        code:
          type: string
          enum:
            [invalid_input, invalid_type, payload_too_large, model_timeout,
             model_unavailable, provider_not_configured, memory_budget_exceeded,
             conflict_pending_review, embedding_model_mismatch, not_found,
             duplicate_record, storage_write_failed, storage_read_failed,
             lock_conflict, stage_not_reached, internal_error]
        message:
          type: string
          minLength: 1
          maxLength: 500
        details:
          type: object
        recoverable:
          type: boolean
        command:
          type: string
          enum:
            [toggle_panel, optimize_prompt, save_pe_record, submit_feedback,
             retrieve_memories, run_nightly_job, get_config, set_config,
             begin_dot_drag, set_panel_view]
        occurred_at:
          type: string
          format: date-time
```

---

## 六、「待确认」清单

以下条目在本契约中标注为「待确认」，**未经确认不得当作既定契约实现**（AGENTS.md §6 第 13 条）。

1. **PE 记录列表/详情接口缺失**：10 个 command 中没有「读取 PE 记录」的列表或详情接口，因此分页约定只落在 `retrieve_memories`。是否新增 `list_pe_records`（第 11 个命令）需确认——新增会突破 AGENTS.md §4 第 4 条的命令数量约束，需同步改 AGENTS.md。
2. **`api_key_present` 字段是否保留**：它不在 ARC-04 给定的配置四字段内，仅用于避免前端误判「已配置但无 key」。若团队要求配置面严格只暴露 `provider` / `endpoint` / `model_name` / `embedding_model`，该字段应删除。
3. **「连续未命中」的精确定义**：AGENTS.md 只写「未命中 → 触发连续未命中计数」。本契约暂按「检索候选池命中但因 Top-K / 预算裁剪未注入 → 该条目未命中 +1」实现；「当次任务完全无匹配」不产生条目级未命中。若定义为「每次任务未命中即全体 +1」，实现需改，且 `deprecated` 触发速度会显著加快。
4. **`save_pe_record` 的幂等性**：当前按「L1 只追加 = 非幂等」实现，超时重试会产生重复 PE 记录。是否引入客户端生成的 `idempotency_key` 字段做去重，需确认。
5. **数值上限与默认值**：`max_prompt_chars`（20000）、`optimize_timeout_ms`（30000 / 上限 300000）、`query` 长度（2000）、`signals` 条数（32）、`memory_ids` 条数（20）、`top_k`（5 / 上限 20）均为**可配置的建议初值，需实测校准**，不得以魔法数硬编码。
6. **token 计数口径**：`total_token_count` / `injected_token_count` 用哪个 tokenizer 计算未定义（中文场景下不同 tokenizer 差异明显）。选型需实测，且必须与 `memory.md` 文件头 `token_budget: 800` 的口径一致，否则会出现「按 A 算不超 800、按 B 算超 800」。
7. **域缩写取值表**：AGENTS.md §9.2 记录「`<域缩写>` 取值表未定义——暂定 `ui-design` / `writing` / `code` / `general` 四域」。本契约 `Domain` 枚举与 `M-[A-Z]+-[0-9]{3,}` 的 ID 模式据此暂定，待取值表确定后同步修订。
8. **provider 端点默认值**：三种 provider 的默认 `endpoint` 本文档一律不预设，需产品侧确认是否提供默认 URL。**v0.2 现状**：默认端点只出现在**前端预设表**（`src/components/Settings.tsx` 的 `PRESET_OPTIONS`，5 条预设），Rust 侧不内置任何服务地址（AGENTS.md §4 第 6 条）。是否要在后端也内置一份，待确认。
9. **`records/` 目录拆分**：记录数 > 30 时启用 `records/` 拆分（来自 `docs/02-私人记忆库设计.md` 待办）。拆分规则会影响 `save_pe_record` 的写入路径与 `run_nightly_job` 的扫描路径，需确认。
10. **Rust 工具链未安装**：AGENTS.md §9.1 记录本机 `cargo` / `rustc` 不在 PATH。本文档的命令签名与 Schema **未经 `cargo check` / `cargo test` 验证**，签名落地时需补编译验证。
11. **「点外部收起」在设置视图下的例外**：AGENTS.md §1 功能 ③ 写的是「点击展开、点外部或 `Esc` 收起」，本轮实现让**设置视图**不因失焦而收起（点外部、点别处都不收，`Esc` 与关闭按钮仍收）。理由是设置流程必然要切到别的应用复制内容，收起会丢表单。此为产品行为偏离，**需确认是否接受**，或改为「设置视图下点外部收起但先强制落盘」。
12. **API key 本地明文的取舍**：AGENTS.md §4 第 6 条原文只允许「环境变量或系统钥匙串」。本轮为「粘贴即用」放宽为**本机明文**（`%APPDATA%` 的 config.json），不进 git、不进 `memory/`、后端永不回显。若产品要求更严，下一步应改用 Windows DPAPI（`CryptProtectData`）加密后再落盘 —— 本期未实现。
13. **`provider` × `protocol` 的组合合法性**：当前**不做组合白名单**，任意 provider 配任意 protocol 都放行。实际有效的只有两条线：OpenAI 兼容协议（MiniMax 开放平台 / OpenAI / Ollama）与 Anthropic 协议（Anthropic 官方 / MiniMax Token Plan）。是否在 `set_config` 上加组合校验，待确认。
14. **`api_key_env` 的历史迁移是否可接受**：旧版本把明文误填进 `api_key_env`，现由加载期自动搬到 `api_key` 并复位变量名（只迁移一次）。若产品希望用户手动重新配置、拒绝任何自动搬运，需改为仅报错提示。
