//! `tpt-weave workload` — aggregate Phase 18 JSONL capture events.

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use std::path::Path;
use tpt_weave_eval::WorkloadCapture;

pub fn run(cli: &Cli, capture: &str, duration_seconds: Option<&str>) -> Result<Rendered, CliError> {
    let capture_path = if Path::new(capture).is_absolute() {
        std::path::PathBuf::from(capture)
    } else {
        cli.path.join(capture)
    };
    let session_id = capture_path
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("jsonl-capture");
    let capture = WorkloadCapture::load_jsonl_with_session_id(session_id, &capture_path)
        .map_err(|error| CliError::internal(format!("{capture_path:?}: {error}")))?;
    let duration = duration_seconds
        .map(str::parse::<f64>)
        .transpose()
        .map_err(|_| CliError::usage("--duration must be a number"))?;
    if duration.is_some_and(|seconds| !seconds.is_finite() || seconds <= 0.0) {
        return Err(CliError::usage("--duration must be greater than zero"));
    }
    let report = capture.report(duration);
    let human = format!(
        "workload session: {}\n  events: {}\n  raw tokens: {}\n  delivered tokens: {}\n  JEv tokens: {}\n  output tokens: {}\n  net token reduction: {:.1}%\n  repeated context: {}\n  repeated tool output: {}\n  exploration: {}\n  dependency traversal: {}\n  total latency: {} ms\n  cache hits: {}\n  model success rate: {}\n  cost savings: {}\n",
        report.session_id,
        report.events,
        report.raw_tokens,
        report.delivered_tokens,
        report.jev_tokens,
        report.output_tokens,
        report.net_token_reduction * 100.0,
        report.repeated_context_tokens,
        report.repeated_tool_output_tokens,
        report.repository_exploration_tokens,
        report.dependency_traversal_tokens,
        report.total_latency_ms,
        report.cache_hits,
        report
            .task_success_rate
            .map(|value| format!("{:.1}%", value * 100.0))
            .unwrap_or_else(|| "not measured".to_string()),
        report
            .cost_savings_usd
            .map(|value| format!("${value:.6}"))
            .unwrap_or_else(|| "not measured".to_string()),
    );
    Ok(Rendered::new(
        human.trim_end(),
        serde_json::to_value(&report).map_err(|error| CliError::internal(error.to_string()))?,
    ))
}
