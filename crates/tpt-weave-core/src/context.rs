//! Hierarchical context request/response types (spec.md sections 9, 10, 16
//! and 26).

use crate::config::DEFAULT_CONTEXT_BUDGET;
use crate::ids::{ContextId, RepositoryId, SymbolId};
use crate::tokens::TokenAccounting;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Hierarchical representation level (spec.md section 9). Declaration order
/// defines the partial order: `Metadata < ... < Full`.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContextLevel {
    /// Level 0 — path, module, LOC, export counts.
    Metadata,
    /// Level 1 — symbol names.
    Symbols,
    /// Level 2 — signatures only.
    Signatures,
    /// Level 3 — structure without function bodies.
    Skeleton,
    /// Level 4 — requested implementation only.
    Implementation,
    /// Level 5 — complete source.
    Full,
}

impl ContextLevel {
    /// All levels ordered from cheapest to most detailed.
    pub const ALL: [ContextLevel; 6] = [
        ContextLevel::Metadata,
        ContextLevel::Symbols,
        ContextLevel::Signatures,
        ContextLevel::Skeleton,
        ContextLevel::Implementation,
        ContextLevel::Full,
    ];

    /// Returns the numeric level (0..=5).
    pub fn as_number(self) -> u8 {
        self as u8
    }

    /// Inverse of [`ContextLevel::as_number`].
    pub fn from_number(number: u8) -> Option<Self> {
        Self::ALL.get(number as usize).copied()
    }

    /// Stable snake_case name (matches the serde representation).
    pub fn name(self) -> &'static str {
        match self {
            ContextLevel::Metadata => "metadata",
            ContextLevel::Symbols => "symbols",
            ContextLevel::Signatures => "signatures",
            ContextLevel::Skeleton => "skeleton",
            ContextLevel::Implementation => "implementation",
            ContextLevel::Full => "full",
        }
    }
}

impl fmt::Display for ContextLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// What a context candidate is drawn from.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextSource {
    /// Repository-level overview material.
    Repository,
    /// A single file (repository-relative path).
    File(String),
    /// A module (`::`-separated path).
    Module(String),
    /// A symbol.
    Symbol(SymbolId),
    /// A dependency package.
    Dependency(String),
    /// Reduced output of a tool invocation (the command that produced it).
    ToolOutput(String),
}

impl ContextSource {
    /// Deterministic key used to derive [`ContextId`]s.
    pub fn key(&self) -> String {
        match self {
            ContextSource::Repository => "repository".to_string(),
            ContextSource::File(path) => format!("file:{path}"),
            ContextSource::Module(module) => format!("module:{module}"),
            ContextSource::Symbol(symbol) => format!("symbol:{}", symbol.canonical_key()),
            ContextSource::Dependency(package) => format!("dependency:{package}"),
            ContextSource::ToolOutput(command) => format!("tool:{command}"),
        }
    }
}

/// A unit of context that may be selected for delivery (spec.md section 26).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextCandidate {
    pub id: ContextId,
    pub source: ContextSource,
    pub level: ContextLevel,
    /// Estimated tokens this candidate occupies at `level`.
    pub token_estimate: u32,
    /// Ids of candidates that should be delivered together with this one.
    pub relationships: Vec<ContextId>,
    /// Deterministic relevance score in `0.0..=1.0`, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
}

impl ContextCandidate {
    /// Creates a candidate with a deterministic [`ContextId`] derived from its
    /// source and level.
    pub fn new(source: ContextSource, level: ContextLevel, token_estimate: u32) -> Self {
        let id = ContextId::deterministic(&[&source.key(), &level.as_number().to_string()]);
        Self {
            id,
            source,
            level,
            token_estimate,
            relationships: Vec::new(),
            score: None,
        }
    }

    /// Records a related candidate that must travel with this one.
    pub fn with_relationship(mut self, related: ContextId) -> Self {
        self.relationships.push(related);
        self
    }

    /// Attaches a relevance score.
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = Some(score);
        self
    }
}

/// Request for task context (spec.md sections 10 and 26).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextRequest {
    pub repository: RepositoryId,
    /// Natural-language task description used for relevance selection.
    pub task: String,
    /// Token ceiling for the delivered context.
    pub budget_tokens: u32,
    /// Deepest level deliverable without an explicit `expand`.
    pub max_level: ContextLevel,
    /// Whether dependency packages may enter the candidate set.
    pub include_dependencies: bool,
}

impl ContextRequest {
    /// Default budget; mirrors [`crate::config::DEFAULT_CONTEXT_BUDGET`].
    pub const DEFAULT_BUDGET: u32 = DEFAULT_CONTEXT_BUDGET;

    /// Creates a request with conservative defaults:
    /// budget [`Self::DEFAULT_BUDGET`], max level `Skeleton`, dependencies on.
    pub fn new(repository: impl Into<String>, task: impl Into<String>) -> Self {
        Self {
            repository: RepositoryId::new(repository),
            task: task.into(),
            budget_tokens: Self::DEFAULT_BUDGET,
            max_level: ContextLevel::Skeleton,
            include_dependencies: true,
        }
    }

    /// Overrides the token budget.
    pub fn with_budget_tokens(mut self, budget_tokens: u32) -> Self {
        self.budget_tokens = budget_tokens;
        self
    }

    /// Caps the deliverable representation level.
    pub fn with_max_level(mut self, max_level: ContextLevel) -> Self {
        self.max_level = max_level;
        self
    }

    /// Excludes dependency packages from the candidate set.
    pub fn without_dependencies(mut self) -> Self {
        self.include_dependencies = false;
        self
    }
}

/// Delivered context: the selected candidates plus token accounting
/// (spec.md sections 16 and 26).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextResponse {
    /// Deterministic id for this selection (stable for an identical set).
    pub context_id: ContextId,
    pub repository: RepositoryId,
    /// Selected candidates in delivery order.
    pub candidates: Vec<ContextCandidate>,
    pub tokens: TokenAccounting,
    /// Decision provider that approved the selection
    /// (e.g. `typesafe/jev-1.13`); `None` means deterministic only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_provider: Option<String>,
}

impl ContextResponse {
    /// Builds a response, deriving `context_id` from the repository and the
    /// selected candidate ids.
    pub fn new(
        repository: RepositoryId,
        candidates: Vec<ContextCandidate>,
        tokens: TokenAccounting,
        decision_provider: Option<String>,
    ) -> Self {
        let mut parts: Vec<String> = vec![repository.as_str().to_string()];
        parts.extend(candidates.iter().map(|c| c.id.as_str().to_string()));
        let part_refs: Vec<&str> = parts.iter().map(String::as_str).collect();
        let context_id = ContextId::deterministic(&part_refs);
        Self {
            context_id,
            repository,
            candidates,
            tokens,
            decision_provider,
        }
    }

    /// Sum of all candidate token estimates (may differ from
    /// `tokens.selected_tokens` when estimates were refined after selection).
    pub fn total_tokens(&self) -> u64 {
        self.candidates
            .iter()
            .map(|c| u64::from(c.token_estimate))
            .sum()
    }
}

