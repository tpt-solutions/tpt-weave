//! Decision schemas and categories (todo.md Phase 8 "Decisions", spec.md
//! section 4.2; exact schemas finalised here per spec.md section 26).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

/// The question sent to a decision provider (spec.md section 26).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRequest {
    /// The question to answer.
    pub question: String,
    /// The allowed choices, in canonical order.
    pub choices: Vec<String>,
    /// The reduced deterministic context the decision is made over.
    pub context: String,
}

/// A provider's typed answer (spec.md section 26).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    /// One of [`DecisionRequest::choices`].
    pub choice: String,
    /// Calibrated confidence in `0.0..=1.0`.
    pub confidence: f32,
}

/// A decision answer plus its observability record (todo.md Phase 8:
/// latency, token usage, confidence).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionOutcome {
    /// The typed answer.
    pub decision: Decision,
    /// Wall-clock time the provider took.
    pub latency: Duration,
    /// Input tokens billed for the call.
    pub input_tokens: u32,
    /// Output tokens billed for the call.
    pub output_tokens: u32,
    /// Cost in USD when the provider reports it (spec.md section 4).
    pub cost_usd: Option<f64>,
}

/// A decision category from spec.md section 4.2 (plus tool-result
/// retention), each with its fixed choice set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionCategory {
    /// `candidate -> relevant / irrelevant`
    Relevance,
    /// `representation -> keep / expand / defer`
    Expansion,
    /// `dependency -> traverse / do-not-traverse`
    DependencyTraversal,
    /// `context item -> retain / summarize / release`
    ContextRetention,
    /// `context item -> keep / cache / evict`
    ContextEviction,
    /// `tool result -> retain / summarize / discard`
    ToolResultRetention,
    /// `task -> bugfix / feature / ...`
    TaskClassification,
    /// `none / metadata / ... / full-file`
    RetrievalDepth,
}

impl DecisionCategory {
    /// Every category, in a stable order (todo.md Phase 8 "Decisions").
    pub const ALL: [DecisionCategory; 8] = [
        DecisionCategory::Relevance,
        DecisionCategory::Expansion,
        DecisionCategory::DependencyTraversal,
        DecisionCategory::ContextRetention,
        DecisionCategory::ContextEviction,
        DecisionCategory::ToolResultRetention,
        DecisionCategory::TaskClassification,
        DecisionCategory::RetrievalDepth,
    ];

    /// The fixed choice set for this category (spec.md section 4.2).
    pub fn choices(self) -> &'static [&'static str] {
        match self {
            DecisionCategory::Relevance => &["relevant", "irrelevant"],
            DecisionCategory::Expansion => &["keep", "expand", "defer"],
            DecisionCategory::DependencyTraversal => &["traverse", "do-not-traverse"],
            DecisionCategory::ContextRetention => &["retain", "summarize", "release"],
            DecisionCategory::ContextEviction => &["keep", "cache", "evict"],
            DecisionCategory::ToolResultRetention => &["retain", "summarize", "discard"],
            DecisionCategory::TaskClassification => &[
                "bugfix",
                "feature",
                "refactor",
                "docs",
                "test",
                "build",
                "exploration",
            ],
            DecisionCategory::RetrievalDepth => &[
                "none",
                "metadata",
                "symbols",
                "signatures",
                "skeleton",
                "implementation",
                "full-file",
            ],
        }
    }

    /// The deterministic choice used when confidence is below the policy
    /// threshold (todo.md Policy: "low-confidence fallback"): a
    /// safety-leaning default per category.
    pub fn safe_choice(self) -> &'static str {
        match self {
            DecisionCategory::Relevance => "relevant",
            DecisionCategory::Expansion => "keep",
            DecisionCategory::DependencyTraversal => "traverse",
            DecisionCategory::ContextRetention => "retain",
            DecisionCategory::ContextEviction => "keep",
            DecisionCategory::ToolResultRetention => "retain",
            DecisionCategory::TaskClassification => "exploration",
            DecisionCategory::RetrievalDepth => "skeleton",
        }
    }

    /// Builds the provider request for `subject` under `context`.
    pub fn ask(self, subject: &str, context: &str) -> DecisionRequest {
        let question = match self {
            DecisionCategory::Relevance => {
                format!("Is `{subject}` relevant to the task?")
            }
            DecisionCategory::Expansion => {
                format!("How should `{subject}` be represented?")
            }
            DecisionCategory::DependencyTraversal => {
                format!("Traverse dependency `{subject}`?")
            }
            DecisionCategory::ContextRetention => {
                format!("Retain context item `{subject}`?")
            }
            DecisionCategory::ContextEviction => {
                format!("What should happen to context item `{subject}`?")
            }
            DecisionCategory::ToolResultRetention => {
                format!("What should happen to the tool result for `{subject}`?")
            }
            DecisionCategory::TaskClassification => format!("Classify this task: {subject}"),
            DecisionCategory::RetrievalDepth => {
                format!("What retrieval depth is required for `{subject}`?")
            }
        };
        DecisionRequest {
            question,
            choices: self.choices().iter().map(|c| (*c).to_string()).collect(),
            context: context.to_string(),
        }
    }
}

impl fmt::Display for DecisionCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            DecisionCategory::Relevance => "relevance",
            DecisionCategory::Expansion => "expansion",
            DecisionCategory::DependencyTraversal => "dependency_traversal",
            DecisionCategory::ContextRetention => "context_retention",
            DecisionCategory::ContextEviction => "context_eviction",
            DecisionCategory::ToolResultRetention => "tool_result_retention",
            DecisionCategory::TaskClassification => "task_classification",
            DecisionCategory::RetrievalDepth => "retrieval_depth",
        };
        f.write_str(name)
    }
}
