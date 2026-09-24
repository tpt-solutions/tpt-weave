//! Reproducible experiment matrices for Phases 15 and 17.
//!
//! The harness records measurements supplied by a real run. It does not
//! fabricate JEv answers, model quality, cache hits, or dollar savings.

use crate::metrics::estimate_text_tokens;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Instant;
use tpt_weave_context::{ContextError, Selection, SourceProvider, estimate_tokens, represent_file};
use tpt_weave_core::ContextLevel;
use tpt_weave_graph::RepositoryGraph;
use tpt_weave_tools::{ToolKind, ToolOutput, reduce};

/// Current experiment-report schema.
pub const EXPERIMENT_REPORT_SCHEMA: u32 = 1;

/// One measured variant in an experiment matrix.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VariantMeasurement {
    pub variant: String,
    pub raw_tokens: u64,
    pub delivered_tokens: u64,
    #[serde(default)]
    pub jev_tokens: u64,
    #[serde(default)]
    pub accuracy: Option<f64>,
    #[serde(default)]
    pub latency_ms: u64,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl VariantMeasurement {
    /// Creates a measurement with no JEv overhead or accuracy claim.
    pub fn new(
        variant: impl Into<String>,
        raw_tokens: u64,
        delivered_tokens: u64,
        latency_ms: u64,
    ) -> Self {
        Self {
            variant: variant.into(),
            raw_tokens,
            delivered_tokens,
            jev_tokens: 0,
            accuracy: None,
            latency_ms,
            metadata: BTreeMap::new(),
        }
    }

    /// Net token reduction, including JEv input tokens.
    pub fn net_token_reduction(&self) -> f64 {
        if self.raw_tokens == 0 {
            return 0.0;
        }
        1.0 - (self.delivered_tokens.saturating_add(self.jev_tokens) as f64
            / self.raw_tokens as f64)
    }

    /// Adds an externally measured accuracy in `[0, 1]`.
    pub fn with_accuracy(mut self, accuracy: f64) -> Self {
        self.accuracy = Some(accuracy.clamp(0.0, 1.0));
        self
    }

    /// Adds string-valued measurement annotation.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// A named, serialisable experiment matrix.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentSuite {
    pub schema: u32,
    pub experiment: String,
    pub cases: Vec<VariantMeasurement>,
}

impl ExperimentSuite {
    /// Creates a suite from measured variants.
    pub fn new(experiment: impl Into<String>, cases: Vec<VariantMeasurement>) -> Self {
        Self {
            schema: EXPERIMENT_REPORT_SCHEMA,
            experiment: experiment.into(),
            cases,
        }
    }

    /// Aggregate net reduction across all cases.
    pub fn net_token_reduction(&self) -> f64 {
        let raw: u64 = self.cases.iter().map(|case| case.raw_tokens).sum();
        let delivered: u64 = self
            .cases
            .iter()
            .map(|case| case.delivered_tokens.saturating_add(case.jev_tokens))
            .sum();
        if raw == 0 {
            0.0
        } else {
            1.0 - (delivered as f64 / raw as f64)
        }
    }

    /// Compact human summary.
    pub fn summary(&self) -> String {
        format!(
            "{}: {} variants, net reduction {:.1}%",
            self.experiment,
            self.cases.len(),
            self.net_token_reduction() * 100.0
        )
    }

    /// Pretty JSON representation.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Records all six hierarchical representation levels for one source file.
/// At level 4 every symbol in the file is selected, making the result a
/// meaningful targeted-implementation measurement.
pub fn hierarchy_measurements(
    graph: &RepositoryGraph,
    sources: &dyn SourceProvider,
    path: &str,
) -> Result<Vec<VariantMeasurement>, ContextError> {
    let selection: Selection = graph
        .symbols
        .iter()
        .filter(|record| record.file == path)
        .collect();
    hierarchy_measurements_with_selection(graph, sources, path, &selection)
}

/// Records all levels with an explicit level-4 selection.
pub fn hierarchy_measurements_with_selection(
    graph: &RepositoryGraph,
    sources: &dyn SourceProvider,
    path: &str,
    selection: &Selection,
) -> Result<Vec<VariantMeasurement>, ContextError> {
    let raw_tokens = sources
        .source(path)
        .map(estimate_text_tokens)
        .map(u64::from)
        .unwrap_or_default();
    let mut cases = Vec::with_capacity(ContextLevel::ALL.len());
    for level in ContextLevel::ALL {
        let started = Instant::now();
        let representation = represent_file(graph, sources, path, level, selection)?;
        cases.push(
            VariantMeasurement::new(
                level.name(),
                raw_tokens,
                u64::from(representation.token_estimate),
                started.elapsed().as_millis() as u64,
            )
            .with_metadata("path", path)
            .with_metadata("selected_symbols", selection.len().to_string()),
        );
    }
    Ok(cases)
}

/// Records deterministic and externally measured JEv relevance variants.
pub fn jev_relevance_measurements(
    raw_tokens: u64,
    deterministic_tokens: u64,
    deterministic_accuracy: Option<f64>,
    jev_tokens: u64,
    jev_accuracy: Option<f64>,
    jev_input_tokens: u64,
) -> ExperimentSuite {
    let mut deterministic =
        VariantMeasurement::new("deterministic_only", raw_tokens, deterministic_tokens, 0);
    deterministic.accuracy = deterministic_accuracy;
    let mut jev = VariantMeasurement::new("deterministic_plus_jev", raw_tokens, jev_tokens, 0);
    jev.jev_tokens = jev_input_tokens;
    jev.accuracy = jev_accuracy;
    ExperimentSuite::new("jev_relevance", vec![deterministic, jev])
}

/// Records raw, deterministic, and optionally JEv-augmented tool reduction.
pub fn tool_output_measurements(
    kind: ToolKind,
    output: &ToolOutput,
    jev_input_tokens: Option<u64>,
) -> ExperimentSuite {
    let reduction = reduce(kind, output);
    let raw = reduction.raw_tokens;
    let mut cases = vec![
        VariantMeasurement::new("raw_output", raw, raw, 0),
        VariantMeasurement::new("deterministic_reduction", raw, reduction.reduced_tokens, 0)
            .with_metadata("status", format!("{:?}", reduction.status)),
    ];
    if let Some(jev_tokens) = jev_input_tokens {
        cases.push(
            VariantMeasurement::new("reduction_plus_jev", raw, reduction.reduced_tokens, 0)
                .with_metadata("jev_input_tokens", jev_tokens.to_string()),
        );
        if let Some(case) = cases.last_mut() {
            case.jev_tokens = jev_tokens;
        }
    }
    ExperimentSuite::new("tool_output", cases)
}

/// Four-way Phase 17 cache matrix. Values are measured by the caller.
pub fn cache_measurements(
    no_cache: VariantMeasurement,
    file_cache: VariantMeasurement,
    context_cache: VariantMeasurement,
    tool_result_cache: VariantMeasurement,
) -> ExperimentSuite {
    ExperimentSuite::new(
        "cache",
        vec![no_cache, file_cache, context_cache, tool_result_cache],
    )
}

/// Four-way Phase 17 cross-repository retrieval matrix. Values are measured
/// by the caller; JEv traversal is never synthesised.
pub fn cross_repository_measurements(
    no_traversal: VariantMeasurement,
    full_traversal: VariantMeasurement,
    deterministic_traversal: VariantMeasurement,
    jev_selected_traversal: VariantMeasurement,
) -> ExperimentSuite {
    ExperimentSuite::new(
        "cross_repository_retrieval",
        vec![
            no_traversal,
            full_traversal,
            deterministic_traversal,
            jev_selected_traversal,
        ],
    )
}

/// Builds a suite from externally measured A–E variants. The caller remains
/// responsible for collecting accuracy, JEv, cache, and quality measurements.
pub fn comparison_suite(
    experiment: impl Into<String>,
    cases: impl IntoIterator<Item = VariantMeasurement>,
) -> ExperimentSuite {
    ExperimentSuite::new(experiment, cases.into_iter().collect())
}

/// Estimates the token count of a rendered tool reduction.
pub fn reduction_tokens(kind: ToolKind, output: &ToolOutput) -> u64 {
    u64::from(estimate_tokens(&reduce(kind, output).render()))
}
