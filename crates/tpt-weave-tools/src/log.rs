//! Generic log reduction (spec.md section 14: "logs").

use crate::reduce::{exit_status, push_capped};
use crate::reduction::ReductionStatus;

const ERROR_CAP: usize = 20;

/// Reduces generic logs: totals plus every error line (capped), keeping
/// the failure detail requestable from the stored raw output.
pub(crate) fn log_output(text: &str, exit_code: i32) -> (Vec<String>, ReductionStatus) {
    let mut total = 0u64;
    let mut errors: Vec<String> = Vec::new();
    let mut error_count = 0u64;
    let mut warn_count = 0u64;

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        total += 1;
        if is_error(line) {
            error_count += 1;
            errors.push(line.to_string());
        } else if is_warn(line) {
            warn_count += 1;
        }
    }

    let mut lines = vec![
        "LOG".to_string(),
        format!("lines: {total}"),
        format!("errors: {error_count}"),
        format!("warnings: {warn_count}"),
    ];
    if error_count > 0 {
        lines.push("ERRORS:".to_string());
        push_capped(&mut lines, errors, ERROR_CAP);
    }
    (lines, exit_status(exit_code, text))
}

/// Conservative error detectors: common logger prefixes and structured
/// `level=error` fields (lowercase paths must not false-positive).
fn is_error(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("ERROR")
        || trimmed.starts_with("Error ")
        || trimmed.starts_with("[ERROR]")
        || trimmed.starts_with("[error]")
        || trimmed.contains(" ERROR ")
        || trimmed.contains("level=error")
        || trimmed.contains("level=ERROR")
}

/// Conservative warning detectors matching [`is_error`]'s shape.
fn is_warn(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("WARN")
        || trimmed.starts_with("[WARN]")
        || trimmed.starts_with("[warning]")
        || trimmed.contains(" WARN ")
        || trimmed.contains(" WARNING ")
        || trimmed.contains("level=warn")
        || trimmed.contains("level=WARN")
}
