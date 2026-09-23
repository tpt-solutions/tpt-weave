//! Run captures for baseline and tpt-weave executions (todo.md Phase 11
//! "Baseline" / "tpt-weave"; spec.md section 23).

use serde::{Deserialize, Serialize};

/// One unoptimised (baseline) agent run for a task.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BaselineCapture {
    /// Stable task id within the benchmark corpus.
    pub task_id: String,
    /// Natural-language task description.
    #[serde(default)]
    pub task: String,
    /// The full raw context handed to the model (may be empty when only
    /// token counts were recorded).
    #[serde(default)]
    pub raw_context: String,
    /// Raw context token count (spec: "raw context tokens").
    pub raw_tokens: u64,
    /// Model completion text.
    #[serde(default)]
    pub model_output: String,
    /// Output token count (spec: "output tokens").
    #[serde(default)]
    pub output_tokens: u64,
    /// Whether the task succeeded under the baseline.
    pub task_success: bool,
    /// End-to-end latency in milliseconds.
    pub latency_ms: u64,
    /// Total run cost in USD, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

impl BaselineCapture {
    /// Creates a capture with empty text fields.
    pub fn new(task_id: impl Into<String>, raw_tokens: u64, task_success: bool) -> Self {
        Self {
            task_id: task_id.into(),
            task: String::new(),
            raw_context: String::new(),
            raw_tokens,
            model_output: String::new(),
            output_tokens: 0,
            task_success,
            latency_ms: 0,
            cost_usd: None,
        }
    }

    /// Sets the task description (chainable).
    pub fn with_task(mut self, task: impl Into<String>) -> Self {
        self.task = task.into();
        self
    }

    /// Sets the raw context text and derives [`Self::raw_tokens`] when 0.
    pub fn with_raw_context(mut self, text: impl Into<String>) -> Self {
        self.raw_context = text.into();
        if self.raw_tokens == 0 {
            self.raw_tokens = u64::from(super::metrics::estimate_text_tokens(&self.raw_context));
        }
        self
    }

    /// Sets the model output and derives [`Self::output_tokens`] when 0.
    pub fn with_model_output(mut self, text: impl Into<String>) -> Self {
        self.model_output = text.into();
        if self.output_tokens == 0 {
            self.output_tokens =
                u64::from(super::metrics::estimate_text_tokens(&self.model_output));
        }
        self
    }

    /// Sets latency in milliseconds.
    pub fn with_latency_ms(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    /// Sets cost in USD.
    pub fn with_cost_usd(mut self, cost_usd: f64) -> Self {
        self.cost_usd = Some(cost_usd);
        self
    }

    /// Sets task success.
    pub fn with_success(mut self, success: bool) -> Self {
        self.task_success = success;
        self
    }
}

/// One optimised (tpt-weave) agent run for the same task as a
/// [`BaselineCapture`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeaveCapture {
    /// Must match the paired [`BaselineCapture::task_id`].
    pub task_id: String,
    /// Natural-language task description.
    #[serde(default)]
    pub task: String,
    /// Context actually delivered to the model.
    #[serde(default)]
    pub selected_context: String,
    /// Delivered context token count (spec: "delivered tokens").
    pub selected_tokens: u64,
    /// JEv decision context / questions (when decisions were used).
    #[serde(default)]
    pub jev_context: String,
    /// Decision overhead tokens (spec: "JEv tokens" / "decision tokens").
    pub jev_tokens: u64,
    /// Model completion text.
    #[serde(default)]
    pub model_output: String,
    /// Output token count.
    #[serde(default)]
    pub output_tokens: u64,
    /// Whether the task succeeded under tpt-weave.
    pub task_success: bool,
    /// End-to-end latency in milliseconds.
    pub latency_ms: u64,
    /// Total run cost in USD, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    /// Number of retrieval/expansion calls made during the run
    /// (spec: "number of retrieval calls").
    #[serde(default)]
    pub retrieval_count: u32,
    /// Context misses: items that had to be expanded after the fact
    /// (spec: "context misses").
    #[serde(default)]
    pub context_misses: u32,
    /// Expansions that turned out unnecessary (spec: "unnecessary
    /// expansions").
    #[serde(default)]
    pub unnecessary_expansions: u32,
    /// Cache hits observed during the run.
    #[serde(default)]
    pub cache_hits: u32,
    /// Tokens saved by cache hits (not re-delivered / re-computed).
    #[serde(default)]
    pub cache_savings_tokens: u64,
}

impl WeaveCapture {
    /// Creates a capture with empty text fields.
    pub fn new(task_id: impl Into<String>, selected_tokens: u64, task_success: bool) -> Self {
        Self {
            task_id: task_id.into(),
            task: String::new(),
            selected_context: String::new(),
            selected_tokens,
            jev_context: String::new(),
            jev_tokens: 0,
            model_output: String::new(),
            output_tokens: 0,
            task_success,
            latency_ms: 0,
            cost_usd: None,
            retrieval_count: 0,
            context_misses: 0,
            unnecessary_expansions: 0,
            cache_hits: 0,
            cache_savings_tokens: 0,
        }
    }

    /// Sets the task description (chainable).
    pub fn with_task(mut self, task: impl Into<String>) -> Self {
        self.task = task.into();
        self
    }

    /// Sets the delivered context and derives [`Self::selected_tokens`] when 0.
    pub fn with_selected_context(mut self, text: impl Into<String>) -> Self {
        self.selected_context = text.into();
        if self.selected_tokens == 0 {
            self.selected_tokens =
                u64::from(super::metrics::estimate_text_tokens(&self.selected_context));
        }
        self
    }

    /// Sets the JEv context and derives [`Self::jev_tokens`] when 0.
    pub fn with_jev_context(mut self, text: impl Into<String>) -> Self {
        self.jev_context = text.into();
        if self.jev_tokens == 0 {
            self.jev_tokens = u64::from(super::metrics::estimate_text_tokens(&self.jev_context));
        }
        self
    }

    /// Sets explicit JEv token count (overrides text-derived value).
    pub fn with_jev_tokens(mut self, jev_tokens: u64) -> Self {
        self.jev_tokens = jev_tokens;
        self
    }

    /// Sets the model output and derives [`Self::output_tokens`] when 0.
    pub fn with_model_output(mut self, text: impl Into<String>) -> Self {
        self.model_output = text.into();
        if self.output_tokens == 0 {
            self.output_tokens =
                u64::from(super::metrics::estimate_text_tokens(&self.model_output));
        }
        self
    }

    /// Sets latency in milliseconds.
    pub fn with_latency_ms(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    /// Sets cost in USD.
    pub fn with_cost_usd(mut self, cost_usd: f64) -> Self {
        self.cost_usd = Some(cost_usd);
        self
    }

    /// Sets task success.
    pub fn with_success(mut self, success: bool) -> Self {
        self.task_success = success;
        self
    }

    /// Sets retrieval / miss / expansion / cache counters in one call.
    pub fn with_counts(
        mut self,
        retrieval_count: u32,
        context_misses: u32,
        unnecessary_expansions: u32,
    ) -> Self {
        self.retrieval_count = retrieval_count;
        self.context_misses = context_misses;
        self.unnecessary_expansions = unnecessary_expansions;
        self
    }

    /// Records cache hit statistics.
    pub fn with_cache(mut self, hits: u32, savings_tokens: u64) -> Self {
        self.cache_hits = hits;
        self.cache_savings_tokens = savings_tokens;
        self
    }
}
