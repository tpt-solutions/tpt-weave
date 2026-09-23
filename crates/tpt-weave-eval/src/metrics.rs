//! Metrics for Phase 11 (todo.md "Metrics"; spec.md section 23).

use crate::capture::{BaselineCapture, WeaveCapture};
use serde::{Deserialize, Serialize};

/// Deterministic token estimate matching the rest of the workspace
/// (`ceil(chars / 4)`).
pub fn estimate_text_tokens(text: &str) -> u32 {
    tpt_weave_context::estimate_tokens(text)
}

/// Per-task metrics comparing one baseline capture to one weave capture.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskMetrics {
    /// Task id both captures share.
    pub task_id: String,
    /// Spec: raw context tokens.
    pub raw_tokens: u64,
    /// Spec: delivered tokens.
    pub delivered_tokens: u64,
    /// Spec: output tokens (weave run).
    pub output_tokens: u64,
    /// Spec: total tokens = delivered + JEv + output (weave run).
    pub total_tokens: u64,
    /// Spec: JEv / decision tokens.
    pub jev_tokens: u64,
    /// Gross token reduction: `1 - delivered / raw` (0 when raw is 0).
    pub gross_token_reduction: f64,
    /// **Primary metric** — Net Token Reduction:
    /// `1 - (delivered + jev) / raw` (0 when raw is 0).
    pub net_token_reduction: f64,
    /// Spec: cache savings (tokens).
    pub cache_savings_tokens: u64,
    /// Spec: number of retrieval calls.
    pub retrieval_count: u32,
    /// Spec: context misses.
    pub context_misses: u32,
    /// Spec: unnecessary expansions.
    pub unnecessary_expansions: u32,
    /// Baseline task success.
    pub baseline_success: bool,
    /// Weave task success.
    pub weave_success: bool,
    /// `weave.latency - baseline.latency` (ms; negative = faster).
    pub latency_change_ms: i64,
    /// Latency ratio `weave / baseline` (0.0 when baseline is 0).
    pub latency_change: f64,
    /// Cost delta `weave - baseline` in USD (`None` when either side is
    /// unknown).
    pub cost_change_usd: Option<f64>,
    /// Cost ratio `weave / baseline` (`None` when baseline is 0 or unknown).
    pub cost_change: Option<f64>,
}

impl TaskMetrics {
    /// Computes metrics from a matched baseline/weave pair. The task ids
    /// must be equal.
    pub fn from_pair(
        baseline: &BaselineCapture,
        weave: &WeaveCapture,
    ) -> Result<Self, MetricsError> {
        if baseline.task_id != weave.task_id {
            return Err(MetricsError::TaskIdMismatch {
                baseline: baseline.task_id.clone(),
                weave: weave.task_id.clone(),
            });
        }

        let raw = baseline.raw_tokens;
        let delivered = weave.selected_tokens;
        let jev = weave.jev_tokens;
        let output = weave.output_tokens;
        let total = delivered.saturating_add(jev).saturating_add(output);

        let gross = if raw == 0 {
            0.0
        } else {
            1.0 - (delivered as f64 / raw as f64)
        };
        // Spec §23 primary metric.
        let net = if raw == 0 {
            0.0
        } else {
            1.0 - ((delivered + jev) as f64 / raw as f64)
        };

        let latency_change_ms = weave.latency_ms as i64 - baseline.latency_ms as i64;
        let latency_change = if baseline.latency_ms == 0 {
            0.0
        } else {
            weave.latency_ms as f64 / baseline.latency_ms as f64
        };

        let cost_change_usd = match (baseline.cost_usd, weave.cost_usd) {
            (Some(before), Some(after)) => Some(after - before),
            _ => None,
        };
        let cost_change = match baseline.cost_usd {
            Some(before) if before > 0.0 => weave.cost_usd.map(|after| after / before),
            _ => None,
        };

        Ok(Self {
            task_id: baseline.task_id.clone(),
            raw_tokens: raw,
            delivered_tokens: delivered,
            output_tokens: output,
            total_tokens: total,
            jev_tokens: jev,
            gross_token_reduction: gross,
            net_token_reduction: net,
            cache_savings_tokens: weave.cache_savings_tokens,
            retrieval_count: weave.retrieval_count,
            context_misses: weave.context_misses,
            unnecessary_expansions: weave.unnecessary_expansions,
            baseline_success: baseline.task_success,
            weave_success: weave.task_success,
            latency_change_ms,
            latency_change,
            cost_change_usd,
            cost_change,
        })
    }

    /// Task Success Preservation for this single pair
    /// (spec: successful weave / successful baseline).
    ///
    /// Returns `1.0` when the baseline failed (nothing to preserve),
    /// `0.0` when the baseline succeeded and weave failed, `1.0` when both
    /// succeeded.
    pub fn task_success_preservation(&self) -> f64 {
        // Nothing to preserve when the baseline already failed.
        if self.baseline_success && !self.weave_success {
            0.0
        } else {
            1.0
        }
    }
}

/// Failure while pairing captures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetricsError {
    /// Baseline and weave captures refer to different tasks.
    TaskIdMismatch { baseline: String, weave: String },
}

impl std::fmt::Display for MetricsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricsError::TaskIdMismatch { baseline, weave } => {
                write!(
                    f,
                    "task id mismatch: baseline `{baseline}` vs weave `{weave}`"
                )
            }
        }
    }
}

impl std::error::Error for MetricsError {}

/// Aggregate metrics over an evaluation set (todo.md Phase 11 "Metrics").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AggregateMetrics {
    /// Number of paired tasks.
    pub tasks: usize,
    /// Baseline successes.
    pub baseline_successes: usize,
    /// Weave successes.
    pub weave_successes: usize,
    /// Sum of raw context tokens.
    pub raw_tokens: u64,
    /// Sum of delivered tokens.
    pub delivered_tokens: u64,
    /// Sum of JEv tokens.
    pub jev_tokens: u64,
    /// Sum of output tokens (weave).
    pub output_tokens: u64,
    /// Sum of total tokens (weave: delivered + jev + output).
    pub total_tokens: u64,
    /// Mean gross token reduction.
    pub gross_token_reduction: f64,
    /// Mean **Net Token Reduction** (primary metric).
    pub net_token_reduction: f64,
    /// Sum of cache savings tokens.
    pub cache_savings_tokens: u64,
    /// Sum of retrieval calls.
    pub retrieval_count: u32,
    /// Sum of context misses.
    pub context_misses: u32,
    /// Sum of unnecessary expansions.
    pub unnecessary_expansions: u32,
    /// **Secondary metric** — Task Success Preservation:
    /// weave successes / baseline successes (1.0 when baseline has none).
    pub task_success_preservation: f64,
    /// Mean latency change ratio (weave / baseline); 0 when all baselines
    /// are 0.
    pub latency_change: f64,
    /// Total latency delta in ms (weave - baseline).
    pub latency_change_ms: i64,
    /// Mean cost delta in USD over tasks where both sides were known.
    pub cost_change_usd: Option<f64>,
    /// Mean cost change ratio over tasks with a positive baseline cost.
    pub cost_change: Option<f64>,
}

impl AggregateMetrics {
    /// Folds per-task metrics into aggregate statistics.
    pub fn from_tasks(tasks: &[TaskMetrics]) -> Self {
        if tasks.is_empty() {
            return Self {
                tasks: 0,
                baseline_successes: 0,
                weave_successes: 0,
                raw_tokens: 0,
                delivered_tokens: 0,
                jev_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                gross_token_reduction: 0.0,
                net_token_reduction: 0.0,
                cache_savings_tokens: 0,
                retrieval_count: 0,
                context_misses: 0,
                unnecessary_expansions: 0,
                task_success_preservation: 1.0,
                latency_change: 0.0,
                latency_change_ms: 0,
                cost_change_usd: None,
                cost_change: None,
            };
        }

        let n = tasks.len() as f64;
        let mut baseline_successes = 0usize;
        let mut weave_successes = 0usize;
        let mut raw_tokens = 0u64;
        let mut delivered_tokens = 0u64;
        let mut jev_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut total_tokens = 0u64;
        let mut gross_sum = 0.0f64;
        let mut net_sum = 0.0f64;
        let mut cache_savings_tokens = 0u64;
        let mut retrieval_count = 0u32;
        let mut context_misses = 0u32;
        let mut unnecessary_expansions = 0u32;
        let mut latency_sum = 0.0f64;
        let mut latency_ms = 0i64;
        let mut cost_delta_sum = 0.0f64;
        let mut cost_delta_n = 0usize;
        let mut cost_ratio_sum = 0.0f64;
        let mut cost_ratio_n = 0usize;

        for task in tasks {
            if task.baseline_success {
                baseline_successes += 1;
            }
            if task.weave_success {
                weave_successes += 1;
            }
            raw_tokens += task.raw_tokens;
            delivered_tokens += task.delivered_tokens;
            jev_tokens += task.jev_tokens;
            output_tokens += task.output_tokens;
            total_tokens += task.total_tokens;
            gross_sum += task.gross_token_reduction;
            net_sum += task.net_token_reduction;
            cache_savings_tokens += task.cache_savings_tokens;
            retrieval_count += task.retrieval_count;
            context_misses += task.context_misses;
            unnecessary_expansions += task.unnecessary_expansions;
            latency_sum += task.latency_change;
            latency_ms += task.latency_change_ms;
            if let Some(delta) = task.cost_change_usd {
                cost_delta_sum += delta;
                cost_delta_n += 1;
            }
            if let Some(ratio) = task.cost_change {
                cost_ratio_sum += ratio;
                cost_ratio_n += 1;
            }
        }

        let task_success_preservation = if baseline_successes == 0 {
            1.0
        } else {
            weave_successes as f64 / baseline_successes as f64
        };

        Self {
            tasks: tasks.len(),
            baseline_successes,
            weave_successes,
            raw_tokens,
            delivered_tokens,
            jev_tokens,
            output_tokens,
            total_tokens,
            gross_token_reduction: gross_sum / n,
            net_token_reduction: net_sum / n,
            cache_savings_tokens,
            retrieval_count,
            context_misses,
            unnecessary_expansions,
            task_success_preservation,
            latency_change: latency_sum / n,
            latency_change_ms: latency_ms,
            cost_change_usd: if cost_delta_n == 0 {
                None
            } else {
                Some(cost_delta_sum / cost_delta_n as f64)
            },
            cost_change: if cost_ratio_n == 0 {
                None
            } else {
                Some(cost_ratio_sum / cost_ratio_n as f64)
            },
        }
    }

    /// Human-readable multi-line summary.
    pub fn summary(&self) -> String {
        format!(
            "tasks: {}\nbaseline success: {}\nweave success: {}\n\
             raw tokens: {}\ndelivered tokens: {}\nJEv tokens: {}\n\
             gross reduction: {:.1}%\nnet reduction: {:.1}%\n\
             cache savings: {} tokens\nretrievals: {}\ncontext misses: {}\n\
             unnecessary expansions: {}\ntask success preservation: {:.1}%\n\
             latency change: {:.1}%\nlatency delta: {} ms\ncost change: {}\n",
            self.tasks,
            self.baseline_successes,
            self.weave_successes,
            self.raw_tokens,
            self.delivered_tokens,
            self.jev_tokens,
            self.gross_token_reduction * 100.0,
            self.net_token_reduction * 100.0,
            self.cache_savings_tokens,
            self.retrieval_count,
            self.context_misses,
            self.unnecessary_expansions,
            self.task_success_preservation * 100.0,
            self.latency_change * 100.0,
            self.latency_change_ms,
            self.cost_change_usd
                .map(|d| format!("{d:.6} USD"))
                .unwrap_or_else(|| "n/a".to_string()),
        )
    }
}
