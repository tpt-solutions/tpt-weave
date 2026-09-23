//! Decision provider abstraction, JEv categories, policy and evaluation
//! logging (todo.md Phase 8, spec.md sections 4 and 18).
//!
//! The [`DecisionProvider`] trait keeps the application decoupled from any
//! specific backend: [`provider::MockProvider`] and
//! [`provider::DeterministicFallbackProvider`] ship here, while the
//! OpenRouter JEv client lives in `tpt-weave-openrouter`. The
//! [`DecisionEngine`] runs every decision through the configurable
//! [`DecisionPolicy`] (confidence threshold, safety overrides,
//! low-confidence fallback) and records outcomes for evaluation — and it
//! never fails: provider errors degrade to the deterministic fallback.

#![forbid(unsafe_code)]

mod engine;
mod policy;
mod provider;
mod schema;

pub use engine::{DecisionEngine, DecisionLog, DecisionRecord};
pub use policy::{DecisionPolicy, DecisionSource, Judgement, SafetyOverride};
pub use provider::{
    DecisionProvider, DeterministicFallbackProvider, MockProvider, ProviderError,
    FALLBACK_CONFIDENCE,
};
pub use schema::{Decision, DecisionCategory, DecisionOutcome, DecisionRequest};
