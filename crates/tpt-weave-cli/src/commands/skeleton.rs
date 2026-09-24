//! `tpt-weave skeleton <file>` — one file's hierarchical representation
//! (default: level-3 skeleton; mirrors the MCP `tpt_get_skeleton` tool).

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use tpt_weave_context::{Selection, SourceProvider, represent_file};
use tpt_weave_core::ContextLevel;

pub fn run(cli: &Cli, path: &str, level: Option<u8>) -> Result<Rendered, CliError> {
    let workspace = Workspace::load_index(&cli.path)?;
    let level = ContextLevel::from_number(level.unwrap_or(ContextLevel::Skeleton.as_number()))
        .ok_or_else(|| CliError::usage("level out of range"))?;

    let normalised = path.replace('\\', "/");
    if SourceProvider::source(workspace.sources(), &normalised).is_none() {
        return Err(CliError::not_found(format!(
            "file not in repository sources: {path}"
        )));
    }

    let representation = represent_file(
        workspace.graph(),
        workspace.sources(),
        &normalised,
        level,
        &Selection::new(),
    )?;

    let human = format!(
        "--- {} (level: {}, tokens: {}) ---\n{}",
        representation.path,
        representation.level.name(),
        representation.token_estimate,
        representation.text
    );
    let json = json!({
        "path": representation.path,
        "level": representation.level.name(),
        "token_estimate": representation.token_estimate,
        "text": representation.text,
    });
    Ok(Rendered::new(human, json))
}
