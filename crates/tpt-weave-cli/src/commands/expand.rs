//! `tpt-weave expand <context-id>` — next representation level / full source.

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use tpt_weave_context::{
    Expansion, Retriever, Selection, expand_dependency, expand_module, expand_related,
    expand_symbol, expand_test, represent_file,
};
use tpt_weave_core::{ContextLevel, ContextSource};

pub fn run(
    cli: &Cli,
    id: &str,
    level: Option<u8>,
    kind: Option<&str>,
    package: Option<&str>,
) -> Result<Rendered, CliError> {
    let workspace = Workspace::load_index(&cli.path)?;
    let level = ContextLevel::from_number(level.unwrap_or(ContextLevel::Full.as_number()))
        .ok_or_else(|| CliError::usage("level out of range"))?;

    // Prefer resolving a `ctx:` id through the last context selection when
    // one exists; otherwise treat the argument as a subject key.
    if id.starts_with("ctx:") {
        if let Some(rendered) = expand_context_id(&workspace, cli, id, level)? {
            return Ok(rendered);
        }
    }

    let sources = workspace.sources();
    let graph = workspace.graph();
    let kind = kind.unwrap_or("symbol");

    let expansion = match kind {
        "symbol" => expand_symbol(graph, sources, id, level)?,
        "module" => {
            let package = package
                .ok_or_else(|| CliError::usage("`--kind module` requires `--package <name>`"))?;
            expand_module(graph, sources, package, id, level)?
        }
        "dependency" => expand_dependency(graph, sources, id, level)?,
        "test" => expand_test(graph, sources, id, level)?,
        "related" => expand_related(graph, sources, id, level)?,
        "file" => {
            // Render one file directly at the requested level.
            let representation = represent_file(graph, sources, id, level, &Selection::new())?;
            let human = format!(
                "--- {} (tokens: {}) ---\n{}",
                representation.path, representation.token_estimate, representation.text
            );
            let json = json!({
                "subject": id,
                "kind": "file",
                "level": level.name(),
                "files": [{
                    "path": representation.path,
                    "level": representation.level.name(),
                    "token_estimate": representation.token_estimate,
                    "text": representation.text,
                }],
                "tokens": representation.token_estimate,
            });
            return Ok(Rendered::new(human, json));
        }
        other => {
            return Err(CliError::usage(format!(
                "unknown expand kind `{other}` (symbol|module|dependency|test|related|file)"
            )));
        }
    };

    Ok(render_expansion(&expansion))
}

fn render_expansion(expansion: &Expansion) -> Rendered {
    let mut human = format!(
        "subject: {}\nlevel: {}\nsymbols: {}\ntokens: {}\n",
        expansion.subject,
        expansion.level.name(),
        expansion.symbols.len(),
        expansion.total_tokens(),
    );
    for file in &expansion.files {
        human.push_str(&format!(
            "--- {} (tokens: {}) ---\n{}\n",
            file.path, file.token_estimate, file.text
        ));
    }

    let json = json!({
        "subject": expansion.subject,
        "level": expansion.level.name(),
        "repository": expansion.repository.as_ref().map(|r| r.as_str().to_string()),
        "symbols": expansion.symbols.iter().map(super::symbol::record_json).collect::<Vec<_>>(),
        "files": expansion.files.iter().map(|file| json!({
            "path": file.path,
            "level": file.level.name(),
            "token_estimate": file.token_estimate,
            "text": file.text,
        })).collect::<Vec<_>>(),
        "tokens": expansion.total_tokens(),
    });
    Rendered::new(human.trim_end(), json)
}

/// Resolves a `ctx:` id against candidates already present in the graph by
/// reconstructing candidate ids from their sources (deterministic FNV keys).
/// Returns `Ok(None)` when the id does not match a known source form, so the
/// caller can fall back to subject-key expansion.
fn expand_context_id(
    workspace: &Workspace,
    _cli: &Cli,
    id: &str,
    level: ContextLevel,
) -> Result<Option<Rendered>, CliError> {
    let graph = workspace.graph();
    let sources = workspace.sources();
    let retriever = Retriever::new(graph, sources);

    // Rebuild the overview candidate (always present in every selection).
    let overview = retriever.repository_overview();
    let overview_candidate = tpt_weave_core::ContextCandidate::new(
        ContextSource::Repository,
        ContextLevel::Metadata,
        overview.token_estimate(),
    );
    if overview_candidate.id.as_str() == id {
        let human = overview.text();
        return Ok(Some(Rendered::new(
            human.clone(),
            json!({
                "context_id": id,
                "subject": "repository",
                "level": ContextLevel::Metadata.name(),
                "text": human,
                "token_estimate": overview.token_estimate(),
            }),
        )));
    }

    // Try file candidates: `file:<path>` at each level.
    let mut paths: Vec<String> = graph.symbols.iter().map(|s| s.file.clone()).collect();
    paths.sort();
    paths.dedup();
    for path in &paths {
        for candidate_level in ContextLevel::ALL {
            let candidate = tpt_weave_core::ContextCandidate::new(
                ContextSource::File(path.clone()),
                candidate_level,
                0,
            );
            if candidate.id.as_str() == id {
                let representation = represent_file(
                    graph,
                    sources,
                    path,
                    level.max(candidate_level),
                    &Selection::new(),
                )?;
                let human = format!(
                    "--- {} (tokens: {}) ---\n{}",
                    representation.path, representation.token_estimate, representation.text
                );
                return Ok(Some(Rendered::new(
                    human.clone(),
                    json!({
                        "context_id": id,
                        "subject": path,
                        "kind": "file",
                        "level": representation.level.name(),
                        "token_estimate": representation.token_estimate,
                        "text": representation.text,
                    }),
                )));
            }
        }
    }

    // Symbol candidates.
    for record in &graph.symbols {
        let key = record.id.canonical_key();
        for candidate_level in ContextLevel::ALL {
            let candidate = tpt_weave_core::ContextCandidate::new(
                ContextSource::Symbol(record.id.clone()),
                candidate_level,
                0,
            );
            if candidate.id.as_str() == id {
                let expansion = expand_symbol(graph, sources, &key, level)?;
                return Ok(Some(render_expansion(&expansion)));
            }
        }
    }

    Ok(None)
}
