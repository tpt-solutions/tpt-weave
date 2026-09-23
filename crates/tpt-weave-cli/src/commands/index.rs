//! `tpt-weave index` — build/refresh `.tpt-weave/graph.json`.

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use tpt_weave_graph::graph_path;

pub fn run(cli: &Cli, full: bool) -> Result<Rendered, CliError> {
    let root = Workspace::canonical_root(&cli.path)?;
    let path = graph_path(&root);

    if !full && path.exists() {
        // Reuse the on-disk graph when HEAD still matches (conservative
        // invalidation); otherwise rebuild.
        if let Ok(existing) = tpt_weave_graph::RepositoryGraph::load(&path) {
            let head_ok = tpt_weave_index::GitRepository::discover(&root)
                .ok()
                .and_then(|git| git.current_revision().ok())
                .map(|rev| rev.sha == existing.revision.sha)
                .unwrap_or(true);
            if head_ok {
                let symbols = existing.symbols.len();
                let files = {
                    let mut paths: Vec<&str> =
                        existing.symbols.iter().map(|s| s.file.as_str()).collect();
                    paths.sort_unstable();
                    paths.dedup();
                    paths.len()
                };
                return Ok(Rendered::new(
                    format!(
                        "index up to date ({} symbols, {} files)",
                        symbols, files
                    ),
                    json!({
                        "path": path.display().to_string(),
                        "rebuilt": false,
                        "symbols": symbols,
                        "files": files,
                        "revision": existing.revision.sha,
                        "repository": existing.repository.as_str(),
                    }),
                )
                .with_detail(format!("revision: {}", existing.revision.short())));
            }
        }
    }

    let workspace = Workspace::index_and_save(&root)?;
    let graph = workspace.graph();
    let mut files: Vec<&str> = graph.symbols.iter().map(|s| s.file.as_str()).collect();
    files.sort_unstable();
    files.dedup();
    let symbols = graph.symbols.len();
    let references = graph.references.len();

    Ok(Rendered::new(
        format!(
            "indexed {} symbols in {} files -> {}",
            symbols,
            files.len(),
            path.display()
        ),
        json!({
            "path": path.display().to_string(),
            "rebuilt": true,
            "full": full,
            "symbols": symbols,
            "files": files.len(),
            "references": references,
            "modules": graph.modules.len(),
            "revision": graph.revision.sha,
            "repository": graph.repository.as_str(),
            "unresolved_mentions": graph.unresolved_mentions,
        }),
    )
    .with_detail(format!(
        "revision: {}\npackages: {}",
        graph.revision.short(),
        workspace.repository_name()
    )))
}
