# 05 · 部署与运维提示词集

覆盖「部署运维」阶段的 7 个作业指令：安装包打包、CI/CD 流水线、上线前检查、上线验证冒烟、可观测性配置、回滚预案、操作手册与 SOP。
**本文件已按 Prompt Pad 项目填充完毕**：Prompt Pad 是 Windows 单机桌面应用（Tauri 2 + React 18 + Rust），「部署」= 本地打包 + GitHub Releases 分发 NSIS 安装包，无 Docker/K8s/服务器场景；「监控」= 本地日志文件轮转 + 崩溃转储；「回滚」= 安装包版本回退 + git revert。与原模板场景冲突的 OPS-01/02/05/06 已在正文顶部加适配说明。可直接复制提示词正文发给开发 agent，无需再替换任何内容。

### OPS-01 · Windows 安装包打包配置（tauri.conf.json bundle + NSIS）

- **适用阶段**：部署运维 · 打包分发
- **输入要求**：
  1. 应用技术栈与运行方式：固定值：Prompt Pad——Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust 后端；SQLite + FTS5 + sqlite-vec；ONNX 本地嵌入模型（multilingual-e5-small ~117MB 需随包或首启下载）；Windows 常驻置顶桌面应用，非服务端。
  2. 依赖清单与构建启动命令：固定值：`pnpm install` 安装前端依赖；`cargo build`（由 tauri 驱动）；开发 `pnpm tauri dev`；打包 `pnpm tauri build`；版本锁基于 pnpm-lock.yaml 与 Cargo.lock。
  3. 服务端口与健康检查路径：固定值：无服务端口、无健康检查路径（本地单机应用）；「可运行性自检」改为：启动后单实例检查通过、SQLite 可打开、托盘图标出现。
  4. 目标容器平台与部署目录：固定值：无容器；目标平台 Windows 10/11 x64；安装目录 %LOCALAPPDATA%/prompt-pad；数据目录 %APPDATA%/prompt-pad（含 SQLite、memory/ 目录、日志、ONNX 模型文件）；分发渠道 GitHub Releases（NSIS 安装包 .exe，可选 .msi）。
- **预期输出**：完整 tauri.conf.json 的 bundle 段配置（NSIS 目标、图标、安装模式、卸载行为）、应用标识与版本号段、资源打包清单（ONNX 模型、默认 memory.md 模板）、构建与发布说明（含产物校验、非管理员安装方式），并给出安装包体积预估与打包缓存策略说明。
- **约束条件**：
  - 必须使用 `pnpm tauri build` 的 release 构建，产物不得包含调试符号与源码映射（关闭 debug 符号、前端 sourcemap 不入包）。
  - 必须以普通用户权限安装（NSIS per-user 模式），不得要求管理员权限。
  - 必须同时在 tauri.conf.json 声明 productName、version、identifier 与 bundle.targets = ["nsis"]，缺一不可。
  - 资源排除清单至少 8 条排除项，须含 .git、node_modules、.env、测试与文档目录、src-tauri/target 的中间产物。
  - 依赖安装必须基于 pnpm-lock.yaml 与 Cargo.lock，版本不得写为 latest 或浮动区间（输入已声明浮动除外）。
  - ONNX 模型（multilingual-e5-small ~117MB）的打包策略必须二选一并写明理由：随包打入（体积换离线可用）或首次启动下载（需进度与失败重试）。
  - 不得编造输入中未出现的环境变量、目录或依赖；缺失项以 `「待确认：缺失项」` 注释标注。
  - 输出中不得包含真实密钥、令牌或代码签名证书私钥；签名证书指纹用占位变量表示。

```text
适配说明：Prompt Pad 是 Windows 单机桌面应用，无 Docker/容器场景。本模板由原「Dockerfile 生成」适配为「Windows 安装包打包配置（tauri.conf.json bundle 段 + NSIS）」，产物为 GitHub Releases 分发的 .exe 安装包。

你是一名桌面应用打包工程师，负责为 Prompt Pad（Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust，Windows 10/11 x64 单机应用）生成可直接用于 GitHub Releases 分发的安装包打包配置。

## 输入材料
- 技术栈与运行方式：Tauri 2 + React 18 + TypeScript + Vite + Tailwind CSS + Rust；SQLite + FTS5 + sqlite-vec；ONNX 本地嵌入 multilingual-e5-small（~117MB）；常驻置顶 Pad，无服务端。
- 构建与启动命令：pnpm install；开发 pnpm tauri dev；打包 pnpm tauri build；锁文件 pnpm-lock.yaml 与 Cargo.lock。
- 端口与健康检查：无端口；可运行性自检=单实例检查通过 + SQLite 可打开 + 托盘图标出现。
- 目标平台：Windows 10/11 x64；安装目录 %LOCALAPPDATA%/prompt-pad；数据目录 %APPDATA%/prompt-pad；分发渠道 GitHub Releases（NSIS .exe，可选 .msi）。

## 作业步骤
1. 识别构建与产物分界：前端 dist/ 由 Vite 产出，Rust 由 cargo release 编译，tauri build 合成最终安装包；安装包只保留运行产物。
2. 配置 tauri.conf.json：productName "Prompt Pad"、identifier（反向域名格式）、version、bundle.active=true、bundle.targets=["nsis"]、图标清单、windows 段窗口配置（transparent:true、decorations:false、alwaysOnTop:true、skipTaskbar:true、resizable:false、shadow:false、focus:false）。
3. 安排 NSIS 段：per-user 安装模式（不要求管理员）、安装/卸载页语言、卸载时保留 %APPDATA%/prompt-pad 用户数据的选项、开始菜单与桌面快捷方式策略。
4. 配置资源打包：ONNX 模型文件与默认 memory.md 模板纳入 resources；写明「随包打入 vs 首启下载」二选一的决策与理由。
5. 配置版本与自更新占位：version 单一事实来源（tauri.conf.json 与 Cargo.toml 同步方式）；自更新本期不做，写明「待确认」。
6. 生成资源排除清单（打包时不得入包的目录与文件）。
7. 估算安装包体积：给出预估区间（含/不含 ONNX 模型两种情形）与主要优化点。

## 输出结构
### 1. tauri.conf.json（bundle 与窗口段）
完整相关段内容，代码块用 4 空格缩进，不要使用围栏语法；关键字段后附行内注释说明目的。

### 2. 资源排除清单
每行一个排除项，按目录/文件分类。

### 3. 构建与发布命令
pnpm tauri build（带目标）一条；GitHub Releases 上传产物命令一条（gh release create，tag 用占位变量）。

### 4. 分层与缓存策略
逐层说明：层内容（前端依赖/Rust 依赖/tauri 编译） | 是否可缓存 | 失效条件 | 优化建议。

### 5. 体积预估
预估区间 | 各构成占比（WebView2 运行时依赖说明、ONNX 模型、Rust 二进制、前端 dist） | 主要优化点。

### 6. 待确认项
需要确认的代码签名证书、自更新方案、ONNX 模型分发方式。

## 交付要求
- 配置必须自洽可解析（JSON5/JSONC 允许注释），不得留空字段或占位残缺。
- 最终回复只包含上述六节内容，不附加打包原理讲解或选型对比。
```

**来源**：zh-CN/README.md · 「Dockerfile 生成」；已按 Prompt Pad 项目适配为 Windows 安装包打包

### OPS-02 · GitHub Actions 流水线配置（cargo test + pnpm build + tauri build + 发布 Release）

- **适用阶段**：部署运维 · 流水线
- **输入要求**：
  1. CI/CD 工具与代码托管平台：固定值：GitHub Actions + GitHub 仓库（git 版本控制）。
  2. 技术栈与 lint/test/build/发布命令：固定值：lint 段 `pnpm lint`（ESLint）+ `cargo clippy -- -D warnings` + `cargo fmt --check`；test 段 `pnpm test`（Vitest + React Testing Library）+ `cargo test`；build 段 `pnpm tauri build`（windows-latest  runner，NSIS 产物）；发布段打 tag（v*）触发，tauri-action 上传 .exe/.msi 到 GitHub Releases。
  3. 目标部署平台与凭据注入方式：固定值：部署目标=GitHub Releases（本地打包分发，无服务器部署）；凭据=GITHUB_TOKEN（actions 内置，权限 contents: write 仅限 release job）；可选代码签名证书通过 GitHub Secrets 注入。
  4. 失败通知渠道与接收人：固定值：GitHub Actions 工作流失败状态 + GitHub 邮件通知个人开发者本人（单人项目，无 IM 告警群）。
- **预期输出**：一份可直接落盘的 .github/workflows/ 工作流文件（lint → test → build → release），含并发控制、依赖缓存（pnpm store + cargo registry/target）、构建产物传递、Release 触发、失败告警；另附流水线参数与权限说明清单。
- **约束条件**：
  - 阶段顺序固定为 lint → test → build → release，test 未通过时后续阶段必须跳过。
  - 必须设置顶层 concurrency 取消同分支旧运行，并声明作业级超时时间。
  - 权限最小化：默认 contents: read，release job 单独申请 contents: write，禁止顶层写权限。
  - 密钥一律通过 GitHub Secrets 引用，不得硬编码、不得打印到日志。
  - 缓存 key 必须包含 lockfile 哈希（pnpm-lock.yaml、Cargo.lock）与操作系统标识（windows-latest），不得使用固定 key。
  - 必须包含失败告警步骤，写明通知对象（个人开发者本人）、通知内容（失败阶段、失败 job 链接）与重试次数。
  - 触发策略：push 到 main 与 PR 跑 lint+test；打 v* tag 跑全流程并发布 Release。
  - 不得编造输入中未出现的 Secret 名称、目标平台或命令；缺失项列入「待确认项」。

```text
适配说明：Prompt Pad 无服务器端部署，「deploy」阶段适配为「release」：tauri-action 将 pnpm tauri build 产出的 NSIS 安装包上传到 GitHub Releases，即完成分发。

你是一名 DevOps 工程师，负责基于 GitHub Actions 为 Prompt Pad（Tauri 2 + React 18 + Rust 的 Windows 桌面应用）搭建 lint → test → build → release 的完整流水线。

## 输入材料
- 工具与平台：GitHub Actions + GitHub 仓库，runner 为 windows-latest。
- 阶段命令：lint：pnpm lint + cargo clippy -- -D warnings + cargo fmt --check；test：pnpm test（Vitest）+ cargo test；build：pnpm tauri build（NSIS 产物）；release：打 v* tag 触发，tauri-action 上传 .exe/.msi 到 GitHub Releases。
- 部署目标与凭据方式：GitHub Releases；凭据 GITHUB_TOKEN（release job 单独 contents: write）；代码签名证书走 GitHub Secrets（名称待确认）。
- 通知渠道与接收人：GitHub Actions 失败状态 + GitHub 邮件通知个人开发者本人。

## 作业步骤
1. 定义触发事件（push 到 main、PR、tag v*）与工作流并发策略：同分支新运行取消旧运行。
2. 编排阶段顺序：lint → test → build → release，每个阶段独立 job 并显式声明 needs 依赖关系，test 失败即中止；push/PR 只跑 lint+test，tag v* 跑全流程。
3. 配置依赖安装与缓存：pnpm store 缓存 key=pnpm-lock.yaml 哈希+windows-latest；cargo 缓存 key=Cargo.lock 哈希+windows-latest；缓存命中与否在日志中可辨识。
4. 配置最小权限：顶层 permissions: contents: read，release job 单独声明 contents: write。
5. 配置构建产物传递（actions/upload-artifact 与 download-artifact），使 release 阶段消费 build 产物而非重复构建。
6. 配置 release 步骤：凭据使用 GITHUB_TOKEN，命令取自上述阶段命令（tauri-action）。
7. 配置失败告警：失败时输出失败阶段、失败 job 链接、重试次数（1 次），接收人个人开发者本人。
8. 设定各 job 的 timeout-minutes（lint 10、test 20、build 45、release 15，可按需调整并标注理由）。

## 输出结构
### 1. 工作流文件
完整内容，代码块用 4 空格缩进，不要使用围栏语法；注释中标注建议保存路径 .github/workflows/release.yml。
### 2. 参数与权限说明
参数/secret 名称 | 用途 | 读取位置 | 作用域
### 3. 失败告警说明
触发条件 | 通知渠道 | 接收人 | 重试次数 | 通知内容要素
### 4. 待确认项
缺失的 secret 名称（如代码签名证书）、平台命令或权限范围。

## 交付要求
- 文件可直接保存运行，不得残留未填的占位标记或空 job。
- 最终回复只包含上述四节内容，不附加流水线原理讲解。
```

**来源**：zh-CN/README.md · 「CI/CD 流水线配置」；已按 Prompt Pad 项目适配为 GitHub Actions + GitHub Releases

### OPS-03 · 上线前检查清单

- **适用阶段**：部署运维 · 上线准备
- **输入要求**：
  1. 本次上线范围与版本号：固定值：Prompt Pad v0.1.0 Windows 首版，含实施路线 8 步全量（透明窗口骨架、常驻 DOT、动画+边缘自适应、输入框+模型改写、SQLite 落库、热键+托盘+自启、检索层、夜间记忆沉淀）。
  2. 目标环境与变更窗口：固定值：个人开发者 Windows 11 主力机真机 + GitHub Releases 发布页；变更窗口由个人开发者自定（单人项目无冻结窗口）。
  3. 依赖服务与外部对接方：固定值：大模型端点三选一（MiniMax API / OpenAI 兼容端点 / Ollama 本地模型，由最终用户自配）、GitHub Releases（托管安装包）、WebView2 运行时（Windows 11 自带，Windows 10 需引导安装）、ONNX 模型文件（随包或首启下载）。
  4. 回滚与值班责任人角色：固定值：个人开发者本人（兼开发与值班）；AI 编码 agent 负责执行修复。
- **预期输出**：可勾选的 Markdown 任务清单，7 大类：配置与密钥、数据迁移与备份、监控告警就绪、性能与容量、灰度与回滚预案、合规与安全、运维手册与责任人；每项含检查方法、判定标准、不通过时的处理动作、责任人角色。
- **约束条件**：
  - 必须覆盖上述 7 大类，每类至少 3 项，总条目不少于 24 项。
  - 每项必须同时写明「检查方法」与「不通过时的处理动作」，两栏缺一不可。
  - 每项必须指定责任人角色；本项目固定为「个人开发者本人」或「AI 编码 agent」，禁止编造具体人名。
  - 涉及用户数据的项（SQLite 库、memory/ 目录、prompt_engineering.md）必须包含备份可恢复性验证与回滚时的数据处理说明。
  - 必须包含 Prompt Pad 特有检查项：记忆数据目录 %APPDATA%/prompt-pad 升级前备份、memory/ 目录 git 每日提交已启用、ONNX 模型文件完整性、WebView2 运行时可探测、卸载保留用户数据选项可用。
  - 不得假设用户机器的具体配置值、账号或数据规模；未提供的以 `「待确认：缺失项」` 标注。
  - 必须单列「上线阻塞项」小节，列出未通过项的阻塞级别与最晚处理时间点。
  - 清单总条目不超过 60 项，超出时按优先级合并同类项。

```text
你是一名发布经理，负责产出 Prompt Pad v0.1.0（Windows 首版，实施路线 8 步全量）在个人 Windows 11 真机 + GitHub Releases 发布的检查清单。

## 输入材料
- 上线范围与版本：Prompt Pad v0.1.0，含透明窗口骨架、常驻 DOT、动画+边缘自适应、输入框+模型改写、SQLite 落库、热键+托盘+自启、检索层、夜间记忆沉淀。
- 目标环境与窗口：个人 Windows 11 主力机真机 + GitHub Releases 发布页；单人项目无冻结窗口。
- 依赖服务与对接方：MiniMax API / OpenAI 兼容端点 / Ollama（用户自配三选一）、GitHub Releases、WebView2 运行时、ONNX 模型文件（multilingual-e5-small）。
- 回滚与值班责任角色：个人开发者本人；修复执行者 AI 编码 agent。

## 作业步骤
1. 依据上线范围确定检查重点：本次涉及的窗口行为、记忆数据文件（memory.md、prompt_engineering.md）、SQLite 库、tauri.conf.json 配置、模型端点配置项。
2. 依据依赖列出需要确认连通性与版本兼容性的对象：模型端点三选一各自的连通性自检、GitHub Releases 可上传、WebView2 可探测、ONNX 模型可加载。
3. 按七个类别逐项编写检查项：配置与密钥、数据迁移与备份、监控告警就绪、性能与容量、灰度与回滚预案、合规与安全、运维手册与责任人。
4. 每项写明：检查方法（可执行的动作或命令，如 pnpm tauri build、sqlite3 查询、真机手动验证步骤）、判定标准、不通过时的处理动作、责任人角色（个人开发者本人 / AI 编码 agent）。
5. 必含项目特有项：升级前备份 %APPDATA%/prompt-pad（SQLite + memory/）；memory/ 目录 git 每日提交已启用；卸载安装时「保留用户数据」选项验证；ONNX 模型文件校验（大小/哈希）；旧版本安装包回退重装验证；热键 Ctrl+Space 冲突提示验证。
6. 汇总未通过或未确认的项，形成「上线阻塞项」小节，标注阻塞级别与最晚处理时间点。

## 输出结构
### 1. 检查清单
按七类分节，使用可勾选任务列表，每项一行写全：`- [ ] 检查项 | 检查方法 | 判定标准 | 不通过处理 | 责任人`
### 2. 上线阻塞项
检查项 | 阻塞级别 | 最晚处理时间点 | 责任人角色 | 阻塞影响
### 3. 变更影响面
模块/接口 | 变更内容 | 需通知的依赖方 | 通知方式
### 4. 待确认项
缺失的环境信息、责任分工或对接方信息。

## 交付要求
- 条目总数不少于 24 项、每类不少于 3 项。
- 最终回复只包含上述四节内容，不附加风险提示长文或背景说明。
```

**来源**：新增（流程整合）；已按 Prompt Pad 项目填充

### OPS-04 · 上线验证与冒烟测试

- **适用阶段**：部署运维 · 上线验证
- **输入要求**：
  1. 上线后访问地址与端点清单：固定值：无远端端点（单机应用）；验证对象为本机安装后的 Prompt Pad 可执行文件（%LOCALAPPDATA%/prompt-pad 下 .exe）、系统托盘图标、数据目录 %APPDATA%/prompt-pad（SQLite、memory/、logs/、models/）。
  2. 冒烟需覆盖的关键用户路径：固定值：①NSIS 安装 → 启动 → 托盘图标出现（单实例生效）；②开机自启生效；③Ctrl+Space 唤起 DOT/面板 → 动画完成可输入；④输入提示词 → 所选模型端点返回改写结果；⑤一键复制到剪贴板；⑥点击外部收起、收起态点击穿透；⑦重启应用后 DOT 位置按「显示器+屏内百分比」恢复；⑧一条 PE 记录落库（PE-YYYY-NNN）且记忆检索返回或明确提示「暂无历史经验」。
  3. 依赖组件与预期响应：固定值：SQLite 文件可打开且四表存在（entries / entries_fts / entry_vectors / usage_log）；memory.md 与 prompt_engineering.md 存在且为合法 Markdown；ONNX 模型文件存在且可加载；模型端点三选一配置连通；日志文件在 %APPDATA%/prompt-pad/logs 滚动生成。
  4. 回滚触发条件与回滚命令：固定值：触发条件=任一 P0 冒烟用例失败或出现崩溃/数据丢失；回滚命令=卸载新版 → 安装 GitHub Releases 上的上一版 NSIS 安装包 → 如需代码层回退则 `git revert` 后重新 `pnpm tauri build`；用户数据目录不回滚（只读校验）。
  5. 单步超时时间：固定值：30 秒（本地操作；模型端点调用步骤放宽至 60 秒并单列）。
- **预期输出**：可执行的冒烟脚本（PowerShell 或 Python，本地执行）、冒烟用例表（步骤、预期输出、通过判据、失败处置）、一条端到端主链路验证用例（安装→唤起→改写→复制→落库）与执行说明。
- **约束条件**：
  - 必须产出可直接执行的脚本（含运行命令），不得只给伪代码；窗口行为断言无法脚本化的，用「人工确认提示 + 输入 y/n」交互式步骤实现。
  - 冒烟用例表至少 8 条，每条含明确断言（进程存在、文件存在、表存在、退出码、人工确认结果）。
  - 必须包含 1 条端到端主链路验证，串联至少 3 个环节（安装→唤起→改写→复制→落库至少选 3 个）并校验最终业务结果（PE 记录入库）。
  - 每条用例必须写明失败处置动作，并引用回滚命令（卸载新版→装旧版安装包 / git revert 重打包）。
  - 脚本必须处理超时与连接失败，单步超时不超过 30 秒（模型端点步骤 60 秒）。
  - 不得编造输入中未出现的接口路径、鉴权方式或响应字段；缺失的以 `「待确认：缺失项」` 标注。
  - 脚本中不得写入真实 API Key 或令牌，一律引用环境变量名（如 MINIMAX_API_KEY、OPENAI_COMPAT_API_KEY）。

```text
你是一名值班工程师，负责在 Prompt Pad v0.1.0（Windows 单机 Tauri 2 应用，NSIS 安装包）发布后执行冒烟验证并给出明确的通过/失败判定。

## 输入材料
- 端点清单：无远端端点；验证对象为本机 %LOCALAPPDATA%/prompt-pad 下的 .exe、系统托盘图标、数据目录 %APPDATA%/prompt-pad（SQLite、memory/、logs/、models/）。
- 关键用户路径：①安装→启动→托盘出现（单实例）；②开机自启；③Ctrl+Space 唤起→可输入；④输入提示词→模型端点返回改写；⑤复制到剪贴板；⑥点外部收起+收起态点击穿透；⑦重启后位置按「显示器+屏内百分比」恢复；⑧PE 记录落库且记忆检索返回或提示「暂无历史经验」。
- 依赖组件与预期响应：SQLite 四表存在（entries / entries_fts / entry_vectors / usage_log）；memory.md 与 prompt_engineering.md 为合法 Markdown；ONNX 模型可加载；所选模型端点连通；logs/ 滚动生成。
- 回滚命令：卸载新版 → 安装 GitHub Releases 上一版 NSIS 安装包；代码层回退用 git revert 后 pnpm tauri build 重打包；用户数据目录不回滚。
- 单步超时：30 秒；模型端点调用步骤 60 秒。

## 作业步骤
1. 依据关键用户路径设计冒烟用例：每条路径至少 1 条，窗口行为类（唤起动画、点击穿透、位置恢复）写成脚本暂停 + 人工确认 y/n 的交互步骤。
2. 为每条用例定义断言：进程存在、文件/目录存在、SQLite 表存在、脚本退出码、人工确认结果。
3. 用依赖组件清单校验：sqlite3 查四表、模型文件存在性与大小、日志文件增长。
4. 编写可执行脚本：PowerShell 或 Python，统一处理超时、连接失败、非预期结果三类异常并以非零退出码结束。
5. 编排一条端到端主链路用例：安装 → Ctrl+Space 唤起 → 输入改写 → 复制 → 检查 PE 记录入库，至少串联 3 个环节。
6. 为每条用例写失败处置动作：指向回滚命令（装旧版安装包 / git revert 重打包）或明确的修复动作。

## 输出结构
### 1. 冒烟用例表
用例编号 | 步骤 | 预期输出 | 通过判据 | 失败处置 | 端到端（是/否）

### 2. 冒烟脚本
完整脚本内容，代码块用 4 空格缩进，不要使用围栏语法。含运行命令、超时处理、退出码语义与逐项结果输出；API Key 一律读环境变量（MINIMAX_API_KEY / OPENAI_COMPAT_API_KEY）。

### 3. 端到端主链路用例
链路环节 | 环节间数据传递 | 最终校验点（PE-YYYY-NNN 入库） | 失败可能位置

### 4. 执行说明
运行命令 | 前置依赖 | 所需环境变量 | 预期总耗时 | 退出码含义

### 5. 待确认项
缺失的地址、鉴权方式或响应字段。

## 交付要求
- 脚本直接运行即可，不依赖本文件之外的私有工具（sqlite3、PowerShell 5.1+、Python 3 任选并写明）。
- 最终回复只包含上述五节内容，不附加说明性文字。
```

**来源**：zh-CN/README.md · 「Vibe Coding 的 12 条规则」第 8 条；已按 Prompt Pad 项目填充

### OPS-05 · 本地日志与崩溃监控配置

- **适用阶段**：部署运维 · 可观测性
- **输入要求**：
  1. 技术栈与现有监控/日志组件：固定值：无远端监控/告警组件（单机应用）；Rust 端 tracing/log 落盘到 %APPDATA%/prompt-pad/logs（按日轮转，单文件上限 10MB，保留 7 天）；panic 时写崩溃转储文件到 %APPDATA%/prompt-pad/crash；前端 WebView2 console 错误经 Tauri 事件转发写入同一日志。
  2. 关键业务与接口清单：固定值：热键唤起/收起、面板展开动画、模型改写调用（MiniMax API / OpenAI 兼容端点 / Ollama）、记忆检索与注入组装（≤800 token 裁剪）、夜间记忆沉淀任务、SQLite 读写、ONNX 嵌入推理、剪贴板读写、开机自启注册。
  3. 告警接收人与通知渠道：固定值：个人开发者本人；无 IM/邮件告警通道——「告警」= 应用内「诊断」页展示最近错误 + 日志文件 + 崩溃转储；严重崩溃在下次启动时弹窗提示并引导查看转储。
  4. 已知容量上限与历史峰值：固定值：memory.md ≤800 token 硬上限；阶段一记忆记录 <50 条、阶段二 50-300 条；ONNX 模型 multilingual-e5-small ~117MB/384 维；日志目录上限 70MB（10MB×7 天）；常驻内存目标 ≤150MB（建议值，待基准校准）。
- **预期输出**：日志规范表（级别、必含字段、脱敏项）、关键指标与告警阈值建议表、告警分级与接收人映射表、日志检索语句示例、值班处置流程。
- **约束条件**：
  - 日志级别至少定义 5 级（ERROR/WARN/INFO/DEBUG/TRACE），每级给出使用场景与输出示例。
  - 结构化日志必含字段不少于 6 个（时间、级别、会话 ID、模块、耗时、结果码）。
  - 必须显式列出脱敏字段清单（不少于 5 类：API Key、用户提示词原文、改写结果原文、剪贴板内容、模型端点 base_url 中的内网地址）并给出脱敏后的输出格式；提示词类内容默认只记哈希与长度，不记原文。
  - 指标表不少于 10 项，每项含指标名、类型（Counter/Gauge/Histogram）、阈值、告警级别；指标采集位置为本应用日志打点，非远端时序库。
  - 必须给出至少 3 条日志检索语句示例（错误定位、慢操作、夜间任务维度），语法为 PowerShell Select-String 或 Python 读取 JSON 行。
  - 告警分级为三级（紧急/重要/一般），每级明确接收人角色（个人开发者本人）、通知渠道（启动弹窗/诊断页/仅日志）与响应时限。
  - 阈值必须结合上述容量基线；无基线的写「需按基准测试结果校准」，不得直接给出生产数值。
  - 不得编造输入中未出现的指标名、面板 ID 或接收人联系方式。

```text
适配说明：Prompt Pad 是单机桌面应用，无服务端监控告警体系。本模板由原「监控、日志与告警配置」适配为「本地日志轮转 + 崩溃转储 + 应用内诊断页」，「告警」落地为启动弹窗与诊断页展示，接收人固定为个人开发者本人。

你是一名桌面应用可观测性工程师，负责为 Prompt Pad 的关键路径（热键唤起、模型改写、记忆检索注入、夜间任务、SQLite、ONNX 推理）制定可直接落地的本地日志、指标与「告警」配置。

## 输入材料
- 技术栈与现有组件：Rust tracing/log 落盘 %APPDATA%/prompt-pad/logs（按日轮转、单文件 10MB、保留 7 天）；panic 崩溃转储到 %APPDATA%/prompt-pad/crash；前端 console 错误经 Tauri 事件并入同一日志；无远端组件。
- 关键接口与业务：热键唤起/收起、面板动画、模型改写调用（三选一端点）、记忆检索+注入（≤800 token 裁剪）、夜间沉淀任务、SQLite 读写、ONNX 嵌入、剪贴板读写、自启注册。
- 告警接收人与渠道：个人开发者本人；渠道=启动弹窗（紧急）/应用内诊断页（重要）/仅日志（一般）。
- 容量上限与历史峰值：memory.md ≤800 token；记忆记录阶段一 <50 条；ONNX 模型 ~117MB；日志目录上限 70MB；常驻内存目标 ≤150MB（建议值，待基准校准）。

## 作业步骤
1. 依据上述组件确定日志输出格式为结构化 JSON 行（按日轮转文件），给出 Rust 端与前端转发两层的接入位置。
2. 定义日志级别（ERROR/WARN/INFO/DEBUG/TRACE）与使用场景，明确每级触发条件。
3. 定义结构化日志必含字段，并在写入前对敏感字段脱敏：API Key、提示词原文、改写原文、剪贴板内容、端点内网地址——默认只记 SHA256 前 8 位与长度。
4. 依据关键路径列出本地指标（如唤起耗时 Histogram、改写耗时 Histogram、检索命中数 Counter、夜间任务耗时与处理条数、裁剪丢弃条数、内存占用 Gauge），配套阈值建议。
5. 结合容量基线给出阈值：容量类以基线为参照并留余量；无基线的标注「需按基准测试结果校准」。
6. 定义三级告警：紧急（崩溃/数据丢失→启动弹窗+转储）、重要（改写连续失败/夜间任务失败→诊断页红点）、一般（慢操作→仅日志），写明响应时限。
7. 编写常用日志检索语句（PowerShell Select-String 或 Python 解析 JSON 行）：错误定位、慢操作、夜间任务维度各至少 1 条。
8. 编写值班处置流程：确认、定位、处置、升级、关闭（单人项目：升级=暂停使用+git issue 记录）。

## 输出结构
### 1. 日志规范
级别 | 使用场景 | 必含字段 | 输出示例

### 2. 结构化字段与脱敏
必含字段表（字段 | 含义 | 类型 | 示例）；脱敏清单（字段 | 脱敏方式 | 脱敏后示例）。

### 3. 关键指标表
指标名 | 类型 | 含义 | 采集位置（日志打点） | 阈值 | 告警级别 | 阈值依据

### 4. 告警分级与接收人
级别 | 触发示例 | 接收人角色（个人开发者本人） | 通知渠道（启动弹窗/诊断页/仅日志） | 响应时限 | 升级路径

### 5. 日志检索语句示例
按场景分组：错误定位、慢操作、夜间任务维度，每条给出可粘贴的查询语句与用途说明。

### 6. 值班处置流程
触发 → 确认 → 定位 → 处置 → 升级 → 关闭，每步写明动作、时限与责任人角色。

### 7. 待确认项
缺失的指标数据源、基线或接收人。

## 交付要求
- 检索语句必须使用 PowerShell Select-String 或 Python 实际语法，不得引用不存在的远端查询组件。
- 最终回复只包含上述七节内容，不附加可观测性原理讲解。
```

**来源**：zh-CN/README.md · 「CI/CD 流水线配置」+ 新增（流程整合）；已按 Prompt Pad 项目适配为本地日志与崩溃监控

### OPS-06 · 回滚预案

- **适用阶段**：部署运维 · 应急
- **输入要求**：
  1. 本次变更内容与版本区间：固定值：调用本模板时由使用者填写（格式：Prompt Pad v<旧版本> → v<新版本>，附 git diff --stat 与变更模块清单）。
  2. 可回退的版本或制品：固定值：GitHub Releases 上的历史 NSIS 安装包（.exe/.msi，按 tag v* 归档）+ git 仓库对应 tag（git revert 后 pnpm tauri build 可重新产出旧制品）。
  3. 数据变更与迁移情况：固定值：用户数据位于 %APPDATA%/prompt-pad——SQLite（entries / entries_fts / entry_vectors / usage_log，索引层可重建）、memory.md、prompt_engineering.md（L1 永久逐字不可改）；memory/ 目录已纳入 git 每日提交；schema 迁移仅允许加列/加表，禁止破坏性变更。
  4. 监控指标与告警入口：固定值：本地日志 %APPDATA%/prompt-pad/logs + 崩溃转储 %APPDATA%/prompt-pad/crash + 应用内诊断页；无远端监控。
- **预期输出**：回滚决策树（触发条件 → 判定 → 动作）、代码/配置/数据三类回滚步骤、数据回滚与向前修复判定表、恢复验证清单、复盘要点。
- **约束条件**：
  - 决策树每个叶子节点必须对应明确动作（回滚 / 向前修复 / 持续观察）与责任角色（个人开发者本人 / AI 编码 agent）。
  - 每条触发条件必须绑定至少一个可观测信号（崩溃转储出现、日志 ERROR 计数、诊断页报错、冒烟用例失败）。
  - 代码、配置、数据三类回滚必须分节给出，每节含可粘贴命令或操作步骤；代码回滚=git revert + pnpm tauri build 重打包，制品回滚=安装旧版 NSIS 安装包。
  - 「数据回滚 vs 向前修复」判定表至少 4 行，含判定条件与代价；须覆盖 memory.md 污染（用 memory/ 目录 git 历史精确回滚单条规则）、SQLite 损坏（删库重建索引+L1 重放）、prompt_engineering.md 误改（逐字不可改，只能从 git/备份恢复）等场景。
  - 恢复验证清单不少于 6 项，须包含业务主链路可用性（唤起→改写→复制）与数据一致性校验（PE 记录数、记忆条目数、FTS5 检索一致）。
  - 不得假设用户机器的具体配置值、备份路径（除 %APPDATA%/prompt-pad 及其 git 仓库）或数据规模；未提供的以 `「待确认：缺失项」` 标注。
  - 若变更含不可逆数据操作（如 L1 改写、schema 破坏性变更），必须在文档顶部风险提示中显式标出并要求先验证备份可恢复。

```text
适配说明：Prompt Pad 是单机桌面应用，回滚=安装包版本回退 + git revert 重打包；数据回滚依托 memory/ 目录 git 每日提交与 %APPDATA%/prompt-pad 备份；无服务端回滚、无数据库迁移回滚脚本之外的概念。

你是一名应急负责人，负责为 Prompt Pad 的版本变更（调用时填写的 v<旧版本> → v<新版本>）制定可直接执行的回滚预案。

## 输入材料
- 变更内容与版本区间：调用本模板时由使用者填写，附 git diff --stat 与变更模块清单。
- 可回退版本/制品：GitHub Releases 历史 NSIS 安装包（按 v* tag 归档）+ git 仓库 tag（git revert + pnpm tauri build 可重产旧制品）。
- 数据变更情况：%APPDATA%/prompt-pad 下 SQLite（entries / entries_fts / entry_vectors / usage_log，索引可重建）、memory.md、prompt_engineering.md（逐字不可改）；memory/ 目录 git 每日提交；schema 迁移仅允许加列/加表。
- 监控与告警入口：本地日志 logs/ + 崩溃转储 crash/ + 应用内诊断页。

## 作业步骤
1. 依据使用者填写的变更判断影响面，识别不可逆操作（L1 改写、破坏性 schema 变更、记忆批量晋升）并在顶部风险提示中标出。
2. 定义回滚触发条件，每条绑定可观测信号：crash/ 出现新转储、日志 ERROR 计数超阈值、诊断页报错、冒烟主链路失败、PE 记录数/记忆条目数异常变化。
3. 构建决策树：从触发条件出发逐级判定，叶子节点落到具体动作（回滚 / 向前修复 / 持续观察）与责任角色（个人开发者本人 / AI 编码 agent）。
4. 分写三类回滚步骤：代码/制品（git revert → pnpm tauri build → 或直接安装旧版 NSIS 包）、配置（tauri.conf.json 与模型端点配置，取值来源 git 历史）、数据（memory.md 用 memory/ git 历史回滚单条；SQLite 损坏则删索引库重建并从 L1 重放；prompt_engineering.md 仅从 git/备份恢复）；每步给出命令、执行顺序与验证方式。
5. 建立「数据回滚 vs 向前修复」判定表（至少 4 行：memory.md 污染、SQLite 索引损坏、SQLite 数据损坏、L1 误改），列出可回滚性、判定条件、代价与决策人。
6. 编写恢复验证清单（≥6 项）：服务健康（启动+托盘）、主链路（唤起→改写→复制）、数据一致性（PE 记录数、记忆条目数、FTS5 检索结果与回滚前一致）、memory.md ≤800 token 校验、夜间任务重跑幂等。
7. 编写复盘要点：时间线（发现/决策/执行/恢复）、根因、改进项。

## 输出结构
### 0. 风险提示
不可逆操作清单 | 最坏影响 | 触发前必须完成的前置动作（备份 %APPDATA%/prompt-pad 并验证可恢复）

### 1. 回滚决策树
用缩进列表表示：触发条件（信号 + 阈值）→ 判定分支 → 动作 → 责任角色 → 时限。

### 2. 代码/制品回滚步骤
有序步骤，每步含命令（git revert / pnpm tauri build / 安装旧版 NSIS 包）、执行位置、验证方式。

### 3. 配置回滚步骤
有序步骤，每步含操作项、回滚取值来源（git 历史/版本库）、验证方式。

### 4. 数据回滚步骤
有序步骤，含备份位置（%APPDATA%/prompt-pad 备份 + memory/ git 仓库）、回滚方式、执行窗口与风险提示。

### 5. 数据回滚 vs 向前修复判定表
场景 | 可回滚性 | 判定条件 | 选择回滚的条件 | 选择向前修复的条件 | 代价 | 决策人

### 6. 恢复验证清单
验证项 | 验证方式 | 预期结果 | 责任角色

### 7. 复盘要点
时间线（发现/决策/执行/恢复） | 根因 | 改进项 | 责任人角色

### 8. 待确认项
缺失的备份位置、制品版本或阈值。

## 交付要求
- 所有命令可粘贴执行，敏感值用占位变量表示（如 <版本号>、<提交哈希>）。
- 最终回复只包含上述小节内容，不附加应急理论讲解。
```

**来源**：zh-CN/README.md · 「Vibe Coding 的 12 条规则」第 9 条 + zh-CN/AI-Assisted-Coding.md · 「我的 2026 年 AI 编程工作流」第 5 步；已按 Prompt Pad 项目适配

### OPS-07 · 操作手册与 SOP 生成

- **适用阶段**：部署运维 · 交付文档
- **输入要求**：
  1. 系统功能与角色划分：固定值：Prompt Pad 功能——常驻 DOT（40-56px、拖拽、置顶不抢焦点）、Ctrl+Space 唤起/收起、提示词一键改写（MiniMax API / OpenAI 兼容端点 / Ollama 三选一）、改写结果一键复制、私人记忆库（自动沉淀、memory.md 可查看手改）、托盘菜单（显示/隐藏/退出/设置）、开机自启；角色划分——管理员=个人开发者本人（安装维护、模型端点配置、记忆库裁决与 git 管理、故障排查）；普通用户=最终使用者（日常唤起、改写、复制、查看记忆）。
  2. 目标读者与使用场景：固定值：管理员册读者=个人开发者本人，场景=安装升级、端点配置、记忆库维护（夜间任务结果审查、矛盾条目人工裁决、memory/ git 提交）、日志与崩溃排查；普通用户册读者=终端用户，场景=每日高频唤起改写复制。
  3. 界面截图或页面路径：固定值：调用时由使用者补充截图，建议目录 docs/screenshots/；页面/状态清单：DOT 收起态、面板展开态、设置页（模型端点三选一配置）、诊断页、托盘菜单、记忆查看页。
  4. 常见问题与业务规则：固定值：Ctrl+Space 被占用（设置页改键）、独占全屏程序下 Pad 临时隐藏（退出全屏即恢复）、记忆未生效（检查条目 status 是否 active、是否超 800 token 被裁剪）、模型端点失败（检查 API Key 与网络，Ollama 需确认本地服务已启动）、多显示器位置异常（重置为「主屏+默认百分比坐标」）、无匹配记忆时提示「暂无历史经验」为正常行为。
- **预期输出**：按角色分册的 Markdown 手册（管理员册 + 普通用户册），每步含入口路径、操作动作、预计耗时、完成标志、预期结果、常见错误与处置、截图占位；附术语表、联系方式与 SOP 模板结构说明。
- **约束条件**：
  - 必须分为管理员册与普通用户册两册，章节按角色独立编号。
  - 每个操作步骤必须含：入口路径、操作动作、预计耗时、完成标志、预期结果、常见错误与处置、截图占位。
  - 普通用户册不得出现部署、配置、命令行等运维操作内容（模型端点选择可保留在普通用户册，但 API Key 获取与排查归管理员册）。
  - 管理员册不超过 40 步，普通用户册不超过 30 步。
  - 截图占位统一写为 `> [截图 N：<画面说明>]`，N 在全册内连续编号。
  - 术语表不少于 10 条，每条含术语、英文或英文缩写、解释；须含 DOT、L1/L2/L3、candidate/active/deprecated、FTS5、sqlite-vec、ONNX、NSIS、PE 记录、token 预算。
  - 不得编造输入中未出现的界面元素、按钮名称、菜单路径或联系方式；缺失的以 `「待确认：缺失项」` 标注。

```text
你是一名技术文档工程师，负责为 Prompt Pad（Windows 常驻置顶提示词优化 Pad）的两类读者编写可直接交付的操作手册与 SOP。

## 输入材料
- 系统功能与角色：功能=常驻 DOT、Ctrl+Space 唤起、一键改写（MiniMax API / OpenAI 兼容端点 / Ollama 三选一）、一键复制、私人记忆库（自动沉淀、memory.md 可查可改）、托盘菜单、开机自启；角色=管理员（个人开发者本人：安装升级、端点配置、记忆库裁决与 git 管理、故障排查）与普通用户（日常唤起改写复制）。
- 读者与场景：管理员册=个人开发者本人（维护场景）；普通用户册=终端用户（每日高频使用）。
- 截图与页面路径：调用时补充截图至 docs/screenshots/；状态清单：DOT 收起态、面板展开态、设置页、诊断页、托盘菜单、记忆查看页。
- 常见问题与业务规则：Ctrl+Space 被占用→设置页改键；独占全屏下临时隐藏→退出全屏恢复；记忆未生效→查 status=active 与 800 token 裁剪；模型端点失败→查 API Key/网络/Ollama 本地服务；多显示器位置异常→重置主屏默认坐标；提示「暂无历史经验」为正常行为。

## 作业步骤
1. 依据上述角色划分界定管理员册与普通用户册各自覆盖的操作；普通用户册不出现命令行与运维操作。
2. 把每个功能拆成有序操作步骤：管理员册含安装升级、模型端点配置、记忆库审查与人工裁决、memory/ git 提交、日志与崩溃转储排查、备份与回滚；普通用户册含唤起、输入、改写、复制、查看记忆、改键。
3. 为每步补齐常见错误与处置：依据上述常见问题写明错误现象、原因与用户可自行执行的处置动作。
4. 插入截图占位，格式统一为 `> [截图 N：<画面说明>]`，N 在全册连续编号。
5. 编制术语表（≥10 条，必含 DOT、L1/L2/L3、candidate/active/deprecated、FTS5、sqlite-vec、ONNX、NSIS、PE 记录、token 预算）与联系方式小节（个人项目：GitHub Issues，联系人角色=个人开发者本人）。

## 输出结构
### 1. 文档说明
适用版本 | 目标读者 | 使用方式 | 更新记录表（版本 | 日期 | 变更内容 | 修订人角色）

### 2. 管理员册
按功能分章，章内每步一个小节：步骤编号 | 入口路径 | 操作动作 | 预计耗时 | 完成标志 | 预期结果 | 常见错误与处置 | 截图占位。

### 3. 普通用户册
同结构，按角色再分节，只保留业务操作。

### 4. 常见问题速查
问题现象 | 可能原因 | 处置动作 | 仍无法解决时的升级路径（GitHub Issues）

### 5. 术语表
术语 | 英文/缩写 | 解释，不少于 10 条。

### 6. 联系方式
支持渠道（GitHub Issues） | 服务时间 | 责任人角色（个人开发者本人）

### 7. SOP 模板结构说明
说明本手册的章节如何映射为标准 SOP 字段：目的 | 前置条件 | 操作步骤 | 完成判据 | 异常处置。

## 交付要求
- 每个操作步骤七要素齐全，缺项写「不适用及原因」。
- 最终回复只包含上述七节内容，不附加写作说明或教学性介绍。
```

**来源**：新增（流程整合）+ zh-CN/AI-Gen-Documentation.md · 「用 AI 创建 SOP」；已按 Prompt Pad 项目填充
