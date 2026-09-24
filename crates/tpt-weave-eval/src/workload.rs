//! Serializable Phase 18 workload captures and aggregate reports.
//!
//! The capture format records measurements, not secrets or inferred model
//! quality. A caller can record a real coding session as JSONL and analyse it
//! without changing tpt-weave's runtime behaviour.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

/// Current workload-report schema.
pub const WORKLOAD_REPORT_SCHEMA: u32 = 1;

/// The kind of work represented by one capture event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadEventKind {
    /// Context delivered to a model or agent.
    Context,
    /// Raw or reduced tool output.
    ToolOutput,
    /// Repository listing/search/exploration work.
    RepositoryExploration,
    /// Cross-repository dependency traversal.
    DependencyTraversal,
    /// A model invocation.
    ModelCall,
}

/// One measured event from a coding session.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkloadEvent {
    pub kind: WorkloadEventKind,
    /// Stable event key used to identify repeated context/tool output.
    pub id: String,
    /// Tokens the unoptimized workflow would have consumed.
    pub raw_tokens: u64,
    /// Tokens actually delivered to the next stage.
    pub delivered_tokens: u64,
    #[serde(default)]
    pub jev_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    /// Latency for this event in milliseconds, when measured.
    #[serde(default)]
    pub latency_ms: u64,
    #[serde(default)]
    pub cache_hit: bool,
    #[serde(default)]
    pub success: Option<bool>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub baseline_cost_usd: Option<f64>,
}

impl WorkloadEvent {
    /// Creates a context/tool/exploration event with zero optional metrics.
    pub fn new(
        kind: WorkloadEventKind,
        id: impl Into<String>,
        raw_tokens: u64,
        delivered_tokens: u64,
    ) -> Self {
        Self {
            kind,
            id: id.into(),
            raw_tokens,
            delivered_tokens,
            jev_tokens: 0,
            output_tokens: 0,
            latency_ms: 0,
            cache_hit: false,
            success: None,
            cost_usd: None,
            baseline_cost_usd: None,
        }
    }

    /// Adds JEv input tokens.
    pub fn with_jev_tokens(mut self, tokens: u64) -> Self {
        self.jev_tokens = tokens;
        self
    }

    /// Adds model output tokens.
    pub fn with_output_tokens(mut self, tokens: u64) -> Self {
        self.output_tokens = tokens;
        self
    }

    /// Adds measured event latency in milliseconds.
    pub fn with_latency_ms(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    /// Marks this event as a cache hit.
    pub fn with_cache_hit(mut self) -> Self {
        self.cache_hit = true;
        self
    }

    /// Records model success/failure for a model event.
    pub fn with_success(mut self, success: bool) -> Self {
        self.success = Some(success);
        self
    }

    /// Records observed and baseline costs for a model event.
    pub fn with_costs(mut self, baseline_usd: f64, observed_usd: f64) -> Self {
        self.baseline_cost_usd = Some(baseline_usd);
        self.cost_usd = Some(observed_usd);
        self
    }
}

/// A complete JSONL workload capture.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkloadCapture {
    pub session_id: String,
    pub events: Vec<WorkloadEvent>,
}

impl WorkloadCapture {
    /// Creates a capture from events.
    pub fn new(session_id: impl Into<String>, events: Vec<WorkloadEvent>) -> Self {
        Self {
            session_id: session_id.into(),
            events,
        }
    }

    /// Parses a JSONL event stream, assigning `session_id` to the capture.
    pub fn from_jsonl_with_session_id(
        session_id: impl Into<String>,
        text: &str,
    ) -> Result<Self, WorkloadError> {
        let mut capture = Self::from_jsonl(text)?;
        capture.session_id = session_id.into();
        Ok(capture)
    }

    /// Parses one JSON object per non-empty line.
    pub fn from_jsonl(text: &str) -> Result<Self, WorkloadError> {
        let mut events = Vec::new();
        for (line_number, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let event = serde_json::from_str(line).map_err(|error| WorkloadError::JsonLine {
                line: line_number + 1,
                error,
            })?;
            events.push(event);
        }
        if events.is_empty() {
            return Err(WorkloadError::Empty);
        }
        Ok(Self::new("jsonl-capture", events))
    }

    /// Serialises the capture as JSONL.
    pub fn to_jsonl(&self) -> Result<String, WorkloadError> {
        self.events
            .iter()
            .map(|event| serde_json::to_string(event).map_err(WorkloadError::from))
            .collect::<Result<Vec<_>, _>>()
            .map(|lines| lines.join("\n") + "\n")
    }

    /// Aggregates the capture.
    pub fn report(&self, duration_seconds: Option<f64>) -> WorkloadReport {
        WorkloadReport::from_events(&self.session_id, &self.events, duration_seconds)
    }

    /// Writes this capture as JSONL, creating parent directories.
    pub fn save_jsonl(&self, path: impl AsRef<Path>) -> Result<(), WorkloadError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_jsonl()?)?;
        Ok(())
    }

    /// Loads a JSONL capture from `path`.
    pub fn load_jsonl(path: impl AsRef<Path>) -> Result<Self, WorkloadError> {
        let path = path.as_ref();
        Self::from_jsonl(&std::fs::read_to_string(path)?)
    }

    /// Loads a JSONL capture and assigns an explicit session identifier.
    pub fn load_jsonl_with_session_id(
        session_id: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> Result<Self, WorkloadError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_jsonl_with_session_id(session_id, &text)
    }
}

/// Aggregate Phase 18 measurements.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkloadReport {
    pub schema: u32,
    pub session_id: String,
    pub duration_seconds: Option<f64>,
    pub events: usize,
    pub raw_tokens: u64,
    pub delivered_tokens: u64,
    pub jev_tokens: u64,
    pub output_tokens: u64,
    pub total_latency_ms: u64,
    pub total_tokens: u64,
    pub net_token_reduction: f64,
    pub repeated_context_tokens: u64,
    pub repeated_tool_output_tokens: u64,
    pub repository_exploration_tokens: u64,
    pub dependency_traversal_tokens: u64,
    pub cache_hits: u64,
    pub model_attempts: u64,
    pub model_successes: u64,
    pub task_success_rate: Option<f64>,
    pub observed_cost_usd: Option<f64>,
    pub baseline_cost_usd: Option<f64>,
    pub cost_savings_usd: Option<f64>,
    pub raw_tokens_per_hour: Option<f64>,
    pub delivered_tokens_per_hour: Option<f64>,
}

impl WorkloadReport {
    /// Aggregates events without inventing missing quality or cost data.
    pub fn from_events(
        session_id: &str,
        events: &[WorkloadEvent],
        duration_seconds: Option<f64>,
    ) -> Self {
        let mut raw_tokens: u64 = 0;
        let mut delivered_tokens: u64 = 0;
        let mut jev_tokens: u64 = 0;
        let mut output_tokens: u64 = 0;
        let mut total_latency_ms: u64 = 0;
        let mut repeated_context_tokens: u64 = 0;
        let mut repeated_tool_output_tokens: u64 = 0;
        let mut repository_exploration_tokens: u64 = 0;
        let mut dependency_traversal_tokens: u64 = 0;
        let mut cache_hits = 0;
        let mut model_attempts = 0;
        let mut model_successes = 0;
        let mut observed_cost = 0.0;
        let mut observed_cost_known = false;
        let mut baseline_cost = 0.0;
        let mut baseline_cost_known = false;
        let mut seen: BTreeMap<WorkloadEventKind, BTreeSet<String>> = BTreeMap::new();

        for event in events {
            raw_tokens = raw_tokens.saturating_add(event.raw_tokens);
            delivered_tokens = delivered_tokens.saturating_add(event.delivered_tokens);
            jev_tokens = jev_tokens.saturating_add(event.jev_tokens);
            output_tokens = output_tokens.saturating_add(event.output_tokens);
            total_latency_ms = total_latency_ms.saturating_add(event.latency_ms);
            if event.cache_hit {
                cache_hits += 1;
            }
            if event.success.is_some() {
                model_attempts += 1;
                if event.success == Some(true) {
                    model_successes += 1;
                }
            }
            if let Some(cost) = event.cost_usd {
                observed_cost += cost;
                observed_cost_known = true;
            }
            if let Some(cost) = event.baseline_cost_usd {
                baseline_cost += cost;
                baseline_cost_known = true;
            }
            match event.kind {
                WorkloadEventKind::RepositoryExploration => {
                    repository_exploration_tokens =
                        repository_exploration_tokens.saturating_add(event.raw_tokens);
                }
                WorkloadEventKind::DependencyTraversal => {
                    dependency_traversal_tokens =
                        dependency_traversal_tokens.saturating_add(event.raw_tokens);
                }
                WorkloadEventKind::Context | WorkloadEventKind::ToolOutput => {
                    let ids = seen.entry(event.kind).or_default();
                    if !ids.insert(event.id.clone()) {
                        match event.kind {
                            WorkloadEventKind::Context => {
                                repeated_context_tokens =
                                    repeated_context_tokens.saturating_add(event.delivered_tokens);
                            }
                            WorkloadEventKind::ToolOutput => {
                                repeated_tool_output_tokens = repeated_tool_output_tokens
                                    .saturating_add(event.delivered_tokens);
                            }
                            _ => {}
                        }
                    }
                }
                WorkloadEventKind::ModelCall => {}
            }
        }

        let total_tokens = delivered_tokens
            .saturating_add(jev_tokens)
            .saturating_add(output_tokens);
        let net_token_reduction = if raw_tokens == 0 {
            0.0
        } else {
            1.0 - (delivered_tokens.saturating_add(jev_tokens) as f64 / raw_tokens as f64)
        };
        let duration = duration_seconds.filter(|seconds| *seconds > 0.0);
        Self {
            schema: WORKLOAD_REPORT_SCHEMA,
            session_id: session_id.to_string(),
            duration_seconds: duration,
            events: events.len(),
            raw_tokens,
            delivered_tokens,
            jev_tokens,
            output_tokens,
            total_latency_ms,
            total_tokens,
            net_token_reduction,
            repeated_context_tokens,
            repeated_tool_output_tokens,
            repository_exploration_tokens,
            dependency_traversal_tokens,
            cache_hits,
            model_attempts,
            model_successes,
            task_success_rate: (model_attempts > 0)
                .then_some(model_successes as f64 / model_attempts as f64),
            observed_cost_usd: observed_cost_known.then_some(observed_cost),
            baseline_cost_usd: baseline_cost_known.then_some(baseline_cost),
            cost_savings_usd: (observed_cost_known && baseline_cost_known)
                .then_some(baseline_cost - observed_cost),
            raw_tokens_per_hour: duration.map(|seconds| raw_tokens as f64 * 3600.0 / seconds),
            delivered_tokens_per_hour: duration.map(|seconds| {
                delivered_tokens.saturating_add(jev_tokens) as f64 * 3600.0 / seconds
            }),
        }
    }

    /// Pretty JSON representation.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Writes the report as pretty JSON, creating parent directories.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), WorkloadError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_json())?;
        Ok(())
    }

    /// Loads a report and rejects an unknown report schema.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, WorkloadError> {
        let report: Self = serde_json::from_str(&std::fs::read_to_string(path.as_ref())?)?;
        if report.schema != WORKLOAD_REPORT_SCHEMA {
            return Err(WorkloadError::Schema {
                found: report.schema,
                expected: WORKLOAD_REPORT_SCHEMA,
            });
        }
        Ok(report)
    }
}

/// Errors from workload capture serialisation and file operations.
#[derive(Debug)]
pub enum WorkloadError {
    /// Filesystem failure.
    Io(std::io::Error),
    /// JSONL parsing or serialisation failure.
    Json(serde_json::Error),
    /// A JSONL line was invalid.
    JsonLine {
        line: usize,
        error: serde_json::Error,
    },
    /// The capture contained no events.
    Empty,
    /// A report used an unsupported schema version.
    Schema { found: u32, expected: u32 },
}

impl fmt::Display for WorkloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "workload capture i/o error: {error}"),
            Self::Json(error) => write!(f, "workload capture JSON error: {error}"),
            Self::JsonLine { line, error } => {
                write!(f, "workload capture JSON error on line {line}: {error}")
            }
            Self::Empty => write!(f, "workload capture contains no events"),
            Self::Schema { found, expected } => {
                write!(
                    f,
                    "workload report schema {found} does not match {expected}"
                )
            }
        }
    }
}

impl std::error::Error for WorkloadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) | Self::JsonLine { error, .. } => Some(error),
            Self::Empty | Self::Schema { .. } => None,
        }
    }
}

impl From<std::io::Error> for WorkloadError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for WorkloadError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
