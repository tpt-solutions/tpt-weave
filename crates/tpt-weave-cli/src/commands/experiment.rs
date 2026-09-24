//! `tpt-weave experiment accuracy` — evaluate a labeled corpus locally.

use super::Rendered;
use crate::args::{Cli, ExperimentAction};
use crate::error::CliError;
use std::path::{Path, PathBuf};
use tpt_weave_decisions::DeterministicFallbackProvider;
use tpt_weave_eval::{AccuracyCorpus, evaluate_accuracy};

pub fn run(cli: &Cli, action: ExperimentAction) -> Result<Rendered, CliError> {
    let ExperimentAction::Accuracy { corpus } = action;
    let corpus_path = resolve_path(&cli.path, &corpus);
    let corpus = AccuracyCorpus::load(&corpus_path)
        .map_err(|error| CliError::internal(format!("{}: {error}", corpus_path.display())))?;
    let provider = DeterministicFallbackProvider::new();
    let report = evaluate_accuracy(&provider, &corpus.cases)
        .map_err(|error| CliError::internal(error.to_string()))?;
    let human = format!(
        "accuracy experiment\n  provider: {}\n  cases: {}\n  correct: {}\n  incorrect: {}\n  errors: {}\n  accuracy: {:.1}%",
        report.provider,
        report.total,
        report.correct,
        report.incorrect,
        report.errors,
        report.accuracy * 100.0,
    );
    Ok(Rendered::new(
        human,
        serde_json::to_value(&report).map_err(|error| CliError::internal(error.to_string()))?,
    ))
}

fn resolve_path(root: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
