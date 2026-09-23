//! Decision policy: thresholds, safety overrides, low-confidence
//! fallback (todo.md Phase 8 "Policy", spec.md section 4.3).

use crate::provider::DeterministicFallbackProvider;
use crate::schema::{DecisionCategory, DecisionOutcome};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tpt_weave_core::JevConfig;

/// Where the final choice came from (todo.md Phase 8 policy + evaluation
/// logging).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionSource {
    /// The provider's answer, at or above the confidence threshold.
    Provider,
    /// Confidence was below the threshold; the category's safe choice
    /// replaced it.
    LowConfidence,
    /// A deterministic safety override forced the choice.
    SafetyOverride,
    /// The provider failed or answered outside the allowed choices; the
    /// deterministic fallback decided.
    Fallback,
}

/// A deterministic safety override for one category
/// (todo.md Phase 8 "deterministic safety overrides").
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetyOverride {
    /// The category the override applies to.
    pub category: DecisionCategory,
    /// The choice that category must always receive.
    pub choice: String,
}

/// The final judged decision handed back to callers (todo.md Phase 8).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Judgement {
    /// Which category was decided.
    pub category: DecisionCategory,
    /// The final choice (guaranteed within the category's choices).
    pub choice: String,
    /// Confidence as recorded (provider confidence, fallback confidence,
    /// or 1.0 for safety overrides).
    pub confidence: f32,
    /// Which path produced the choice.
    pub source: DecisionSource,
    /// Provider latency in milliseconds (0 for local paths).
    pub latency_ms: u64,
    /// Input tokens billed by the provider (0 for local paths).
    pub input_tokens: u32,
    /// Output tokens billed by the provider (0 for local paths).
    pub output_tokens: u32,
    /// Provider-reported cost in USD, when available.
    pub cost_usd: Option<f64>,
    /// Provider error detail when [`DecisionSource::Fallback`] was used.
    pub error: Option<String>,
}

/// Thresholds and deterministic overrides applied to provider answers
/// (todo.md Phase 8 "Policy"; spec.md section 4.3: configurable).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionPolicy {
    /// Minimum confidence required to accept a provider answer.
    pub min_confidence: f32,
    /// Deterministic safety overrides, applied before anything else.
    pub safety_overrides: Vec<SafetyOverride>,
}

impl DecisionPolicy {
    /// Creates a policy with an explicit threshold.
    pub fn new(min_confidence: f32) -> Self {
        Self {
            min_confidence,
            safety_overrides: Vec::new(),
        }
    }

    /// Builds the policy from JEv configuration (todo.md: configurable
    /// thresholds via `.tpt-weave/manifest.toml`).
    pub fn from_jev(jev: &JevConfig) -> Self {
        Self::new(jev.min_confidence)
    }

    /// Adds a safety override for one category.
    pub fn with_safety_override(
        mut self,
        category: DecisionCategory,
        choice: impl Into<String>,
    ) -> Self {
        self.safety_overrides.push(SafetyOverride {
            category,
            choice: choice.into(),
        });
        self
    }

    /// The override for `category`, if any.
    pub fn override_for(&self, category: DecisionCategory) -> Option<&SafetyOverride> {
        self.safety_overrides
            .iter()
            .find(|o| o.category == category)
    }

    /// Judges a provider outcome against the policy:
    ///
    /// 1. a matching safety override forces its choice;
    /// 2. below-threshold confidence falls back to the category's safe
    ///    choice ([`DecisionSource::LowConfidence`]);
    /// 3. otherwise the provider's answer stands.
    pub fn apply(&self, category: DecisionCategory, outcome: DecisionOutcome) -> Judgement {
        let judgement = Judgement {
            category,
            choice: outcome.decision.choice.clone(),
            confidence: outcome.decision.confidence,
            source: DecisionSource::Provider,
            latency_ms: outcome.latency.as_millis() as u64,
            input_tokens: outcome.input_tokens,
            output_tokens: outcome.output_tokens,
            cost_usd: outcome.cost_usd,
            error: None,
        };

        if let Some(over) = self.override_for(category) {
            if category.choices().contains(&over.choice.as_str()) {
                return Judgement {
                    choice: over.choice.clone(),
                    confidence: 1.0,
                    source: DecisionSource::SafetyOverride,
                    ..judgement
                };
            }
        }

        if outcome.decision.confidence < self.min_confidence {
            return Judgement {
                choice: category.safe_choice().to_string(),
                source: DecisionSource::LowConfidence,
                ..judgement
            };
        }

        judgement
    }

    /// Builds the fallback judgement used when the provider fails or
    /// answers outside the allowed choices (spec.md section 3.1: remain
    /// useful when JEv is unavailable).
    pub fn fallback(
        &self,
        category: DecisionCategory,
        request: &crate::schema::DecisionRequest,
        latency: Duration,
        error: String,
    ) -> Judgement {
        let fallback = DeterministicFallbackProvider::new();
        let decision = fallback.choose(request);
        Judgement {
            category,
            choice: decision.choice,
            confidence: decision.confidence,
            source: DecisionSource::Fallback,
            latency_ms: latency.as_millis() as u64,
            input_tokens: 0,
            output_tokens: 0,
            cost_usd: None,
            error: Some(error),
        }
    }
}
