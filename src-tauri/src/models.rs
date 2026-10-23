//! 跨层共享的数据模型。
//!
//! 这些结构同时出现在 Rust command 的入参/返回值和前端 `src/api/types.ts` 里，
//! 字段名一律 snake_case，改动必须两边同步。
//!
//! 面向前端的所有响应结构都集中在本文件，字段名与
//! `docs/interface-doc.md` 的契约逐字一致。

use crate::config::{Protocol, Provider};
use serde::{Deserialize, Serialize};

/// 单次提交的提示词长度上限。超长输入既拖慢响应也容易触发服务端截断。
pub const MAX_PROMPT_CHARS: usize = 20_000;

/// 记忆条目类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Rule,
    Preference,
    Pitfall,
    Asset,
}

impl MemoryType {
    pub fn as_str(self) -> &'static str {
        match self {
            MemoryType::Rule => "rule",
            MemoryType::Preference => "preference",
            MemoryType::Pitfall => "pitfall",
            MemoryType::Asset => "asset",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "rule" => Some(MemoryType::Rule),
            "preference" => Some(MemoryType::Preference),
            "pitfall" => Some(MemoryType::Pitfall),
            "asset" => Some(MemoryType::Asset),
            _ => None,
        }
    }
}

/// 记忆条目状态机：candidate → active → deprecated。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    Candidate,
    Active,
    Deprecated,
}

impl MemoryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            MemoryStatus::Candidate => "candidate",
            MemoryStatus::Active => "active",
            MemoryStatus::Deprecated => "deprecated",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "candidate" => Some(MemoryStatus::Candidate),
            "active" => Some(MemoryStatus::Active),
            "deprecated" => Some(MemoryStatus::Deprecated),
            _ => None,
        }
    }
}

/// L2 记忆条目（memory.md 里的一条）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub entry_type: MemoryType,
    pub domain: String,
    pub status: MemoryStatus,
    pub statement: String,
    pub why: String,
    pub how_to_apply: String,
    pub signals: Vec<String>,
    pub evidence: Vec<String>,
    pub weight: i64,
    pub hits: i64,
    pub last_used: Option<String>,
    pub created: String,
}

/// 注入集条目：只含 memory_id + statement + why + how_to_apply。
///
/// 刻意不含 evidence / weight / hits —— 那些是给我们自己看的统计字段，
/// 注入给模型只会稀释信号（AGENTS.md §6 第 5 条）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInjectionItem {
    pub memory_id: String,
    pub statement: String,
    pub why: String,
    pub how_to_apply: String,
}

/// L1 事实层记录（prompt_engineering.md 的一条）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeRecord {
    pub id: String,
    pub original_prompt: String,
    pub optimized_prompt: String,
    pub domain: String,
    pub signals: Vec<String>,
    pub rating: Option<i64>,
    pub created_at: String,
}

/// 面板锚点：哪块屏 + 屏内百分比坐标，非绝对像素。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WindowAnchor {
    pub monitor_index: usize,
    pub x_ratio: f64,
    pub y_ratio: f64,
}

/// `toggle_panel` 的响应。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TogglePanelResponse {
    /// true = 已展开，false = 已收起。
    pub expanded: bool,
    /// 本次播放的展开 / 收起动画时长。
    pub animation_ms: u16,
    pub position: WindowAnchor,
}

/// 面板当前显示的视图。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PadView {
    /// 提示词改写界面。
    Prompt,
    /// 设置界面。
    Settings,
}

/// `set_panel_view` 的入参。
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PanelViewRequest {
    pub view: PadView,
}

/// `optimize_prompt` 的响应。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct OptimizeResponse {
    /// 改写后的提示词正文。
    pub optimized_prompt: String,
    pub provider: String,
    /// 本次实际使用的报文协议（回显，便于排障）。
    pub protocol: String,
    /// 实际生效的模型名（回显当前配置，便于排障）。
    pub model_name: String,
    /// 实际进入模型上下文的记忆 ID。
    pub injected_memory_ids: Vec<String>,
    /// 本次注入上下文的 token 估算值。
    pub injected_token_count: u32,
    /// 端到端耗时（毫秒）。
    pub elapsed_ms: u64,
    /// 本次实际用于路由与落库的 domain。
    pub domain: String,
}

/// `save_pe_record` 的响应。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SavePeRecordResponse {
    /// 形如 `PE-2026-001`。
    pub record_id: String,
    /// UTC RFC 3339 时间戳。
    pub created_at: String,
    /// 本次实际落盘到的层：只可能是 `l1_markdown` 与 `l3_sqlite`。
    pub written_layers: Vec<String>,
}

/// 单条记忆的权重变化。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct WeightDelta {
    /// 契约字段名；与 `entry_id` 同值，兼容旧前端。
    pub memory_id: String,
    pub entry_id: String,
    pub delta: i64,
    pub weight_before: i64,
    pub weight_after: i64,
}

/// `submit_feedback` 的响应。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct FeedbackResult {
    pub record_id: String,
    pub rating: i64,
    /// 本次评分实际产生的权重变更；无注入命中时为空数组。
    pub applied_weight_deltas: Vec<WeightDelta>,
    /// UTC RFC 3339 时间戳。
    pub updated_at: String,
}

/// `retrieve_memories` 的响应。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct MemoryInjection {
    /// 是否命中可用记忆。false 时 `injection_set` 必须为空。
    pub matched: bool,
    /// 实际生效的检索阶段。本期固定 `markdown_domain`。
    pub stage: String,
    /// 未命中原因；命中时为 null。
    pub no_memory_reason: Option<String>,
    /// 未命中时的固定提示文案；命中时为 null。
    pub no_memory_message: Option<String>,
    /// 注入集条目，只含 statement / why / how_to_apply 与回链用的 memory_id。
    pub injection_set: Vec<MemoryInjectionItem>,
    /// 本次实际返回条数（预算裁剪后）。
    pub returned_count: u32,
    /// 过滤后命中的总条数，供分页判断。
    pub total_matched_count: u32,
    pub offset: u32,
    pub top_k: u32,
    /// `offset + returned_count < total_matched_count` 时为 true。
    pub has_more: bool,
    /// 注入集 token 估算总量。
    pub total_token_count: u32,
    /// 本次生效的注入预算。
    pub token_budget: u32,
}

/// `run_nightly_job` 的响应。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct NightlyJobReport {
    /// 处理日期，形如 `2026-10-20`。
    pub run_date: String,
    /// 扫描到的当天 PE 记录条数。
    pub scanned_record_count: u32,
    /// 与既有记忆条目匹配成功的条数。
    pub matched_entry_count: u32,
    /// 新建的 candidate 条目数。
    pub created_candidate_count: u32,
    /// 因匹配到既有条目而 weight +1 的条目数。
    pub weight_bumped_count: u32,
    /// 检测到矛盾、需要人工裁决的条目数。
    pub conflicted_count: u32,
    /// 挂起待人工裁决的记忆 ID。
    pub pending_review_ids: Vec<String>,
    pub token_usage_before: u32,
    pub token_usage_after: u32,
    pub token_budget: u32,
    /// 是否重建 / 增量更新了 L3 索引。
    pub index_rebuilt: bool,
    pub started_at: String,
    pub finished_at: String,
}

/// 配置的对外视图。
///
/// 与磁盘上持久化的 `AppConfig` 分开：API key 明文永远不出现在这里，
/// 只回一个 `api_key_present` 指示位（契约 §4.5 第 3 条）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct AppConfigView {
    pub provider: Provider,
    /// 线上报文协议。
    pub protocol: Protocol,
    pub endpoint: String,
    pub model_name: String,
    pub embedding_model: String,
    pub optimize_timeout_ms: u64,
    /// 是否存在可用 API key（本地明文或环境变量任一存在）；provider = ollama 时恒为 false。
    pub api_key_present: bool,
    /// 存放 API key 的环境变量名（只存变量名，不存明文）。
    pub api_key_env: String,
    pub memory_token_budget: u32,
    pub dot_click_through: bool,
    /// 唤起面板的全局热键，例如 `Ctrl+Q`。
    pub global_shortcut: String,
    /// 是否已启用开机自启。
    pub autostart_enabled: bool,
    /// 配置文件状态：ok / missing / malformed。
    pub config_status: String,
    /// 还缺 endpoint / 模型名 / API key，改写会失败。
    pub needs_setup: bool,
}

/// 估算一段文本的 token 占用。
///
/// 中文按 1 字 ≈ 1 token，其他字符按 4 字符 ≈ 1 token 估算。
/// 这是近似值而非精确分词结果：memory.md 的 800 token 是硬上限，
/// 用保守估算宁可提前拒绝晋升，也不要写爆预算。
pub fn estimate_tokens(text: &str) -> u32 {
    let mut cjk = 0u32;
    let mut other = 0u32;
    for ch in text.chars() {
        if ('\u{4e00}'..='\u{9fff}').contains(&ch)
            || ('\u{3000}'..='\u{303f}').contains(&ch)
            || ('\u{ff00}'..='\u{ffef}').contains(&ch)
        {
            cjk += 1;
        } else {
            other += 1;
        }
    }
    cjk + other.div_ceil(4)
}

/// 单条注入条目的 token 估算值。
///
/// 只算注入模型的那三个字段：`evidence` / `weight` / `hits` 是治理用的统计字段，
/// 既不进上下文，也不该占 token 预算。
pub fn injection_item_tokens(item: &MemoryInjectionItem) -> u32 {
    estimate_tokens(&format!(
        "{} {} {}",
        item.statement, item.why, item.how_to_apply
    ))
}

/// 注入集整体的 token 估算值（逐条累加）。
pub fn injection_tokens(items: &[MemoryInjectionItem]) -> u32 {
    items.iter().map(injection_item_tokens).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 中文按一字一token估算() {
        assert_eq!(estimate_tokens("提示词优化"), 5);
    }

    #[test]
    fn 英文按四字符一token估算() {
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    #[test]
    fn 空串不占token() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn 注入条目不含统计字段() {
        let item = MemoryInjectionItem {
            memory_id: "M-CODE-001".into(),
            statement: "s".into(),
            why: "w".into(),
            how_to_apply: "h".into(),
        };
        let value = serde_json::to_value(&item).unwrap();
        let mut keys: Vec<String> = value
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.to_string())
            .collect();
        keys.sort();
        assert_eq!(keys, vec!["how_to_apply", "memory_id", "statement", "why"]);
    }
}
