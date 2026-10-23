pub mod config_cmd;
pub mod dot;
pub mod memory;
pub mod optimize;
pub mod panel;
pub mod record;

use crate::error::AppError;
use serde_json::json;
use std::sync::{Mutex, MutexGuard};

/// 取锁。锁中毒说明之前有线程在持锁时 panic 了，此时状态不可信，直接报错而不是继续用。
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, AppError> {
    mutex.lock().map_err(|_| {
        AppError::lock_conflict("正在执行另一个写操作，请稍后重试")
            .with_details(json!({ "holder": "internal_state", "retryable": true }))
    })
}
