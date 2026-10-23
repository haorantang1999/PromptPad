<p align="center">
  <img src="docs/assets/readme-hero.jpg" alt="Prompt Pad — 常驻提示词优化 Pad，蜜蜂形态 DOT" width="920" />
</p>

<h1 align="center">Prompt Pad</h1>

<p align="center">
  <strong>Windows 上永远在线的提示词工作室。</strong><br />
  一个热键 · 一只浮窗小蜜蜂 · 越用越懂你的私人记忆。
</p>

<p align="center">
  <a href="./README.md">中文</a> · <a href="./README.en.md">English</a>
</p>

<p align="center">
  <a href="#为什么选-prompt-pad"><img src="https://img.shields.io/badge/产品-本地优先-FF8A00?style=flat-square" alt="本地优先" /></a>
  <a href="#架构一览"><img src="https://img.shields.io/badge/技术栈-Tauri%202%20%2B%20React%2018%20%2B%20Rust-1d1d1f?style=flat-square" alt="技术栈" /></a>
  <a href="#越用越聪明的记忆"><img src="https://img.shields.io/badge/记忆-Markdown%20真相源%20%2B%20FTS5-34c759?style=flat-square" alt="记忆" /></a>
  <a href="#快速开始"><img src="https://img.shields.io/badge/平台-Windows%2011-0078D4?style=flat-square" alt="Windows" /></a>
</p>

---

## 产品主张

如果你每天都在用 AI 写作 / 编程，这些摩擦一定很熟：

- 好用的提示词散落在各个对话标签页里，下次再也找不到
- 同一套纠正话术，每天早上重新打一遍
- 云端「记忆」产品想要你的草稿、密钥和使用习惯

**Prompt Pad 押的是另一边。**

它是 Windows 上的**极小常驻层**：蜜蜂形态的 DOT 浮在所有应用之上。按下 `Ctrl+Q`，丢进一段毛坯提示词，拿回更清晰、可直接执行的版本 —— 同时让一条私人记忆回路，安静地学会「你习惯怎么问」。

> **对话是草稿，文件是大脑。**  
> 模型并不「记住你」——它只记住**去哪里找**。

---

## 为什么选 Prompt Pad

| 优势 | 落地含义 |
|---|---|
| **不用时极小** | 约 64px 浮窗圆点。不用 Electron 常驻吃内存，不占永久侧栏。 |
| **打开时强大** | 一键改写；提供方无关（OpenAI 兼容 / Anthropic Messages / Ollama / MiniMax）。 |
| **记忆会复利** | 夜间沉淀把当天 PE 记录炼成可复用规则；下次改写只注入「挣到席位」的条目。 |
| **中文检索可用** | 阶段二 FTS5 `trigram` BM25（短查询 LIKE 回退）；domain 路由作安全网。 |
| **本地优先** | 本机 SQLite + Markdown。API key 不出机器。没有云同步产品。 |
| **尊重焦点** | 收起态不抢输入焦点；收起后把焦点还给原应用。 |
| **记忆可审计** | `memory.md` 纯 Markdown、可进 git；注入硬上限 **800 token**。 |

<p align="center">
  <img src="docs/assets/readme-privacy.jpg" alt="本地优先：提示词与密钥留在本机" width="920" />
</p>

---

## 60 秒故事

<p align="center">
  <img src="docs/assets/readme-workflow.jpg" alt="三步流程：草稿 → 唤起 → 交付" width="920" />
</p>

1. **任意处写草稿** — VS Code、浏览器、Notion、邮件……先写乱的版本。
2. **唤起小蜜蜂** — `Ctrl+Q`（可配置）。粘贴，或从剪贴板预填。
3. **带走改写结果** — 一键复制。可选 1–5 分评分，驱动记忆权重更新。

没有新开聊天线程，没有翻上下文考古。Pad 出现、帮忙、消失。

---

## 越用越聪明的记忆

<p align="center">
  <img src="docs/assets/readme-memory.jpg" alt="四层记忆架构" width="920" />
</p>

| 层 | 载体 | 职责 |
|---|---|---|
| **L0 临时** | 会话上下文 | 面板关闭即弃 |
| **L1 事实** | `memory/prompt_engineering.md` | 只追加的 PE 记录（`PE-YYYY-NNN`）。永久、逐字不可改 |
| **L2 记忆** | `memory/memory.md` | 提炼后的规则，注入预算 ≤ **800 token** |
| **L3 索引** | SQLite + FTS5 | 可重建的检索索引；**以 Markdown 为准** |

**注入纪律**（多数「AI 记忆」产品会跳过的部分）：

- 只注入 `statement` + `why` + `how_to_apply`
- `candidate` 未验证前永不注入
- 无匹配时明确告诉模型：**「暂无历史经验」**
- 评分：≥4 → `+1` · 无改善 → `0` · 变差 → `−2`（惩罚重于奖励）

状态机：

```text
candidate  --(weight ≥ 2)-->  active  --(weight < −1 或连续 3 次未命中)-->  deprecated
```

---

## 产品原则

1. **TINY WHEN UNUSED** — 跟注意力抢地盘的产品，已经输了。
2. **POWERFUL WHEN OPENED** — 一个面板、一个主操作、立刻改写。
3. **DATA NEVER LEAVES THIS MACHINE** — 本地 Markdown + SQLite。你的 git，你的规则。
4. **MARKDOWN IS THE BRAIN** — 索引可以烧掉；真相必须撑过重建。
5. **MEMORY MUST EARN INJECTION** — 权重、状态、token 预算是产品能力，不是事后补丁。

---

## 架构一览

```text
┌──────────────────────────────────────────────────────────┐
│  React 18 · TypeScript · Tailwind · Zustand              │
│  Dot · Panel · Result · Settings · Status                │
└───────────────────────────┬──────────────────────────────┘
                            │ Tauri IPC（类型化命令）
┌───────────────────────────▼──────────────────────────────┐
│  Rust · Tauri 2                                          │
│  commands/ → services/ → storage/                        │
│  window · provider · memory · nightly · tray · geometry  │
└───────────────────────────┬──────────────────────────────┘
                            │
        ┌───────────────────┴───────────────────┐
        ▼                                       ▼
 memory/*.md（真相源）                    SQLite + FTS5（索引）
```

**我们真正守住的硬约束：**

- 外壳：**Tauri 2**（禁止 Electron）
- 插件白名单：global-shortcut · single-instance · autostart · clipboard-manager
- 托盘走核心 `tauri::tray`
- Windows 优先（本期不做 Linux）

---

## 功能地图

| # | 能力 | 状态 |
|---|---|---|
| ① | 透明置顶无边框常驻窗口 | ✅ |
| ② | 常驻 DOT（拖拽、按屏百分比落位） | ✅ |
| ③ | 展开 / 收起 + 边缘自适应 | ✅ |
| ④ | 一键改写（OpenAI / Anthropic 双协议） | ✅ |
| ⑤ | PE 记录 → SQLite + Markdown | ✅ |
| ⑥ | 热键 · 托盘 · 自启 · 单实例 | ✅ |
| ⑦ | 检索：domain 回退 + **FTS5 trigram BM25** | ✅ |
| ⑧ | 夜间记忆沉淀 | ✅ |
| ⑨ | 记忆注入 ≤800 token | ✅ |
| ⑩ | 评分 → 权重更新 | ✅ |
| ⑪ | 冷启动豁免（前 10 次） | ✅ |
| — | 向量 / sqlite-vec 阶段三 | 🔒 语料够了再开 |

---

## 快速开始

### 环境

- Windows 11
- Node.js 22+ · pnpm 12+
- Rust stable（MSVC）。若工具链在 D 盘，把 `D:\Rust\cargo\bin` 加入 `PATH`。

### 安装与运行

```bash
pnpm install
pnpm tauri dev      # 完整桌面应用
```

仅前端 UI：

```bash
pnpm dev            # http://localhost:1420
pnpm typecheck && pnpm test && pnpm lint
```

质量闸：

```bash
pnpm check          # rust 结构 · geometry · sqlite DDL · docs JSON
cargo test  --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```

### 接入模型

从托盘或面板齿轮进入**设置**。预设含 MiniMax、Anthropic、OpenAI 兼容、Ollama。

密钥可放在：

1. `%APPDATA%` 配置里的本机明文字段（永不提交），或
2. 你配置的环境变量名

后端**永不回显**明文密钥 —— 只回 `api_key_present`。

---

## 仓库结构

```text
PromptPad/
├── AGENTS.md                 # 工程事实基线（先读）
├── README.md                 # 中文（默认）
├── README.en.md              # English
├── src/                      # React 18 UI
├── src-tauri/                # Rust 后端
├── memory/                   # L1 + L2 Markdown 大脑（纳入 git）
├── docs/                     # 规格 · 架构 · 库表 · 接口 · 配图
├── scripts/                  # 零依赖质量闸 + 图标生成
└── software_prompts/         # 参考提示词模板（只读）
```

深入阅读：

| 文档 | 回答什么 |
|---|---|
| [`AGENTS.md`](AGENTS.md) | 不可协商约束与当前状态 |
| [`docs/spec.md`](docs/spec.md) | 产品需求与验收标准 |
| [`docs/architecture.md`](docs/architecture.md) | 分层与依赖规则 |
| [`docs/database-design.md`](docs/database-design.md) | SQLite / FTS5 表结构 |
| [`docs/interface-doc.md`](docs/interface-doc.md) | Tauri 命令契约 |

---

## 适合谁

- 每天改写提示词**几十次**的人
- 想要记忆、但**不想把第二个大脑上传给厂商**的人
- 讨厌 Electron 常驻「AI 侧栏」、永远不肯让路的人

一周只问一次 AI，聊天窗口就够了。  
若提示词是你的手艺本身 —— Prompt Pad 是基础设施。

---

## 路线图快照

- [x] 本地改写 Pad + 焦点友好浮窗  
- [x] Markdown 为真相的记忆 + 夜间沉淀  
- [x] 阶段二中文可用的 FTS 检索  
- [ ] 阶段三本地 embedding + sqlite-vec（按真实语料规模开门）  
- [ ] 可选 DPAPI 加密落盘 API key  
- [ ] 更完整的首次引导 / 冷启动向导  

---

## 许可与贡献

个人 / 早期产品阶段。公开表面稳定后欢迎 Issue 与用心的 PR。

---

<p align="center">
  <em>不用时极小。打开时强大。记住的，是你的。</em><br />
  <strong>Prompt Pad</strong>
</p>
