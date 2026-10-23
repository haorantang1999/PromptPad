import { call } from "./client";
import type {
  AppConfigView,
  ConfigPatch,
  DotDragOutcome,
  FeedbackRequest,
  FeedbackResult,
  MemoryInjection,
  NightlyJobReport,
  OptimizeRequest,
  OptimizeResponse,
  PadView,
  PanelAction,
  RetrieveMemoriesRequest,
  RunNightlyJobRequest,
  SavePeRecordRequest,
  SavePeRecordResponse,
  TogglePanelResponse,
} from "./types";

/**
 * 后端 command 契约（10 个，与 docs/interface-doc.md 一致）。
 * 这里只做类型收敛，不含业务逻辑。
 *
 * invoke 的 payload key 必须与 Rust 侧的命令参数名逐字一致：
 * Tauri 2 是按参数名从 payload 取值的，名字对不上就取不到值。
 */

/** 展开 / 收起面板。mode 缺省为 toggle。 */
export function togglePanel(mode?: PanelAction): Promise<TogglePanelResponse> {
  return call<TogglePanelResponse>("toggle_panel", { mode: mode ?? "toggle" });
}

/** 提交原始提示词，调用当前配置的模型改写。 */
export function optimizePrompt(request: OptimizeRequest): Promise<OptimizeResponse> {
  return call<OptimizeResponse>("optimize_prompt", { request });
}

/** 把本次改写落库为 PE-YYYY-NNN 记录。 */
export function savePeRecord(request: SavePeRecordRequest): Promise<SavePeRecordResponse> {
  return call<SavePeRecordResponse>("save_pe_record", { request });
}

/** 提交评分，驱动记忆权重 +1 / 0 / −2 与连续未命中计数。 */
export function submitFeedback(request: FeedbackRequest): Promise<FeedbackResult> {
  return call<FeedbackResult>("submit_feedback", { request });
}

/** 按 domain 检索注入集；只返回 statement + why + how_to_apply。 */
export function retrieveMemories(
  request: RetrieveMemoriesRequest,
): Promise<MemoryInjection> {
  return call<MemoryInjection>("retrieve_memories", { request });
}

/** 触发夜间记忆沉淀流水线。 */
export function runNightlyJob(
  request?: RunNightlyJobRequest,
): Promise<NightlyJobReport> {
  return call<NightlyJobReport>("run_nightly_job", {
    request: request ?? { run_date: null, dry_run: null },
  });
}

/** 读取当前配置。 */
export function getConfig(): Promise<AppConfigView> {
  return call<AppConfigView>("get_config");
}

/**
 * 写入配置补丁。
 *
 * 补丁语义：字段为 null 表示保持原值。
 * api_key 走补丁字段（空字符串 = 清除本地明文），后端不回显明文。
 */
export function setConfig(request: ConfigPatch): Promise<AppConfigView> {
  return call<AppConfigView>("set_config", { request });
}

/**
 * 开始一次 DOT 交互监视，直到左键松开。
 *
 * 圆球是 Tauri 拖拽区，按下即启动系统拖拽循环并独占鼠标，
 * 浏览器不再派发 mouseup / click，点击与拖拽只能由后端判定。
 * 后端在判定为「原地点击」时会自行展开面板。
 */
export function beginDotDrag(): Promise<DotDragOutcome> {
  return call<DotDragOutcome>("begin_dot_drag");
}

/**
 * 告诉后端面板当前显示的是哪个视图。
 *
 * 后端据此决定「点面板外部收起」在设置视图下是否生效。
 */
export function setPanelView(view: PadView): Promise<void> {
  return call<void>("set_panel_view", { request: { view } });
}
