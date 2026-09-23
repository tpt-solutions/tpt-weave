//! Evaluation reports: paired tasks + aggregate metrics (todo.md Phase 11).

use crate::capture::{BaselineCapture, WeaveCapture};
use crate::metrics::{AggregateMetrics, MetricsError, TaskMetrics};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One baseline/weave pair with its computed metrics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskEvaluation {
    pub baseline: BaselineCapture,
    pub weave: WeaveCapture,
    pub metrics: TaskMetrics,
}

impl TaskEvaluation {
    /// Pairs two captures and computes metrics.
    pub fn try_pair(baseline: BaselineCapture, weave: WeaveCapture) -> Result<Self, MetricsError> {
        let metrics = TaskMetrics::from_pair(&baseline, &weave)?;
        Ok(Self {
            baseline,
            weave,
            metrics,
        })
    }
}

/// A full evaluation run over one or more tasks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvalReport {
    /// Optional label (e.g. repository name or run id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Schema version of the report format (independent of index schema).
    pub schema: u32,
    /// Per-task evaluations.
    pub tasks: Vec<TaskEvaluation>,
    /// Folded aggregate metrics.
    pub aggregate: AggregateMetrics,
}

/// Current eval report schema.
pub const EVAL_REPORT_SCHEMA: u32 = 1;

impl EvalReport {
    /// Builds a report from task evaluations.
    pub fn from_tasks(label: Option<String>, tasks: Vec<TaskEvaluation>) -> Self {
        let metric_list: Vec<TaskMetrics> = tasks.iter().map(|t| t.metrics.clone()).collect();
        let aggregate = AggregateMetrics::from_tasks(&metric_list);
        Self {
            label,
            schema: EVAL_REPORT_SCHEMA,
            tasks,
            aggregate,
        }
    }

    /// Pairs captures, failing on the first task-id mismatch.
    pub fn try_from_pairs(
        label: Option<String>,
        pairs: impl IntoIterator<Item = (BaselineCapture, WeaveCapture)>,
    ) -> Result<Self, MetricsError> {
        let mut tasks = Vec::new();
        for (baseline, weave) in pairs {
            tasks.push(TaskEvaluation::try_pair(baseline, weave)?);
        }
        Ok(Self::from_tasks(label, tasks))
    }

    /// Pretty JSON serialisation.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Single-line JSON serialisation.
    pub fn to_json_compact(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Parses a report from JSON text.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// Writes the report as pretty JSON to `path`, creating parents.
    pub fn save(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_json())?;
        Ok(())
    }

    /// Loads a report from `path`.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, EvalReportError> {
        let text = std::fs::read_to_string(path.as_ref())?;
        let report: Self = serde_json::from_str(&text)?;
        if report.schema != EVAL_REPORT_SCHEMA {
            return Err(EvalReportError::Schema {
                found: report.schema,
                expected: EVAL_REPORT_SCHEMA,
            });
        }
        Ok(report)
    }

    /// Human-readable summary (aggregate only).
    pub fn summary(&self) -> String {
        let label = self.label.as_deref().unwrap_or("evaluation");
        format!("== {label} ==\n{}", self.aggregate.summary())
    }
}

/// Errors loading an evaluation report.
#[derive(Debug)]
pub enum EvalReportError {
    /// Filesystem failure.
    Io(std::io::Error),
    /// JSON parse failure.
    Json(serde_json::Error),
    /// Report schema mismatch.
    Schema { found: u32, expected: u32 },
}

impl std::fmt::Display for EvalReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalReportError::Io(err) => write!(f, "report i/o error: {err}"),
            EvalReportError::Json(err) => write!(f, "invalid report json: {err}"),
            EvalReportError::Schema { found, expected } => write!(
                f,
                "eval report schema {found} does not match expected {expected}"
            ),
        }
    }
}

impl std::error::Error for EvalReportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EvalReportError::Io(err) => Some(err),
            EvalReportError::Json(err) => Some(err),
            EvalReportError::Schema { .. } => None,
        }
    }
}

impl From<std::io::Error> for EvalReportError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for EvalReportError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}
