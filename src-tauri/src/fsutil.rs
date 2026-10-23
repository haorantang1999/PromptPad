//! 本机文件原子写。
//!
//! Windows 上固定名的 `*.tmp` 若上次崩溃残留且被占用，下一次 `fs::write` /
//! `fs::rename` 会直接失败，前端只会看到笼统的 `storage_write_failed`。
//! 这里用「进程内唯一临时名 + rename，失败再 copy 覆盖」把这条路径兜住。

use crate::error::AppError;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 原子写入：先落到同目录唯一临时文件，再替换目标。
pub fn write_atomic(path: &Path, body: impl AsRef<[u8]>) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp = unique_tmp_path(path);
    fs::write(&tmp, body.as_ref()).map_err(|err| {
        let _ = fs::remove_file(&tmp);
        AppError::storage_failure("写入临时文件失败").with_details(err.to_string())
    })?;

    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(rename_err) => {
            // rename 在目标被占用 / 部分 Windows 场景下会拒替换：退回 copy 覆盖。
            match fs::copy(&tmp, path) {
                Ok(_) => {
                    let _ = fs::remove_file(&tmp);
                    Ok(())
                }
                Err(copy_err) => {
                    let _ = fs::remove_file(&tmp);
                    Err(AppError::storage_failure("替换目标文件失败").with_details(format!(
                        "rename={rename_err}; copy={copy_err}; path={}",
                        path.display()
                    )))
                }
            }
        }
    }
}

/// 清理目录里残留的原子写临时文件（上次崩溃 / 杀软拦截留下的半截）。
pub fn cleanup_stale_tmps(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0usize;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        // 匹配 `foo.md.tmp` / `config.json.tmp` / `.#foo.123.456.tmp`
        if name.ends_with(".tmp") || name.contains(".tmp.") {
            if fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

fn unique_tmp_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    parent.join(format!(".#{stem}.{}.{}.tmp", std::process::id(), nanos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn 原子写可覆盖已存在文件() {
        let dir = std::env::temp_dir().join("prompt_pad_fsutil_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.json");
        write_atomic(&path, b"{\"a\":1}").unwrap();
        write_atomic(&path, b"{\"a\":2}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":2}");
        assert_eq!(cleanup_stale_tmps(&dir), 0);
        let _ = fs::remove_dir_all(&dir);
    }
}
