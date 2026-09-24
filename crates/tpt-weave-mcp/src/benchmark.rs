//! Lightweight MCP tool-dispatch benchmark (todo.md Phase 15).

use crate::workspace::Workspace;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;

/// Aggregate timing for repeated MCP tool dispatches.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct McpBenchmark {
    pub tool: String,
    pub rounds: u32,
    pub total_ms: f64,
    pub mean_ms: f64,
    pub errors: u64,
}

/// Dispatches one tool repeatedly and records latency without retaining tool
/// output in the report.
pub fn benchmark_tool(
    workspace: &Workspace,
    tool: &str,
    args: &Value,
    rounds: u32,
) -> McpBenchmark {
    let rounds = rounds.max(1);
    let mut total = 0.0f64;
    let mut errors = 0u64;
    for _ in 0..rounds {
        let started = Instant::now();
        if workspace.call_tool(tool, args).is_err() {
            errors += 1;
        }
        total += started.elapsed().as_secs_f64() * 1000.0;
    }
    McpBenchmark {
        tool: tool.to_string(),
        rounds,
        total_ms: total,
        mean_ms: total / f64::from(rounds),
        errors,
    }
}
