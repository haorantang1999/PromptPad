use crate::error::{AppError, ErrorCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::Path;

/// 大模型提供方。三个提供方统一走 OpenAI 兼容的 chat/completions 协议，
/// 差异只在 endpoint 与是否需要 API key。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Minimax,
    OpenaiCompat,
    Ollama,
    Anthropic,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Minimax => "minimax",
            Provider::OpenaiCompat => "openai_compat",
            Provider::Ollama => "ollama",
            Provider::Anthropic => "anthropic",
        }
    }

    /// 本地模型（Ollama）不需要 API key，其余需要从环境变量读取。
    pub fn requires_api_key(self) -> bool {
        !matches!(self, Provider::Ollama)
    }
}

/// 接口协议（线上报文格式）。
///
/// 提供方与协议是**两件事**：同一个 MiniMax 既有 OpenAI 兼容端点，
/// 也有 Anthropic 兼容端点（Token Plan 订阅用的就是后者），
/// 而 Claude Code、Cursor 这类工具走的又是 Anthropic 协议。
/// 因此「用哪家」与「怎么说话」必须分开配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    /// OpenAI chat/completions：`POST {base}/v1/chat/completions`。
    OpenaiChatCompletions,
    /// Anthropic messages：`POST {base}/v1/messages`。
    AnthropicMessages,
}

impl Protocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Protocol::OpenaiChatCompletions => "openai_chat_completions",
            Protocol::AnthropicMessages => "anthropic_messages",
        }
    }

    /// 协议默认路径。endpoint 只填到「协议根」时由这里补全。
    pub fn default_path(self) -> &'static str {
        match self {
            Protocol::OpenaiChatCompletions => "/v1/chat/completions",
            Protocol::AnthropicMessages => "/v1/messages",
        }
    }

    /// 版本段。endpoint 恰好填到这里时，只补协议尾段。
    pub fn version_suffix(self) -> &'static str {
        "/v1"
    }

    /// 协议尾段。接在版本段之后。
    pub fn tail_suffix(self) -> &'static str {
        match self {
            Protocol::OpenaiChatCompletions => "/chat/completions",
            Protocol::AnthropicMessages => "/messages",
        }
    }

    /// 完整请求路径的后缀。
    ///
    /// endpoint 已经以其中之一结尾，说明用户粘贴的就是完整请求地址，
    /// 必须原样使用 —— 再补一次路径会拼出 `.../chat/completions/chat/completions`。
    pub fn full_suffixes(self) -> &'static [&'static str] {
        match self {
            Protocol::OpenaiChatCompletions => &["/chat/completions", "/v1/chat/completions"],
            Protocol::AnthropicMessages => &["/messages", "/v1/messages"],
        }
    }
}

/// Anthropic 协议要求必填 `max_tokens`。
///
/// 取 4096：改写一段提示词用不了这么多，而上限越大越容易在长上下文模型上被截断。
pub const ANTHROPIC_MAX_TOKENS: u32 = 4096;

/// Anthropic 协议版本头。
///
/// Anthropic Messages API 用版本头区分报文格式；这个值是其当前公开版本。
pub const ANTHROPIC_VERSION: &str = "2023-06-01";

/// 允许的嵌入模型名。
///
/// 换模型必须重建全库索引（每条向量记录当时使用的模型名），
/// 因此取值集合是封闭的，不能让调用方随便写。
pub const EMBEDDING_MODELS: [&str; 2] = ["multilingual-e5-small", "bge-m3"];

/// 改写调用超时的取值区间（毫秒）。
pub const OPTIMIZE_TIMEOUT_RANGE: (u64, u64) = (1_000, 300_000);

/// 注入记忆的 token 硬上限。
///
/// 800 是产品级硬约束（AGENTS.md §6 第 7 条），因此配置也不能把它调大。
pub const MEMORY_TOKEN_BUDGET_MAX: u32 = 800;

/// 默认的全局热键。
///
/// `Ctrl+Q` 而不是 `Ctrl+Space`：空格键在大量编辑器里是「补全 / 触发 snippet」，
/// 与 AI 助手的常见快捷键冲突面太大；`Ctrl+Q` 冲突面小得多。
pub const DEFAULT_GLOBAL_SHORTCUT: &str = "Ctrl+Q";

/// 热键字符串是否可用。
///
/// 只做形状校验（形如 `Ctrl+Q`）：真正能不能注册由系统决定，
/// 注册失败会在绑定时报错，而不是在这里假装成功。
pub fn is_valid_shortcut(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.is_ascii()
        && value.contains('+')
        && value.split('+').all(|part| !part.trim().is_empty())
}

/// 应用配置。序列化后的字段名与前端 `AppConfigView` 一一对应（snake_case）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AppConfig {
    #[serde(default = "default_provider")]
    pub provider: Provider,
    /// 线上报文协议。
    ///
    /// 与 provider 分开：MiniMax 同时提供 OpenAI 兼容端点与 Anthropic 兼容端点
    /// （Token Plan 订阅走的是后者），Claude Code 这类工具也走 Anthropic 协议。
    /// 「用哪家」与「怎么说话」必须能分别配置。
    #[serde(default = "default_protocol")]
    pub protocol: Protocol,
    /// 模型服务地址。由使用者配置，代码不内置任何具体服务的地址。
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub model_name: String,
    /// 本地嵌入模型名。阶段一 multilingual-e5-small，换 bge-m3 必须重建索引。
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    /// 本地保存的 API key 明文。
    ///
    /// 只落在 `%APPDATA%` 的 config.json 里，**不进 git 仓库、不进 `memory/`**。
    /// AGENTS.md §4 第 6 条原文只允许「环境变量或系统钥匙串」，
    /// 这里为了让「粘贴即用」放宽为本地明文，已在 `docs/interface-doc.md` §〇 修订表登记。
    #[serde(default)]
    pub api_key: String,
    /// 存放 API key 的环境变量名；只存变量名，不存明文。
    ///
    /// 本地明文为空时才回退到这里 —— 环境变量仍然是「不想把 key 落盘」时的首选。
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,
    /// 单次模型调用超时（毫秒）。
    #[serde(default = "default_optimize_timeout_ms")]
    pub optimize_timeout_ms: u64,
    /// 单次注入记忆的 token 硬上限。
    #[serde(default = "default_memory_token_budget")]
    pub memory_token_budget: u32,
    /// 收起态 DOT 是否开启点击穿透。
    #[serde(default)]
    pub dot_click_through: bool,
    /// 唤起面板的全局热键。
    ///
    /// 做成配置项而不是写死：用户要在「设置」里改键，改完必须立刻生效。
    #[serde(default = "default_global_shortcut")]
    pub global_shortcut: String,
    /// 开机自启。默认关闭：常驻应用不该在没问过用户的情况下进启动项。
    #[serde(default)]
    pub autostart_enabled: bool,
}

/// 配置文件在磁盘上的状态。
///
/// 与内存里的 `AppConfig` 分开：文件缺失或损坏时应用仍用默认值启动，
/// 但必须让设置页知道「这不是一份已保存的配置」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFileStatus {
    Ok,
    Missing,
    Malformed,
}

impl ConfigFileStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ConfigFileStatus::Ok => "ok",
            ConfigFileStatus::Missing => "missing",
            ConfigFileStatus::Malformed => "malformed",
        }
    }
}

fn default_provider() -> Provider {
    Provider::OpenaiCompat
}

fn default_protocol() -> Protocol {
    Protocol::OpenaiChatCompletions
}

fn default_embedding_model() -> String {
    "multilingual-e5-small".to_string()
}

fn default_api_key_env() -> String {
    "PROMPT_PAD_API_KEY".to_string()
}

fn default_optimize_timeout_ms() -> u64 {
    30_000
}

fn default_memory_token_budget() -> u32 {
    MEMORY_TOKEN_BUDGET_MAX
}

fn default_global_shortcut() -> String {
    DEFAULT_GLOBAL_SHORTCUT.to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            protocol: default_protocol(),
            endpoint: String::new(),
            model_name: String::new(),
            embedding_model: default_embedding_model(),
            api_key: String::new(),
            api_key_env: default_api_key_env(),
            optimize_timeout_ms: default_optimize_timeout_ms(),
            memory_token_budget: default_memory_token_budget(),
            dot_click_through: false,
            global_shortcut: default_global_shortcut(),
            autostart_enabled: false,
        }
    }
}

impl AppConfig {
    /// 校验配置是否足以发起一次模型调用。
    ///
    /// endpoint 与 model_name 刻意留空：不同提供方的服务地址与模型名各不相同，
    /// 写死默认值等于凭空捏造一个不存在的服务，因此必须由使用者显式配置。
    pub fn validate_for_request(&self) -> Result<(), AppError> {
        if self.endpoint.trim().is_empty() {
            return Err(
                AppError::provider_not_configured("尚未配置模型服务地址 endpoint").with_details(
                    json!({ "missing": "endpoint", "provider": self.provider.as_str() }),
                ),
            );
        }
        if self.model_name.trim().is_empty() {
            return Err(
                AppError::provider_not_configured("尚未配置模型名 model_name").with_details(
                    json!({ "missing": "model_name", "provider": self.provider.as_str() }),
                ),
            );
        }
        if !self.endpoint.starts_with("http://") && !self.endpoint.starts_with("https://") {
            return Err(AppError::provider_not_configured(
                "模型服务地址必须以 http:// 或 https:// 开头",
            )
            .with_details(
                json!({ "missing": "endpoint", "provider": self.provider.as_str(), "reason": "invalid_scheme" }),
            ));
        }
        let (min, max) = OPTIMIZE_TIMEOUT_RANGE;
        if self.optimize_timeout_ms < min || self.optimize_timeout_ms > max {
            return Err(
                AppError::invalid_type("改写调用超时超出允许区间").with_details(
                    json!({ "field": "optimize_timeout_ms", "expected": format!("{min}..={max}") }),
                ),
            );
        }
        if self.memory_token_budget == 0 || self.memory_token_budget > MEMORY_TOKEN_BUDGET_MAX {
            return Err(AppError::invalid_type("记忆 token 预算超出允许区间").with_details(
                json!({ "field": "memory_token_budget", "expected": format!("1..={MEMORY_TOKEN_BUDGET_MAX}") }),
            ));
        }
        // API key 缺失时引导用户去设置，而不是让它裸奔。
        if self.provider.requires_api_key() && self.api_key().is_none() {
            return Err(
                AppError::provider_not_configured("尚未配置模型 API key").with_details(
                    json!({ "missing": "api_key", "provider": self.provider.as_str() }),
                ),
            );
        }
        Ok(())
    }

    /// 取本次调用实际使用的 API key。
    ///
    /// 顺序：本地明文优先，其次环境变量。
    /// 取不到时返回 Ok(None)，因为 Ollama 这类本地服务本来就不需要 key。
    pub fn api_key(&self) -> Option<String> {
        if !self.provider.requires_api_key() {
            return None;
        }
        let local = self.api_key.trim();
        if !local.is_empty() {
            return Some(local.to_string());
        }
        let env_name = self.api_key_env.trim();
        if env_name.is_empty() {
            return None;
        }
        std::env::var(env_name)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }

    /// 当前是否存在可用 API key。ollama 恒为 false。
    pub fn api_key_present(&self) -> bool {
        self.api_key().is_some()
    }

    /// 改写主路径还缺必填项：空 endpoint / 空模型名 / 需要 key 却没有。
    pub fn needs_setup(&self) -> bool {
        self.endpoint.trim().is_empty()
            || self.model_name.trim().is_empty()
            || (self.provider.requires_api_key() && self.api_key().is_none())
    }

    /// 把配置里的 endpoint 解析成最终请求地址。
    ///
    /// 四段规则，顺序敏感：
    /// 1. 末尾斜杠先去掉；
    /// 2. 已经以协议的完整路径结尾（用户粘贴的是完整请求地址）→ **原样使用**，
    ///    再补一次会拼出 `.../chat/completions/chat/completions`；
    /// 3. 只填到版本段（`/v1`）→ 补协议尾段；
    /// 4. 其余情况（协议根，如 `https://api.anthropic.com` 或 MiniMax 的 `/anthropic`）
    ///    → 补协议默认路径。
    pub fn request_url(&self) -> String {
        let base = self.endpoint.trim().trim_end_matches('/');
        if base.is_empty() {
            return String::new();
        }
        for suffix in self.protocol.full_suffixes() {
            if base.ends_with(suffix) {
                return base.to_string();
            }
        }
        if base.ends_with(self.protocol.version_suffix()) {
            return format!("{base}{}", self.protocol.tail_suffix());
        }
        format!("{base}{}", self.protocol.default_path())
    }

    /// 嵌入模型名是否在允许集合内。
    pub fn is_valid_embedding_model(value: &str) -> bool {
        EMBEDDING_MODELS.contains(&value)
    }
}

/// 是否是一个可用的环境变量名。
///
/// 判据是「能不能当变量名用」：首字符必须是字母或下划线，其余只能是字母、数字、下划线。
/// API key 明文（形如 `sk-...`）必然含非法字符或以数字开头，因此能被这条规则识别出来，
/// 用于把历史上误填进 `api_key_env` 的明文搬到正确的字段里。
pub fn is_env_var_name(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    let Some(first) = value.chars().next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// 从磁盘读取配置；文件不存在时返回默认配置（首次启动的正常路径）。
///
/// 启动路径走 `load_with_status`（要带文件状态）；本函数留给单测与简单读取。
#[cfg_attr(not(test), allow(dead_code))]
pub fn load(path: &Path) -> Result<AppConfig, AppError> {
    Ok(load_with_status(path)?.0)
}

/// 读取配置并报告文件状态。损坏的文件**不会被覆盖**。
pub fn load_with_status(path: &Path) -> Result<(AppConfig, ConfigFileStatus), AppError> {
    if !path.exists() {
        return Ok((AppConfig::default(), ConfigFileStatus::Missing));
    }
    let raw = fs::read_to_string(path).map_err(|err| {
        AppError::storage_read_failed("配置文件读取失败").with_details(err.to_string())
    })?;
    if raw.trim().is_empty() {
        return Ok((AppConfig::default(), ConfigFileStatus::Missing));
    }
    let mut config: AppConfig = match serde_json::from_str(&raw) {
        Ok(config) => config,
        Err(_) => {
            // 常驻形态比配置更重要：启动用默认值，磁盘原件留给用户在设置页重填。
            return Ok((AppConfig::default(), ConfigFileStatus::Malformed));
        }
    };
    // token 预算是产品级硬上限，手改配置也不能把它调大。
    if config.memory_token_budget == 0 || config.memory_token_budget > MEMORY_TOKEN_BUDGET_MAX {
        config.memory_token_budget = MEMORY_TOKEN_BUDGET_MAX;
    }
    // 历史遗留迁移：`api_key_env` 曾被直接填成 key 明文。
    // 那样的值不是合法的环境变量名，原样留着的话该 key 永远查不到环境变量 —— 等于没配。
    // 这里把它搬进 `api_key` 字段并把变量名复位成默认值。
    if config.api_key.trim().is_empty() && !is_env_var_name(&config.api_key_env) {
        config.api_key = config.api_key_env.trim().to_string();
        config.api_key_env = default_api_key_env();
        // 立刻落盘：迁移只应发生一次，否则磁盘上会一直留着
        // 「明文躺在变量名字段里」的历史布局，每次启动都搬一遍。
        save(&config, path)?;
    }
    Ok((config, ConfigFileStatus::Ok))
}

/// 原子写回配置：先写临时文件再替换，避免进程中断留下半截 JSON。
pub fn save(config: &AppConfig, path: &Path) -> Result<(), AppError> {
    let body = serde_json::to_string_pretty(config)?;
    crate::fsutil::write_atomic(path, body)
}

/// 校验嵌入模型名，非法时返回 `invalid_type`。
pub fn validate_embedding_model(value: &str) -> Result<(), AppError> {
    if AppConfig::is_valid_embedding_model(value) {
        return Ok(());
    }
    Err(
        AppError::with_message(ErrorCode::InvalidType, "嵌入模型名不在允许取值内")
            .with_details(json!({ "field": "embedding_model", "expected": EMBEDDING_MODELS })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 默认超时与预算符合契约() {
        let config = AppConfig::default();
        assert_eq!(config.optimize_timeout_ms, 30_000);
        assert_eq!(config.memory_token_budget, 800);
        assert_eq!(config.embedding_model, "multilingual-e5-small");
    }

    #[test]
    fn 嵌入模型名只接受两个取值() {
        assert!(AppConfig::is_valid_embedding_model("multilingual-e5-small"));
        assert!(AppConfig::is_valid_embedding_model("bge-m3"));
        assert!(!AppConfig::is_valid_embedding_model(
            "text-embedding-3-small"
        ));
        assert!(validate_embedding_model("bge-m3").is_ok());
        assert_eq!(
            validate_embedding_model("nope").unwrap_err().code,
            "invalid_type"
        );
    }

    #[test]
    fn 缺少端点时返回未配置() {
        let config = AppConfig::default();
        let err = config.validate_for_request().unwrap_err();
        assert_eq!(err.code, "provider_not_configured");
    }

    #[test]
    fn 超时越界返回类型错误() {
        let config = AppConfig {
            endpoint: "https://example.invalid/v1".to_string(),
            model_name: "m".to_string(),
            optimize_timeout_ms: 10,
            ..AppConfig::default()
        };
        assert_eq!(
            config.validate_for_request().unwrap_err().code,
            "invalid_type"
        );
    }

    /// 端点补全规则：完整地址原样用、版本段补尾段、协议根补默认路径。
    #[test]
    fn 端点补全规则符合协议() {
        let openai = AppConfig {
            protocol: Protocol::OpenaiChatCompletions,
            endpoint: "https://api.example.com/v1/chat/completions".to_string(),
            ..AppConfig::default()
        };
        // 粘贴完整地址时不能再补路径，否则会拼成 chat/completions/chat/completions。
        assert_eq!(
            openai.request_url(),
            "https://api.example.com/v1/chat/completions"
        );

        let openai_root = AppConfig {
            protocol: Protocol::OpenaiChatCompletions,
            endpoint: "https://api.example.com/v1".to_string(),
            ..AppConfig::default()
        };
        assert_eq!(
            openai_root.request_url(),
            "https://api.example.com/v1/chat/completions"
        );

        let openai_base = AppConfig {
            protocol: Protocol::OpenaiChatCompletions,
            endpoint: "https://api.example.com".to_string(),
            ..AppConfig::default()
        };
        assert_eq!(
            openai_base.request_url(),
            "https://api.example.com/v1/chat/completions"
        );

        let anthropic = AppConfig {
            protocol: Protocol::AnthropicMessages,
            endpoint: "https://api.minimaxi.com/anthropic".to_string(),
            ..AppConfig::default()
        };
        assert_eq!(
            anthropic.request_url(),
            "https://api.minimaxi.com/anthropic/v1/messages"
        );

        let anthropic_v1 = AppConfig {
            protocol: Protocol::AnthropicMessages,
            endpoint: "https://api.anthropic.com/v1/".to_string(),
            ..AppConfig::default()
        };
        assert_eq!(
            anthropic_v1.request_url(),
            "https://api.anthropic.com/v1/messages"
        );

        let anthropic_full = AppConfig {
            protocol: Protocol::AnthropicMessages,
            endpoint: "https://api.anthropic.com/v1/messages".to_string(),
            ..AppConfig::default()
        };
        assert_eq!(
            anthropic_full.request_url(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn 本地明文优先于环境变量() {
        let config = AppConfig {
            provider: Provider::Minimax,
            api_key: "  sk-local  ".to_string(),
            api_key_env: "PROMPT_PAD_TEST_MISSING_ENV".to_string(),
            ..AppConfig::default()
        };
        assert_eq!(config.api_key().as_deref(), Some("sk-local"));
        assert!(config.api_key_present());
    }

    #[test]
    fn 误填进环境变量名的明文被搬回本地字段() {
        // 复现历史状态：自动保存把用户粘贴的 key 写进了 api_key_env。
        // 用明显假值，禁止把真实 key 写进单测（会进 git）。
        const FAKE_KEY: &str = "sk-cp-NOT_A_REAL_KEY_FOR_UNIT_TEST_ONLY";
        let mut config = AppConfig {
            api_key_env: FAKE_KEY.to_string(),
            ..AppConfig::default()
        };
        if config.api_key.trim().is_empty() && !is_env_var_name(&config.api_key_env) {
            config.api_key = config.api_key_env.trim().to_string();
            config.api_key_env = default_api_key_env();
        }
        assert_eq!(config.api_key, FAKE_KEY);
        assert_eq!(config.api_key_env, "PROMPT_PAD_API_KEY");
    }

    #[test]
    fn 合法环境变量名不会被当成明文搬走() {
        assert!(is_env_var_name("PROMPT_PAD_API_KEY"));
        assert!(is_env_var_name("_private1"));
        assert!(!is_env_var_name("sk-cp-abc"));
        assert!(!is_env_var_name("1LEADING_DIGIT"));
        assert!(!is_env_var_name(""));
        assert!(!is_env_var_name("  "));
    }

    #[test]
    fn 本地模型不要求任何key() {
        let config = AppConfig {
            provider: Provider::Ollama,
            api_key: "sk-should-be-ignored".to_string(),
            ..AppConfig::default()
        };
        // Ollama 是本地服务，不需要 key：即使本地存了明文也不该被取用。
        assert!(config.api_key().is_none());
        assert!(!config.api_key_present());
    }

    /// 加载期迁移必须同时改内存与磁盘，否则下次启动还会再搬一遍。
    #[test]
    fn 加载时迁移明文并立即落盘() {
        let dir = std::env::temp_dir().join("prompt_pad_config_migrate_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"api_key_env":"sk-cp-secret-value","endpoint":"https://x.com/v1","model_name":"m"}"#,
        )
        .unwrap();

        let config = load(&path).unwrap();
        assert_eq!(config.api_key, "sk-cp-secret-value");
        assert_eq!(config.api_key_env, "PROMPT_PAD_API_KEY");

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"api_key\""), "迁移结果必须已落盘：{raw}");
        assert!(raw.contains("\"api_key_env\": \"PROMPT_PAD_API_KEY\""));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
