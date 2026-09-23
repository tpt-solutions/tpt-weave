//! `tpt-weave context "<task>"` — build task context within the budget.

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use tpt_weave_context::Retriever;
use tpt_weave_core::{ContextLevel, ContextRequest};

pub fn run(
    cli: &Cli,
    task: &str,
    budget: Option<u32>,
    max_level: Option<u8>,
    no_deps: bool,
) -> Result<Rendered, CliError> {
    let workspace = Workspace::load_index(&cli.path)?;
    let manifest = workspace.manifest().cloned();

    let mut request = ContextRequest::new(workspace.repository_name(), task);
    let requested = budget.unwrap_or(0);
    if let Some(manifest) = manifest.as_ref() {
        request = request.with_budget_tokens(manifest.context.clamp(requested));
    } else if requested > 0 {
        request = request.with_budget_tokens(requested);
    }

    if let Some(level) = max_level {
        let level = ContextLevel::from_number(level)
            .ok_or_else(|| CliError::usage("level out of range"))?;
        request = request.with_max_level(level);
    }
    if no_deps {
        request = request.without_dependencies();
    }

    let retriever = Retriever::new(workspace.graph(), workspace.sources());
    let changed = workspace.changed_files();
    let response = retriever.retrieve(&request, &changed)?;

    let mut human = format!(
        "context: {}\ntask: {}\ncandidates: {}\n",
        response.context_id.as_str(),
        request.task,
        response.candidates.len()
    );
    for candidate in &response.candidates {
        let source = match &candidate.source {
            ContextSource::Repository => "repository".to_string(),
            ContextSource::File(path) => format!("file:{path}"),
            ContextSource::Module(module) => format!("module:{module}"),
            ContextSource::Symbol(symbol) => format!("symbol:{}", symbol.canonical_key()),
            ContextSource::Dependency(package) => format!("dependency:{package}"),
            ContextSource::ToolOutput(command) => format!("tool:{command}"),
        };
        human.push_str(&format!(
            "{}\t{}\ttokens={}\tscore={}\n",
            candidate.id.as_str(),
            source,
            candidate.token_estimate,
            candidate
                .score
                .map(|s| format!("{s:.3}"))
                .unwrap_or_else(|| "-".to_string()),
        ));
    }
    human.push_str(&response.tokens.summary());

    // Under --strict-decisions, a remote provider that was expected but is
    // unavailable fails closed (exit 5). Without the flag, deterministic
    // retrieval alone still exits 0 (docs/decisions.md §3).
    if cli.strict_decisions {
        if let Some(manifest) = manifest.as_ref() {
            if manifest.jev.enabled && manifest.privacy.remote_decisions {
                let key_set = std::env::var(&manifest.provider.api_key_env)
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false);
                if !key_set {
                    return Err(CliError::decision(format!(
                        "decision provider key env `{}` is not set",
                        manifest.provider.api_key_env
                    )));
                }
            }
        }
    }

    let json = json!({
        "context_id": response.context_id.as_str(),
        "repository": response.repository.as_str(),
        "task": request.task,
        "budget_tokens": request.budget_tokens,
        "max_level": request.max_level.name(),
        "include_dependencies": request.include_dependencies,
        "candidates": response.candidates,
        "tokens": response.tokens,
        "decision_provider": response.decision_provider,
        "changed_files": changed,
        "summary": response.tokens.summary(),
    });

    let mut rendered = Rendered::new(human.trim_end(), json);
    rendered =
        rendered.with_detail("expand a candidate: tpt-weave expand <context-id> --level skeleton");
    Ok(rendered)
}

use tpt_weave_core::ContextSource;
