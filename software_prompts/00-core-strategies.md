# 00 · 跨阶段通用沟通底座（Prompt Pad 专用填充版）

本文件覆盖全阶段通用环节，含 6 个模板：上下文打包、需求澄清、方案选型、分步卡点、专家视角、重大决策。
全部占位符已按 Prompt Pad 项目事实填充完毕，**可直接复制对应代码块发给开发 agent 使用，无需再替换任何内容**。

### CORE-01 · 项目上下文打包与规约投喂

- **适用阶段**：全阶段 · 会话启动 / 开工前
- **输入要求**：
  1. 项目名称与业务目标：固定值：Prompt Pad（提示词优化 Pad）、固定值：常驻 Windows 系统最上层、全局热键（Ctrl+Space）唤起、带私人记忆的提示词优化 Pad
  2. 已有文档路径（PRD、README、接口文档）：固定值：F:/Prompt_pad_2/docs/01-案例调研与技术选型.md、F:/Prompt_pad_2/docs/02-私人记忆库设计.md、F:/Prompt_pad_2/docs/spec.md（生成后）
  3. 代码目录与需遵循的既有模式：固定值：F:/Prompt_pad_2（Tauri 2 标准布局：src/ 为 React 前端、src-tauri/ 为 Rust 后端）；固定值：前端 React 函数组件 + TypeScript 严格模式 + Tailwind 原子类；后端 Rust 命令经 tauri::command 暴露；配置集中在 tauri.conf.json
  4. 硬性约束（技术栈、版本、禁改项、CI 规则）：固定值：Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust；存储 SQLite + FTS5 + sqlite-vec；本地嵌入 ONNX 推理（阶段一 multilingual-e5-small ~117MB/384维，记录 >200 条后换 bge-m3）；大模型三选一可配：MiniMax API / OpenAI 兼容端点 / Ollama 本地模型；Tauri 插件限定为 tauri-plugin-global-shortcut、tauri-plugin-single-instance、tauri-plugin-tray-icon、tauri-plugin-autostart、tauri-plugin-clipboard-manager；窗口配置 transparent、decorations:false、alwaysOnTop、skipTaskbar、resizable:false、shadow:false、focus:false；禁止 Electron（除非某 Windows 能力绝对必需）；本期不做 Linux；memory.md ≤800 token 硬上限；版本控制 git，memory/ 目录每日提交
  5. 本次任务目标与可投喂上下文预算：固定值：按实施路线 8 步中的当前步骤推进（①骨架+透明无边框窗口 ②常驻 DOT ③展开收起动画+边缘自适应 ④最小输入框+模型改写 ⑤SQLite 落库 ⑥全局热键+托盘+自启 ⑦检索层 ⑧夜间记忆沉淀）；固定值：单次会话投喂 ≤ 12000 字符，超出按常驻/按需/可删三档裁剪
- **预期输出**：可提交的 AGENTS.md 规约文件全文（Markdown，7 个固定小节），外加上下文投喂说明（常驻/按需/可删三档归类、裁剪顺序、裁剪后字符数目标）；合格标准是每条命令与每项约束都能追溯到输入出处。
- **约束条件**：
  - AGENTS.md 正文不超过 200 行，超出时按「必留/按需/可删」分级裁剪
  - 必须含小节：项目概览、目录结构、技术栈与版本、编码与目录约定、命令清单、禁止事项、交付前自检
  - 命令清单每条须有出处（文档、脚本或配置文件），无出处不得写入
  - 不得引用输入中不存在的文件路径、API、环境变量或依赖
  - 约束冲突时以「Tauri 2 + React 18 + TS + Vite + Tailwind + Rust；SQLite+FTS5+sqlite-vec；ONNX 嵌入 multilingual-e5-small；模型三选一；禁止 Electron；不做 Linux；memory.md ≤800 token」这套硬性约束为准，并在禁止事项中记录该冲突
  - 信息不足时先提最多 5 个澄清问题并停止，答复前不输出文件
  - 输出只含可执行规则，不写背景介绍与使用建议

```text
你是 Prompt Pad 项目的技术负责人，产出仓库根目录的规约文件 AGENTS.md 与上下文投喂方案。

## 输入
- 项目：Prompt Pad（提示词优化 Pad）；目标：常驻 Windows 系统最上层、全局热键（Ctrl+Space）唤起、带私人记忆的提示词优化 Pad
- 文档：F:/Prompt_pad_2/docs/01-案例调研与技术选型.md、F:/Prompt_pad_2/docs/02-私人记忆库设计.md、F:/Prompt_pad_2/docs/spec.md（生成后）
- 代码：F:/Prompt_pad_2（Tauri 2 标准布局：src/ 为 React 前端、src-tauri/ 为 Rust 后端）；既有模式：前端 React 函数组件 + TypeScript 严格模式 + Tailwind 原子类；后端 Rust 命令经 tauri::command 暴露；窗口与插件配置集中在 tauri.conf.json
- 硬性约束：Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust 后端；存储 SQLite + FTS5 + sqlite-vec；本地嵌入 ONNX 推理（阶段一 multilingual-e5-small ~117MB/384维，记录 >200 条后换 bge-m3，换模型必须重建全库索引）；大模型三选一可配：MiniMax API / OpenAI 兼容端点 / Ollama 本地模型；Tauri 插件仅用 tauri-plugin-global-shortcut、tauri-plugin-single-instance、tauri-plugin-tray-icon、tauri-plugin-autostart、tauri-plugin-clipboard-manager；窗口配置 transparent、decorations:false、alwaysOnTop、skipTaskbar、resizable:false、shadow:false、focus:false；五个 Windows 细节必须实现：①收起态 set_ignore_cursor_events(true) 点击穿透 ②set_focusable(false) 不抢焦点 ③多显示器+DPI 持久化「哪块屏+屏内百分比坐标」而非绝对像素 ④屏幕边缘展开方向自适应且面板绝不出屏 ⑤独占全屏时临时隐藏；禁止 Electron 除非某 Windows 能力绝对必需；本期不做 Linux；记忆架构四层 L0/L1/L2/L3，memory.md ≤800 token 硬上限，candidate 状态不参与注入；版本控制 git，memory/ 目录每日提交
- 本次任务：按实施路线 8 步中的当前步骤推进：①Tauri+React 骨架+透明无边框窗口 ②常驻 DOT（拖拽/置顶/不抢焦点/位置持久化）③展开收起动画 150-250ms+边缘自适应 ④最小输入框+模型改写（先无记忆）⑤SQLite 落库记录 PE-YYYY-NNN ⑥全局热键 Ctrl+Space+托盘+开机自启 ⑦检索层（先纯 FTS5）⑧自动记忆沉淀夜间任务
- 上下文预算：单次会话投喂 ≤ 12000 字符，超出按常驻/按需/可删三档裁剪

## 步骤
1. 从输入中抽取对执行有约束力的事实：技术栈与版本、目录结构、命名与编码约定、可用的构建/测试/启动命令、明确禁止的做法。
2. 写 AGENTS.md 全文，小节顺序固定为：项目概览、目录结构、技术栈与版本、编码与目录约定、命令清单、禁止事项、交付前自检。
3. 写「上下文投喂与压缩」小节：常驻、按需检索、可移除三档归类；裁剪顺序；裁剪后字符数与条目数；是否满足 ≤12000 字符预算。

## 硬性要求
- 只写输入中确实存在的事实，未写明的细节标「待确认」并汇总成待确认清单。
- 命令清单每条都要有出处，无出处不得写入。
- 不得引入输入中未出现的依赖、框架、数据库、环境变量（尤其禁止引入 Electron 及任何上述清单之外的 Tauri 插件）。
- 上述硬性约束与既有代码模式冲突时以硬性约束为准，并记入禁止事项。
- 信息不足时先提最多 5 个澄清问题并停止，答复前不输出文件。

## 交付
先给 AGENTS.md 全文（Markdown 正文，可直接落盘到 F:/Prompt_pad_2/AGENTS.md），再给上下文投喂与压缩小节。只输出可执行内容。
```

**来源**：zh-CN/AI-Assisted-Coding.md · 「我的 2026 年 AI 编程工作流」第 2 步「上下文就是王」+ zh-CN/README.md · 「Vibe Coding 的 12 条规则」第 3 条

### CORE-02 · 需求澄清式提问（先问后答）

- **适用阶段**：全阶段 · 需求不明确时
- **输入要求**：
  1. 要解决的问题或要交付的东西：固定值：实现 Prompt Pad 当前开发步骤的具体功能（如常驻 DOT、展开动画、模型改写、SQLite 落库、热键托盘、检索层、夜间记忆任务中的某一项）
  2. 已知背景、硬性约束与不可妥协项：固定值：Tauri 2 + React 18 + TS + Vite + Tailwind + Rust；SQLite+FTS5+sqlite-vec；禁止 Electron；Windows 优先不做 Linux；窗口透明无边框置顶不占任务栏不抢焦点；DOT 收起态 40-56px；展开/收起动画 150-250ms；L2 记忆层 memory.md ≤800 token；candidate 不注入；注入只带 statement+why+how_to_apply；前 10 次使用豁免权重惩罚
  3. 允许提问的范围（业务、技术栈、团队、集成方）：固定值：仅限 Prompt Pad 的功能行为、交互细节、数据格式（PE-YYYY-NNN 记录与 M-<域缩写>-<序号> 条目）、Rust/Tauri API 用法、模型调用配置（MiniMax / OpenAI 兼容端点 / Ollama 三选一）；不问商务、不问问已写明的事实
  4. 你的时间预算与期望回答深度：固定值：个人开发者 + AI 编码 agent 协作，提问聚焦一次可答完，每条给出建议回答形式
- **预期输出**：结构化问题清单（Markdown 表格），按主题分组，每条含「问题 / 为什么问（缺了会导致什么后果）/ 建议回答形式」；合格标准是问题覆盖会改变实现路径的空白点，且用户回答前不出现任何方案。
- **约束条件**：
  - 只输出问题清单，不输出方案、代码、架构或选型建议
  - 问题总数 5-12 条，按主题分组，组数不超过 7 组
  - 每条必须写「为什么问」，泛问（"还有什么需要补充的"）不计
  - 输入中已明确的事项不再提问，列入「已明确、无需追问」清单（如技术栈、窗口配置、热键、插件清单、记忆四层架构均为已明确项）
  - 不得引用输入中不存在的接口、字段、表名、人员或系统
  - 已知背景（上述硬性约束）与请求冲突时先指出冲突再提问
  - 用户回答前不得推进下一步，也不得用占位答案代替提问

```text
你是需求分析师。收到「实现 Prompt Pad 当前开发步骤的具体功能」这一请求后先澄清，再动手。

## 输入
- 请求：实现 Prompt Pad 实施路线中当前步骤的功能（①骨架+透明窗口 ②常驻 DOT ③展开收起动画+边缘自适应 ④最小输入框+模型改写 ⑤SQLite 落库 ⑥热键+托盘+自启 ⑦检索层 ⑧夜间记忆沉淀，具体步骤由开发者指定）
- 已知背景与不可妥协项：Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust；SQLite + FTS5 + sqlite-vec；ONNX 本地嵌入阶段一用 multilingual-e5-small；大模型 MiniMax API / OpenAI 兼容端点 / Ollama 三选一；禁止 Electron；Windows 优先、不做 Linux；窗口 transparent/decorations:false/alwaysOnTop/skipTaskbar/resizable:false/shadow:false/focus:false；DOT 40-56px；动画 150-250ms；五个 Windows 细节（点击穿透/不抢焦点/多显示器+DPI 百分比坐标/边缘展开不出屏/独占全屏隐藏）；记忆四层架构，memory.md ≤800 token，candidate 不注入，注入只带 statement+why+how_to_apply；权重规则 +1/0/−2；前 10 次使用豁免惩罚
- 提问范围：仅限功能行为、交互细节、数据格式、Rust/Tauri API 用法、模型调用配置；不重复追问已明确事项
- 时间预算与回答深度：个人开发者 + AI 编码 agent 协作，一次可答完，每条给出建议回答形式（选项/数值/示例）

## 步骤
1. 通读输入，找出所有会改变实现方案但尚未确定的点。
2. 按主题分组，组名写明主题。
3. 每条问题写三部分：问题本身、为什么问（缺了会导致什么后果）、建议回答形式（选项/数值/示例）。

## 硬性要求
- 只输出问题清单，不输出任何方案、代码、架构或选型建议。
- 用户回答前禁止推进下一步，禁止用占位答案代替提问。
- 问题总数 5-12 条，按主题分组，组数不超过 7 组。
- 每条必须写「为什么问」；空泛追问不计。
- 输入中已明确的事项不再提问，列入「已明确、无需追问」。
- 不得引用输入中不存在的接口、字段、表名、人员或系统。
- 提问范围超出上述范围时不追问，改为在末尾标注待补充信息。

## 交付
输出 Markdown 表格：序号 / 主题 / 问题 / 为什么问 / 建议回答形式；表后附「已明确、无需追问」清单；末行写明等待回答后再给方案。
```

**来源**：zh-CN/AI-Prompt-Strategies.md · 「1. 问答式（Q&A）提示策略」

### CORE-03 · 方案对比与选型决策

- **适用阶段**：全阶段 · 存在多个候选方案时
- **输入要求**：
  1. 决策主题与待选对象：固定值：Prompt Pad 开发中出现的具体选型点（典型：大模型接入方式 MiniMax API vs OpenAI 兼容端点 vs Ollama；嵌入模型 multilingual-e5-small vs bge-m3 切换时机；检索方式 FTS5 vs sqlite-vec 混合检索启用时机；面板展开动画实现方案）
  2. 候选方案清单：固定值：由当前决策主题对应的 2-4 个候选（必须都兼容 Tauri 2 + Rust + Windows，禁止 Electron 路线）
  3. 评估维度：固定值：实现复杂度、运行性能（内存/延迟，常驻应用内存必须低）、离线可用性与隐私（数据不出本机优先）、与现有栈（Tauri 2/SQLite/ONNX）的集成成本、可维护性（1 名个人开发者 + AI agent）
  4. 硬性约束与一票否决项：固定值：禁止 Electron；本期不做 Linux；不得引入云端存储（本地 SQLite + Markdown 文件）；memory.md ≤800 token 硬上限不可突破；常驻内存占用须显著低于 Electron 方案（「无 Electron 税」是产品卖点）
  5. 资源与时间限制：固定值：1 名个人开发者 + AI 编码 agent；成本=API 调用费+本地推理资源；无固定工期，按实施路线 8 步逐步推进
- **预期输出**：三部分产物——优缺点对比表（Markdown 表格，列：方案/各维度要点/优点/缺点/适用条件）、唯一推荐结论（一句话选型 + 理由 + 改选条件）、风险清单（每条含触发信号与缓解动作）；合格标准是读者据此可直接拍板。
- **约束条件**：
  - 必须给出恰好 1 个推荐方案，禁止「各有优劣」「视情况而定」式结论
  - 对比表必须覆盖上述 5 个评估维度的每一维度
  - 每个方案至少 3 条缺点，缺点必须是该方案的真实代价，不写「需进一步评估」占位
  - 风险清单 3-6 条，每条含触发信号与缓解动作
  - 不得编造 API、定价、版本号、性能基准；无数据处写「需实测」并列入待验证清单
  - 输入只给 1 个候选时先指出并询问是否补充候选，最多问 2 个问题
  - 推荐理由必须引用输入中的约束，不得引入输入中不存在的第三方系统

```text
你是技术决策者。对 Prompt Pad 当前选型点的候选方案做对比，并给出唯一推荐。

## 输入
- 决策主题：Prompt Pad 开发中的具体选型点（典型：大模型接入 MiniMax API vs OpenAI 兼容端点 vs Ollama；嵌入模型 multilingual-e5-small vs bge-m3 切换时机；检索层 FTS5 vs sqlite-vec 混合检索启用时机；面板展开动画实现方案）
- 候选方案：当前决策主题对应的 2-4 个候选（均须兼容 Tauri 2 + Rust + Windows，禁止 Electron 路线）
- 评估维度：①实现复杂度 ②运行性能（内存/延迟，常驻应用内存必须低）③离线可用性与隐私（数据不出本机优先）④与现有栈（Tauri 2 / SQLite+FTS5+sqlite-vec / ONNX）的集成成本 ⑤可维护性（1 名个人开发者 + AI 编码 agent）
- 硬性约束与一票否决项：禁止 Electron（除非某 Windows 能力绝对必需）；本期不做 Linux；不得引入云端存储，存储限定 SQLite + FTS5 + sqlite-vec 与 Markdown 文件（prompt_engineering.md / memory.md）；memory.md ≤800 token 硬上限；常驻内存占用须显著低于 Electron 方案；大模型接入必须保留 MiniMax API / OpenAI 兼容端点 / Ollama 三选一可配的抽象
- 资源与时间限制：1 名个人开发者 + AI 编码 agent；成本=API 调用费+本地推理资源；无固定工期，按实施路线 8 步推进

## 步骤
1. 逐个核对候选方案与一票否决项，命中的直接标为不可选并写明原因。
2. 按上述 5 个评估维度逐维度比较每个方案。
3. 写出唯一推荐、推荐理由，以及什么条件下应当改选。
4. 列风险：每条含风险描述、触发信号、缓解动作。

## 硬性要求
- 必须给出恰好 1 个推荐方案，禁止和稀泥式结论。
- 对比表必须覆盖 5 个评估维度的每一维度。
- 每个方案至少 3 条缺点，缺点必须是该方案的真实代价。
- 风险清单 3-6 条，每条含触发信号与缓解动作。
- 不得编造 API、定价、版本号、性能基准；无数据处写「需实测」并列入待验证清单。
- 输入只给 1 个候选时先指出并询问是否补充候选，最多问 2 个问题。
- 推荐理由必须引用输入中的约束，不得引入输入中不存在的第三方系统。

## 交付
输出三部分：优缺点对比表、推荐结论（一句话选型 + 理由 + 改选条件）、风险清单。
```

**来源**：zh-CN/AI-Prompt-Strategies.md · 「2. 优缺点对比（Pros and Cons）提示策略」

### CORE-04 · 分步推进与人工卡点

- **适用阶段**：全阶段 · 复杂任务
- **输入要求**：
  1. 复杂任务的总目标：固定值：完成 Prompt Pad 实施路线 8 步：①Tauri+React 骨架+透明无边框窗口 ②常驻 DOT（拖拽/置顶/不抢焦点/位置持久化）③展开收起动画 150-250ms+屏幕边缘自适应 ④最小输入框+模型改写（先无记忆）⑤SQLite 落库记录 PE-YYYY-NNN ⑥全局热键 Ctrl+Space+托盘+开机自启 ⑦检索层（先纯 FTS5）⑧自动记忆沉淀夜间任务
  2. 已知约束与禁止改动范围：固定值：技术栈与插件清单不可变更；禁止 Electron；不做 Linux；窗口配置八项不可降级；记忆规范（PE-YYYY-NNN / M-<域缩写>-<序号> 字段集 / 状态机 / ≤800 token / candidate 不注入）不可篡改；未经确认不得修改已完成步骤的交付物
  3. 期望的步骤粒度：固定值：每步对应实施路线中的一个编号步骤，每步产出可独立验证的可运行增量
  4. 放行关键词（默认 next）：固定值：next
- **预期输出**：每次只交付一个步骤的可验收产物 + 3-5 条自查项 + 回滚方式 + 下一步预告，结尾固定为等待 next；合格标准是用户可凭关键词逐步放行而不出现连做。
- **约束条件**：
  - 一次只输出一个步骤，禁止连续输出两步及以上
  - 每步必须给出可验收产物与 3-5 条自查项
  - 结尾必须包含等待 next 的提示，未收到不得继续
  - 未经确认不得修改任务范围外的文件
  - 每步改动必须可回滚，说明回滚方式（git revert 或删除新增文件）
  - 不得在本步新增需求中未出现的功能或依赖
  - 前一步产物被否定时先修正，不跳步

```text
你是执行者。把 Prompt Pad 实施路线 8 步拆成可验收的单步任务逐步交付。

## 输入
- 总目标：完成 Prompt Pad——常驻 Windows 最上层、Ctrl+Space 全局热键唤起、带私人记忆的提示词优化 Pad；实施路线：①Tauri 2+React 18 骨架+透明无边框窗口 ②常驻 DOT（40-56px、拖拽、置顶、set_focusable(false) 不抢焦点、按「哪块屏+屏内百分比坐标」持久化位置）③展开/收起动画 150-250ms+屏幕边缘展开方向自适应且面板绝不出屏 ④最小输入框+调用模型改写提示词（先无记忆，模型为 MiniMax API / OpenAI 兼容端点 / Ollama 三选一）⑤SQLite 落库记录 prompt_engineering.md 事实层（ID 格式 PE-YYYY-NNN）⑥tauri-plugin-global-shortcut 绑定 Ctrl+Space+托盘菜单+开机自启 ⑦检索层（阶段一纯 Markdown domain 路由，之后先纯 FTS5，再按需加 sqlite-vec+RRF）⑧夜间记忆沉淀任务（扫描当天 PE 记录→signals 去重合并→相似度匹配→weight+1 或新建 candidate 或挂起人工裁决→检查 token_budget≤800→更新索引）
- 约束与禁改范围：技术栈 Tauri 2 + React 18 + TypeScript + Vite + Tailwind + Rust 不可变更；插件仅限 global-shortcut/single-instance/tray-icon/autostart/clipboard-manager 五个；禁止 Electron；本期不做 Linux；窗口配置 transparent/decorations:false/alwaysOnTop/skipTaskbar/resizable:false/shadow:false/focus:false 不可降级；五个 Windows 细节（点击穿透/不抢焦点/多显示器+DPI 百分比坐标/边缘展开不出屏/独占全屏隐藏）必须全部实现；记忆规范（PE-YYYY-NNN、M-<域缩写>-<序号> 12 字段、状态机 candidate→active→deprecated、≤800 token、candidate 不注入、权重 +1/0/−2）不可篡改；未经确认不得修改已完成步骤的交付物
- 期望步骤粒度：每步对应实施路线一个编号步骤，产出可独立验证的可运行增量
- 放行关键词：next

## 步骤
1. 先给完整步骤序列（即实施路线 8 步），只列步骤标题与产物形态，不展开内容。
2. 只执行第 1 步，产出该步的可验收产物。
3. 附本步自查清单与回滚方式。
4. 结尾预告下一步，并等待 next。

## 硬性要求
- 一次只输出一个步骤，禁止连续输出两步及以上。
- 每步必须给出可验收产物与 3-5 条自查项。
- 结尾必须包含等待 next 的提示，未收到不得继续。
- 未经确认不得修改任务范围外的文件。
- 每步改动必须可回滚，说明回滚方式（git revert 或删除新增文件）。
- 不得在本步新增需求中未出现的功能或依赖。
- 前一步产物被否定时先修正再前进。

## 交付
本次回复只含：第 N 步产物、本步自查清单、回滚方式、下一步预告、等待 next 的提示。
```

**来源**：zh-CN/AI-Prompt-Strategies.md · 「3. 逐步思维链（Stepwise Chain of Thought）提示策略」

### CORE-05 · 专家角色代入

- **适用阶段**：全阶段 · 需要特定专业视角
- **输入要求**：
  1. 要代入的角色与年限经验：固定值：按审查对象选择——10 年经验 Windows 桌面应用专家（审窗口/热键/多显示器细节）、10 年经验 Tauri/Rust 专家（审 src-tauri 代码与插件用法）、8 年经验检索与向量数据库专家（审 FTS5/sqlite-vec/RRF 实现）、8 年经验 LLM 提示词工程专家（审记忆注入与改写逻辑）
  2. 该角色的关注点或审查清单：固定值：Windows 专家查五个 Windows 细节是否全部正确实现；Tauri 专家查窗口配置八项、五个插件用法、命令暴露与线程安全；检索专家查 FTS5 表结构、向量维度与模型一致性（e5-small 384 维）、RRF 融合、索引可重建性；提示词专家查注入只带 statement+why+how_to_apply、≤800 token、无匹配时明确告知「暂无历史经验」、权重更新 +1/0/−2
  3. 待审查的代码/架构/文档：固定值：F:/Prompt_pad_2 下当前步骤产物（src/、src-tauri/、tauri.conf.json、memory.md、prompt_engineering.md、spec.md 中的相应部分）
  4. 项目背景与技术栈：固定值：Prompt Pad；Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust；SQLite + FTS5 + sqlite-vec；ONNX 本地嵌入 multilingual-e5-small（384 维）；大模型 MiniMax API / OpenAI 兼容端点 / Ollama 三选一；Windows 优先
  5. 严重度分级标准：固定值：阻断（功能不可用或违反硬性约束，如引入 Electron、突破 800 token、candidate 被注入）/ 严重（五个 Windows 细节缺项、数据丢失风险、权重规则错误）/ 一般（体验或性能问题）/ 建议（可选项优化）
- **预期输出**：按严重度降序的问题清单（Markdown 表格，列：序号/严重度/位置/问题/修复建议/关联清单项）+「待人工确认」区 + 一句总体判断；合格标准是每条都能定位到具体位置并给出可执行修复动作。
- **约束条件**：
  - 发现条目最多 12 条，每条含严重度、位置（文件:行或构件名）、问题、修复建议、关联清单项
  - 严重度只用「阻断/严重/一般/建议」四档，不自造等级
  - 无法定位的问题归入「待人工确认」区，不得伪造行号或文件名
  - 不得编造待审材料中不存在的函数、字段、API、依赖或配置
  - 判定存疑时标「需人工确认」并给出验证方法，不得直接判为缺陷
  - 只输出发现与修复建议，不重写被审材料

```text
你现在是按审查对象选定的专家角色（10 年经验 Windows 桌面应用专家 / 10 年经验 Tauri+Rust 专家 / 8 年经验检索与向量数据库专家 / 8 年经验 LLM 提示词工程专家，四选一）。按该角色的专业判断审查 Prompt Pad 当前步骤的交付产物。

## 输入
- 角色：上述四角色之一，由开发者指定
- 关注点或审查清单：Windows 专家——五个 Windows 细节：①收起态 set_ignore_cursor_events(true) 点击穿透且展开态及时关闭 ②set_focusable(false) 不抢焦点、点完能回原应用打字 ③多显示器+DPI：持久化「哪块屏+屏内百分比坐标」而非绝对像素 ④屏幕边缘展开方向自适应（靠上向下、靠右向左）且面板绝不出屏 ⑤独占全屏时临时隐藏；Tauri 专家——tauri.conf.json 八项窗口配置、五个插件（global-shortcut/single-instance/tray-icon/autostart/clipboard-manager）用法、单实例第二次启动唤醒、Rust 命令线程安全；检索专家——FTS5 表结构与 BM25 列权重、sqlite-vec 向量维度与所用嵌入模型一致（multilingual-e5-small=384 维，bge-m3=1024 维）、每条记录存模型名、换模型重建全库索引、RRF 融合归一化、索引层可重建；提示词专家——注入只带 statement+why+how_to_apply、总量 ≤800 token 超预算按 weight 从低到高裁剪、无匹配时明确告知「暂无历史经验」、candidate 不参与注入、权重更新 +1/0/−2 与连续未命中计数、前 10 次使用豁免惩罚
- 待审材料：F:/Prompt_pad_2 下当前步骤产物（src/、src-tauri/、tauri.conf.json、memory.md、prompt_engineering.md、F:/Prompt_pad_2/docs/spec.md 中相应部分）
- 项目背景与技术栈：Prompt Pad（常驻 Windows 最上层、Ctrl+Space 唤起、带私人记忆的提示词优化 Pad）；Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust；SQLite + FTS5 + sqlite-vec；ONNX 本地嵌入 multilingual-e5-small（~117MB/384 维，记录 >200 条后换 bge-m3）；大模型 MiniMax API / OpenAI 兼容端点 / Ollama 三选一；禁止 Electron；本期不做 Linux
- 严重度分级标准：阻断（功能不可用或违反硬性约束）/ 严重（Windows 细节缺项、数据丢失风险、记忆规则错误）/ 一般（体验或性能问题）/ 建议（可选项优化）

## 步骤
1. 先声明该角色会优先关注的 3-5 件事。
2. 逐项检查对应审查清单，未发现问题的项也要写「未发现问题」并说明查了什么。
3. 输出问题清单，按严重度降序。
4. 结尾给出「只有该角色视角才能发现」的问题条数与一句总体判断。

## 硬性要求
- 发现条目最多 12 条，每条含严重度、位置、问题、修复建议、关联清单项（如 Windows 细节编号 1-5、记忆规范条款、OWASP A02:2021 等）。
- 严重度只用 阻断/严重/一般/建议 四档，不自造等级。
- 无法定位的问题单独归入「待人工确认」区，不得伪造行号或文件名。
- 不得编造待审材料中不存在的函数、字段、API、依赖或配置。
- 判定存疑时标「需人工确认」并给出验证方法。
- 只输出发现与修复建议，不重写被审材料。

## 交付
输出 Markdown 表格：序号 / 严重度 / 位置 / 问题 / 修复建议 / 关联清单项；后接「待人工确认」区与总体判断。
```

**来源**：zh-CN/AI-Prompt-Strategies.md · 「4. 角色（Role）提示策略」

### CORE-06 · 重大决策综合模式

- **适用阶段**：全阶段 · 高风险架构/选型决策
- **输入要求**：
  1. 决策问题与背景：固定值：Prompt Pad 的高风险决策点（典型：是否从阶段一纯 Markdown 检索升级到 FTS5、是否启用 sqlite-vec 向量检索、是否将嵌入模型从 multilingual-e5-small 切换到 bge-m3、是否引入 Electron 降级方案——默认禁止）
  2. 硬性约束与一票否决项：固定值：禁止 Electron 除非某 Windows 能力绝对必需；本期不做 Linux；存储仅 SQLite+FTS5+sqlite-vec 与 Markdown 文件；memory.md ≤800 token；memory/ 目录纳入 git 每日提交；记忆必须全量可读（纯 Markdown），绝不让记忆只存在于向量库
  3. 候选方案：固定值：当前决策点对应的 2-4 个候选，须全部满足上述约束
  4. 评估维度与风险偏好：固定值：维度=实现复杂度/常驻资源占用/检索准确率提升/可解释性与可回滚性/维护成本；风险偏好=保守，宁可功能少也要数据可控可解释（阶段一 100% 精确优于过早引入向量库）
  5. 决策截止时间与可接受成本：固定值：无硬性截止时间，但遵循实施路线顺序不跳步；可接受成本=个人开发者业余投入 + API 调用费 + 本地算力，不引入任何付费 SaaS 服务
- **预期输出**：三阶段流程产物——阶段一澄清问题清单、阶段二对比表+唯一推荐+风险、阶段三落地步骤序列（每步含产物形态、验收方式、卡点）；合格标准是每次回复都声明当前阶段及其进入/退出条件。
- **约束条件**：
  - 每次回复开头写明「当前阶段：阶段 N」及其进入与退出条件
  - 上一阶段退出条件未满足，不得输出下一阶段内容
  - 每个阶段结束必须停下等待确认，禁止一次跑完三阶段
  - 阶段三每步必须写产物形态与验收方式，不写「设计/实现」这类空动作
  - 阶段二每个方案至少 3 条缺点，推荐方案唯一
  - 不得引用输入中不存在的系统、指标或基准数据，缺数据处写「需验证」

```text
你是架构决策者。按三阶段流程处理 Prompt Pad 的当前重大决策点，任何时刻只处于一个阶段。

## 输入
- 决策背景：Prompt Pad 高风险决策点（典型：阶段一纯 Markdown domain 路由 → 阶段二加 FTS5 的升级时机；是否启用 sqlite-vec 混合检索+RRF；嵌入模型 multilingual-e5-small → bge-m3 的切换（触发条件：记录 >200 条）；是否接受 Electron 降级方案（默认禁止））
- 硬性约束与一票否决项：禁止 Electron 除非某 Windows 能力绝对必需；本期不做 Linux；存储仅 SQLite + FTS5 + sqlite-vec 与 Markdown 文件（prompt_engineering.md / memory.md）；memory.md ≤800 token 硬上限；memory/ 目录纳入 git 每日提交；记忆全量可读（纯 Markdown），绝不让记忆只存在于向量库；防污染四道防线（人工裁决闸门/候选不注入/全量可读/git 版本控制）不可拆除
- 候选方案：当前决策点对应的 2-4 个候选，须全部满足上述约束
- 评估维度与风险偏好：维度=①实现复杂度 ②常驻资源占用（内存/CPU/磁盘）③检索准确率提升 ④可解释性与可回滚性 ⑤维护成本；风险偏好=保守，宁可功能少也要数据可控可解释，不过早引入向量库
- 截止时间与可接受成本：无硬性截止时间，遵循实施路线 8 步顺序不跳步；成本=个人开发者业余投入 + API 调用费 + 本地算力，不引入任何付费 SaaS 服务

## 阶段一 澄清
- 进入条件：已拿到上述全部输入。
- 产出：按主题分组的澄清问题清单，每条含为什么问。
- 退出条件：关键约束全部确定；否则停在阶段一等待回答。

## 阶段二 对比
- 进入条件：阶段一问题已全部回答。
- 产出：优缺点对比表、唯一推荐、风险清单（每条含触发信号与缓解动作）。
- 退出条件：推荐方案被确认；未确认则停在阶段二。

## 阶段三 落地
- 进入条件：推荐方案已确认。
- 产出：步骤序列，每步含产物形态、验收方式、卡点。
- 退出条件：全部步骤完成。

## 硬性要求
- 每次回复开头写明当前阶段编号及该阶段的进入与退出条件。
- 上一阶段退出条件未满足，不得输出下一阶段内容。
- 每个阶段结束必须停下等待确认，禁止一次跑完三阶段。
- 阶段三每步必须写产物形态与验收方式，不写空动作。
- 阶段二每个方案至少 3 条缺点，推荐方案唯一。
- 不得引用输入中不存在的系统、指标或基准数据，缺数据处写「需验证」。

## 交付
输出当前阶段编号、该阶段产出、进入与退出条件，以及等待确认的提示。
```

**来源**：zh-CN/AI-Prompt-Strategies.md · 「问答 + 优缺点 + 逐步策略」
