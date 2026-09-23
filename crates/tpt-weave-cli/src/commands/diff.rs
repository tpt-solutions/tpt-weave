//! `tpt-weave diff` — reduced working-tree diff.

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use std::process::Command as ProcessCommand;
use tpt_weave_tools::{ToolKind, ToolOutput, reduce};

pub fn run(cli: &Cli) -> Result<Rendered, CliError> {
    let root = Workspace::canonical_root(&cli.path)?;
    let git = tpt_weave_index::GitRepository::discover(&root).map_err(|error| {
        CliError::not_found(format!(
            "not a git repository ({}): {error}",
            root.display()
        ))
    })?;

    // Unified diff of tracked changes against HEAD (including staged).
    let output = ProcessCommand::new("git")
        .args(["-c", "core.quotepath=false", "diff", "HEAD"])
        .current_dir(git.root())
        .output()
        .map_err(|e| CliError::internal(format!("failed to run git diff: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let exit_code = output.status.code().unwrap_or(1);
    let tool = ToolOutput::new("git diff HEAD", stdout)
        .with_stderr(stderr)
        .with_exit_code(exit_code);
    let reduction = reduce(ToolKind::GitDiff, &tool);

    let human = format!(
        "{}\nraw_tokens: {}\nreduced_tokens: {}\nreduction: {:.1}%",
        reduction.render(),
        reduction.raw_tokens,
        reduction.reduced_tokens,
        reduction.reduction() * 100.0
    );

    let json = json!({
        "command": reduction.command,
        "exit_code": reduction.exit_code,
        "status": reduction.status,
        "lines": reduction.lines,
        "raw_tokens": reduction.raw_tokens,
        "reduced_tokens": reduction.reduced_tokens,
        "reduction": reduction.reduction(),
        "failed": reduction.failed(),
    });

    let mut rendered = Rendered::new(human, json);
    if !tool.stderr.trim().is_empty() {
        rendered = rendered.with_detail(tool.stderr);
    }
    Ok(rendered)
}
