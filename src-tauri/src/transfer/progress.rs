//! Progress reporting for transfer operations via Tauri events.

use super::types::TransferProgress;
use tauri::Emitter;

pub const TRANSFER_PROGRESS_EVENT: &str = "transfer-progress";

pub fn emit_progress(app_handle: &tauri::AppHandle, progress: &TransferProgress) {
    let _ = app_handle.emit(TRANSFER_PROGRESS_EVENT, progress);
}

pub fn create_progress(
    operation: &str,
    phase: &str,
    processed_rows: u64,
    total_rows: Option<u64>,
    elapsed_ms: u64,
) -> TransferProgress {
    let percent = match total_rows {
        Some(total) if total > 0 => (processed_rows as f32 / total as f32) * 100.0,
        _ => 0.0,
    };

    let estimated_remaining_ms = match total_rows {
        Some(total) if processed_rows > 0 && elapsed_ms > 0 && total > processed_rows => {
            let remaining = total - processed_rows;
            let rate = processed_rows as f64 / elapsed_ms as f64;
            if rate > 0.0 {
                Some((remaining as f64 / rate) as u64)
            } else {
                None
            }
        }
        _ => None,
    };

    TransferProgress {
        operation: operation.to_string(),
        phase: phase.to_string(),
        current_table: None,
        total_rows,
        processed_rows,
        skipped_rows: 0,
        error_count: 0,
        percent,
        elapsed_ms,
        estimated_remaining_ms,
        message: None,
    }
}
