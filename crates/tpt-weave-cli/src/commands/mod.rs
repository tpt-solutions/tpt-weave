//! Subcommand dispatch and per-command implementations.

mod adopt;
mod cache;
mod context;
mod diff;
mod doctor;
mod expand;
mod help;
mod index;
mod init;
mod integration;
mod overview;
mod refs;
mod skeleton;
mod stats;
mod symbol;
mod workload;

use crate::args::{Cli, Command};
use crate::error::CliError;
use serde_json::Value;

/// One command's dual representation: human text + JSON payload.
#[derive(Clone, Debug)]
pub struct Rendered {
    pub human: String,
    pub json: Value,
    /// Extra human detail printed only under `--verbose`.
    pub detail: String,
}

impl Rendered {
    /// Convenience constructor with empty verbose detail.
    pub fn new(human: impl Into<String>, json: Value) -> Self {
        Self {
            human: human.into(),
            json,
            detail: String::new(),
        }
    }

    /// Builder for verbose-only detail lines.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        if !self.detail.is_empty() && !self.detail.ends_with('\n') {
            self.detail.push('\n');
        }
        self
    }
}

/// Runs the selected subcommand.
pub fn dispatch(cli: &Cli) -> Result<Rendered, CliError> {
    match &cli.command {
        Command::Help => Ok(help::run()),
        Command::Version => Ok(help::version()),
        Command::Adopt => adopt::run(cli),
        Command::Init { force } => init::run(cli, *force),
        Command::Index { full } => index::run(cli, *full),
        Command::Doctor => doctor::run(cli),
        Command::Overview => overview::run(cli),
        Command::Symbol { name } => symbol::run(cli, name),
        Command::Refs { name } => refs::run(cli, name),
        Command::Expand {
            id,
            level,
            kind,
            package,
        } => expand::run(cli, id, *level, kind.as_deref(), package.as_deref()),
        Command::Context {
            task,
            budget,
            max_level,
            no_deps,
        } => context::run(cli, task, *budget, *max_level, *no_deps),
        Command::Skeleton { path, level } => skeleton::run(cli, path, *level),
        Command::Diff => diff::run(cli),
        Command::Stats => stats::run(cli),
        Command::Cache { action } => cache::run(cli, *action),
        Command::Integration { action, write } => integration::run(cli, *action, *write),
        Command::Workload {
            capture,
            duration_seconds,
        } => workload::run(cli, capture, duration_seconds.as_deref()),
    }
}
