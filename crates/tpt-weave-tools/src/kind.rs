//! Tool kind classification (todo.md Phase 6 "Support: ...").

use serde::{Deserialize, Serialize};

/// The kind of tool output being reduced. Each kind selects a dedicated
/// deterministic reducer (spec.md section 14).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// `cargo check` / rustc-style compiler diagnostics.
    CargoCheck,
    /// `cargo test` harness output.
    CargoTest,
    /// `cargo clippy` lint output.
    CargoClippy,
    /// `cargo build` output.
    CargoBuild,
    /// `cargo fmt --check` / rustfmt diffs.
    CargoFmt,
    /// `git status` (porcelain or long form).
    GitStatus,
    /// `git diff` unified diff output.
    GitDiff,
    /// Directory / file listings (one path per line).
    FileListing,
    /// grep/ripgrep-style search results (`path:line:content`).
    Search,
    /// JSON tool results.
    Json,
    /// Raw compiler diagnostics not tied to a cargo subcommand.
    Diagnostics,
    /// Generic log output.
    Log,
}

impl ToolKind {
    /// Infers the kind from a command line. Unknown commands reduce as
    /// [`ToolKind::Log`] (the generic reducer).
    pub fn infer(command: &str) -> Self {
        let lowered = command.to_ascii_lowercase();
        if lowered.contains("cargo clippy") || lowered.contains("clippy-driver") {
            ToolKind::CargoClippy
        } else if lowered.contains("cargo test") || lowered.contains("nextest") {
            ToolKind::CargoTest
        } else if lowered.contains("cargo fmt") || lowered.contains("rustfmt") {
            ToolKind::CargoFmt
        } else if lowered.contains("cargo check") {
            ToolKind::CargoCheck
        } else if lowered.contains("cargo build") || lowered.contains("cargo rustc") {
            ToolKind::CargoBuild
        } else if lowered.contains("git status") {
            ToolKind::GitStatus
        } else if lowered.contains("git diff") {
            ToolKind::GitDiff
        } else if lowered.contains("cargo ") {
            ToolKind::CargoCheck
        } else if is_search(&lowered) {
            ToolKind::Search
        } else if is_listing(&lowered) {
            ToolKind::FileListing
        } else if lowered.starts_with("jq ") || lowered == "jq" {
            ToolKind::Json
        } else {
            ToolKind::Log
        }
    }
}

fn is_search(lowered: &str) -> bool {
    ["grep ", "rg ", "ripgrep", "ack ", "ag ", "fd -H --search"]
        .iter()
        .any(|needle| lowered.contains(needle))
        || lowered.starts_with("grep")
        || lowered.starts_with("rg")
}

fn is_listing(lowered: &str) -> bool {
    let tokens: Vec<&str> = lowered.split_whitespace().collect();
    matches!(
        tokens.first().copied(),
        Some("ls") | Some("dir") | Some("find") | Some("fd") | Some("tree")
    )
}
