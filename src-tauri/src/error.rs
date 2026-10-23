use serde::Serialize;
use serde_json::{json, Value};
use std::fmt;

/// 业务错误码。
///
/// 取值集合与 `docs/interface-doc.md` §3.2 的错误码表**逐字一致**（小写蛇形）。
/// 前端按 `code` 分支处理，因此不可新增、不可改名、不可改大小写。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// 入参为空 / 缺少必填字段 / ID 格式不符。
    InvalidInput,
    /// 类型或取值范围不合法。
    InvalidType,
    /// 长度或条数越界。
    PayloadTooLarge,
    /// 模型调用超时。
    ModelTimeout,
    /// 模型服务不可用（5xx / 连接失败 / 响应不可解析）。
    ModelUnavailable,
    /// 模型接入信息缺失。
    ProviderNotConfigured,
    /// 记忆库 token 预算被突破，拒绝写入。
    MemoryBudgetExceeded,
    /// 记忆条目互相矛盾，挂起人工裁决。
    ///
    /// 夜间任务按契约**不**用这个码报错（改为在 `pending_review_ids` 挂起），
    /// 该码留给后续「人工裁决入口」触发自动写入的场景。
    #[allow(dead_code)]
    ConflictPendingReview,
    /// 向量库中记录的模型名与当前配置不一致，须重建全库索引。
    #[allow(dead_code)]
    EmbeddingModelMismatch,
    /// 记录或记忆条目不存在。
    NotFound,
    /// PE 记录 ID 冲突。
    DuplicateRecord,
    /// 写入本机存储失败。
    StorageWriteFailed,
    /// 读取本机存储失败。
    StorageReadFailed,
    /// 与其他写操作并发。
    LockConflict,
    /// 记忆库规模尚未进入请求的检索阶段。
    #[allow(dead_code)]
    StageNotReached,
    /// 未归类的内部异常兜底。
    InternalError,
}

impl ErrorCode {
    /// 对外暴露的错误码字符串。
    pub fn code(self) -> &'static str {
        match self {
            ErrorCode::InvalidInput => "invalid_input",
            ErrorCode::InvalidType => "invalid_type",
            ErrorCode::PayloadTooLarge => "payload_too_large",
            ErrorCode::ModelTimeout => "model_timeout",
            ErrorCode::ModelUnavailable => "model_unavailable",
            ErrorCode::ProviderNotConfigured => "provider_not_configured",
            ErrorCode::MemoryBudgetExceeded => "memory_budget_exceeded",
            ErrorCode::ConflictPendingReview => "conflict_pending_review",
            ErrorCode::EmbeddingModelMismatch => "embedding_model_mismatch",
            ErrorCode::NotFound => "not_found",
            ErrorCode::DuplicateRecord => "duplicate_record",
            ErrorCode::StorageWriteFailed => "storage_write_failed",
            ErrorCode::StorageReadFailed => "storage_read_failed",
            ErrorCode::LockConflict => "lock_conflict",
            ErrorCode::StageNotReached => "stage_not_reached",
            ErrorCode::InternalError => "internal_error",
        }
    }

    /// 默认文案：面向用户的中文提示，不含堆栈与密钥。
    pub fn default_message(self) -> &'static str {
        match self {
            ErrorCode::InvalidInput => "请求内容为空或缺少必填字段",
            ErrorCode::InvalidType => "参数类型或取值范围不正确",
            ErrorCode::PayloadTooLarge => "内容超出长度上限",
            ErrorCode::ModelTimeout => "模型调用超时，请稍后重试或在设置中调大超时时间",
            ErrorCode::ModelUnavailable => "模型服务暂时不可用，请稍后重试",
            ErrorCode::ProviderNotConfigured => "尚未配置模型接入信息，请到设置页填写",
            ErrorCode::MemoryBudgetExceeded => "记忆库已达 800 token 上限，本次晋升被拒绝",
            ErrorCode::ConflictPendingReview => "检测到与既有记忆矛盾的条目，已挂起等待人工裁决",
            ErrorCode::EmbeddingModelMismatch => "嵌入模型已变更，需重建索引后才能使用向量检索",
            ErrorCode::NotFound => "未找到对应的记录或记忆条目",
            ErrorCode::DuplicateRecord => "该 PE 记录 ID 已存在",
            ErrorCode::StorageWriteFailed => "写入本机存储失败",
            ErrorCode::StorageReadFailed => "读取本机存储失败",
            ErrorCode::LockConflict => "正在执行另一个写操作，请稍后重试",
            ErrorCode::StageNotReached => "当前数据规模尚未进入该检索阶段",
            ErrorCode::InternalError => "内部错误，请查看日志",
        }
    }

    /// 可恢复性：true = 用户可重试 / 改配置 / 换输入；false = 阻止写入或需人工裁决。
    pub fn is_recoverable(self) -> bool {
        !matches!(
            self,
            ErrorCode::MemoryBudgetExceeded
                | ErrorCode::ConflictPendingReview
                | ErrorCode::EmbeddingModelMismatch
                | ErrorCode::InternalError
        )
    }
}

/// 统一错误结构。序列化后为 `{ code, message, details }`，前端据此分支处理。
#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    /// 业务错误码，取值来自 `ErrorCode`。
    pub code: &'static str,
    /// 面向用户的中文提示文案。
    pub message: String,
    /// 结构化补充信息（字段名随 `code` 变化）；无补充时为 null。
    pub details: Value,
    /// 错误码枚举本身：用于判定可恢复性，不参与序列化。
    #[serde(skip)]
    kind: ErrorCode,
}

impl AppError {
    /// 用错误种类的默认文案构造。
    pub fn new(kind: ErrorCode) -> Self {
        Self {
            code: kind.code(),
            message: kind.default_message().to_string(),
            // 契约 §3.1 规定 `details` 恒为对象、无补充时为空对象。
            // 早先这里是 Value::Null，前端读 `details?.field` 与 `details.field`
            // 行为不一致，JSON Schema 也校验不过。
            details: json!({}),
            kind,
        }
    }

    /// 带更准确的文案构造：默认文案是兜底，具体调用点应给出更贴切的提示。
    pub fn with_message(kind: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            ..Self::new(kind)
        }
    }

    /// 附带结构化上下文（参数名、失败原因等），便于排障时定位到具体调用点。
    ///
    /// 契约要求 `details` 恒为对象，因此纯文本细节统一包进 `reason`，
    /// 避免同一字段在不同错误码下出现「字符串 / 对象」两种形状。
    pub fn with_details(mut self, details: impl Into<Value>) -> Self {
        self.details = match details.into() {
            Value::Object(map) => Value::Object(map),
            Value::Null => json!({}),
            other => json!({ "reason": other }),
        };
        self
    }

    /// 是否可恢复。
    pub fn is_recoverable(&self) -> bool {
        self.kind.is_recoverable()
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::InvalidInput, message)
    }

    pub fn invalid_type(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::InvalidType, message)
    }

    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::PayloadTooLarge, message)
    }

    pub fn model_timeout(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::ModelTimeout, message)
    }

    /// 模型服务不可用。
    ///
    /// 名称沿用旧实现是为了让 `lib.rs` 等既有调用点无需改动；
    /// 实际错误码是契约里的 `model_unavailable`。
    pub fn provider_unavailable(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::ModelUnavailable, message)
    }

    pub fn provider_not_configured(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::ProviderNotConfigured, message)
    }

    pub fn memory_budget_exceeded(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::MemoryBudgetExceeded, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::NotFound, message)
    }

    pub fn duplicate_record(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::DuplicateRecord, message)
    }

    /// 写入本机存储失败。
    ///
    /// 名称沿用旧实现是为了让 `window_ctl.rs` 等既有调用点无需改动；
    /// 实际错误码是契约里的 `storage_write_failed`。
    pub fn storage_failure(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::StorageWriteFailed, message)
    }

    pub fn storage_read_failed(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::StorageReadFailed, message)
    }

    pub fn lock_conflict(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::LockConflict, message)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::InternalError, message)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 可恢复性写进日志：排查时一眼能分清「让用户重试」与「必须改配置 / 人工裁决」。
        let kind = if self.is_recoverable() {
            "可恢复"
        } else {
            "不可恢复"
        };
        write!(f, "[{}/{kind}] {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::storage_failure("SQLite 操作失败").with_details(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::storage_failure("JSON 序列化失败").with_details(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::storage_failure("文件读写失败").with_details(err.to_string())
    }
}

/// Tauri 运行期错误（窗口、菜单、IPC 等）统一归到 `internal_error`：
/// 它们不属于存储故障，也不是用户输入问题。
impl From<tauri::Error> for AppError {
    fn from(err: tauri::Error) -> Self {
        AppError::internal_error("Tauri 运行期调用失败").with_details(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 错误码全部是小写蛇形() {
        let codes = [
            ErrorCode::InvalidInput,
            ErrorCode::InvalidType,
            ErrorCode::PayloadTooLarge,
            ErrorCode::ModelTimeout,
            ErrorCode::ModelUnavailable,
            ErrorCode::ProviderNotConfigured,
            ErrorCode::MemoryBudgetExceeded,
            ErrorCode::ConflictPendingReview,
            ErrorCode::EmbeddingModelMismatch,
            ErrorCode::NotFound,
            ErrorCode::DuplicateRecord,
            ErrorCode::StorageWriteFailed,
            ErrorCode::StorageReadFailed,
            ErrorCode::LockConflict,
            ErrorCode::StageNotReached,
            ErrorCode::InternalError,
        ];
        for code in codes {
            assert!(code
                .code()
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_'));
            assert!(!code.default_message().is_empty());
        }
        // 16 个码互不重复。
        let mut seen: Vec<&str> = codes.iter().map(|c| c.code()).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 16);
    }

    #[test]
    fn 纯文本细节被包进_reason() {
        let err = AppError::invalid_input("提示词为空").with_details("original_prompt");
        assert_eq!(err.details["reason"], "original_prompt");
    }

    #[test]
    fn 对象细节原样保留() {
        let err = AppError::invalid_type("评分越界")
            .with_details(json!({ "field": "rating", "expected": "1..=5" }));
        assert_eq!(err.details["field"], "rating");
    }

    #[test]
    fn 序列化形状是_code_message_details() {
        let err = AppError::not_found("记录不存在");
        let value = serde_json::to_value(&err).unwrap();
        assert_eq!(value["code"], "not_found");
        assert_eq!(value["message"], "记录不存在");
        // 契约 §3.1：details 恒为对象，无补充时是空对象。
        assert!(value["details"].is_object());
        assert_eq!(value["details"].as_object().unwrap().len(), 0);
    }
}
