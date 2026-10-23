---
format_version: 1
token_budget: 800          # 硬上限，超出即拒绝晋升
domain: <global>
updated: 2026-10-21
entry_count: 0
active_count: 0
deprecated_count: 0
---

<!--
本文件是 L2 记忆层。

规则（来自 docs/02-私人记忆库设计.md）：
- 总 token 不得超过 800，超出即拒绝晋升。
- 每条记忆必须有唯一 ID、明确陈述、来源凭据、可计算的权重。
- status 只有 candidate / active / deprecated 三种；candidate 不参与注入。
- weight 由真实成败累积，不由人工设定：评分 ≥4 → +1；无改善 → 0；变差 → −2。
- 本文件是纯 Markdown：随时能打开看、能手改、能 diff。绝不让记忆只存在于向量库里。
- 整个 memory/ 目录纳入 git，每日一提交。

条目格式（下面的示例是格式说明，请按需保留或删除）：

## M-<域缩写>-<序号> · 一句话规则名

- **type**: rule              # rule | preference | pitfall | asset
- **domain**: ui-design       # 与 L1 的 domain 对齐，用于路由
- **status**: candidate       # candidate | active | deprecated
- **statement**: 当任务涉及品牌色时，必须把不可替换的品牌色单列为「硬性约束」，把「需要更多变化」写为软偏好并显式声明不得替代主色。
- **why**: 防止模型用生成的色彩替换品牌主色，导致品牌识别崩塌。
- **how_to_apply**: 在提示词中单开一章「硬性品牌约束」，逐条列出不可违反项。
- **signals**: [S-HARD-SOFTSPLIT]
- **evidence**: [PE-2026-001]        # 来源记录，可追溯到原文
- **weight**: 0                       # 成功次数 − 失败次数，由系统自动维护
- **hits**: 0                         # 被注入的次数
- **last_used**: null
- **created**: 2026-10-21
-->
