//! `tpt-weave index` — build/refresh `.tpt-weave/graph.json`.

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use std::time::Instant;
use tpt_weave_graph::graph_path;

pub fn run(cli: &Cli, full: bool) -> Result<Rendered, CliError> {
    let root = Workspace::canonical_root(&cli.path)?;
    let path = graph_path(&root);
    let started = Instant::now();

    if !full && path.exists() {
        // Reuse the on-disk graph only when HEAD and the working tree are
        // unchanged. Uncommitted source edits must enter the incremental path;
        // non-git trees have no reliable HEAD, so rebuild conservatively.
        if let Ok(existing) = tpt_weave_graph::RepositoryGraph::load(&path) {
            let head_ok = tpt_weave_index::GitRepository::discover(&root)
                .ok()
                .map(|git| {
                    git.current_revision()
                        .map(|rev| rev.sha == existing.revision.sha)
                        .unwrap_or(false)
                        && git
                            .status()
                            .map(|status| status.is_empty())
                            .unwrap_or(false)
                })
                .unwrap_or(false);
            let manifest_ok = !Workspace::manifest_newer_than_graph(&root, &path);
            if head_ok && manifest_ok {
                let symbols = existing.symbols.len();
                let files = {
                    let mut paths: Vec<&str> =
                        existing.symbols.iter().map(|s| s.file.as_str()).collect();
                    paths.sort_unstable();
                    paths.dedup();
                    paths.len()
                };
                let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
                return Ok(Rendered::new(
                    format!(
                        "index up to date ({} symbols, {} files, {:.1} ms)",
                        symbols, files, elapsed_ms
                    ),
                    json!({
                        "path": path.display().to_string(),
                        "rebuilt": false,
                        "symbols": symbols,
                        "files": files,
                        "revision": existing.revision.sha,
                        "repository": existing.repository.as_str(),
                        "elapsed_ms": elapsed_ms,
                    }),
                )
                .with_detail(format!("revision: {}", existing.revision.short())));
            }
        }
    }

    let workspace = Workspace::index_and_save(&root)?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    let graph = workspace.graph();
    let parse_cache = workspace.parse_cache_stats();
    let mut files: Vec<&str> = graph.symbols.iter().map(|s| s.file.as_str()).collect();
    files.sort_unstable();
    files.dedup();
    let symbols = graph.symbols.len();
    let references = graph.references.len();

    Ok(Rendered::new(
        format!(
            "indexed {} symbols in {} files -> {} ({:.1} ms)",
            symbols,
            files.len(),
            path.display(),
            elapsed_ms
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
            "parse_cache": parse_cache,
            "elapsed_ms": elapsed_ms,
        }),
    )
    .with_detail(format!(
        "revision: {}\npackages: {}\nparse cache: {} hits / {} misses\nelapsed: {:.1} ms",
        graph.revision.short(),
        workspace.repository_name(),
        parse_cache.hits,
        parse_cache.misses,
        elapsed_ms
    )))
}
