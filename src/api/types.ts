/**
 * 前端 ↔ Rust 后端的类型契约。
 *
 * 约定：
 * - 字段命名一律 snake_case，与 Rust 侧 serde 结构逐字段对应。
 * - 所有可选性用 `| null` 显式表达，避免 undefined 在 IPC 序列化后语义漂移。
 * - 这里的类型必须与 docs/interface-doc.md 的契约保持一致，改动需同步两边。
 */

/** 大模型提供方，可配。 */
export type Provider = "minimax" | "openai_compat" | "ollama" | "anthropic";

/**
 * 调用协议。
 *
 * 与 Provider 相互独立：同一个提供方可能同时暴露多种协议，
 * 例如 MiniMax 既有 OpenAI 兼容端点，也有 Anthropic 兼容端点（Token Plan 订阅走后者）。
 */
export type Protocol = "openai_chat_completions" | "anthropic_messages";

/** 记忆条目类型。 */
export type MemoryType = "rule" | "preference" | "pitfall" | "asset";

/** 记忆条目状态机：candidate → active → deprecated。 */
export type MemoryStatus = "candidate" | "active" | "deprecated";

/** 面板形态。 */
export type PanelMode = "collapsed" | "expanded";

/** toggle_panel 的动作语义，避免为「只收起」再开一个命令。 */
export type PanelAction = "toggle" | "expand" | "collapse";

/**
 * begin_dot_drag 的结果。
 *
 * 圆球既是 Tauri 拖拽区又是按钮：按下即启动系统拖拽循环并独占鼠标，
 * 浏览器收不到 mouseup / click，点击与拖拽只能由后端从系统层面区分。
 */
export type DotDragOutcome = "clicked" | "dragged";

/**
 * 面板视图：提示词 / 设置。
 *
 * 后端要靠它决定「点面板外部收起」在设置视图下是否生效 ——
 * 设置界面里用户要去别的应用复制 API key，收起面板会把填了一半的表单弄丢。
 */
export type PadView = "prompt" | "settings";

/** 面板锚点：哪块屏 + 屏内百分比坐标，非绝对像素。 */
export interface WindowAnchor {
  /** 所在显示器序号。 */
  monitor_index: number;
  /** 屏内水平百分比坐标。 */
  x_ratio: number;
  /** 屏内垂直百分比坐标。 */
  y_ratio: number;
}

/** toggle_panel 的响应。 */
export interface TogglePanelResponse {
  /** true = 已展开，false = 已收起。 */
  expanded: boolean;
  /** 本次播放的展开 / 收起动画时长。 */
  animation_ms: number;
  position: WindowAnchor;
}

/** 改写请求。 */
export interface OptimizeRequest {
  /** 原始提示词草稿。 */
  original_prompt: string;
  /** 任务域，用于记忆路由；为空时由后端推断。 */
  domain: string | null;
  /** 由 retrieve_memories 返回的注入记忆 ID；null 表示由后端自行检索。 */
  memory_ids: string[] | null;
}

/** 改写响应。 */
export interface OptimizeResponse {
  /** 改写后的提示词。 */
  optimized_prompt: string;
  provider: Provider;
  /** 本次实际使用的调用协议。 */
  protocol: Protocol;
  /** 实际生效的模型名。 */
  model_name: string;
  /** 本次实际注入的记忆条目 ID。 */
  injected_memory_ids: string[];
  /** 本次注入内容的 token 估算值。 */
  injected_token_count: number;
  /** 端到端耗时（毫秒）。 */
  elapsed_ms: number;
  /** 本次实际用于路由与落库的 domain。 */
  domain: string;
}

/** PE 事实层记录（prompt_engineering.md 的结构化投影）。 */
export interface PeRecord {
  /** 形如 PE-2026-001。 */
  id: string;
  original_prompt: string;
  optimized_prompt: string;
  domain: string;
  signals: string[];
  /** 用户评分 1-5，未评分为 null。 */
  rating: number | null;
  created_at: string;
}

/** 落库请求。 */
export interface SavePeRecordRequest {
  original_prompt: string;
  optimized_prompt: string;
  domain: string | null;
  /** 本次改写暴露的信号标签。 */
  signals: string[] | null;
  /** 可选初始评分 1-5。 */
  rating: number | null;
  /**
   * 本次改写实际注入的记忆条目 ID（来自 optimize_prompt 响应）。
   * 后端据此写入 usage_log 的 inject 事件，使后续评分能定位到被注入的条目。
   */
  injected_memory_ids: string[] | null;
}

/** 落库响应。 */
export interface SavePeRecordResponse {
  /** 形如 PE-2026-001。 */
  record_id: string;
  /** UTC RFC 3339 时间戳。 */
  created_at: string;
  /** 实际落盘到的层：只可能是 l1_markdown 与 l3_sqlite。 */
  written_layers: string[];
}

/** 记忆条目完整形态（含 evidence / weight 等统计字段，不进注入集）。 */
export interface MemoryEntry {
  id: string;
  type: MemoryType;
  domain: string;
  status: MemoryStatus;
  statement: string;
  why: string;
  how_to_apply: string;
  signals: string[];
  evidence: string[];
  weight: number;
  hits: number;
  last_used: string | null;
  created: string;
}

/** 注入集条目：只含 statement + why + how_to_apply，绝不含统计字段。 */
export interface MemoryInjectionItem {
  /** 形如 M-CODE-001。 */
  memory_id: string;
  statement: string;
  why: string;
  how_to_apply: string;
}

/** 记忆检索请求。 */
export interface RetrieveMemoriesRequest {
  domain: string | null;
  /** 检索关键词，用于阶段二 FTS5。 */
  query: string | null;
  /** Top-K 条数上限（分页步长）。 */
  top_k: number | null;
  /** 分页偏移。 */
  offset: number | null;
}

/** 记忆注入结果。 */
export interface MemoryInjection {
  /** 是否命中可用记忆；false 时 injection_set 必须为空。 */
  matched: boolean;
  /** 实际生效的检索阶段，本期固定 markdown_domain。 */
  stage: string;
  /** 未命中原因；命中时为 null。 */
  no_memory_reason: string | null;
  /** 未命中时的固定提示文案「暂无历史经验」；命中时为 null。 */
  no_memory_message: string | null;
  injection_set: MemoryInjectionItem[];
  /** 本次实际返回条数（预算裁剪后）。 */
  returned_count: number;
  /** 过滤后命中的总条数，供分页判断。 */
  total_matched_count: number;
  offset: number;
  top_k: number;
  /** offset + returned_count < total_matched_count 时为 true。 */
  has_more: boolean;
  /** 注入总量估算，必须 ≤ token_budget。 */
  total_token_count: number;
  /** 本次生效的注入预算。 */
  token_budget: number;
}

/** 反馈请求：驱动权重 +1 / 0 / −2。 */
export interface FeedbackRequest {
  record_id: string;
  /** 用户评分 1-5；≥4 记 +1，其余按规则处理。 */
  rating: number;
  /** 可选备注，仅落 usage_log 供人工复核。 */
  note: string | null;
}

/** 单条记忆的权重变化。 */
export interface WeightDelta {
  memory_id: string;
  entry_id: string;
  delta: number;
  weight_before: number;
  weight_after: number;
}

/** 反馈结果。 */
export interface FeedbackResult {
  record_id: string;
  rating: number;
  /** 本次评分实际产生的权重变更。 */
  applied_weight_deltas: WeightDelta[];
  /** UTC RFC 3339 时间戳。 */
  updated_at: string;
}

/** 夜间沉淀请求。 */
export interface RunNightlyJobRequest {
  /** 处理日期 YYYY-MM-DD；null 表示今天。 */
  run_date: string | null;
  /** true 时只计算不写盘。 */
  dry_run: boolean | null;
}

/** 夜间沉淀任务报告。 */
export interface NightlyJobReport {
  /** 处理日期 YYYY-MM-DD。 */
  run_date: string;
  /** 扫描到的当天 PE 记录数。 */
  scanned_record_count: number;
  /** 与既有记忆条目匹配成功的条数。 */
  matched_entry_count: number;
  /** 新建的 candidate 条目数。 */
  created_candidate_count: number;
  /** 因匹配到既有条目而 weight +1 的条目数。 */
  weight_bumped_count: number;
  /** 命中人工裁决闸门、需用户确认的条目数。 */
  conflicted_count: number;
  /** 挂起待人工裁决的记忆 ID。 */
  pending_review_ids: string[];
  /** 运行前的 memory.md token 占用。 */
  token_usage_before: number;
  /** 运行后的 memory.md token 占用。 */
  token_usage_after: number;
  /** 当前 memory.md 的 token 预算上限。 */
  token_budget: number;
  /** 索引是否已更新。 */
  index_rebuilt: boolean;
  started_at: string;
  finished_at: string;
}

/**
 * 配置视图（API 出参）。
 *
 * API key 明文永远不出现，只回 api_key_present 指示位。
 */
export interface AppConfigView {
  provider: Provider;
  /** 本次调用使用的协议。 */
  protocol: Protocol;
  endpoint: string;
  model_name: string;
  embedding_model: string;
  /** 单次模型调用超时（毫秒）。 */
  optimize_timeout_ms: number;
  /** 本地明文 api_key 或环境变量任一存在即为 true。 */
  api_key_present: boolean;
  /** 存放 API key 的环境变量名，例如 PROMPT_PAD_MINIMAX_API_KEY。 */
  api_key_env: string;
  /** 注入记忆的 token 硬上限，默认 800。 */
  memory_token_budget: number;
  /** 收起态 DOT 是否开启点击穿透。 */
  dot_click_through: boolean;
  /** 唤起面板的全局热键。 */
  global_shortcut: string;
  /** 是否开机自启。 */
  autostart_enabled: boolean;
  /** 配置文件状态：ok / missing / malformed。 */
  config_status: "ok" | "missing" | "malformed";
  /** 改写主路径还缺必填项。 */
  needs_setup: boolean;
}

/** 配置视图的历史别名，保留给仍按旧名引用的组件。 */
export type AppConfig = AppConfigView;

/**
 * 配置补丁：未出现的字段保持原值。
 *
 * api_key 为空字符串表示「清除已存的本地明文」，null 表示不修改。
 */
export interface ConfigPatch {
  provider: Provider | null;
  protocol: Protocol | null;
  endpoint: string | null;
  model_name: string | null;
  embedding_model: string | null;
  optimize_timeout_ms: number | null;
  api_key_env: string | null;
  /** API key 明文；空字符串 = 清除本地明文，后端永不回显。 */
  api_key: string | null;
  memory_token_budget: number | null;
  dot_click_through: boolean | null;
  global_shortcut: string | null;
  autostart_enabled: boolean | null;
}
