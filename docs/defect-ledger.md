# Prompt Pad · 缺陷台账（defect-ledger）

> 本文档汇总 4 路专家审查 + 真机 UI 闸发现的全部缺陷，给出**分级裁决**与**当前状态**。
> 事实基线是 `AGENTS.md`；接口契约是 `docs/interface-doc.md`。本台账不替代这两份，只记录「问题 → 裁决 → 落地位置」。

## 一、来源与分级口径

| 来源 | 范围 | 报告状态 |
|---|---|---|
| 专家 A | 窗口与桌面交互层（`window_ctl` / `geometry` / `fullscreen` / `tray` / `lib.rs` 事件） | ✅ 已取得（静态审查 + 核对 tao 0.37.1 / tauri 2.12.5 源码 + ACL 清单） |
| 专家 B | 记忆与数据层（`services/nightly` / `services/record` / `storage/db` / `storage/markdown`） | ✅ 已取得（静态审查 + 实跑 `cargo test` 复核基线） |
| 专家 C | 模型适配与配置（`services/provider` / `commands/config_cmd` / `config` / `error` / `models`） | ✅ 已取得（静态审查 + 核对 reqwest 0.13.5 / tauri ipc 源码） |
| 专家 D | 前端与契约对齐（`src/**` ↔ `docs/interface-doc.md`） | ❌ **未取得**：审查任务被取消，报告丢失。本轮以「前端四门 + 契约逐字段复核」替代 |
| 真机闸 | `scripts/check-app-ui.mjs`（15 条断言） | ✅ 每轮改动后复跑 |

**分级口径**

| 级别 | 判定标准 |
|---|---|
| 阻断 | 硬性约束在代码里**根本没实现**，或该功能路径**必然失败** |
| 高 | 用户可见的主路径出错，但存在绕行方式 |
| 中 | 边界/契约/一致性偏差，长期会造成数据或体验问题 |
| 低 | 健壮性、极端场景、可读性 |

**裁决口径**

- **已修**：本轮已改代码并有单测或真机断言钉住。
- **已登记**：确认为真实偏差，但本轮不修，已写进 `docs/interface-doc.md` §〇 修订表或 §六 待确认清单。
- **待拍板**：修法本身有产品取舍，需要用户决定方向。
- **待修**：方向明确、可直接实施，排在下一版。

---

## 二、总览

| 领域 | 阻断 | 高 | 中 | 低 | 合计 | 本轮已修 |
|---|---|---|---|---|---|---|
| A 窗口与交互 | 4 | 6 | 6 | 2 | 18 | 4 |
| B 记忆与数据 | 3 | 5 | 8 | 7 | 23 | 0 |
| C 模型与配置 | 0（1 项按高计） | 4 | 9 | 6 | 19 | 7 |
| 真机 | 1 | 2 | 2 | 0 | 5 | 4 |
| **合计** | **8** | **17** | **25** | **15** | **65** | **15** |

> B 域的 3 处阻断（B1/B2/B3）是记忆层的架构级问题，**修法涉及「真相源」选择，属产品取舍**，已在 `AGENTS.md` §10 遗留项登记，待用户拍板（见本文 §六）。

---

## 三、A 域 · 窗口与桌面交互层

| 编号 | 级别 | 现象 | 状态 | 裁决 |
|---|---|---|---|---|
| A-01 | 阻断 | DOT 拖不动：`data-tauri-drag-region` 走 `plugin:window\|start_dragging`，受 ACL 管控，`core:window:default` **不含** `allow-start-dragging`，invoke 被静默拒绝 | ✅ 已修 | v0.2 已加 `core:window:allow-start-dragging`；真机拖拽断言已通过 |
| A-02 | 阻断 | 开机自启从未生效：只注册了 `tauri_plugin_autostart`，全项目无 `enable()/disable()` 调用，`AppConfig` 也无自启字段 | ✅ 已修 | v0.4：`autostart_enabled` + `set_config` 调 `enable()/disable()` + 设置页开关 |
| A-03 | 阻断 | 焦点无法还给原窗口：`collapse()` 只做 `set_focusable(false)`，`WS_EX_NOACTIVATE` 不让已激活窗口失活 → 收起后按键被 Pad 吞掉 | ✅ 已修 | v0.2 已加 `capture_foreground` / `restore_foreground`（`GetForegroundWindow` → `SetForegroundWindow`） |
| A-04 | 高 | 全屏中展开：窗口被 `hide()` 但状态记为 Expanded，退出全屏后 DOT **永久不可见**（只能连按两次热键） | ✅ 已修 | v0.4：全屏时跳过 expand 且不改 mode、不 hide；展开失败还原 DOT |
| A-05 | 高 | 全屏检测误判 + 漏判：用 `GetSystemMetrics(SM_CXSCREEN)`（主屏尺寸）比前台窗口矩形 → 副屏全屏漏判；任务栏自动隐藏时**任何最大化窗口**被误判为独占全屏，面板根本不出现 | ✅ 已修 | v0.4：`MonitorFromWindow` + `GetMonitorInfo`；`SW_SHOWMAXIMIZED` 排除最大化窗口 |
| A-06 | 高 | 展开/收起**无任何动画**：`animation_ms` 只是回传给前端的常量 200，Rust 侧 `set_position`+`set_size` 一步到位 | ⏳ 待修 | 已登记 `AGENTS.md` §10 遗留项 5。产品取舍：动画要在 Rust 侧插值还是前端 CSS 过渡，待定 |
| A-07 | 高 | DOT 尺寸写死 48px，`set_config` 改不动（`docs/spec.md` §5 声明了 `dot_size_px` 40–56） | ⏳ 待修 | v0.3 已把常量改为 64（容器）但**仍未取自配置**。修法：`AppConfig` 增 `dot_size_px` 并做 40–56 校验 |
| A-08 | 高 | 二次启动只 `show()+set_focus()`，用户看不到变化，且 `force_window_active` 把键盘焦点抢给 48px 的 DOT | ✅ 已修 | v0.2 改为 `toggle_panel(Expand)` |
| A-09 | 高 | 面板已展开时再次「展开」会**跳位**：`expand()` 拿 `window.outer_position()`（此时是面板自己的位置）当 DOT 锚点再算一次落位 | ✅ 已修 | v0.4：目标形态与当前相同则直接返回 |
| A-10 | 高 | 恢复位置时 DPI 用错显示器：`restore_position` 用 `window.scale_factor()`（窗口当前所在屏），而锚点可能指向另一块屏 → 贴右缘时 DOT 有一截悬在屏外 | ✅ 已修 | v0.4：用锚点屏 `scale_factor()` |
| A-11 | 中 | 面板可能被任务栏压住：夹紧用的是**整块显示器矩形**，不是 Windows 工作区 | ✅ 已修 | v0.4：落位与恢复用 `monitor.work_area()` |
| A-12 | 中 | 落位屏与锚点屏判定口径不一致：`current_monitor()` 是 `MonitorFromWindow(DEFAULTTONEAREST)`（按矩形交集），锚点屏按**左上角点**判定 | ⏳ 待修 | 统一为同一口径 |
| A-13 | 中 | 托盘「设置」点进去没有任何设置界面 | ✅ 已修 | v0.2 改为 `store_view(Settings)` + 广播 `pad://view` + 展开面板；面板标题栏另加齿轮入口 |
| A-14 | 中 | 展开后 600ms 内点面板外部，第一次点击被吞掉 | ⏳ 待修 | `FOCUS_GRACE_MS` 时间窗太大。修法：改用「是否处于展开动作中」的显式标志 |
| A-15 | 中 | `ensure_topmost` 只设 `WS_EX_TOPMOST` 样式，**不等于**改 z-order；且 tao 下次改样式会整体覆写 ex-style | ✅ 已修 | v0.4：`SetWindowPos(HWND_TOPMOST)` + `WS_EX_NOACTIVATE` 收起态 |
| A-16 | 中 | 热键写死 `Ctrl+Space`，`set_config` 改不了 | ✅ 已修 | v0.2 做成配置项 `global_shortcut`，改键先注册新键、成功后再解旧键 |
| A-17 | 低 | 默认配置下 DOT 四周的透明角落仍吞掉鼠标点击 | ✅ 已登记 | 已知取舍（`AGENTS.md` §9 第 2 条）：`dot_click_through` 默认 `false` 保住「点击 DOT 展开」 |
| A-18 | 低（潜伏） | 每次拖动都会同步写一次磁盘（`Moved` 事件 → `save_anchor`），主线程 IO | ✅ 已修 | v0.4：锚点落盘 250ms 节流 |

---

## 四、B 域 · 记忆与数据层（**方向待用户拍板**）

> 这 3 处阻断不是「忘了写」，而是**真相源选择**的问题：`memory.md` 是准，还是 SQLite 是准。
> 两种改法都成立，但影响面不同，必须由产品侧定。详见 §六。

| 编号 | 级别 | 现象 | 状态 | 裁决 |
|---|---|---|---|---|
| B1 | 阻断 | 评分权重只写 SQLite、不回写 `memory.md`；夜间任务以 Markdown 旧值覆盖索引 → 「变差 −2」永久失效、`weight < −1` 降级路径**实际不可达**、被用户判负的记忆在下次夜间任务后**复活并重新注入** | ✅ 已修 | v0.4 改法 A：`submit_feedback` 回写 `memory.md` 的 weight / last_used / 状态 |
| B2 | 阻断 | 无「清空索引 → 从 `memory/*.md` 全量重建」路径（`docs/database-design.md:504` 明文要求）。索引丢失/换机后 `next_pe_id` 从头开始 → 每次保存都 `duplicate_record`，**PE 记录永久无法落库** | ✅ 已修 | v0.4：启动时 `hydrate::rebuild_from_markdown`；PE 序号取 Markdown 与索引较大者 |
| B3 | 阻断 | 夜间任务「超预算但本次无新候选」时**必然报错并永久卡死**（在写盘前 return Err）；`deprecated` 被计入 800 预算；消解方式是**直接删条目** | ✅ 已修 | v0.4 改法 A：`deprecated` 不占文件预算；超限只丢掉本次新候选，任务正常结束 |
| B-H1 | 高 | `dry_run=true` 仍改 SQLite（`bump_miss_streak` / `reset_miss_streak` / 写 `miss` 事件），与契约「不改 SQLite」冲突 | ✅ 已修 | v0.4：dry_run / 已有日期标记时跳过 SQLite 写入 |
| B-H2 | 高 | `miss_streak` 按**运行次数**而非**不同运行日**累计 → 同一天跑 3 次即误降级 | ✅ 已修 | v0.4：`miss_logged` 按日去重 |
| B-H3 | 高 | 同日重跑 → 同一份 PE 记录被重复计权 → candidate 在**一天内**自动晋升 active 并进入注入集（防线 2「至少验证 2 次」被降格成「跑两次任务」） | ✅ 已修 | v0.4：`.nightly-{date}` 标记，同日再跑只出报告 |
| B-H4 | 高 | 矛盾检测规则 2 形参 `signals` 完全未用 → 同域两条旧条目共享 signal 就判矛盾 → **该域的记忆增长被永久冻结**，`pending_review_ids` 指向与当天无关的旧条目 | ✅ 已修 | v0.4：同域共享 signal 不再冻结新候选 |
| B-H5 | 高 | Markdown 解析器静默丢字段（折行 / `*` 项目符号 / 全角冒号 → 字段被丢弃后回写覆盖原文）；不检测重复 ID | ⏳ 待修 | 解析器返回「无法解析行数 + 重复 ID」，夜间任务据此拒绝运行；解析器放宽折行与符号 |
| B-M1 | 中 | 预算裁剪用 `continue` 跳过装不下的条目 → 低权重短条目可挤掉高权重长条目，注入集不再是 weight 降序前缀 | ✅ 已修 | v0.4：超预算 `break` |
| B-M2 | 中 | L1「逐字不可改、只追加」在索引层被反着实现：`upsert_pe_record` 是 `ON CONFLICT DO UPDATE`，会改写 L1 行的 `content/title/domain/signals` | ✅ 已修 | v0.4：`ON CONFLICT DO NOTHING` |
| B-M3 | 中 | 拒绝晋升的错误路径上索引**已经改了**（miss 计数），`memory.md` 却没写 → 半截状态，重跑再计一次 | ⏳ 待修 | 预算判定前置到任何写库之前 |
| B-M4 | 中 | 800 token 按「整份 `memory.md` 渲染结果」计（含字段名与标题），与「注入总量」口径不是一回事 | ⏳ 待拍板 | 与 B3 合并处理：统一为「注入字段合计」口径 |
| B-M5 | 中 | `delete_memories_not_in` 删 `entries` 行会 `ON DELETE CASCADE` 连带删掉 `usage_log` 流水 → 审计链断裂 | ⏳ 待修 | 改软删（`deleted_at`）或 FK 改 `ON DELETE SET NULL` + 保留 `entry_id` 快照 |
| B-M6 | 中 | FTS 增量同步与业务写入**不在同一事务** → 进程在两条语句之间中断即产生 FTS 与 `entries` 不一致 | ⏳ 待修 | `conn.transaction()` 包裹 |
| B-M7 | 中 | `deprecated` 条目仍参与夜间匹配加权，`weight`/`evidence` 持续增长 | ✅ 已修 | v0.4：匹配跳过 deprecated |
| B-M8 | 中 | L1 写成功、L3 写失败时错误响应**不带 `written_layers`** → 调用方无法知道 L1 已落盘，重试只会拿到 `duplicate_record` | ⏳ 待修 | 错误 details 携带 `written_layers: ["l1_markdown"]` |
| B-L1~L7 | 低 | 注入排序 tiebreaker 不全序 / L2 `created_at` 用业务日期 / `aliases` 恒空 / signal 与 domain 长度未限 / 「命中但超预算」被报成「无命中」/ miss 判定把 `candidate` 也算进去 / `run_date` 用 UTC | ⏳ 待修 | 批量清理，可下一版一并处理 |

---

## 五、C 域 · 模型适配与配置

| 编号 | 级别 | 现象 | 状态 | 裁决 |
|---|---|---|---|---|
| C-01 | 高 | endpoint 拼接口径与契约示例冲突：实现无条件拼 `"{endpoint}/chat/completions"`，契约示例给的是完整路径 → 用户照契约填必然 404 | ✅ 已修 | v0.3 改为 `AppConfig::request_url()`：已是完整路径 → 原样使用；只到 `/v1` → 补尾段；协议根 → 补默认路径。契约修订表第 19 条同步登记 |
| C-02 | 高 | Tauri 在进入命令体前自行反序列化参数，失败时返回 `tauri::Error::InvalidArgs` → 前端收到**一个 JSON 字符串**，不是 `{code,message,details}`，整类 `invalid_type` 永远不会以契约形状到达前端 | ⏳ 待修 | 改法：命令入参收 `request: serde_json::Value`，命令体内 `from_value::<T>()` 并映射为 `AppError::invalid_type`（参数名仍叫 `request`，符合修订表第 1 条）。**只影响畸形客户端，前端是唯一调用方时优先级可后置** |
| C-03 | 高 | `set_config({endpoint:""})` 成功落盘（前端用 `string \| null` 区分「未改 / 清空」，空串是合法请求），要等到下次改写才报「未配置」 | ✅ 已修 | v0.3 在 `set_config` 赋值前校验非空 → `invalid_input` + `details{field, reason:"empty_input"}`（`endpoint` 与 `model_name` 都已覆盖） |
| C-04 | 高 | 401/403 一律映射成 `model_unavailable` + `retryable:true` → key 填错时用户永远等不到「去检查 key」的指引 | ✅ 已修 | v0.3 按状态码分流：401/403 → `provider_not_configured`（`details.missing="api_key"`）；其它 4xx → `retryable:false`；5xx/429 → `retryable:true` |
| C-05 | 中 | 响应体读取超时被误判成「模型响应不是合法 JSON」 | ✅ 已修 | v0.3 读体前先判 `err.is_timeout()` → `model_timeout` |
| C-06 | 中 | 非 2xx 的响应体读取失败被 `unwrap_or_default()` 吞掉（真实超时被记成 5xx） | ✅ 已修 | v0.3 显式 match：超时 → `model_timeout`，其它 → `model_unavailable` + `reason` |
| C-07 | 中 | `details` 在 `object` 与 `null` 间摇摆，契约 §3.1 规定恒为 object；`error.rs` 自己的注释与代码互相矛盾 | ✅ 已修 | v0.3 `AppError::new` 与 `with_details` 统一为 `json!({})`，单测同步更新 |
| C-08 | 中 | 上游错误体原文进 `details.reason` → 部分兼容中转会回显令牌（`无效令牌 sk-xxx`），明文会流到前端与日志 | ✅ 已修 | v0.3 新增 `redact_secret()`，用真实 key 值替换为 `***` |
| C-09 | 中 | `set_config` 返回扁平 `AppConfigView`，契约 §2.8 要求 `{config, updated_at}` | ✅ 已登记 | 前后端内部自洽（前端无 `SetConfigResponse` 类型）。修法要两端同改，收益低。**已登记在 `docs/interface-doc.md` §〇 修订表第 2 条邻接位置** |
| C-10 | 中 | `WeightDelta` 只有 `{entry_id, delta}`，契约要求 `[memory_id, delta, weight_before, weight_after]` | ✅ 已修 | v0.4：`memory_id` + `entry_id` + before/after |
| C-11 | 中 | `provider_not_configured` 的 `details` 有两套形状（`{field,reason}` vs `{missing,provider}`） | ✅ 已修 | v0.3 统一为 `{missing, provider}`（`endpoint` / `model_name` / `api_key` 三处） |
| C-12 | 中 | `storage_write_failed` / `storage_read_failed` 的 `details` 缺 `target` 与 `retryable:true` | ⏳ 待修 | 构造点补 details |
| C-13 | 中 | `max_prompt_chars` 是代码常量而非契约说的「配置项」；`temperature: 0.3` 同样是魔法数 | ⏳ 待修 | `AppConfig` 增 `max_prompt_chars`（默认 20000）并由配置传入 |
| C-14 | 中 | 配置文件少一个字段 → 整份配置**静默重置为默认**（`unwrap_or_default()` 吞掉解析错误） | ✅ 已修 | v0.4：损坏不覆盖原件，`config_status=malformed` |
| C-15 | 中 | `optimize_prompt` 的 `domain` 只用于路由、不回传；PE 归档的 domain 由前端另传 → 两者可背离（记忆路由用 A 域、PE 记录写 B 域） | ✅ 已修 | v0.4：响应回传生效 domain，前端用它落库 |
| C-16 | 低 | `{"choices":[{"message":{"content":null}}]}` 这类合法 JSON 被报成「不是合法 JSON」 | ⏳ 待修 | `content` 改 `Option<String>`、`message` 给 `#[serde(default)]` |
| C-17 | 低 | 响应体无大小上限 | ⏳ 待修 | 限长读取 |
| C-18 | 低 | 同步命令在主线程做文件 I/O（`get_config` / `set_config`） | ⏳ 待修 | 改 `async fn` + `spawn_blocking` |
| C-19 | 低 | 请求结构未加 `#[serde(deny_unknown_fields)]` → 字段拼写错误被静默忽略（表现为「保存了但没生效」） | ⏳ 待修 | 10 个 Request 结构加 `deny_unknown_fields` |
| C-20 | 低 | `memory_ids` 未按契约校验 ID 格式 `^M-[A-Z]+-[0-9]{3,}$` | ⏳ 待修 | 逐条校验，不符 → `invalid_input` |
| C-21 | 低 | `panic="abort"` 使任何 panic 直接终止常驻进程 | ✅ 已登记 | 与 Tauri 官方模板一致，本范围内无 `unwrap/expect`（仅测试中）。记一笔即可 |

---

## 六、真机闸发现的缺陷

| 编号 | 级别 | 现象 | 状态 | 裁决 |
|---|---|---|---|---|
| R-01 | 阻断 | DOT 与面板都拖不动 | ✅ 已修 | A-01：ACL 缺 `core:window:allow-start-dragging`；面板另缺 `data-tauri-drag-region` |
| R-02 | 阻断 | DOT 单击打不开 | ✅ 已修 | Tauri 拖拽区上浏览器**不再派发 mouseup/click**（实测确认），改为后端 `begin_dot_drag` 轮询左键状态 + 4 物理像素阈值判定（契约 8 → 9 个命令） |
| R-03 | 高 | 拖面板时 Windows 把面板置为非激活，被「点外部收起」误判 → 一拖就变回圆球 | ✅ 已修 | 判据改为「光标是否仍在面板矩形内」（`cursor_inside()`） |
| R-04 | 高 | 托盘「设置」进不去 | ✅ 已修 | 改为广播 `pad://view`="settings" 再展开；面板标题栏另加齿轮入口 |
| R-05 | 中 | 展开/收起时 `set_size` 连带移动窗口，算好的落位被顶掉 → 面板出屏 59px；且 `Moved` 事件把面板位置当成用户拖拽写回锚点 | ✅ 已修 | 铁律：`set_size` → `set_position`；**先改 `state.mode` 再动窗口** |
| R-06 | 中 | DOT 常落在另一个置顶窗口（Chrome render widget）之下时鼠标事件到不了它 | 🟡 缓解 | 环境性，非代码缺陷。闸脚本已加「先用 `SetWindowPos` 挪到 5 个候选点、命中测试通过才测拖拽」；仍偶发 1 条不通过 |
| R-07 | 低 | `set_config` 把用户粘贴的 key 明文写进 `api_key_env`（变量名字段） | ✅ 已修 | v0.3 新增独立 `api_key` 本地明文字段；`api_key_env` 加载期校验必须是合法环境变量名，否则把明文搬回 `api_key` 并复位变量名（一次性迁移），`set_config` 侧直接拒绝非法变量名 |
| R-08 | 低 | 配置文件里残留了用户真实密钥（明文） | ✅ 已修 | 见 R-07；密钥只落 `%APPDATA%` 的 config.json，**不进 git、不进 `memory/`** |
| R-09 | 低 | 项目根目录残留 `.devlog.txt` / `.screen.png` / `.dot.png` | ✅ 已修 | 已删除并写进 `.gitignore` |

---

## 七、需用户拍板的事项

### 1. 记忆层真相源：B1 / B2 / B3 的方向

三处阻断的修法都围绕同一个问题：**`memory.md` 与 SQLite 谁是准**。

| 方案 | 做法 | 优点 | 代价 |
|---|---|---|---|
| **A. Markdown 为准（强化现有设计）** | 反馈路径同步回写 `memory.md`；夜间任务继续用 Markdown 覆盖索引 | 符合 AGENTS.md「Markdown 为准」与「纯 Markdown 全量可读可 diff」 | 每次反馈都要重写整个 `memory.md`；索引重建路径（B2）仍需单独实现 |
| **B. 索引为准** | 夜间任务解析 Markdown 后**不覆盖**索引的 `weight`/`hits`；Markdown 的 weight 降级为展示字段 | 改动小、反馈路径零 IO | 违背「Markdown 为准」，需修订 `docs/database-design.md` §2.1；`memory.md` 与界面显示的权重会不一致 |
| **C. 混合** | `weight`/`hits`/`status` 归索引管；`statement`/`why`/`how_to_apply`/`signals` 归 Markdown 管 | 职责清晰 | 状态机跨越两个存储，规则最复杂 |

**建议**：先单独处理 **B2**（索引重建路径，与真相源无关，是纯缺失功能，且不做它会让功能 ⑤ 在换机后彻底不可用），再在 A / B / C 中选一个处理 B1 与 B3。

### 2. 800 token 预算的口径（B3 / B-M4）

- 口径 A（现状）：`memory.md` 整个渲染结果的 token 数 → **实测 4 条真实条目 = 746 token、5 条 = 933**，800 上限实际只装得下 4 条。
- 口径 B（AGENTS §1 的字面表述）：注入量 = `statement` + `why` + `how_to_apply` 合计。

两者不是一回事，且口径 A 与「阶段二 50–300 条」的路线图互斥。**需要明确：800 到底约束「文件大小」还是「单次注入量」？**

### 3. 设置视图「点外部不收起」是产品行为偏离

AGENTS.md §1 功能 ③ 写的是「点外部或 `Esc` 收起」，v0.2 让设置视图不因失焦收起（`docs/interface-doc.md` §六 第 11 条已登记）。**需确认是否接受**。

### 4. API key 存本机明文

AGENTS.md §4 第 6 条原文只允许「环境变量或系统钥匙串」，v0.3 放宽为 `%APPDATA%` 的 config.json 明文（不进 git、不进 `memory/`、后端永不回显）。若要求更严，下一步应改用 Windows DPAPI（`CryptProtectData`）加密落盘 —— 本期未实现。

### 5. `provider` × `protocol` 的组合白名单

当前任意 provider 配任意 protocol 都放行。实际只有两条有效线：OpenAI 兼容协议（MiniMax 开放平台 / OpenAI / Ollama）与 Anthropic 协议（Anthropic 官方 / MiniMax Token Plan）。是否加组合校验待确认。
