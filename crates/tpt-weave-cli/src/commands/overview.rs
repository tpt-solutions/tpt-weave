//! `tpt-weave overview` — level-0 repository overview.

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use tpt_weave_context::Retriever;

pub fn run(cli: &Cli) -> Result<Rendered, CliError> {
    let workspace = Workspace::load_index(&cli.path)?;
    let retriever = Retriever::new(workspace.graph(), workspace.sources());
    let overview = retriever.repository_overview();

    let human = overview.text();
    let json = json!({
        "repository": overview.repository.as_str(),
        "revision": overview.revision.sha,
        "revision_short": overview.revision.short(),
        "crates": overview.crates,
        "files": overview.files,
        "symbols": overview.symbols,
        "public_symbols": overview.public_symbols,
        "modules": overview.modules,
        "dependencies": overview.dependencies,
        "external_repositories": overview
            .external_repositories
            .iter()
            .map(|r| r.as_str().to_string())
            .collect::<Vec<_>>(),
        "token_estimate": overview.token_estimate(),
        "text": human,
    });
    Ok(Rendered::new(human, json))
}
