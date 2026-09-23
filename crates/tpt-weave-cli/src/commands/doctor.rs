//! `tpt-weave doctor` — validate manifest, index freshness, git and provider.

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::{NO_REVISION, Workspace};
use serde_json::{Value, json};
use tpt_weave_core::{Manifest, SCHEMA_VERSION, manifest_path};
use tpt_weave_graph::{GraphError, RepositoryGraph, graph_path};
use tpt_weave_index::GitRepository;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Check {
    name: &'static str,
    ok: bool,
    detail: String,
    /// Exit code to surface when this check fails.
    fail_code: i32,
}

impl Check {
    fn pass(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            ok: true,
            detail: detail.into(),
            fail_code: 0,
        }
    }

    fn fail(name: &'static str, detail: impl Into<String>, fail_code: i32) -> Self {
        Self {
            name,
            ok: false,
            detail: detail.into(),
            fail_code,
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "check": self.name,
            "ok": self.ok,
            "detail": self.detail,
        })
    }
}

pub fn run(cli: &Cli) -> Result<Rendered, CliError> {
    let root = Workspace::canonical_root(&cli.path)?;
    let mut checks = Vec::new();

    // --- manifest ---------------------------------------------------------
    let manifest_path = manifest_path(&root);
    let manifest = if manifest_path.exists() {
        match Manifest::load(&manifest_path) {
            Ok(manifest) => {
                checks.push(Check::pass(
                    "manifest",
                    format!("loaded {}", manifest_path.display()),
                ));
                Some(manifest)
            }
            Err(error) => {
                checks.push(Check::fail("manifest", error.to_string(), 3));
                None
            }
        }
    } else {
        checks.push(Check::fail(
            "manifest",
            format!("missing {}", manifest_path.display()),
            3,
        ));
        None
    };

    // --- git --------------------------------------------------------------
    let git = GitRepository::discover(&root).ok();
    match &git {
        Some(repo) => match repo.current_revision() {
            Ok(revision) => checks.push(Check::pass(
                "git",
                format!("{} @ {}", revision.short(), revision.sha),
            )),
            Err(error) => checks.push(Check::fail("git", error.to_string(), 1)),
        },
        None => checks.push(Check::pass("git", "not a git repository (skipped)")),
    }

    // --- index freshness --------------------------------------------------
    let graph_path = graph_path(&root);
    if !graph_path.exists() {
        checks.push(Check::fail(
            "index",
            "missing; run `tpt-weave index`",
            4,
        ));
    } else {
        match RepositoryGraph::load(&graph_path) {
            Ok(graph) => {
                let mut ok = true;
                let mut detail = format!(
                    "schema {} @ {}",
                    graph.schema,
                    graph.revision.short()
                );
                if graph.schema != SCHEMA_VERSION {
                    ok = false;
                    detail = format!(
                        "schema {} != expected {SCHEMA_VERSION}; re-run `tpt-weave index`",
                        graph.schema
                    );
                    checks.push(Check::fail("index", detail, 4));
                } else if let (Some(repo), true) = (git.as_ref(), ok) {
                    match repo.current_revision() {
                        Ok(current) if current.sha != graph.revision.sha => {
                            checks.push(Check::fail(
                                "index",
                                format!(
                                    "stale (HEAD {}, index {}); re-run `tpt-weave index`",
                                    current.short(),
                                    graph.revision.short()
                                ),
                                4,
                            ));
                            ok = false;
                        }
                        Ok(_) => {}
                        Err(error) => {
                            checks.push(Check::fail("index", error.to_string(), 1));
                            ok = false;
                        }
                    }
                }
                if ok {
                    checks.push(Check::pass(
                        "index",
                        format!(
                            "{} symbols, {} references",
                            graph.symbols.len(),
                            graph.references.len()
                        ),
                    ));
                }
            }
            Err(GraphError::SchemaMismatch { found, expected }) => checks.push(Check::fail(
                "index",
                format!("schema {found} != {expected}; re-run `tpt-weave index`"),
                4,
            )),
            Err(error) => checks.push(Check::fail("index", error.to_string(), 4)),
        }
    }

    // --- provider config --------------------------------------------------
    if let Some(manifest) = manifest.as_ref() {
        match (manifest.provider.validate(), manifest.jev.validate()) {
            (Ok(()), Ok(())) => {
                if manifest.jev.enabled && manifest.privacy.remote_decisions {
                    let key_set = std::env::var(&manifest.provider.api_key_env)
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false);
                    if key_set {
                        checks.push(Check::pass(
                            "provider",
                            format!(
                                "{} / {} (key env {} set)",
                                manifest.provider.name, manifest.jev.model, manifest.provider.api_key_env
                            ),
                        ));
                    } else {
                        checks.push(Check::fail(
                            "provider",
                            format!(
                                "env {} is not set (remote decisions enabled)",
                                manifest.provider.api_key_env
                            ),
                            1,
                        ));
                    }
                } else {
                    checks.push(Check::pass(
                        "provider",
                        format!(
                            "local-only (remote_decisions=false); config {} ok",
                            manifest.provider.name
                        ),
                    ));
                }
            }
            (Err(error), _) | (_, Err(error)) => {
                checks.push(Check::fail("provider", error.to_string(), 3));
            }
        }
    } else {
        checks.push(Check::fail("provider", "manifest unavailable", 3));
    }

    // --- non-git placeholder note ----------------------------------------
    if git.is_none() {
        let _ = NO_REVISION;
    }

    render(&checks)
}

fn render(checks: &[Check]) -> Result<Rendered, CliError> {
    let mut human = String::new();
    let mut failed = 0usize;
    // Prefer the most specific documented code: not-found (3) outranks a
    // stale index (4), which outranks internal (1).
    let mut worst = 0i32;
    for check in checks {
        let mark = if check.ok { "ok" } else { "FAIL" };
        human.push_str(&format!("{mark:<4}  {}: {}\n", check.name, check.detail));
        if !check.ok {
            failed += 1;
            // Prefer the most specific documented code: not-found (3)
            // outranks a stale index (4), which outranks internal (1).
            if worst == 0 || code_rank(check.fail_code) < code_rank(worst) {
                worst = check.fail_code;
            }
        }
    }
    if failed == 0 {
        human.push_str("doctor: all checks passed\n");
    } else {
        human.push_str(&format!("doctor: {failed} check(s) failed\n"));
    }

    let json = json!({
        "ok": failed == 0,
        "failed": failed,
        "checks": checks.iter().map(Check::to_json).collect::<Vec<_>>(),
    });
    let rendered = Rendered::new(human.trim_end(), json);
    if failed > 0 {
        return Err(mapped_doctor_error(worst, rendered));
    }
    Ok(rendered)
}

fn code_rank(code: i32) -> i32 {
    match code {
        3 => 0,
        4 => 1,
        5 => 2,
        1 => 3,
        other => other,
    }
}

fn mapped_doctor_error(code: i32, rendered: Rendered) -> CliError {
    // Preserve the full check list for JSON consumers via the message body.
    let summary = rendered
        .json
        .get("checks")
        .and_then(Value::as_array)
        .map(|checks| {
            checks
                .iter()
                .filter(|check| check.get("ok").and_then(Value::as_bool) == Some(false))
                .filter_map(|check| {
                    let name = check.get("check")?.as_str()?;
                    let detail = check.get("detail")?.as_str()?;
                    Some(format!("{name}: {detail}"))
                })
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_else(|| rendered.human.clone());
    match code {
        3 => CliError::not_found(summary),
        4 => CliError::stale(summary),
        _ => CliError::internal(summary),
    }
}
