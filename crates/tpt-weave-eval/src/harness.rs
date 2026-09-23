//! Local (model-free) baseline vs weave comparison (todo.md Phase 11;
//! used by `tpt-weave adopt` as its baseline context benchmark, spec.md
//! section 28).
//!
//! The "baseline" is the naive full-source token volume of every indexed
//! file; the "weave" side is what deterministic retrieval delivers for the
//! given task under a budget. No model is invoked, so task success is
//! reported as "structural" (retrieval produced a non-empty selection).

use crate::capture::{BaselineCapture, WeaveCapture};
use crate::metrics::{AggregateMetrics, estimate_text_tokens};
use crate::report::{EvalReport, TaskEvaluation};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tpt_weave_context::{Retriever, SourceProvider};
use tpt_weave_core::{ContextLevel, ContextRequest, TokenAccounting};
use tpt_weave_graph::RepositoryGraph;

/// Failure to run a local comparison.
#[derive(Debug)]
pub enum HarnessError {
    /// Retrieval failed.
    Context(String),
    /// Empty repository (no sources / symbols).
    Empty,
}

impl std::fmt::Display for HarnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HarnessError::Context(msg) => write!(f, "context error: {msg}"),
            HarnessError::Empty => write!(f, "no sources available for evaluation"),
        }
    }
}

impl std::error::Error for HarnessError {}

impl From<tpt_weave_context::ContextError> for HarnessError {
    fn from(error: tpt_weave_context::ContextError) -> Self {
        HarnessError::Context(error.to_string())
    }
}

/// Result of a local baseline/weave comparison for one task.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalComparison {
    /// Task id (defaults to a short hash of the task text).
    pub task_id: String,
    /// Task description used for retrieval.
    pub task: String,
    /// Naive full-source token volume (baseline raw context).
    pub raw_tokens: u64,
    /// Delivered context tokens from deterministic retrieval.
    pub delivered_tokens: u64,
    /// Accounting produced by the retriever (raw/selected/saved).
    pub tokens: TokenAccounting,
    /// Number of selected candidates.
    pub candidates: usize,
    /// Wall-clock retrieval latency in milliseconds.
    pub latency_ms: u64,
    /// Whether retrieval produced a non-empty selection.
    pub structural_success: bool,
    /// Budget applied to the request.
    pub budget_tokens: u32,
    /// Maximum representation level used.
    pub max_level: String,
}

impl LocalComparison {
    /// Builds the paired baseline/weave captures for this comparison.
    pub fn into_pair(self) -> (BaselineCapture, WeaveCapture) {
        let baseline = BaselineCapture::new(self.task_id.clone(), self.raw_tokens, true)
            .with_task(self.task.clone())
            .with_latency_ms(self.latency_ms)
            .with_model_output(format!(
                "baseline: {} raw tokens ({} candidates not selected)",
                self.raw_tokens, self.candidates
            ));
        let weave = WeaveCapture::new(self.task_id.clone(), self.delivered_tokens, true)
            .with_task(self.task.clone())
            .with_jev_tokens(self.tokens.jev_tokens)
            .with_latency_ms(self.latency_ms)
            .with_counts(1, 0, 0)
            .with_model_output(format!(
                "weave: {} delivered tokens over {} candidates",
                self.delivered_tokens, self.candidates
            ));
        (baseline, weave)
    }
}

/// Runs one local comparison: full-source baseline vs budgeted retrieval.
///
/// `graph` must already be loaded; `sources` provides file text for
/// representation estimates.
pub fn local_comparison(
    graph: &RepositoryGraph,
    sources: &dyn SourceProvider,
    task: &str,
    budget_tokens: u32,
    max_level: ContextLevel,
) -> Result<LocalComparison, HarnessError> {
    let raw_tokens = full_source_tokens(graph, sources);
    if raw_tokens == 0 && graph.symbols.is_empty() {
        return Err(HarnessError::Empty);
    }

    let request = ContextRequest::new(graph.repository.as_str(), task)
        .with_budget_tokens(budget_tokens)
        .with_max_level(max_level);

    let retriever = Retriever::new(graph, sources);
    let started = Instant::now();
    let response = retriever.retrieve(&request, &[])?;
    let latency_ms = started.elapsed().as_millis() as u64;

    // Prefer the retriever's own accounting; fall back to candidate sums.
    let mut tokens = response.tokens;
    if tokens.raw_tokens < raw_tokens {
        tokens = TokenAccounting::new(
            raw_tokens,
            tokens.selected_tokens,
            tokens.jev_tokens,
            tokens.model_tokens,
            tokens.cache_hit,
        );
    }

    Ok(LocalComparison {
        task_id: short_task_id(task),
        task: task.to_string(),
        raw_tokens,
        delivered_tokens: tokens.selected_tokens,
        tokens,
        candidates: response.candidates.len(),
        latency_ms,
        structural_success: !response.candidates.is_empty(),
        budget_tokens,
        max_level: max_level.name().to_string(),
    })
}

/// Naive baseline: sum of token estimates of every distinct file source
/// referenced by the symbol table (or all sources when the table is empty).
pub fn full_source_tokens(graph: &RepositoryGraph, sources: &dyn SourceProvider) -> u64 {
    let mut paths: Vec<&str> = graph.symbols.iter().map(|s| s.file.as_str()).collect();
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return 0;
    }
    let mut total = 0u64;
    for path in paths {
        if let Some(text) = sources.source(path) {
            total = total.saturating_add(u64::from(estimate_text_tokens(text)));
        }
    }
    total
}

/// Builds a single-task eval report for the local comparison (what
/// `tpt-weave adopt` writes as its baseline benchmark).
pub fn local_report(label: Option<String>, comparison: LocalComparison) -> EvalReport {
    let pair = comparison.into_pair();
    let task = TaskEvaluation::try_pair(pair.0, pair.1).expect("local pair matches ids");
    EvalReport::from_tasks(label, vec![task])
}

/// Aggregate helper for multiple local comparisons.
pub fn local_aggregate(comparisons: &[LocalComparison]) -> AggregateMetrics {
    let mut pairs = Vec::with_capacity(comparisons.len());
    for comparison in comparisons {
        let (baseline, weave) = comparison.clone().into_pair();
        if let Ok(task) = TaskEvaluation::try_pair(baseline, weave) {
            pairs.push(task);
        }
    }
    EvalReport::from_tasks(None, pairs).aggregate
}

/// Short stable id for a task string.
fn short_task_id(task: &str) -> String {
    let digest = tpt_weave_core::hash::fnv1a64_hex(task.as_bytes());
    format!("local-{digest}")
}
