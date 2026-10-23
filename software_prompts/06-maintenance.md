# 维护与迭代阶段提示词模板

> 本文件已按 **Prompt Pad（提示词优化 Pad）** 项目填充完毕：所有占位符均已替换为项目固定值，不留任何待填项，可直接整段复制发给开发 agent 使用。项目事实以 `docs/01-案例调研与技术选型.md` 与 `docs/02-私人记忆库设计.md` 为准。

覆盖维护迭代阶段的六个环节：迭代规划、增量开发、发布、本地支持、知识沉淀、质量治理。使用方式：整段复制给 agent；一次只用一个模板，不要把多个任务塞进同一轮会话。

### MNT-01 · 迭代需求评估与版本计划

- **适用阶段**：维护迭代 · 迭代规划
- **输入要求**：
  1. 新需求原始描述：固定值：Prompt Pad 版本规划——v0.1 = 实施路线步骤①-④（Tauri 2 + React 骨架与透明无边框窗口 / 常驻 DOT：拖拽、置顶、不抢焦点、位置持久化 / 展开收起动画 + 边缘自适应 / 最小可用输入框 + 调用模型做提示词改写）；v0.2 = 步骤⑤-⑥（SQLite 落库开始记录 PE-YYYY-NNN / 全局热键 Ctrl+Space + 托盘 + 开机自启）；v0.3 = 步骤⑦（检索层：阶段一纯 Markdown domain 路由 <50 条 → 阶段二 FTS5 50-300 条 → 阶段三 sqlite-vec + RRF 300+ 条）；v0.4 = 步骤⑧（夜间记忆沉淀任务）
  2. 现有系统与版本现状：固定值：从零起步的 Windows 优先单机应用；技术栈 Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust 后端；存储 SQLite + FTS5 + sqlite-vec；本地嵌入 ONNX 推理（阶段一 multilingual-e5-small ~117MB/384 维，记录 >200 条后换 bge-m3，换模型必须重建全库索引且每条记录存模型名）；大模型三选一可配：MiniMax API / OpenAI 兼容端点 / Ollama 本地模型；Tauri 插件：tauri-plugin-global-shortcut、tauri-plugin-single-instance、tauri-plugin-tray-icon、tauri-plugin-autostart、tauri-plugin-clipboard-manager；窗口配置 transparent、decorations:false、alwaysOnTop、skipTaskbar、resizable:false、shadow:false、focus:false
  3. 团队与时间约束（人力、迭代长度、截止日）：固定值：1 名个人开发者 + AI 编码 agent；无硬性截止日；迭代粒度按 v0.1 → v0.2 → v0.3 → v0.4 推进，每个版本切片只交付对应路线步骤
  4. 已知风险与依赖方：固定值：五个 Windows 细节——点击穿透（set_ignore_cursor_events 展开关/收起初）、不抢焦点（set_focusable(false)）、多显示器 + DPI（持久化「哪块屏 + 屏内百分比坐标」而非绝对像素）、边缘展开方向自适应不出屏、独占全屏时临时隐藏；Tauri 官方 issue #116（alwaysOnTop + skipTaskbar 在 Linux 基本无解，本期不做 Linux）；夜间任务的幂等性与冲突挂起条目的清理；嵌入模型更换时的全库索引重建；外部依赖仅为三个可配大模型端点之一，无其他依赖方
- **预期输出**：Markdown 文档，含三部分——优先级矩阵表（列：需求项 / 价值 / 成本 / 风险 / 合计 / 优先级）、版本切片清单（每片含 Must-Should-Could 归类、验收标准、依赖、人日估算）、排期与风险缓冲表；合格判据是每条需求都能追到一个切片，每个切片都有可判定的验收标准。
- **约束条件**：
  - 先输出澄清问题并停止，待用户回答后再进入评估；信息不足时不得自行假定需求范围
  - 优先级矩阵必须覆盖全部给定需求项，不得增删
  - 价值、成本、风险各按高 3 分 / 中 2 分 / 低 1 分打分，合计 3-9 分
  - 版本切片不超过 5 个，每个切片只交付一项能力
  - 每个切片必须写明可判定的验收标准与前置依赖
  - 排期须留出不少于总工期 20% 的风险缓冲
  - 不得编造用户量、性能指标或历史数据作为决策依据
  - 全文中文，模块名、命令与标识符保留英文原样

```text
# 角色
你是负责交付落地的资深技术负责人，对版本按期上线与范围可控负责。

# 任务
评估这批新增需求，并切分成可交付的版本计划。

# 输入
- 新需求：Prompt Pad 版本规划——v0.1 = 实施路线步骤①-④（Tauri 2 + React 骨架与透明无边框窗口 / 常驻 DOT：拖拽、置顶、不抢焦点、位置持久化 / 展开收起动画 + 边缘自适应 / 最小可用输入框 + 调用模型做提示词改写）；v0.2 = 步骤⑤-⑥（SQLite 落库开始记录 PE-YYYY-NNN / 全局热键 Ctrl+Space + 托盘 + 开机自启）；v0.3 = 步骤⑦（检索层：阶段一纯 Markdown domain 路由 <50 条 → 阶段二 FTS5 50-300 条 → 阶段三 sqlite-vec + RRF 300+ 条）；v0.4 = 步骤⑧（夜间记忆沉淀任务）
- 系统现状：从零起步的 Windows 优先单机应用；技术栈 Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust 后端；存储 SQLite + FTS5 + sqlite-vec；本地嵌入 ONNX 推理（阶段一 multilingual-e5-small ~117MB/384 维，记录 >200 条后换 bge-m3，换模型必须重建全库索引且每条记录存模型名）；大模型三选一可配：MiniMax API / OpenAI 兼容端点 / Ollama 本地模型；Tauri 插件：tauri-plugin-global-shortcut、tauri-plugin-single-instance、tauri-plugin-tray-icon、tauri-plugin-autostart、tauri-plugin-clipboard-manager；窗口配置 transparent、decorations:false、alwaysOnTop、skipTaskbar、resizable:false、shadow:false、focus:false；DOT 收起态 40-56px，展开/收起动画 150-250ms
- 团队与时间约束：1 名个人开发者 + AI 编码 agent；无硬性截止日；迭代粒度按 v0.1 → v0.2 → v0.3 → v0.4 推进，每个版本切片只交付对应路线步骤
- 已知风险与依赖：五个 Windows 细节——点击穿透、不抢焦点、多显示器 + DPI（持久化「哪块屏 + 屏内百分比坐标」）、边缘展开方向自适应不出屏、独占全屏时临时隐藏；Tauri 官方 issue #116（alwaysOnTop + skipTaskbar 在 Linux 基本无解，本期不做 Linux）；夜间任务的幂等性与冲突挂起条目的清理；嵌入模型更换时的全库索引重建；外部依赖仅为三个可配大模型端点之一，无其他依赖方

# 执行步骤
1. 澄清：列出最多 8 个问题，标注每个问题影响的决策点，然后停止等待答复；未获答复不得进入评估。
2. 打分：按价值 / 成本 / 风险三项打分（高=3、中=2、低=1），输出 Markdown 表格，列：需求项 / 价值 / 成本 / 风险 / 合计 / 优先级，按合计分降序排列。
3. 切片：把需求项归入 Must / Should / Could，切片数不超过 5 个，每片只交付一项能力。
4. 定义：逐片写出目标、涉及模块、验收标准（写明通过条件）、前置依赖、人力估算（人日）。
5. 排期：给出切片顺序、里程碑，以及不少于总工期 20% 的风险缓冲。
6. 取舍：写明为保交付砍掉了什么，以及触发回退前一版本的条件。

# 交付格式
## 澄清问题
## 优先级矩阵
## 版本切片
## 排期与风险缓冲
## 取舍与回退条件

# 边界
- 只使用上面给出的输入，不得引入未提供的系统事实、用户数据或性能数字。
- 不得把未排期的需求写进切片。
- 全文中文，模块名、命令与标识符保留英文原样。
```

**来源**：zh-CN/Forward-Deployed-Engineer.md · 「界定工作范围、安排交付节奏，及早清除阻碍」+ zh-CN/AI-Prompt-Strategies.md · 「问答式（Q&A）提示策略」

### MNT-02 · 增量功能实现

- **适用阶段**：维护迭代 · 增量开发
- **输入要求**：
  1. 增量功能的目标与验收标准：固定值：实现夜间记忆沉淀任务（实施路线步骤⑧）——扫描当天新增 PE 记录 → 对每条记录的 signals 去重合并 → 与现有 memory.md 条目做相似度匹配（匹配到已有条目则 weight+1 并补充 evidence；无匹配且 weight 达标则新建 candidate；与已有条目矛盾则标记冲突、挂起待人工裁决）→ 检查 token_budget（≤800 token，超出则从 weight 最低的 deprecated 起裁剪）→ 重建/增量更新索引库；验收标准：任务可重复执行且幂等（同一天跑两次结果一致）、candidate 条目不参与注入、矛盾条目不自动写入只挂起
  2. 相关既有代码位置与片段：固定值：src-tauri/（Rust 后端：窗口控制、SQLite 访问、ONNX 嵌入推理、文件读写）与 src/（React 18 + TypeScript 前端）；memory/ 目录（memory.md、prompt_engineering.md、records/）；SQLite 四张表 entries / entries_fts / entry_vectors / usage_log
  3. 项目技术栈与代码风格约定：固定值：Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust 后端；存储 SQLite + FTS5 + sqlite-vec；包管理 pnpm；Rust 工具链 stable；禁止 Electron 除非绝对必需；版本控制 git
  4. 既有测试与文档位置：固定值：docs/ 目录（spec.md、architecture.md、adr/、interface-doc.md、操作手册）；memory/ 目录纳入 git 每日提交；L1 记录格式见 prompt_engineering.md（ID 格式 PE-YYYY-NNN，含 signals 字段，永久逐字不可改）；L2 条目格式见 memory.md（ID 格式 M-<域缩写>-<序号>，字段 type/domain/status/statement/why/how_to_apply/signals/evidence/weight/hits/last_used/created）
- **预期输出**：Markdown 形式的实现方案与完整代码，含改动点清单表（列：文件路径 / 改动类型 / 说明）、向后兼容处理说明、开关与灰度建议、需同步更新的测试与文档清单；合格判据是每个改动都对应到具体代码位置。
- **约束条件**：
  - 本轮只实现一个增量切片，方案中不得出现第二个功能
  - 代码沿用 Tauri 2 + React 18 + TypeScript + Rust 的语言与风格，不引入新依赖
  - 必须逐项说明对既有调用方的影响；存在破坏性变更时单独标注并给出迁移步骤
  - 必须给出 feature flag 名称、默认值、灰度比例与关闭方式
  - 必须列出需新增或修改的测试用例及文档条目
  - 不得引用输入中不存在的文件、函数或接口；缺失信息先提问并停止
  - 不得顺带重构无关代码

```text
# 角色
你是在既有代码库中做增量交付的资深工程师，须保持现有代码风格与线上行为不变。

# 任务
在既有代码库中实现下面这一个增量功能，产出可直接落地的代码与配套清单。

# 输入
- 增量目标与验收标准：实现夜间记忆沉淀任务（实施路线步骤⑧）——扫描当天新增 PE 记录 → 对每条记录的 signals 去重合并 → 与现有 memory.md 条目做相似度匹配（匹配到已有条目则 weight+1 并补充 evidence；无匹配且 weight 达标则新建 candidate；与已有条目矛盾则标记冲突、挂起待人工裁决）→ 检查 token_budget（≤800 token，超出则从 weight 最低的 deprecated 起裁剪）→ 重建/增量更新索引库；验收标准：任务可重复执行且幂等（同一天跑两次结果一致）、candidate 条目不参与注入、矛盾条目不自动写入只挂起
- 相关代码位置与片段：src-tauri/（Rust 后端：窗口控制、SQLite 访问、ONNX 嵌入推理、文件读写）与 src/（React 18 + TypeScript 前端）；memory/ 目录（memory.md、prompt_engineering.md、records/）；SQLite 四张表 entries / entries_fts / entry_vectors / usage_log
- 技术栈与风格约定：Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust 后端；存储 SQLite + FTS5 + sqlite-vec；包管理 pnpm；Rust 工具链 stable；禁止 Electron 除非绝对必需；版本控制 git
- 既有测试与文档：docs/ 目录（spec.md、architecture.md、adr/、interface-doc.md、操作手册）；memory/ 目录纳入 git 每日提交；L1 记录 ID 格式 PE-YYYY-NNN（含 signals 字段，永久逐字不可改）；L2 条目 ID 格式 M-<域缩写>-<序号>，字段 type/domain/status/statement/why/how_to_apply/signals/evidence/weight/hits/last_used/created，status 取值 candidate|active|deprecated；状态机：candidate --(weight≥2 且无失败)--> active --(weight<-1 或连续3次未命中)--> deprecated；权重规则：评分≥4 +1、无改善 0、变差 -2、未命中触发连续未命中计数

# 执行步骤
1. 定位：列出将被改动的文件与函数，逐条说明改动原因，以及它如何对应目标验收标准。
2. 实现：给出完整可运行代码，风格与上述 src-tauri/ 和 src/ 既有代码一致，含错误处理与边界情况（空值、类型错误、超长输入）。
3. 兼容：逐项说明是否影响已有调用方；如影响，给出兼容层或迁移步骤。
4. 开关：给出 feature flag 名称、默认值、灰度比例与回滚方式。
5. 配套：列出需新增或修改的测试用例（含断言点）与需同步更新的文档条目。

# 交付格式
## 改动点清单
| 文件路径 | 改动类型 | 说明 |
| --- | --- | --- |
## 实现代码
## 向后兼容与开关
## 测试与文档同步清单
## 待确认问题

# 边界
- 只改与本增量直接相关的文件，不做顺带重构。
- 不得引用输入中不存在的文件路径、函数名或依赖；信息不足时先提问并停止。
- 全文中文，代码、命令、路径保留英文原样。
```

**来源**：zh-CN/README.md · 「基础代码生成」+ zh-CN/README.md · 「Vibe Coding 的 12 条规则」第 6 条「把任务拆成小步骤」

### MNT-03 · 版本发布说明与变更记录

- **适用阶段**：维护迭代 · 发布
- **输入要求**：
  1. 版本号与发布日期：固定值：按版本规划 v0.1（路线步骤①-④，最小可用）/ v0.2（步骤⑤-⑥）/ v0.3（步骤⑦）/ v0.4（步骤⑧）；发布日期以当次发版 git 提交日期为准
  2. 本次变更条目：固定值：以当次发布的 git 提交与已实现功能清单为准；示例 v0.1 变更：透明无边框常驻窗口（transparent、decorations:false、alwaysOnTop、skipTaskbar、resizable:false、shadow:false、focus:false）、DOT 收起态 40-56px、150-250ms 展开/收起动画、边缘展开方向自适应不出屏、最小可用输入框 + 提示词一键改写
  3. 影响范围与不兼容点：固定值：本地单机应用，无服务端；不兼容点主要为三类——SQLite 表结构变更（entries / entries_fts / entry_vectors / usage_log）、memory.md 与 prompt_engineering.md 格式变更、嵌入模型更换（multilingual-e5-small → bge-m3 属破坏性变更，必须重建全库索引且每条记录存模型名）；PE 记录数 >30 后启用 records/ 拆分亦属存储布局变更
  4. 部署与回滚方式：固定值：Tauri 打包安装包覆盖安装；回滚 = 重装上一版本安装包 + 用 git 回滚 memory/ 目录（该目录每日提交，可精确回滚某条规则的引入）+ 回退事前备份的 SQLite 数据文件
  5. 输入未提供项的占位标记：固定值：「待确认」
- **预期输出**：Markdown 文档，含 CHANGELOG 条目表（列：类型 / 变更内容 / 影响范围 / 迁移动作）、编号的升级步骤清单、编号的回滚步骤清单、一句话客户公告稿；合格判据是每条变更都能追到一个升级动作或「无需动作」的结论。
- **约束条件**：
  - 变更类型只取 新增 / 修复 / 优化 / 破坏性变更 四种
  - 破坏性变更必须单独成条，附迁移步骤与预计影响范围
  - 升级与回滚步骤必须可执行、编号排序，不写「按需操作」这类模糊表述
  - 公告稿不超过 60 字，不含内部代号、错误码与未发布特性名
  - 不得编造版本号、日期、影响用户数或性能提升数字；输入未给出的写「待确认」并列入待确认
  - 不得输出上述变更条目之外的变更
  - 全文中文，版本号、命令、接口路径保留英文原样

```text
# 角色
你负责版本对外沟通与升级指引，产出的内容要能直接进入发布流程。

# 任务
为 Prompt Pad 的当次发版版本（v0.1 / v0.2 / v0.3 / v0.4 之一，以当次实际为准）编写发布说明、变更记录、升级与回滚指引，以及一份客户公告稿。

# 输入
- 版本号与日期：按版本规划 v0.1（路线步骤①-④，最小可用）/ v0.2（步骤⑤-⑥）/ v0.3（步骤⑦）/ v0.4（步骤⑧）；发布日期以当次发版 git 提交日期为准
- 变更条目：以当次发布的 git 提交与已实现功能清单为准；示例 v0.1 变更：透明无边框常驻窗口（transparent、decorations:false、alwaysOnTop、skipTaskbar、resizable:false、shadow:false、focus:false）、DOT 收起态 40-56px、150-250ms 展开/收起动画、边缘展开方向自适应不出屏、最小可用输入框 + 提示词一键改写
- 不兼容点与影响范围：本地单机应用，无服务端；不兼容点主要为三类——SQLite 表结构变更（entries / entries_fts / entry_vectors / usage_log）、memory.md 与 prompt_engineering.md 格式变更、嵌入模型更换（multilingual-e5-small → bge-m3 属破坏性变更，必须重建全库索引且每条记录存模型名）；PE 记录数 >30 后启用 records/ 拆分亦属存储布局变更
- 部署与回滚方式：Tauri 打包安装包覆盖安装；回滚 = 重装上一版本安装包 + 用 git 回滚 memory/ 目录（该目录每日提交，可精确回滚某条规则的引入）+ 回退事前备份的 SQLite 数据文件

# 执行步骤
1. 归类：把当次变更条目逐条归入 新增 / 修复 / 优化 / 破坏性变更；无法归类的进「待确认」并标出。
2. 写 CHANGELOG：每条按 类型 / 变更内容 / 影响范围 / 迁移动作 四列成表，影响范围用模块或接口名表达，迁移动作写具体操作。
3. 写升级步骤：按执行顺序编号，覆盖配置变更、数据迁移、依赖版本、服务启停顺序。
4. 写回滚步骤：按执行顺序编号，注明回滚前提、回滚后的遗留影响与数据处理方式。
5. 写公告稿：面向客户的一句话，不超过 60 字。

# 交付格式
## CHANGELOG
## 升级步骤
## 回滚步骤
## 客户公告稿

# 边界
- 不得编造版本号、日期、用户规模、性能数字；输入未提供的信息写「待确认」并列入待确认。
- 不得加入输入中没有的变更。
- 全文中文，版本号、命令、接口路径、文件名保留英文原样。
```

**来源**：新增（流程整合）

### MNT-04 · 本地故障排障（日志+崩溃转储+复现步骤）

- **适用阶段**：维护迭代 · 技术支持
- **输入要求**：
  1. 故障现象与报错原文：固定值：本机故障现象与报错原文，取自应用日志与崩溃转储；本项目高发故障面：Ctrl+Space 全局热键失效或与系统/其他软件冲突、窗口在多显示器或 DPI 变化后飘出屏幕、展开方向错误出屏、独占全屏（游戏/视频）下置顶失效或未临时隐藏、点击穿透状态错乱（收起态拦鼠标或展开态收不到 hover）、夜间记忆沉淀任务崩溃或产出异常、SQLite 锁定或索引损坏、ONNX 嵌入推理失败
  2. 复现步骤与影响范围：固定值：本机（Windows 11）复现步骤，须注明显示器数量与缩放比例、是否处于独占全屏、Pad 当前展开/收起状态、当次使用的模型端点（MiniMax API / OpenAI 兼容端点 / Ollama）
  3. 相关代码与配置片段：固定值：src-tauri/ 与 src/ 相关片段、tauri.conf.json 窗口配置（transparent、decorations:false、alwaysOnTop、skipTaskbar、resizable:false、shadow:false、focus:false）、当日日志文件、崩溃转储、memory/ 目录最近 git 提交
  4. 环境信息（版本、部署时间、最近变更）：固定值：Windows 11；Prompt Pad 版本号（v0.1-v0.4）；Tauri 2 + React 18 + TypeScript + Rust stable；最近 git 提交摘要；当前嵌入模型名（multilingual-e5-small 或 bge-m3）
- **预期输出**：Markdown 排障报告，含七段——信息收集清单、复现判断、根因假设表（假设 / 验证步骤 / 判定标准）、最小修复、临时绕过、根治与预防措施、需用户补充的信息；合格判据是每条根因假设都配一条可执行的验证操作。
- **约束条件**：
  - 先给信息收集清单再给结论，已具备与缺失项分开列出
  - 根因假设按可能性降序，最多 5 条，每条配验证步骤与判定标准
  - 最小修复与临时绕过必须分开写，各自带影响范围与启用条件
  - 必须标注排查所需的日志关键字、指标名与时间窗口；输入未提供的指标名不得虚构
  - 根治方案至少含 1 条可自动化的预防措施（测试、启动校验、监控告警）
  - 不得引用输入中不存在的文件、函数、错误码；信息不足时先提澄清问题并停止
  - 不得给出未经验证的影响范围或性能数字

```text
# 适配说明
本模板已由「线上工单排障」适配为「本地故障排障」：Prompt Pad 是本地单机应用，无线上工单系统，「工单」指本机故障记录，「值班同事/客户」指开发者本人；排障依据为应用日志 + 崩溃转储 + 本机复现步骤。

# 角色
你是负责本机排障的高级工程师，负责把本地故障收敛为可执行的排障与处置方案。

# 任务
针对下面这条本机故障做定位与处置，产出可直接执行的报告。

# 输入
- 故障现象与报错原文：本机故障现象与报错原文，取自应用日志与崩溃转储；本项目高发故障面：Ctrl+Space 全局热键失效或冲突、窗口在多显示器或 DPI 变化后飘出屏幕、展开方向错误出屏、独占全屏下置顶失效或未临时隐藏、点击穿透状态错乱、夜间记忆沉淀任务崩溃或产出异常、SQLite 锁定或索引损坏、ONNX 嵌入推理失败
- 复现步骤与影响范围：本机（Windows 11）复现步骤，须注明显示器数量与缩放比例、是否处于独占全屏、Pad 当前展开/收起状态、当次使用的模型端点（MiniMax API / OpenAI 兼容端点 / Ollama）
- 相关代码与配置：src-tauri/ 与 src/ 相关片段、tauri.conf.json 窗口配置（transparent、decorations:false、alwaysOnTop、skipTaskbar、resizable:false、shadow:false、focus:false）、当日日志文件、崩溃转储、memory/ 目录最近 git 提交
- 环境信息：Windows 11；Prompt Pad 版本号（v0.1-v0.4）；Tauri 2 + React 18 + TypeScript + Rust stable；最近 git 提交摘要；当前嵌入模型名（multilingual-e5-small 或 bge-m3）

# 执行步骤
1. 信息收集：列出还需要的材料（日志片段、崩溃转储、配置项、版本差异、时间窗口），分「已具备」与「缺失」两栏。
2. 复现判断：依据复现步骤判定能否在本机复现、复现需要什么条件（多显示器、DPI 缩放、独占全屏、展开/收起状态）；不能复现时写明需要的观测手段。
3. 根因假设：按可能性降序给出不超过 5 条，每条写 假设 / 验证步骤（具体命令或操作）/ 判定标准 / 不成立时的下一步。
4. 最小修复：给出改动最小、不引入新缺陷的方案与影响范围。
5. 临时绕过：给出可立即执行的规避手段（配置开关、回退上一安装包、git 回滚 memory/ 目录、临时关闭夜间任务）及其代价。
6. 根治与预防：给出根治方案，并至少一条可自动化的预防措施。

# 交付格式
## 1. 信息收集清单
## 2. 复现判断
## 3. 根因假设
## 4. 最小修复
## 5. 临时绕过
## 6. 根治与预防
## 7. 需用户补充的信息

# 边界
- 不得引用输入中不存在的文件、函数、错误码、指标名；不得编造数据量级与性能数字。
- 关键信息缺失时先提澄清问题并停止，不得凭空补齐。
- 全文中文，命令、路径、日志关键字保留英文原样。
```

**来源**：新增（流程整合）+ zh-CN/README.md · 「错误解释与修复」

### MNT-05 · 文档体系生成

- **适用阶段**：维护迭代 · 知识沉淀
- **输入要求**：
  1. 项目名称、技术栈与功能范围：固定值：Prompt Pad（提示词优化 Pad）——常驻 Windows 系统最上层、全局热键（Ctrl+Space）唤起、带私人记忆的提示词优化 Pad；Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust 后端；存储 SQLite + FTS5 + sqlite-vec；本地嵌入 ONNX 推理；大模型三选一可配（MiniMax API / OpenAI 兼容端点 / Ollama）；四层记忆架构 L0 临时层 / L1 事实层 prompt_engineering.md / L2 记忆层 memory.md（≤800 token 硬上限）/ L3 索引层 SQLite（可重建）
  2. 目标读者与交付目标：固定值：读者为开发者本人与 AI 编码 agent；目标是让 agent 无需口头交代即可接手维护，并保证 memory/ 目录的记忆文件人工可读、可 diff、可 git 回滚
  3. 现有文档与代码目录：固定值：docs/ 目录（spec.md、architecture.md、adr/、interface-doc.md、操作手册）；memory/ 目录（memory.md、prompt_engineering.md、records/，records/ 在记录数 >30 时启用拆分）；src/（前端）与 src-tauri/（Rust 后端）
  4. 必须产出的文档清单：固定值：README、安装部署、API 参考（Tauri commands 与 SQLite 表结构 entries / entries_fts / entry_vectors / usage_log）、运维手册（夜间任务、memory/ 目录 git 每日提交、索引重建）、SOP（L2 条目晋升/降级/废弃审计、嵌入模型更换时的全库索引重建）、FAQ、架构说明
  5. README 骨架中待填写内容的占位标记：固定值：[待填写]
- **预期输出**：Markdown 文档，含文档规划表（列：文档名 / 目标读者 / 内容要点 / 建议工具 / 产出格式 / 优先级）共七类文档，与一份可直接保存为 README.md 的骨架全文；合格判据是每类文档都指定了读者、工具与格式，且 README 骨架含概述、安装、启动、配置项、目录结构、常用命令、许可证。
- **约束条件**：
  - 规划表必须覆盖 README、安装部署、API 参考、运维手册、SOP、FAQ、架构说明七类
  - 每类文档必须给出目标读者、内容要点、建议工具、产出格式四要素，工具须匹配语言生态
  - README 骨架必须包含概述、安装、启动、配置项、目录结构、常用命令、许可证七个章节
  - 待填写处统一用 [待填写] 标记，不得写示例业务内容
  - 代码类文档须标注工具与生成命令（Sphinx、Javadoc、Doxygen、DocFX、Swagger-OpenAPI）
  - 不得引用输入中不存在的目录、文件或依赖
  - 全文中文，工具名、命令、路径、许可证名保留英文原样

```text
# 角色
你负责知识沉淀的技术文档工程师，产出要能被他人直接接手维护。

# 任务
为 Prompt Pad 规划一整套文档体系，并写出 README 骨架全文。

# 输入
- 项目概况：Prompt Pad（提示词优化 Pad）——常驻 Windows 系统最上层、全局热键（Ctrl+Space）唤起、带私人记忆的提示词优化 Pad；Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust 后端；存储 SQLite + FTS5 + sqlite-vec；本地嵌入 ONNX 推理；大模型三选一可配（MiniMax API / OpenAI 兼容端点 / Ollama）；四层记忆架构 L0 临时层 / L1 事实层 prompt_engineering.md / L2 记忆层 memory.md（≤800 token 硬上限）/ L3 索引层 SQLite（可重建）
- 读者与目标：读者为开发者本人与 AI 编码 agent；目标是让 agent 无需口头交代即可接手维护，并保证 memory/ 目录的记忆文件人工可读、可 diff、可 git 回滚
- 现有文档与目录：docs/ 目录（spec.md、architecture.md、adr/、interface-doc.md、操作手册）；memory/ 目录（memory.md、prompt_engineering.md、records/，records/ 在记录数 >30 时启用拆分）；src/（前端）与 src-tauri/（Rust 后端）
- 必出文档清单：README、安装部署、API 参考（Tauri commands 与 SQLite 表结构 entries / entries_fts / entry_vectors / usage_log）、运维手册（夜间任务、memory/ 目录 git 每日提交、索引重建）、SOP（L2 条目晋升/降级/废弃审计、嵌入模型更换时的全库索引重建）、FAQ、架构说明

# 执行步骤
1. 规划：为下列每类文档写一行表格记录，字段为 文档名 / 目标读者 / 内容要点 / 建议工具 / 产出格式 / 优先级（高、中、低）。类别为 README、安装部署、API 参考、运维手册、SOP、FAQ、架构说明。
2. 选工具：按语言与文档类型匹配工具并写明生成命令——Sphinx 对应 Python，Javadoc 对应 Java，Doxygen 对应 C/C++，DocFX 对应 .NET，Swagger-OpenAPI 对应 REST API，Excalidraw 用于架构图与流程图；本项目 Rust 后端对应 rustdoc（cargo doc），TypeScript 前端对应 TypeDoc。
3. 写 README：给出可直接保存的完整骨架，含项目概述、安装、启动、配置项、目录结构、常用命令、许可证七个章节。
4. 标缺口：列出当前缺少的素材（截图、接口清单、部署拓扑），并写明补充方式。

# 交付格式
## 文档规划表
| 文档名 | 目标读者 | 内容要点 | 建议工具 | 产出格式 | 优先级 |
## README 骨架
## 待补素材清单

# 边界
- 待填写内容统一用 [待填写] 标记，不得编造项目名、命令、端口或依赖版本。
- 不得引用输入中不存在的目录、文件或服务。
- 全文中文，工具名、命令、路径、许可证名保留英文原样。
```

**来源**：zh-CN/AI-Gen-Documentation.md 全文 + zh-CN/Forward-Deployed-Engineer.md · 「把行之有效的工作模式沉淀为工具、行动手册或可复用的构建模块」

### MNT-06 · 技术债与遗留问题盘点

- **适用阶段**：维护迭代 · 质量治理
- **输入要求**：
  1. 待盘点代码范围（仓库或目录）：固定值：整个 Prompt Pad 仓库——src/（React 18 + TypeScript 前端）、src-tauri/（Rust 后端）、docs/（spec.md、architecture.md、adr/、interface-doc.md、操作手册）、memory/（memory.md、prompt_engineering.md、records/）
  2. 已知问题与历史故障记录：固定值：Tauri 官方 issue #116（alwaysOnTop + skipTaskbar 在 Linux 基本无解，本期不做 Linux）；独占全屏下置顶失效需临时隐藏；L2 memory.md ≤800 token 硬上限的强制裁剪；夜间任务的幂等性保障与冲突挂起条目的人工裁决清理；嵌入模型更换（multilingual-e5-small → bge-m3）时的全库索引重建与每条记录的模型名落库；PE 记录数 >30 后 records/ 拆分规则待定（见 docs 待办）；memory/ 目录 git 每日提交是否落实；冷启动三路径（批量回填、AI 迁移导入 preference 记忆、前 10 次豁免权重惩罚）是否实现
  3. 重点关注维度：固定值：五类全覆盖（代码异味 / 性能 / 安全 / 依赖与配置 / 文档缺失），另加本项目特有维度——memory/ 目录 git 版本管理与每日提交落实度、L2 条目晋升/降级/废弃的状态机审计（candidate 是否被误注入、deprecated 是否仍占 token 预算）、索引层可重建性（L3 SQLite 损坏后能否从 L1/L2 全量重建）、防污染四道防线（人工裁决闸门 / 候选不注入 / memory.md 纯 Markdown 可读 / memory/ 目录 git 每日提交）
  4. 团队可投入的修复工时：固定值：1 名个人开发者 + AI 编码 agent；每个盘点周期可投入工时由当次会话指定（建议 5-10 人日）
- **预期输出**：Markdown 报告，含问题清单表（列：编号 / 分类 / 位置 / 问题描述 / 影响 / 成本(人日) / 优先级）覆盖五类、分类小结、偿还路线图（按批次排列，每批含目标、条目编号、预计工时、完成判据）；合格判据是每条问题都带可定位的文件位置与成本估算。
- **约束条件**：
  - 分类必须覆盖代码异味、性能、安全、依赖与配置、文档缺失五类；无发现也要写「无」并说明检查方式
  - 每条问题的位置必须写到文件路径加函数名或行号
  - 成本统一用人日，范围 0.5-20 人日，并注明估算依据
  - 优先级用 P0 / P1 / P2 / P3 标注，P0 与 P1 合计不超过总条目的 30%
  - 路线图每批标注目标、条目编号、预计工时与完成判据，总工时不得超过 1 名个人开发者 + AI 编码 agent 在当次会话指定的可投入工时（建议 5-10 人日）
  - 不得编造性能指标、生产数据量级或漏洞编号；无法确认的标「待确认」并给出确认方法
  - 只输出报告，不修改任何文件

```text
# 角色
你负责代码库质量治理的技术负责人，只做盘点和排期，不改代码。

# 任务
对 Prompt Pad 指定代码范围做技术债与遗留问题盘点，输出问题清单与偿还路线图。

# 输入
- 代码范围：整个 Prompt Pad 仓库——src/（React 18 + TypeScript 前端）、src-tauri/（Rust 后端）、docs/（spec.md、architecture.md、adr/、interface-doc.md、操作手册）、memory/（memory.md、prompt_engineering.md、records/）
- 已知问题与历史故障记录：Tauri 官方 issue #116（alwaysOnTop + skipTaskbar 在 Linux 基本无解，本期不做 Linux）；独占全屏下置顶失效需临时隐藏；L2 memory.md ≤800 token 硬上限的强制裁剪；夜间任务的幂等性保障与冲突挂起条目的人工裁决清理；嵌入模型更换（multilingual-e5-small → bge-m3）时的全库索引重建与每条记录的模型名落库；PE 记录数 >30 后 records/ 拆分规则待定；memory/ 目录 git 每日提交是否落实；冷启动三路径（批量回填、AI 迁移导入 preference 记忆、前 10 次豁免权重惩罚）是否实现
- 重点维度：五类全覆盖（代码异味 / 性能 / 安全 / 依赖与配置 / 文档缺失），另加本项目特有维度——memory/ 目录 git 版本管理与每日提交落实度、L2 条目晋升/降级/废弃的状态机审计（candidate 是否被误注入、deprecated 是否仍占 token 预算）、索引层可重建性（L3 SQLite 损坏后能否从 L1/L2 全量重建）、防污染四道防线（人工裁决闸门 / 候选不注入 / memory.md 纯 Markdown 可读 / memory/ 目录 git 每日提交）
- 可投入工时：1 名个人开发者 + AI 编码 agent；每个盘点周期可投入工时由当次会话指定（建议 5-10 人日）

# 执行步骤
1. 归档：按 代码异味 / 性能 / 安全 / 依赖与配置 / 文档缺失 五类归档，每类给条数小结。
2. 定位：每条写明文件路径加函数名或行号，说明现象与触发条件。
3. 影响：写清影响面（受影响模块、用户可见表现、故障可能性），不写无法验证的量级数字。
4. 估成本：给人日估算与依据，标注 P0 / P1 / P2 / P3。
5. 给最小方案：每条给出不超过 3 个文件、1 天人日以内的最小修复做法。
6. 排路线图：按优先级分批，每批写目标、条目编号、预计工时、完成判据，总工时不超过当次会话指定的可投入工时。

# 交付格式
## 问题清单
| 编号 | 分类 | 位置 | 问题描述 | 影响 | 成本(人日) | 优先级 |
## 分类小结
## 偿还路线图

# 边界
- 不得引用输入中不存在的文件、函数或依赖；不得编造性能数字、生产数据量级或 CVE 编号。
- 无法确认的问题标「待确认」并写明确认方法，不得当作结论。
- 全文中文，路径、函数名、命令保留英文原样。
```

**来源**：zh-CN/README.md · 「面向最佳实践的代码评审」+「性能优化」+「SQL 查询优化」+「Vibe Coding 的 12 条规则」第 12 条「让 AI 解释代码」
