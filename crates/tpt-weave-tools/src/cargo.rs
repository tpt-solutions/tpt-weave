//! cargo check / test / clippy / build / fmt reducers (spec.md section 14).

use crate::reduce::{exit_status, push_capped};
use crate::reduction::ReductionStatus;

const DIAGNOSTIC_CAP: usize = 20;

/// Reduces `cargo test` harness output; falls back to diagnostics when the
/// run never produced a test result (e.g. compile failure).
pub(crate) fn test_output(text: &str, exit_code: i32) -> (Vec<String>, ReductionStatus) {
    let is_harness = text.contains("test result:") || text.contains("failures:");
    if !is_harness {
        return diagnostic_output(text, "TEST", exit_code);
    }

    let mut passed = 0u64;
    let mut failed = 0u64;
    let mut ignored = 0u64;
    let mut harnesses = 0u64;
    let mut failure_names: Vec<String> = Vec::new();
    let mut in_failures = false;
    let mut in_detail = false;

    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(rest) = line.trim_start().strip_prefix("test result:") {
            harnesses += 1;
            passed += count_before(rest, "passed");
            failed += count_before(rest, "failed");
            ignored += count_before(rest, "ignored");
            in_failures = false;
            in_detail = false;
        } else if line.trim_end() == "failures:" {
            // The second `failures:` block lists names; the first is
            // followed by `---- name stdout ----` detail sections.
            in_failures = true;
            in_detail = false;
        } else if in_failures && line.starts_with("---- ") {
            in_detail = true;
        } else if in_failures && !in_detail {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if line.starts_with(' ') || line.starts_with('\t') {
                failure_names.push(trimmed.to_string());
            } else {
                in_failures = false;
            }
        }
    }

    let total = passed + failed;
    let success = failed == 0 && exit_code == 0;
    let mut lines = Vec::new();
    lines.push(if success { "TEST OK" } else { "TEST FAILURE" }.to_string());
    lines.push(format!("tests: {total}"));
    lines.push(format!("passed: {passed}"));
    lines.push(format!("failed: {failed}"));
    if ignored > 0 {
        lines.push(format!("ignored: {ignored}"));
    }
    if harnesses > 1 {
        lines.push(format!("harnesses: {harnesses}"));
    }

    let status = if failed > 0 {
        ReductionStatus::Failure {
            reason: format!("{failed} tests failed"),
        }
    } else {
        exit_status(exit_code, text)
    };

    if !failure_names.is_empty() {
        lines.push("FAILURES:".to_string());
        push_capped(&mut lines, failure_names, DIAGNOSTIC_CAP);
    }

    (lines, status)
}

/// Parses `5 passed; 0 failed` style counts ("up to the count word").
fn count_before(rest: &str, word: &str) -> u64 {
    let mut total = 0u64;
    for chunk in rest.split(';') {
        let chunk = chunk.trim();
        let Some(stripped) = chunk.strip_suffix(word) else {
            continue;
        };
        let number = stripped
            .split_whitespace()
            .next_back()
            .and_then(|token| token.parse::<u64>().ok());
        if let Some(value) = number {
            total += value;
        }
    }
    total
}

/// One parsed rustc-style diagnostic.
struct Diagnostic {
    severity: Severity,
    code: Option<String>,
    message: String,
    location: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Severity {
    Error,
    Warning,
}

/// Reduces rustc/clippy diagnostics (`header` names the tool: CHECK,
/// CLIPPY, BUILD, DIAGNOSTICS, TEST).
pub(crate) fn diagnostic_output(
    text: &str,
    header: &str,
    exit_code: i32,
) -> (Vec<String>, ReductionStatus) {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut current: Option<Diagnostic> = None;

    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(after) = line.trim_start().strip_prefix("--> ") {
            if let Some(diag) = current.as_mut() {
                if diag.location.is_none() {
                    diag.location = parse_location(after);
                }
            }
            continue;
        }
        if let Some(diag) = parse_diagnostic_header(line) {
            if let Some(previous) = current.replace(diag) {
                diagnostics.push(previous);
            }
        }
    }
    if let Some(diag) = current {
        diagnostics.push(diag);
    }

    let errors = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();

    let success = errors == 0 && exit_code == 0;
    let mut lines = Vec::new();
    lines.push(format!(
        "{header} {}",
        if success { "OK" } else { "FAILURE" }
    ));
    lines.push(format!("errors: {errors}"));
    lines.push(format!("warnings: {warnings}"));

    if !diagnostics.is_empty() {
        lines.push("DIAGNOSTICS:".to_string());
        let rendered = diagnostics.iter().map(|d| {
            let severity = match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            let code = d
                .code
                .as_ref()
                .map(|c| format!("[{c}]"))
                .unwrap_or_default();
            let location = d
                .location
                .as_ref()
                .map(|l| format!(" ({l})"))
                .unwrap_or_default();
            format!("{severity}{code}: {}{location}", d.message)
        });
        push_capped(&mut lines, rendered, DIAGNOSTIC_CAP);
    }

    let status = if errors > 0 {
        ReductionStatus::Failure {
            reason: format!("{errors} errors"),
        }
    } else {
        exit_status(exit_code, text)
    };
    (lines, status)
}

/// Parses `error[E0308]: message` / `warning: message` line starts.
fn parse_diagnostic_header(line: &str) -> Option<Diagnostic> {
    let severity = if line.starts_with("error") {
        Severity::Error
    } else if line.starts_with("warning") {
        Severity::Warning
    } else {
        return None;
    };
    let rest = line.trim_start();
    let rest = rest
        .strip_prefix("error")
        .or_else(|| rest.strip_prefix("warning"))?;
    // `rest` is now `:` / `[CODE]:` then the message.
    let (code, message) = if let Some(after) = rest.strip_prefix('[') {
        let (code, tail) = after.split_once(']')?;
        let tail = tail.strip_prefix(':')?;
        (Some(code.to_string()), tail.trim().to_string())
    } else {
        let tail = rest.strip_prefix(':')?;
        (None, tail.trim().to_string())
    };
    if message.is_empty() {
        return None;
    }
    Some(Diagnostic {
        severity,
        code,
        message,
        location: None,
    })
}

/// Parses `src/lib.rs:10:5` or `src/lib.rs:10` after `-->`, returning
/// `path:line`.
fn parse_location(after_arrow: &str) -> Option<String> {
    // `path:line:col` first (the usual rustc shape).
    if let Some(colon) = after_arrow.rfind(':') {
        let line_col = &after_arrow[colon + 1..];
        if let Some(line_end) = after_arrow[..colon].rfind(':') {
            let path = &after_arrow[..line_end];
            let line = &after_arrow[line_end + 1..colon];
            if is_digits(line) && is_digits(line_col) && !path.is_empty() {
                return Some(format!("{path}:{line}"));
            }
        }
        // `path:line` (no column).
        let path = &after_arrow[..colon];
        let line = line_col;
        if is_digits(line) && !path.is_empty() {
            return Some(format!("{path}:{line}"));
        }
    }
    None
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

/// Reduces `cargo fmt --check` / rustfmt diff output.
pub(crate) fn fmt_output(text: &str, exit_code: i32) -> (Vec<String>, ReductionStatus) {
    let mut files: Vec<String> = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("Diff in ") else {
            continue;
        };
        let path = if let Some(at) = rest.find(" at line ") {
            rest[..at].trim().to_string()
        } else {
            rest.trim_end_matches(':').trim().to_string()
        };
        if !path.is_empty() && !files.contains(&path) {
            files.push(path);
        }
    }

    let count = files.len();
    let success = count == 0 && exit_code == 0;
    let mut lines = Vec::new();
    lines.push(format!("FMT {}", if success { "OK" } else { "FAILURE" }));
    lines.push(format!("files needing format: {count}"));
    if count > 0 {
        lines.push("FORMAT:".to_string());
        push_capped(&mut lines, files, DIAGNOSTIC_CAP);
    }

    let status = if count > 0 {
        ReductionStatus::Failure {
            reason: format!("{count} files need formatting"),
        }
    } else {
        exit_status(exit_code, text)
    };
    (lines, status)
}
