//! Reduction entry point and shared helpers (todo.md Phase 6).

use crate::cargo;
use crate::git;
use crate::json;
use crate::kind::ToolKind;
use crate::listing;
use crate::log;
use crate::reduction::{Reduction, ReductionStatus};
use crate::search;
use serde::{Deserialize, Serialize};
use tpt_weave_context::estimate_tokens;

/// Raw capture of one tool invocation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolOutput {
    /// The command line that produced the output.
    pub command: String,
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Process exit code.
    pub exit_code: i32,
}

impl ToolOutput {
    /// Captures stdout only, with exit code 0.
    pub fn new(command: impl Into<String>, stdout: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            stdout: stdout.into(),
            stderr: String::new(),
            exit_code: 0,
        }
    }

    /// Attaches stderr.
    pub fn with_stderr(mut self, stderr: impl Into<String>) -> Self {
        self.stderr = stderr.into();
        self
    }

    /// Attaches the exit code.
    pub fn with_exit_code(mut self, exit_code: i32) -> Self {
        self.exit_code = exit_code;
        self
    }

    /// Combined text reducers parse: stdout, plus stderr when present.
    pub fn combined(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }

    /// Content persisted by [`RawStore`](crate::RawStore): stdout and
    /// stderr, separated when both are present.
    pub fn stored_content(&self) -> String {
        self.combined()
    }
}

/// Deterministically reduces raw tool output to a compact record
/// (spec.md section 14, todo.md Phase 6).
pub fn reduce(kind: ToolKind, output: &ToolOutput) -> Reduction {
    let text = output.combined();
    let (mut lines, status) = match kind {
        ToolKind::CargoTest => cargo::test_output(&text, output.exit_code),
        ToolKind::CargoCheck => cargo::diagnostic_output(&text, "CHECK", output.exit_code),
        ToolKind::CargoClippy => cargo::diagnostic_output(&text, "CLIPPY", output.exit_code),
        ToolKind::CargoBuild => cargo::diagnostic_output(&text, "BUILD", output.exit_code),
        ToolKind::Diagnostics => cargo::diagnostic_output(&text, "DIAGNOSTICS", output.exit_code),
        ToolKind::CargoFmt => cargo::fmt_output(&text, output.exit_code),
        ToolKind::GitStatus => git::status_output(&text, output.exit_code),
        ToolKind::GitDiff => git::diff_output(&text, output.exit_code),
        ToolKind::FileListing => listing::listing_output(&text, output.exit_code),
        ToolKind::Search => search::search_output(&text, output.exit_code),
        ToolKind::Json => json::json_output(&text, output.exit_code),
        ToolKind::Log => log::log_output(&text, output.exit_code),
    };

    // The raw output always remains with the caller, who can store it.
    lines.push(
        if text.trim().is_empty() {
            "DETAIL AVAILABLE: no"
        } else {
            "DETAIL AVAILABLE: yes"
        }
        .to_string(),
    );

    let raw_tokens = u64::from(estimate_tokens(&text));
    let rendered = lines.join("\n");
    let reduced_tokens = u64::from(estimate_tokens(&rendered));

    Reduction {
        kind,
        command: output.command.clone(),
        exit_code: output.exit_code,
        status,
        lines,
        raw_tokens,
        reduced_tokens,
        detail: None,
    }
}

/// Fallback failure reason: the last non-empty line of the output,
/// otherwise the exit code.
pub(crate) fn tail_reason(text: &str, exit_code: i32) -> String {
    text.lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .map(|line| {
            const MAX: usize = 160;
            if line.chars().count() > MAX {
                let truncated: String = line.chars().take(MAX).collect();
                format!("{truncated}…")
            } else {
                line.to_string()
            }
        })
        .unwrap_or_else(|| format!("exit code {exit_code}"))
}

/// Exit-code status for reducers that do not judge pass/fail themselves.
pub(crate) fn exit_status(exit_code: i32, text: &str) -> ReductionStatus {
    if exit_code == 0 {
        ReductionStatus::Success
    } else {
        ReductionStatus::Failure {
            reason: tail_reason(text, exit_code),
        }
    }
}

/// Appends at most `cap` items, then a `... +N more` marker.
pub(crate) fn push_capped<I>(out: &mut Vec<String>, items: I, cap: usize)
where
    I: IntoIterator<Item = String>,
{
    let mut skipped = 0usize;
    let mut kept = 0usize;
    for item in items {
        if kept < cap {
            out.push(item);
            kept += 1;
        } else {
            skipped += 1;
        }
    }
    if skipped > 0 {
        out.push(format!("... +{skipped} more"));
    }
}
