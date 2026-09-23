//! Decision provider abstraction plus local providers (todo.md Phase 8
//! "Provider abstraction"; spec.md sections 18 and 3.1: remain useful when
//! JEv is unavailable).

use crate::schema::{Decision, DecisionOutcome, DecisionRequest};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tpt_weave_core::hash::fnv1a64;

/// Confidence assigned to deterministic fallback decisions — deliberately
/// below the default `min_confidence` (0.70) so policy treats it as
/// untrusted model output would be.
pub const FALLBACK_CONFIDENCE: f32 = 0.35;

/// Errors from a decision provider (todo.md Phase 8 "provider errors").
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderError {
    /// The API key environment variable is unset or empty.
    MissingApiKey { env: String },
    /// Transport failure after all retry attempts.
    Transport { detail: String, attempts: u32 },
    /// The request timed out after all retry attempts.
    Timeout { attempts: u32 },
    /// The provider answered with a non-success status.
    Http { status: u16, message: String },
    /// The response body did not match the decisions schema.
    InvalidResponse { detail: String },
    /// Configuration is unusable (bad endpoint, invalid threshold, ...).
    Config { detail: String },
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::MissingApiKey { env } => {
                write!(f, "missing API key: set the {env} environment variable")
            }
            ProviderError::Transport { detail, attempts } => {
                write!(f, "transport error after {attempts} attempt(s): {detail}")
            }
            ProviderError::Timeout { attempts } => {
                write!(f, "timeout after {attempts} attempt(s)")
            }
            ProviderError::Http { status, message } => {
                write!(f, "provider error {status}: {message}")
            }
            ProviderError::InvalidResponse { detail } => {
                write!(f, "invalid provider response: {detail}")
            }
            ProviderError::Config { detail } => write!(f, "provider config error: {detail}"),
        }
    }
}

impl std::error::Error for ProviderError {}

/// A structured decision source (spec.md section 18: the application is
/// never coupled to a specific provider).
pub trait DecisionProvider {
    /// Stable provider name for logs (e.g. `mock`, `openrouter`).
    fn name(&self) -> &str;

    /// Answers one decision request.
    fn decide(&self, request: &DecisionRequest) -> Result<DecisionOutcome, ProviderError>;
}

/// A scripted provider for tests and dry runs (todo.md Phase 8 "mock
/// provider").
pub struct MockProvider {
    name: String,
    default: Decision,
    script: Mutex<VecDeque<Decision>>,
    requests: Mutex<Vec<DecisionRequest>>,
}

impl MockProvider {
    /// A mock that always answers with `default`.
    pub fn new(default: Decision) -> Self {
        Self {
            name: "mock".to_string(),
            default,
            script: Mutex::new(VecDeque::new()),
            requests: Mutex::new(Vec::new()),
        }
    }

    /// A mock that consumes `script` before repeating `default`.
    pub fn with_script(default: Decision, script: impl IntoIterator<Item = Decision>) -> Self {
        let mut provider = Self::new(default);
        *provider.script.lock().unwrap_or_else(|e| e.into_inner()) =
            script.into_iter().collect();
        provider
    }

    /// A mock configured from a choice + confidence pair.
    pub fn answering(choice: impl Into<String>, confidence: f32) -> Self {
        Self::new(Decision {
            choice: choice.into(),
            confidence,
        })
    }

    /// Every request received, in order (todo.md Phase 8 mock provider).
    pub fn requests(&self) -> Vec<DecisionRequest> {
        self.requests
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl DecisionProvider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn decide(&self, request: &DecisionRequest) -> Result<DecisionOutcome, ProviderError> {
        self.requests
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(request.clone());
        let started = Instant::now();
        let decision = {
            let mut script = self.script.lock().unwrap_or_else(|e| e.into_inner());
            script.pop_front().unwrap_or_else(|| self.default.clone())
        };
        Ok(DecisionOutcome {
            decision,
            latency: started.elapsed(),
            input_tokens: 0,
            output_tokens: 0,
            cost_usd: None,
        })
    }
}

/// Deterministic fallback provider (todo.md Phase 8: "deterministic
/// fallback provider"): picks a choice from a stable hash of the request,
/// with a confidence below any sensible threshold.
pub struct DeterministicFallbackProvider {
    name: String,
}

impl DeterministicFallbackProvider {
    /// Creates the fallback provider.
    pub fn new() -> Self {
        Self {
            name: "deterministic".to_string(),
        }
    }

    /// Picks a choice deterministically: same request → same choice,
    /// always within `request.choices`.
    pub fn choose(&self, request: &DecisionRequest) -> Decision {
        if request.choices.is_empty() {
            return Decision {
                choice: String::new(),
                confidence: 0.0,
            };
        }
        let material = format!("{}\u{1f}{}", request.question, request.context);
        let index = (fnv1a64(material.as_bytes()) % request.choices.len() as u64) as usize;
        Decision {
            choice: request.choices[index].clone(),
            confidence: FALLBACK_CONFIDENCE,
        }
    }
}

impl Default for DeterministicFallbackProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DecisionProvider for DeterministicFallbackProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn decide(&self, request: &DecisionRequest) -> Result<DecisionOutcome, ProviderError> {
        let started = Instant::now();
        Ok(DecisionOutcome {
            decision: self.choose(request),
            latency: started.elapsed(),
            input_tokens: 0,
            output_tokens: 0,
            cost_usd: None,
        })
    }
}
