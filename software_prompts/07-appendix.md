# 工具附录：跨阶段专项提示词模板

> 本文件已按 **Prompt Pad（提示词优化 Pad）** 项目填充完毕：所有占位符均已替换为项目固定值与真实用例（正则校验 memory.md 条目格式与 PE-YYYY-NNN / M-XX-NNN 编号、PE 记录批量回填导入、SQLite 四表与 FTS5+RRF 查询优化、Ollama 本地模型选型），不留任何待填项，可直接整段复制发给开发 agent 使用。项目事实以 `docs/01-案例调研与技术选型.md` 与 `docs/02-私人记忆库设计.md` 为准。

覆盖跨阶段复用的四个专项工具：正则表达式、Pandas 数据清洗、SQL 查询优化、本地 LLM 与工具链选型。使用方式：整段复制给 agent；一次只用一个模板，不要把多个任务塞进同一轮会话。

### APP-01 · 正则表达式生成

- **适用阶段**：工具附录（跨阶段） · 文本处理
- **输入要求**：
  1. 需要匹配的文本场景与样例：固定值：校验 memory/ 目录下记忆文件的条目格式与编号——L1 事实层 prompt_engineering.md 的记录 ID 格式为 PE-YYYY-NNN（如 PE-2026-001），L2 记忆层 memory.md 的条目 ID 格式为 M-<域缩写>-<序号>（如 M-UI-001，域缩写为 2-4 位大写字母），条目标题行形如 `## M-UI-001 · 一句话规则名`；同时校验 memory.md 条目必备字段是否齐全：type / domain / status / statement / why / how_to_apply / signals / evidence / weight / hits / last_used / created，status 取值限 candidate|active|deprecated
  2. 正则引擎与目标语言：固定值：Rust `regex` crate（Rust stable 工具链），用于 src-tauri/ 侧的 prompt_engineering.md 自动校验脚本；辅以 JavaScript（V8）正则用于 React 前端预览校验；在 regex101.com 验证时选 PCRE2 flavor
  3. 期望的匹配或替换行为：固定值：对 ID 做整行锚定匹配并捕获分组（PE 记录捕获年份 YYYY 与序号 NNN；M 条目捕获域缩写与序号）；对字段名做行首 `- **字段名**:` 形态匹配以检查字段齐全性；匹配失败即判该条记录格式错误并输出所在行号
  4. 边界情况要求：固定值：序号必须为 3 位前导零数字（PE-2026-001 合法，PE-2026-1 非法）；年份限 4 位数字；域缩写限 2-4 位大写字母；不得误匹配正文中的邻近形态（如 PE-2026-0010、pe-2026-001、M-ui-001、普通文本「M-」开头）；字段名匹配须容忍字段值含中文与标点
- **预期输出**：Markdown 文档，含五部分——正则表达式代码块（标明 flags）、逐段解释表（片段 / 含义 / 作用）、测试用例表（至少 5 条正例与 5 条反例，每条写输入、预期是否匹配、预期截取结果）、易错点清单、regex101.com 验证步骤；合格判据是正则可直接粘贴使用且每条用例的预期结果明确。
- **约束条件**：
  - 正则表达式必须单独放在代码块中，并标明所用 flags
  - 正例不少于 5 条、反例不少于 5 条，反例需覆盖易被误匹配的邻近形态
  - 易错点至少覆盖三项：贪婪与惰性量词、字符类内转义、Unicode 与多字节字符、单词边界 `\b`、跨引擎差异、灾难性回溯
  - 必须给出在 https://regex101.com/ 上验证的步骤，含选择哪个 flavor、粘贴哪段 flags、怎样确认反例未被匹配
  - 不得使用目标引擎不支持的语法；存在差异时显式标注适用引擎
  - 信息不足时先提澄清问题并停止，不得凭空假设输入格式
  - 全文中文，表达式与 flags 保留英文原样

```text
# 角色
你是熟悉多语言正则引擎的文本处理工程师，产出的表达式可直接粘贴使用。

# 任务
为指定场景编写正则表达式，并交付可验证的完整说明。

# 输入
- 场景与样例：校验 memory/ 目录下记忆文件的条目格式与编号——L1 事实层 prompt_engineering.md 的记录 ID 格式为 PE-YYYY-NNN（如 PE-2026-001），L2 记忆层 memory.md 的条目 ID 格式为 M-<域缩写>-<序号>（如 M-UI-001，域缩写为 2-4 位大写字母），条目标题行形如 `## M-UI-001 · 一句话规则名`；同时校验 memory.md 条目必备字段是否齐全：type / domain / status / statement / why / how_to_apply / signals / evidence / weight / hits / last_used / created，status 取值限 candidate|active|deprecated
- 引擎与目标语言：Rust `regex` crate（Rust stable 工具链），用于 src-tauri/ 侧的 prompt_engineering.md 自动校验脚本；辅以 JavaScript（V8）正则用于 React 前端预览校验；在 regex101.com 验证时选 PCRE2 flavor
- 匹配或替换预期：对 ID 做整行锚定匹配并捕获分组（PE 记录捕获年份 YYYY 与序号 NNN；M 条目捕获域缩写与序号）；对字段名做行首 `- **字段名**:` 形态匹配以检查字段齐全性；匹配失败即判该条记录格式错误并输出所在行号
- 边界要求：序号必须为 3 位前导零数字（PE-2026-001 合法，PE-2026-1 非法）；年份限 4 位数字；域缩写限 2-4 位大写字母；不得误匹配正文中的邻近形态（如 PE-2026-0010、pe-2026-001、M-ui-001、普通文本「M-」开头）；字段名匹配须容忍字段值含中文与标点

# 执行步骤
1. 出表达式：给出最终正则，单独代码块展示并标注 flags（如 i、m、s、g）。
2. 拆解：把表达式按片段拆开，逐段写含义与作用，用表格呈现。
3. 测试用例：给不少于 5 条正例与不少于 5 条反例，每条写 输入 / 预期是否匹配 / 预期截取结果。
4. 易错点：至少说明 3 项——贪婪与惰性量词、字符类内的转义、Unicode 与多字节字符、锚点与单词边界 \b、跨引擎差异（PCRE、PCRE2、JS、Java、Python、Rust regex crate）、灾难性回溯。
5. 验证步骤：说明在 https://regex101.com/ 上选哪个 flavor、粘贴哪段 flags、如何确认反例未被匹配。

# 交付格式
## 正则表达式
## 逐段解释
## 测试用例（正例与反例）
## 易错点
## 验证步骤

# 边界
- 不得使用目标引擎不支持的语法；存在差异时标注适用引擎。
- 不得编造样例数据；测试用例只使用上述场景给出的形态（PE-YYYY-NNN、M-<域缩写>-<序号>、条目标题行与字段行），样例不足时先提问并停止。
- 全文中文，表达式、flags、语法标记保留英文原样。
```

**来源**：zh-CN/README.md · 「6. 生成正则表达式」

### APP-02 · Pandas 数据清洗

- **适用阶段**：工具附录（跨阶段） · 数据处理
- **输入要求**：
  1. 数据文件路径与格式：固定值：冷启动批量回填用的历史 PE 记录导出文件 memory/records/import/pe_raw_export.csv（UTF-8 CSV，来自用户历史提示词使用记录的整理导出）
  2. 字段说明与主键定义：固定值：字段对应 L1 事实层 prompt_engineering.md 的记录结构——pe_id（主键，格式 PE-YYYY-NNN）、date、domain、original_prompt、optimized_prompt、strategy、result_score（1-5 整数）、signals、notes；主键为 pe_id
  3. 缺失值与重复行的处理策略：固定值：pe_id 缺失或格式不符（非 PE-YYYY-NNN）的行剔除并记入脏数据清单；result_score 缺失保留为空（该记录仅作事实存档，不参与权重计算）；domain 缺失填 global；signals 缺失填空数组；重复 pe_id 保留 date 最早的一条；original_prompt 与 optimized_prompt 为必填文本，任一空则剔除并记脏数据
  4. 目标导出格式与路径：固定值：双路导出——① 中间清洗结果 memory/records/import/pe_clean.csv（UTF-8、不带索引列）；② 逐字不可改的 L1 Markdown 记录片段，写入 memory/records/ 目录（记录数 >30 时按年月拆分为 records/YYYY-MM.md），供夜间任务批量提炼候选规则
- **预期输出**：Markdown 文档，含可运行的 Python 脚本代码块（去重、缺失值填充、列名 snake_case 规范化、类型转换、导出）、处理前后数据质量指标表（行数 / 列数 / 缺失率 / 重复率）、列名转换对照表、异常处理说明；合格判据是脚本可直接运行且指标字段与处理步骤一一对应。
- **约束条件**：
  - 脚本必须可直接运行，含 import、读取、清洗、导出与主入口，输出路径固定为 memory/records/import/pe_clean.csv 与 memory/records/
  - 去重必须基于主键 pe_id，不得直接整行 drop_duplicates
  - 缺失值策略按列分别写明，并解释选择理由
  - 规范化后列名必须全为小写 snake_case，并给出转换前后对照表
  - 必须输出处理前后的行数、列数、缺失率、重复率四项指标
  - 不得编造数据行数、字段名或处理结果数值；脚本须对缺失文件与类型冲突做显式异常处理
  - 全文中文，代码、路径、列名保留英文原样

```text
# 角色
你是数据工程师，交付的清洗脚本要能被他人直接运行并复现结果。

# 任务
对 Prompt Pad 冷启动批量回填的历史 PE 记录编写 Pandas 清洗脚本，并给出处理前后的数据质量指标。

# 输入
- 数据文件：memory/records/import/pe_raw_export.csv（UTF-8 CSV，来自用户历史提示词使用记录的整理导出）
- 字段与主键：字段对应 L1 事实层 prompt_engineering.md 的记录结构——pe_id（主键，格式 PE-YYYY-NNN）、date、domain、original_prompt、optimized_prompt、strategy、result_score（1-5 整数）、signals、notes；主键为 pe_id
- 清洗策略：pe_id 缺失或格式不符（非 PE-YYYY-NNN）的行剔除并记入脏数据清单；result_score 缺失保留为空（该记录仅作事实存档，不参与权重计算）；domain 缺失填 global；signals 缺失填空数组；重复 pe_id 保留 date 最早的一条；original_prompt 与 optimized_prompt 为必填文本，任一空则剔除并记脏数据
- 导出目标：双路导出——① 中间清洗结果 memory/records/import/pe_clean.csv（UTF-8、不带索引列）；② 逐字不可改的 L1 Markdown 记录片段，写入 memory/records/ 目录（记录数 >30 时按年月拆分为 records/YYYY-MM.md），供夜间任务批量提炼候选规则

# 执行步骤
1. 读取：按 memory/records/import/pe_raw_export.csv 的格式选择读取方式，指定 dtype 与编码，处理读入异常。
2. 去重：按主键 pe_id 去重（保留 date 最早一条），记录被去掉的行数。
3. 列名规范化：统一为小写 snake_case，输出转换前后对照表。
4. 缺失值：按列套用上述清洗策略，分别写明填充、删除或保留的理由。
5. 类型转换：把 date、result_score 等列转成目标类型；类型冲突时记录行数并单独处理。
6. 导出：写出 pe_clean.csv（UTF-8、无索引列）与 memory/records/ 下的 L1 Markdown 片段（记录数 >30 时按年月拆分 records/YYYY-MM.md），写明编码与索引设置。
7. 指标：输出处理前后的行数、列数、缺失率、重复率四项对照。

# 交付格式
## 清洗脚本
## 列名转换对照表
## 数据质量指标（处理前与处理后）
## 异常处理说明
## 需人工确认的脏数据清单

# 边界
- 不得编造数据行数、列名或处理结果；所有数字必须由脚本实际计算得出。
- 不得使用上述字段说明未定义的字段；字段不存在时先提问并停止。
- 全文中文，代码、路径、列名保留英文原样。
```

**来源**：zh-CN/README.md · 「数据处理与分析 → 使用 Pandas 进行数据清洗」

### APP-03 · SQL 查询优化

- **适用阶段**：工具附录（跨阶段） · 数据性能
- **输入要求**：
  1. 待优化的 SQL 与数据库类型：固定值：Prompt Pad 检索层的混合检索 SQL，数据库为 SQLite（启用 FTS5 与 sqlite-vec 扩展）；查询形态为阶段三的两路召回 + RRF 融合——entries_fts 全文召回（BM25 排序）与 entry_vectors 向量 KNN 召回（sqlite-vec）两路并行，再用 Reciprocal Rank Fusion 合并归一化到 0-1，最后按 domain 路由与 status=active、weight>0 过滤并按 weight 降序取 Top-K；阶段二的等效查询为纯 FTS5 关键词 + 模糊匹配
  2. 表结构与数据量级：固定值：四张表——entries（L1/L2 条目原文与元数据：id、layer、domain、status、weight、hits、last_used、created、model_name 等）、entries_fts（FTS5 虚表，索引 entries 的可检索文本列）、entry_vectors（sqlite-vec 向量表，384 维 multilingual-e5-small 或 1024 维 bge-m3，每条记录存嵌入时使用的模型名）、usage_log（注入与评分日志：entry_id、score、created）；数据量级：阶段一 <50 条（无检索，纯 Markdown domain 路由）、阶段二 50-300 条、阶段三 300+ 条
  3. 当前执行耗时与瓶颈现象：固定值：Ctrl+Space 唤起后注入链路必须在用户无感知延迟内完成；具体耗时与瓶颈以 EXPLAIN QUERY PLAN 实测为准（常见嫌疑：FTS5 召回未走索引、向量 KNN 的 k 值过大、RRF 融合在应用层做全量排序、usage_log 无索引导致连续未命中计数查询全表扫描）
  4. 可用的索引与约束：固定值：entries_fts（FTS5 全文索引）、entry_vectors（sqlite-vec 向量索引）、usage_log(entry_id, created) 复合索引候选；entries 主键索引；其余以实际建表语句为准
- **预期输出**：Markdown 文档，含四部分——低效点列表（表：位置 / 问题 / 后果）、优化后的等价查询代码块、EXPLAIN 验证步骤、索引建议表（列：表 / 索引列 / 索引类型 / 预期作用 / 写代价）；合格判据是优化后查询与原查询返回相同结果集，且每条低效点都对应具体改写。
- **约束条件**：
  - 优化后查询必须与原查询语义等价，行数与字段取值范围一致；存在差异时明确标注
  - 每条低效点必须指明原查询中的具体子句或表达式
  - 必须给出 EXPLAIN 或等效执行计划命令，并说明要看哪些字段
  - 索引建议不超过 5 条，每条写明列顺序、类型与写放大代价
  - 复杂度与扫描行数只写预期方向，不得编造具体行数、耗时或提速倍数
  - 不得引用输入中不存在的表、列或索引；信息不足时先提问并停止
  - 全文中文，SQL 关键字、表名列名、命令保留英文原样

```text
# 角色
你是数据库性能工程师，交付的改写必须与原查询语义等价且可在生产执行。

# 任务
优化 Prompt Pad 检索层给定的 SQL 查询，输出低效点、等价改写、验证步骤与索引建议。

# 输入
- 原查询与数据库类型：Prompt Pad 检索层的混合检索 SQL，数据库为 SQLite（启用 FTS5 与 sqlite-vec 扩展）；查询形态为阶段三的两路召回 + RRF 融合——entries_fts 全文召回（BM25 排序）与 entry_vectors 向量 KNN 召回（sqlite-vec）两路并行，再用 Reciprocal Rank Fusion 合并归一化到 0-1，最后按 domain 路由与 status=active、weight>0 过滤并按 weight 降序取 Top-K；阶段二的等效查询为纯 FTS5 关键词 + 模糊匹配
- 表结构与数据量级：四张表——entries（L1/L2 条目原文与元数据：id、layer、domain、status、weight、hits、last_used、created、model_name 等）、entries_fts（FTS5 虚表，索引 entries 的可检索文本列）、entry_vectors（sqlite-vec 向量表，384 维 multilingual-e5-small 或 1024 维 bge-m3，每条记录存嵌入时使用的模型名）、usage_log（注入与评分日志：entry_id、score、created）；数据量级：阶段一 <50 条、阶段二 50-300 条、阶段三 300+ 条
- 当前耗时与瓶颈：Ctrl+Space 唤起后注入链路必须在用户无感知延迟内完成；具体耗时与瓶颈以 EXPLAIN QUERY PLAN 实测为准（常见嫌疑：FTS5 召回未走索引、向量 KNN 的 k 值过大、RRF 融合在应用层做全量排序、usage_log 无索引导致连续未命中计数查询全表扫描）
- 已有索引：entries_fts（FTS5 全文索引）、entry_vectors（sqlite-vec 向量索引）、usage_log(entry_id, created) 复合索引候选；entries 主键索引；其余以实际建表语句为准

# 执行步骤
1. 拆句：标出子查询、JOIN、聚合、排序、过滤子句各自所在位置。
2. 列低效点：用表格逐条给出 位置 / 问题 / 后果，每条必须指到原查询的具体子句。
3. 改写：给出优化后的完整 SQL，保持结果集语义等价，逐处标注改写对应的低效点编号。
4. 验证：给出 SQLite 的 EXPLAIN QUERY PLAN 命令与判定方法，说明要看扫描行数、排序与临时表、连接顺序中的哪些。
5. 索引建议：给出不超过 5 条，写明列顺序、索引类型（复合、覆盖、部分）与写代价。
6. 预期：用方向性表述说明复杂度与扫描行数的变化，如更少扫描、消除排序、提前收敛，不写具体数字。

# 交付格式
## 低效点列表
## 优化后查询
## EXPLAIN 验证步骤
## 索引建议
## 语义等价性说明

# 边界
- 不得引用输入中不存在的表、列或索引（仅限 entries / entries_fts / entry_vectors / usage_log 及其上述字段）；不得编造执行计划、耗时或提速倍数。
- 信息不足时先提澄清问题并停止。
- 全文中文，SQL、表名列名、命令保留英文原样。
```

**来源**：zh-CN/README.md · 「SQL 查询优化」

### APP-04 · 本地 LLM 与工具链选型

- **适用阶段**：工具附录（跨阶段） · 工程工具链决策
- **输入要求**：
  1. 硬件配置（GPU 型号与显存、系统内存、是否 Apple Silicon）：固定值：开发者机器为 Windows 11；非 Apple Silicon；GPU 型号、显存与系统内存以本机实测为准（选型报告第一步必须实测，不得假定）
  2. 目标项目类型与代码规模：固定值：Prompt Pad——Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust 的个人规模单机应用；本地模型的两个用途：① 作为提示词改写的大模型后端（三选一可配中的 Ollama 本地模型档位）；② 作为 AI 编码 agent 的辅助推理
  3. 隐私与成本约束：固定值：隐私优先——memory/ 目录的记忆数据不出本机，嵌入推理必须本地 ONNX（multilingual-e5-small / bge-m3）；提示词改写允许回退云端（MiniMax API / OpenAI 兼容端点）；本地模型零边际成本
  4. 期望的上下文长度与日常任务：固定值：提示词改写为短上下文任务（注入记忆 ≤800 token + 当前提示词）；编码辅助需覆盖单文件修改到跨文件改造；工具链事实：Ollama 跑本地模型、包管理 pnpm、Rust 工具链 stable
- **预期输出**：Markdown 选型报告，含五部分——可跑模型档位表（列：参数规模 / 量化档位 / 预估占用显存 / 适用任务）、速度与质量取舍表、上下文窗口与显存联动测算表、本地工具接线步骤清单、回退云端的触发条件清单；合格判据是每条建议都标明适用硬件前提，且显存测算写出计算式。
- **约束条件**：
  - 显存测算必须写出计算式（参数量 × 量化位数 ÷ 8 × 系数），不得只给结论数字
  - 本机为 Windows 11，须区分独立显卡（以 VRAM 为上限）与核显/无独显（共享系统内存，注明可用内存占比与退化后果）两种情形；Apple Silicon 统一内存情形不适用，不展开
  - 必须给出上下文长度与显存占用的联动关系，并指出默认 4000 token 上下文对真实项目的不足
  - 接线步骤必须包含：Ollama 安装与拉取模型、启动本地服务（ollama serve，默认 http://localhost:11434）、开启 OpenAI API 兼容端点（http://localhost:11434/v1）、在 Prompt Pad 的模型配置中选择 Ollama 档位并填写 base_url 与模型名、验证一次改写请求
  - 必须列出必须回到云端模型的具体条件（跨仓库大范围改造、要求长上下文、要求稳定吞吐、本地模型改写质量不达标等）
  - 不得编造输入中没有的硬件参数；速度数值只能作为量级参考并标注为参考值
  - 全文中文，工具名、参数、命令保留英文原样

```text
# 角色
你是本地 AI 编码环境选型顾问，结论必须能被用户在自己的硬件上复现。

# 任务
基于用户硬件与 Prompt Pad 项目情况，给出本地模型（Ollama 场景）与工具链的选型方案。

# 输入
- 硬件：开发者机器为 Windows 11；非 Apple Silicon；GPU 型号、显存与系统内存以本机实测为准（选型报告第一步必须实测，不得假定）
- 项目：Prompt Pad——Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust 的个人规模单机应用；本地模型的两个用途：① 作为提示词改写的大模型后端（三选一可配中的 Ollama 本地模型档位）；② 作为 AI 编码 agent 的辅助推理
- 约束：隐私优先——memory/ 目录的记忆数据不出本机，嵌入推理必须本地 ONNX（multilingual-e5-small / bge-m3）；提示词改写允许回退云端（MiniMax API / OpenAI 兼容端点）；本地模型零边际成本
- 使用需求：提示词改写为短上下文任务（注入记忆 ≤800 token + 当前提示词）；编码辅助需覆盖单文件修改到跨文件改造；工具链事实：Ollama 跑本地模型、包管理 pnpm、Rust 工具链 stable

# 执行步骤
1. 判瓶颈：先实测并算出可用显存。独立显卡以 VRAM 为上限；核显/无独显以共享系统内存按可用比例折算，并说明被系统与其它应用挤占后的退化后果。
2. 定档位：给出可跑模型档位表，列为 参数规模 / 量化档位 / 预估占用 / 适用任务，计算式写明 参数量 × 量化位数 ÷ 8 × 系数。
3. 取舍：给出速度与质量取舍表，说明大模型质量更高但吞吐显著下降的区间。
4. 算上下文：测算上下文长度与显存占用的联动，指出默认 4000 token 上下文覆盖不了真实项目时的取用方式（提示词改写为短上下文任务可接受，编码辅助需上调）。
5. 接线：列出 Ollama 安装与拉取模型、启动本地服务（ollama serve，默认 http://localhost:11434）、开启 OpenAI API 兼容端点（http://localhost:11434/v1）、在 Prompt Pad 的模型配置中选择 Ollama 档位并填写 base_url 与模型名、验证一次改写请求的步骤。
6. 定回退：列出必须回到云端模型（MiniMax API / OpenAI 兼容端点）的条件，如跨仓库大范围改造、要求长上下文、要求稳定吞吐、本地模型改写质量不达标等。

# 交付格式
## 硬件判定与显存预算
## 可跑模型档位表
## 速度与质量取舍
## 上下文与显存联动
## 本地工具接线步骤
## 回退云端的触发条件

# 边界
- 不得编造输入中不存在的硬件参数；速度数值只作量级参考并标注为参考值。
- 结论必须标注适用前提，包括显卡型号、显存、内存、Windows 11 平台。
- 全文中文，工具名、参数、路径、命令保留英文原样。
```

**来源**：zh-CN/Local-AI-Coding.md 全文
