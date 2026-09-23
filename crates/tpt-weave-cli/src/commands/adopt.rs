//! `tpt-weave adopt` — onboard: init (+ .gitignore) + index --full +
//! register + baseline benchmark + doctor (spec.md section 28).

use super::{Rendered, doctor, index, init};
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use std::path::PathBuf;
use tpt_weave_core::{ContextLevel, DEFAULT_CONTEXT_BUDGET, MANIFEST_DIR};
use tpt_weave_eval::{HarnessError, local_comparison, local_report};

/// Report artefact written by the adopt baseline benchmark.
pub const BASELINE_REPORT_FILE: &str = "baseline-report.json";

pub fn run(cli: &Cli) -> Result<Rendered, CliError> {
    let init_rendered = init::run(cli, false)?;
    let index_rendered = index::run(cli, true)?;
    let register_rendered = register(cli)?;
    let benchmark_rendered = benchmark(cli)?;
    let doctor_rendered = doctor::run(cli)?;

    let steps = json!({
        "init": init_rendered.json,
        "index": index_rendered.json,
        "register": register_rendered.json,
        "benchmark": benchmark_rendered.json,
        "doctor": doctor_rendered.json,
    });
    let human = format!(
        "adopted repository\n  {}\n  {}\n  {}\n  {}\n  {}",
        init_rendered.human.lines().next().unwrap_or_default(),
        index_rendered.human.lines().next().unwrap_or_default(),
        register_rendered.human.lines().next().unwrap_or_default(),
        benchmark_rendered.human.lines().next().unwrap_or_default(),
        doctor_rendered.human.lines().next().unwrap_or_default(),
    );
    Ok(
        Rendered::new(human, json!({ "adopted": true, "steps": steps })).with_detail(format!(
            "{}{}{}{}{}",
            init_rendered.detail,
            index_rendered.detail,
            register_rendered.detail,
            benchmark_rendered.detail,
            doctor_rendered.detail
        )),
    )
}

/// Reports cross-repository links already attached during `index` (spec §11).
fn register(cli: &Cli) -> Result<Rendered, CliError> {
    let root = Workspace::canonical_root(&cli.path)?;
    let cargo = tpt_weave_index::CargoIndex::load(&root)?;
    let discovered = crate::workspace::discover_tpt_dependencies(&cargo);
    let graph = tpt_weave_graph::RepositoryGraph::load(tpt_weave_graph::graph_path(&root))?;
    let registered = graph.external_links.len();

    let detail = if discovered.is_empty() {
        "no tpt-* path dependencies discovered".to_string()
    } else {
        discovered
            .iter()
            .map(|(package, repository)| format!("{package} -> {repository}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    Ok(Rendered::new(
        format!("registered {registered} cross-repository link(s)"),
        json!({
            "registered": registered,
            "discovered": discovered
                .iter()
                .map(|(package, repository)| json!({
                    "package": package,
                    "repository": repository,
                }))
                .collect::<Vec<_>>(),
        }),
    )
    .with_detail(detail))
}

/// Local model-free baseline vs weave benchmark (spec §23 methodology).
fn benchmark(cli: &Cli) -> Result<Rendered, CliError> {
    let workspace = Workspace::load_index(&cli.path)?;
    let budget = workspace
        .manifest()
        .map(|m| m.context.clamp(0))
        .unwrap_or(DEFAULT_CONTEXT_BUDGET);
    let task = format!(
        "baseline: survey the public API of {}",
        workspace.repository_name()
    );

    let comparison = local_comparison(
        workspace.graph(),
        workspace.sources(),
        &task,
        budget,
        ContextLevel::Skeleton,
    )
    .map_err(|error| match error {
        HarnessError::Empty => CliError::internal(format!("baseline benchmark: {error}")),
        HarnessError::Context(msg) => CliError::internal(format!("baseline benchmark: {msg}")),
    })?;

    let report = local_report(Some(workspace.repository_name().to_string()), comparison);
    let path = report_path(workspace.root());
    report.save(&path).map_err(|error| {
        CliError::internal(format!("failed to write {}: {error}", path.display()))
    })?;

    let aggregate = &report.aggregate;
    let human = format!(
        "baseline benchmark: {} raw -> {} delivered tokens (net {:.1}%)",
        aggregate.raw_tokens,
        aggregate.delivered_tokens,
        aggregate.net_token_reduction * 100.0,
    );
    Ok(Rendered::new(
        human,
        json!({
            "path": path.display().to_string(),
            "task": task,
            "budget_tokens": budget,
            "raw_tokens": aggregate.raw_tokens,
            "delivered_tokens": aggregate.delivered_tokens,
            "jev_tokens": aggregate.jev_tokens,
            "gross_token_reduction": aggregate.gross_token_reduction,
            "net_token_reduction": aggregate.net_token_reduction,
            "tasks": aggregate.tasks,
        }),
    )
    .with_detail(aggregate.summary()))
}

/// Path of the adopt baseline report inside `.tpt-weave/`.
fn report_path(root: &std::path::Path) -> PathBuf {
    root.join(MANIFEST_DIR).join(BASELINE_REPORT_FILE)
}
