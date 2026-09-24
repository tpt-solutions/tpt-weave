//! Reproducible experiment matrices for Phases 15 and 17.
//!
//! The harness records measurements supplied by a real run. It does not
//! fabricate JEv answers, model quality, cache hits, or dollar savings.

use crate::metrics::estimate_text_tokens;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Instant;
use tpt_weave_context::{ContextError, Selection, SourceProvider, estimate_tokens, represent_file};
use tpt_weave_core::ContextLevel;
use tpt_weave_decisions::{DecisionCategory, DecisionProvider, DecisionRequest};
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

    /// Parses and validates an experiment report.
    pub fn from_json(text: &str) -> Result<Self, ExperimentReportError> {
        let suite: Self = serde_json::from_str(text).map_err(ExperimentReportError::Json)?;
        suite.validate()?;
        Ok(suite)
    }

    /// Validates schema, labels, and externally supplied measurements.
    pub fn validate(&self) -> Result<(), ExperimentReportError> {
        if self.schema != EXPERIMENT_REPORT_SCHEMA {
            return Err(ExperimentReportError::Schema {
                found: self.schema,
                expected: EXPERIMENT_REPORT_SCHEMA,
            });
        }
        if self.experiment.trim().is_empty() {
            return Err(ExperimentReportError::Invalid(
                "experiment name must not be empty".to_string(),
            ));
        }
        let mut variants = std::collections::BTreeSet::new();
        for case in &self.cases {
            if case.variant.trim().is_empty() {
                return Err(ExperimentReportError::Invalid(
                    "variant name must not be empty".to_string(),
                ));
            }
            if !variants.insert(case.variant.as_str()) {
                return Err(ExperimentReportError::Invalid(format!(
                    "duplicate variant `{}`",
                    case.variant
                )));
            }
            if let Some(accuracy) = case.accuracy
                && (!accuracy.is_finite() || !(0.0..=1.0).contains(&accuracy))
            {
                return Err(ExperimentReportError::Invalid(format!(
                    "accuracy for `{}` must be finite and in [0, 1]",
                    case.variant
                )));
            }
        }
        Ok(())
    }

    /// Writes a validated report as pretty JSON.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ExperimentReportError> {
        self.validate()?;
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(ExperimentReportError::Io)?;
        }
        fs::write(path, self.to_json()).map_err(ExperimentReportError::Io)
    }

    /// Loads and validates a report from `path`.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ExperimentReportError> {
        Self::from_json(&fs::read_to_string(path).map_err(ExperimentReportError::Io)?)
    }
}

/// Errors loading or validating an experiment report.
#[derive(Debug)]
pub enum ExperimentReportError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
    Schema { found: u32, expected: u32 },
}

impl std::fmt::Display for ExperimentReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "experiment report I/O error: {error}"),
            Self::Json(error) => write!(f, "experiment report JSON error: {error}"),
            Self::Invalid(detail) => write!(f, "invalid experiment report: {detail}"),
            Self::Schema { found, expected } => write!(
                f,
                "experiment report schema {found} does not match {expected}"
            ),
        }
    }
}

impl std::error::Error for ExperimentReportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Invalid(_) | Self::Schema { .. } => None,
        }
    }
}

impl From<std::io::Error> for ExperimentReportError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ExperimentReportError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// One labeled decision used by a local or external accuracy corpus.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LabeledDecision {
    pub category: DecisionCategory,
    pub subject: String,
    pub context: String,
    /// The expected choice from [`DecisionCategory::choices`].
    pub expected_choice: String,
}

impl LabeledDecision {
    /// Creates a labeled decision case.
    pub fn new(
        category: DecisionCategory,
        subject: impl Into<String>,
        context: impl Into<String>,
        expected_choice: impl Into<String>,
    ) -> Self {
        Self {
            category,
            subject: subject.into(),
            context: context.into(),
            expected_choice: expected_choice.into(),
        }
    }

    fn request(&self) -> DecisionRequest {
        self.category.ask(&self.subject, &self.context)
    }
}

/// Current labeled-accuracy corpus schema.
pub const ACCURACY_CORPUS_SCHEMA: u32 = 1;

/// A reproducible, local-only labeled corpus for provider evaluation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccuracyCorpus {
    pub schema: u32,
    pub cases: Vec<LabeledDecision>,
}

impl AccuracyCorpus {
    /// Creates a corpus with the current schema.
    pub fn new(cases: Vec<LabeledDecision>) -> Self {
        Self {
            schema: ACCURACY_CORPUS_SCHEMA,
            cases,
        }
    }

    /// Parses and validates a JSON corpus.
    pub fn from_json(text: &str) -> Result<Self, ExperimentReportError> {
        let corpus: Self = serde_json::from_str(text).map_err(ExperimentReportError::Json)?;
        corpus.validate()?;
        Ok(corpus)
    }

    /// Validates schema, corpus size, and every expected choice.
    pub fn validate(&self) -> Result<(), ExperimentReportError> {
        if self.schema != ACCURACY_CORPUS_SCHEMA {
            return Err(ExperimentReportError::Schema {
                found: self.schema,
                expected: ACCURACY_CORPUS_SCHEMA,
            });
        }
        if self.cases.is_empty() {
            return Err(ExperimentReportError::Invalid(
                "accuracy corpus must contain at least one case".to_string(),
            ));
        }
        for case in &self.cases {
            if !case
                .category
                .choices()
                .iter()
                .any(|choice| *choice == case.expected_choice)
            {
                return Err(ExperimentReportError::Invalid(format!(
                    "expected choice `{}` is not valid for `{}`",
                    case.expected_choice, case.category
                )));
            }
        }
        Ok(())
    }

    /// Pretty JSON representation.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Writes the validated corpus as pretty JSON.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ExperimentReportError> {
        self.validate()?;
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(ExperimentReportError::Io)?;
        }
        fs::write(path, self.to_json()).map_err(ExperimentReportError::Io)
    }

    /// Loads and validates a corpus from `path`.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ExperimentReportError> {
        Self::from_json(&fs::read_to_string(path).map_err(ExperimentReportError::Io)?)
    }
}

/// Accuracy results for one provider/corpus pair.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccuracyReport {
    pub schema: u32,
    pub provider: String,
    pub total: u64,
    pub correct: u64,
    pub incorrect: u64,
    pub errors: u64,
    pub accuracy: f64,
}

/// Evaluates a provider against labeled decisions without changing the
/// provider or policy implementation.
pub fn evaluate_accuracy(
    provider: &dyn DecisionProvider,
    cases: &[LabeledDecision],
) -> Result<AccuracyReport, ExperimentReportError> {
    if cases.is_empty() {
        return Err(ExperimentReportError::Invalid(
            "accuracy corpus must contain at least one case".to_string(),
        ));
    }
    let mut correct = 0u64;
    let mut errors = 0u64;
    for case in cases {
        let request = case.request();
        if !request
            .choices
            .iter()
            .any(|choice| choice == &case.expected_choice)
        {
            return Err(ExperimentReportError::Invalid(format!(
                "expected choice `{}` is not valid for `{}`",
                case.expected_choice, case.category
            )));
        }
        match provider.decide(&request) {
            Ok(outcome) if outcome.decision.choice == case.expected_choice => correct += 1,
            Ok(_) => {}
            Err(_) => errors += 1,
        }
    }
    let total = cases.len() as u64;
    Ok(AccuracyReport {
        schema: EXPERIMENT_REPORT_SCHEMA,
        provider: provider.name().to_string(),
        total,
        correct,
        incorrect: total - correct - errors,
        errors,
        accuracy: correct as f64 / total as f64,
    })
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
