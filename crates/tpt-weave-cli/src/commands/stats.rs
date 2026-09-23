//! `tpt-weave stats` — token accounting + cache statistics.

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use tpt_weave_cache::FilesystemCache;
use tpt_weave_context::Retriever;
use tpt_weave_core::{ContextLevel, ContextRequest};

pub fn run(cli: &Cli) -> Result<Rendered, CliError> {
    let workspace = Workspace::load_index(&cli.path)?;
    let retriever = Retriever::new(workspace.graph(), workspace.sources());
    let overview = retriever.repository_overview();

    // Accounting for a default empty-task selection (overview + changed files).
    let mut request = ContextRequest::new(workspace.repository_name(), "");
    if let Some(manifest) = workspace.manifest() {
        request = request.with_budget_tokens(manifest.context.clamp(0));
        request = request.with_max_level(ContextLevel::Skeleton);
    }
    let changed = workspace.changed_files();
    let response = retriever.retrieve(&request, &changed)?;

    let cache = FilesystemCache::for_repository(workspace.root());
    let cache_stats = cache.statistics()?;

    let human = format!(
        "{}\n\ncontext (empty task):\n  candidates: {}\n  {}\n\ncache:\n  entries: {}\n  bytes: {}\n  hit_rate: {:.3}\n  hits: {}\n  misses: {}\n  stores: {}\n",
        overview.text(),
        response.candidates.len(),
        response.tokens.summary().replace('\n', "\n  "),
        cache_stats.entries,
        cache_stats.bytes,
        cache_stats.hit_rate(),
        cache_stats.hits,
        cache_stats.misses,
        cache_stats.stores,
    );

    let json = json!({
        "overview": {
            "repository": overview.repository.as_str(),
            "revision": overview.revision.sha,
            "files": overview.files,
            "symbols": overview.symbols,
            "public_symbols": overview.public_symbols,
            "modules": overview.modules,
            "dependencies": overview.dependencies,
            "token_estimate": overview.token_estimate(),
        },
        "context": {
            "context_id": response.context_id.as_str(),
            "candidates": response.candidates.len(),
            "tokens": response.tokens,
        },
        "cache": cache_stats,
    });

    Ok(Rendered::new(human.trim_end(), json))
}
