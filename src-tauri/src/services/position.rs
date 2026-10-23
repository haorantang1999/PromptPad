//! DOT 位置持久化。
//!
//! 持久化的是「哪块屏 + 屏内百分比坐标」而不是绝对像素：
//! 换分辨率、插拔扩展屏、拖动任务栏之后，绝对像素会让 DOT 飘到屏幕外。

use crate::error::AppError;
use crate::geometry::Anchor;
use std::fs;
use std::path::Path;

/// 读取锚点。文件缺失或损坏时回落到默认锚点（屏幕右侧中部），
/// 不因为位置文件坏了就让 Pad 起不来。
pub fn load_anchor(path: &Path) -> Anchor {
    let Ok(raw) = fs::read_to_string(path) else {
        return Anchor::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

/// 原子写回锚点：先写临时文件再改名。
pub fn save_anchor(anchor: &Anchor, path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(anchor)?)?;
    fs::rename(&tmp, path)?;
    Ok(())
}
