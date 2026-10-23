# Prompt Pad · 软件开发全流程 Agent 提示词（已填充版）

52 个提示词模板，已按 **Prompt Pad 项目**（常驻 Windows 最上层、Ctrl+Space 全局唤起、带私人记忆的提示词优化 Pad）的实际需求完成全部占位符填充，**无需再替换任何内容，可整段复制直接发给开发 agent**。

> 源模板：`C:/Users/Administrator/Desktop/agent-prompts/`（保留不动）。
> 项目事实来源：`F:/Prompt_pad_2/docs/01-案例调研与技术选型.md`、`F:/Prompt_pad_2/docs/02-私人记忆库设计.md`。

---

## 1. 项目速览（全部模板共享的事实基线）

| 项 | 固定值 |
|---|---|
| 项目 | Prompt Pad —— 常驻系统最上层、全局热键唤起、带私人记忆的提示词优化 Pad |
| 平台 | Windows 优先（后续可扩 macOS，本期不做 Linux） |
| 技术栈 | Tauri 2 + React 18 + TypeScript + Vite + Tailwind + Rust；SQLite + FTS5 + sqlite-vec；ONNX 本地嵌入（multilingual-e5-small → bge-m3）；MiniMax API / OpenAI 兼容端点 / Ollama 三选一 |
| 记忆架构 | L0 临时 / L1 事实（prompt_engineering.md，PE-YYYY-NNN）/ L2 记忆（memory.md，≤800 token，M-XX-NNN）/ L3 SQLite 索引 |
| 团队 | 1 名个人开发者 + AI 编码 agent |
| 版本规划 | v0.1=骨架+DOT+动画+改写 · v0.2=落库+热键托盘 · v0.3=检索层 · v0.4=夜间记忆沉淀 |

## 2. 阶段映射

| 流程环节 | 环节交付物 | 文件 | 模板 |
|---|---|---|---|
| ① 需求沟通 | spec.md、范围文档、用户故事 | [01-requirements.md](01-requirements.md) | REQ-01 ~ REQ-07 |
| ③ 系统设计 | 架构文档、数据库设计、接口文档、ADR、风险登记册 | [02-architecture.md](02-architecture.md) | ARC-01 ~ ARC-07 |
| ④ 开发阶段 | 任务拆解、实现、评审、规约 | [03-implementation.md](03-implementation.md) | IMP-01 ~ IMP-09 |
| ⑤ 测试阶段 | 测试用例、基准、缺陷修复、测试报告 | [04-testing.md](04-testing.md) | TST-01 ~ TST-06 |
| ⑥ 软件上线 | NSIS 安装包、GitHub Actions、检查清单、SOP | [05-deployment.md](05-deployment.md) | OPS-01 ~ OPS-07 |
| ⑦ 售后维护 | 迭代评估、发布说明、排障、文档体系 | [06-maintenance.md](06-maintenance.md) | MNT-01 ~ MNT-06 |
| 全程贯穿 | 沟通底座 | [00-core-strategies.md](00-core-strategies.md) | CORE-01 ~ CORE-06 |
| 跨阶段专项 | 工具附录 | [07-appendix.md](07-appendix.md) | APP-01 ~ APP-04 |

## 3. 推荐使用顺序（配合实施路线 8 步）

1. 开工先跑 **CORE-01** → 产出 `AGENTS.md`，后续每次会话加载。
2. 需求阶段：**REQ-01 → REQ-02 → REQ-03 → REQ-04 → REQ-05**，产出 spec.md 与范围文档。
3. 设计阶段：**ARC-01 → ARC-02 → ARC-03 → ARC-04 → ARC-06 → ARC-07**。
4. 开发阶段：**IMP-01** 拆任务 → 每任务 **IMP-02** 实现 → **IMP-04** 评审；重大改动用 **CORE-04** 分步卡点（回复 `next` 放行）。
5. 测试阶段：**TST-01 → TST-02 → TST-05 → TST-06**；遇 bug 用 **TST-03 / TST-04**。
6. 上线：**OPS-01 → OPS-02 → OPS-03 → OPS-04**；运维期 **OPS-05**（本地日志）+ **OPS-06**（回滚）。
7. 迭代期：**MNT-01** 评估 → **MNT-02** 实现 → **MNT-03** 发布说明；排障 **MNT-04**；文档 **MNT-05**；技术债 **MNT-06**。

## 4. 相对源模板的关键适配（个人单机项目）

| 模板 | 适配 |
|---|---|
| REQ-06 报价估算 | 无商务报价 → 资源投入与 API 成本估算，付款节点 → 自验收费节点 |
| OPS-01 Dockerfile | 单机桌面应用 → Windows 安装包打包配置（tauri.conf.json bundle + NSIS） |
| OPS-02 CI/CD | → GitHub Actions：lint → test → tauri build → 发布 GitHub Releases |
| OPS-05 监控告警 | → 本地日志轮转（10MB×7 天）+ 崩溃转储 + 应用内诊断页 |
| OPS-06 回滚预案 | → 安装包版本回退 + git revert + memory/ 目录 git 精确回滚 |
| MNT-04 线上工单 | → 本地故障排障（日志 + 崩溃转储 + 复现步骤） |
| ARC-04 API 设计 | 无 HTTP 接口 → Tauri command 签名表 + serde JSON Schema |
| IMP-07 迁移 | 收敛为唯一真实场景：实验性 Python 验证脚本 → Rust（src-tauri/） |

适配模板正文顶部均有一行「适配说明」，不影响模板结构与防幻觉条款。

## 5. 使用铁律

- 每个模板的「约束条件」里都含防幻觉条款：agent 产出后必须人工核对输入之外的 API、字段、指标、路径是否真实存在，再决定是否采纳。
- 所有路径占位已固化为 `F:/Prompt_pad_2/docs/` 下的具体文件；放行关键词统一为 `next`。
- 复杂任务务必用 **CORE-04** 分步推进，禁止让小参数模型连做多步。
