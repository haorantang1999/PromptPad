//! 无业务语义的时间原语（基础设施层）。
//!
//! 单独成模块的原因：`storage::db` 是数据层，接入层与应用层都不该为了取「今天是哪天」
//! 而伸手进数据层。把这些纯函数提到基础设施层后，接入层、服务层、数据层都从同一处取时间，
//! 依赖方向才是自上而下的。
//!
//! 一律使用 UTC：AGENTS.md §4 要求的 `YYYY-MM-DD` / ISO-8601 都在 UTC 下生成，
//! 避免本机时区把「当天」算错一天。

/// 当前时间的 ISO-8601 字符串（秒精度）。
pub fn now_iso() -> String {
    format_utc(now_secs())
}

/// 今天的日期（UTC），形如 `2026-10-21`。
pub fn today_date() -> String {
    format_utc(now_secs()).chars().take(10).collect()
}

/// 昨天的日期（UTC）。夜间任务默认处理已结束的一天，避免把当天未写完的 PE 算进去。
pub fn yesterday_date() -> String {
    format_utc(now_secs().saturating_sub(86_400))
        .chars()
        .take(10)
        .collect()
}

/// 当前年份（UTC）。
pub fn current_year() -> i64 {
    let days = (now_secs() / 86_400) as i64;
    civil_from_days(days).0
}

/// 当前时间戳（秒）。
fn now_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 极简 UTC 时间格式化，避免为时间格式引入额外依赖。
fn format_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// 由「距 1970-01-01 的天数」换算公历日期（Howard Hinnant 的 civil_from_days 算法）。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 日期换算正确() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
    }

    #[test]
    fn iso_串以日期开头() {
        let iso = now_iso();
        assert_eq!(iso.len(), 20);
        assert!(iso.ends_with('Z'));
        assert_eq!(iso.chars().take(10).count(), 10);
    }
}
