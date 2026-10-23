import { invoke } from "@tauri-apps/api/core";

/**
 * 后端统一错误结构。Rust 侧 AppError 序列化成这个形状，
 * 前端据此分支处理，而不是解析错误文案。
 */
export interface BackendError {
  code: string;
  message: string;
  details: unknown;
}

/**
 * 业务错误码，与 Rust 侧 `ErrorCode::code()` 逐字一致。
 *
 * 取值集合固定为 `docs/interface-doc.md` §三 错误码表里的 16 个小写蛇形值 ——
 * 后端按 `code` 分支处理，前端不得自行改名或新增。
 */
export const ErrorCode = {
  /** 入参为空 / 缺少必填字段 / ID 格式不符。 */
  INVALID_INPUT: "invalid_input",
  /** 类型或取值范围不合法。 */
  INVALID_TYPE: "invalid_type",
  /** 长度或条数越界。 */
  PAYLOAD_TOO_LARGE: "payload_too_large",
  /** 模型调用超时。 */
  MODEL_TIMEOUT: "model_timeout",
  /** 模型服务不可用（5xx / 连接失败 / 响应不可解析）。 */
  MODEL_UNAVAILABLE: "model_unavailable",
  /** 模型接入信息缺失。 */
  PROVIDER_NOT_CONFIGURED: "provider_not_configured",
  /** 记忆库 token 预算被突破，拒绝写入。不可恢复。 */
  MEMORY_BUDGET_EXCEEDED: "memory_budget_exceeded",
  /** 记忆条目互相矛盾，挂起人工裁决。不可恢复。 */
  CONFLICT_PENDING_REVIEW: "conflict_pending_review",
  /** 向量库模型名与当前配置不一致，须重建全库索引。不可恢复。 */
  EMBEDDING_MODEL_MISMATCH: "embedding_model_mismatch",
  /** 记录或记忆条目不存在。 */
  NOT_FOUND: "not_found",
  /** PE 记录 ID 冲突。 */
  DUPLICATE_RECORD: "duplicate_record",
  /** 写入本机存储失败。 */
  STORAGE_WRITE_FAILED: "storage_write_failed",
  /** 读取本机存储失败。 */
  STORAGE_READ_FAILED: "storage_read_failed",
  /** 与其他写操作并发。 */
  LOCK_CONFLICT: "lock_conflict",
  /** 记忆库规模尚未进入请求的检索阶段。 */
  STAGE_NOT_REACHED: "stage_not_reached",
  /** 未归类的内部异常兜底。不可恢复。 */
  INTERNAL_ERROR: "internal_error",
} as const;

/** 不可恢复的错误码：只能靠用户改配置或人工介入，提示「稍后重试」是错的。 */
const UNRECOVERABLE_CODES: ReadonlySet<string> = new Set([
  ErrorCode.MEMORY_BUDGET_EXCEEDED,
  ErrorCode.CONFLICT_PENDING_REVIEW,
  ErrorCode.EMBEDDING_MODEL_MISMATCH,
  ErrorCode.INTERNAL_ERROR,
]);

export type ErrorCodeValue = (typeof ErrorCode)[keyof typeof ErrorCode];

/** 承载后端错误码的前端异常。 */
export class AppError extends Error {
  readonly code: string;
  readonly details: unknown;

  constructor(code: string, message: string, details?: unknown) {
    super(message);
    this.name = "AppError";
    this.code = code;
    this.details = details;
  }

  /**
   * 可恢复错误可以给用户一个「稍后重试」的提示；
   * 不可恢复错误只能让用户改配置或人工介入。
   */
  get recoverable(): boolean {
    return !UNRECOVERABLE_CODES.has(this.code);
  }
}

/**
 * 唯一的 IPC 出口。组件不得直接调用 invoke，统一走这里，
 * 保证错误归一化只有一处实现。
 */
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (raw) {
    throw normalizeError(raw);
  }
}

function normalizeError(raw: unknown): AppError {
  if (raw instanceof AppError) {
    return raw;
  }
  if (typeof raw === "object" && raw !== null && "code" in raw && "message" in raw) {
    const payload = raw as Partial<BackendError>;
    return new AppError(
      String(payload.code),
      String(payload.message),
      payload.details ?? null,
    );
  }
  if (typeof raw === "string") {
    return new AppError(ErrorCode.INTERNAL_ERROR, raw);
  }
  return new AppError(ErrorCode.INTERNAL_ERROR, "后端调用失败", raw);
}
