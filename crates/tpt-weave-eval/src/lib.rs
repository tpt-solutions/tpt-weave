//! Evaluation harness for tpt-weave (todo.md Phase 11; spec.md section 23).
//!
//! Three layers:
//!
//! - [`capture`] — serialisable baseline and tpt-weave run captures
//!   (raw/selected context, model output, success, latency, cost);
//! - [`metrics`] — per-task and aggregate metrics, including the primary
//!   **Net Token Reduction** and secondary **Task Success Preservation**;
//! - [`report`] — a full evaluation report that can be written as JSON.
//!
//! [`harness`] runs a local (model-free) baseline vs weave comparison over
//! an indexed repository, used by `tpt-weave adopt` as its baseline context
//! benchmark (spec.md section 28).

#![forbid(unsafe_code)]

pub mod capture;
pub mod harness;
pub mod metrics;
pub mod report;

pub use capture::{BaselineCapture, WeaveCapture};
pub use harness::{
    HarnessError, LocalComparison, full_source_tokens, local_aggregate, local_comparison,
    local_report,
};
pub use metrics::{AggregateMetrics, MetricsError, TaskMetrics, estimate_text_tokens};
pub use report::{EVAL_REPORT_SCHEMA, EvalReport, EvalReportError, TaskEvaluation};
