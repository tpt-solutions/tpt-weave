//! The decision engine: provider + policy + evaluation log
//! (todo.md Phase 8 "Decisions" and "Log decision outcomes for
//! evaluation").

use crate::policy::{DecisionPolicy, DecisionSource, Judgement};
use crate::provider::DecisionProvider;
use crate::schema::DecisionCategory;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

/// One logged decision (todo.md Phase 8 "Log decision outcomes for
/// evaluation"; consumed by the Phase 11 eval harness).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionRecord {
    /// The category decided.
    pub category: DecisionCategory,
    /// The subject the question was about.
    pub subject: String,
    /// The exact question asked.
    pub question: String,
    /// The choices offered.
    pub choices: Vec<String>,
    /// The provider that was asked.
    pub provider: String,
    /// The judged outcome.
    pub judgement: Judgement,
}

/// In-memory decision log with optional JSONL export
/// (todo.md Phase 8 "Log decision outcomes for evaluation").
#[derive(Debug, Default)]
pub struct DecisionLog {
    entries: Mutex<Vec<DecisionRecord>>,
}

impl DecisionLog {
    /// An empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a record.
    pub fn record(&self, record: DecisionRecord) {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(record);
    }

    /// All records, in order.
    pub fn entries(&self) -> Vec<DecisionRecord> {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Removes every record.
    pub fn clear(&self) {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// Writes every record as JSON Lines to `path` (truncating), for the
    /// evaluation harness. Returns the number of records written.
    pub fn write_jsonl(&self, path: impl AsRef<Path>) -> io::Result<usize> {
        let entries = self.entries();
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = String::new();
        for entry in &entries {
            out.push_str(&serde_json::to_string(entry).map_err(io::Error::other)?);
            out.push('\n');
        }
        fs::write(path, out)?;
        Ok(entries.len())
    }
}

/// Runs decisions through a provider, judges them against the policy and
/// records every outcome. `decide` never fails: provider errors become
/// deterministic fallbacks (spec.md section 3.1).
pub struct DecisionEngine {
    provider: Box<dyn DecisionProvider>,
    policy: DecisionPolicy,
    log: DecisionLog,
}

impl DecisionEngine {
    /// Creates an engine around a provider and policy.
    pub fn new(provider: impl DecisionProvider + 'static, policy: DecisionPolicy) -> Self {
        Self {
            provider: Box::new(provider),
            policy,
            log: DecisionLog::new(),
        }
    }

    /// The engine's shared log.
    pub fn log(&self) -> &DecisionLog {
        &self.log
    }

    /// The policy in force.
    pub fn policy(&self) -> &DecisionPolicy {
        &self.policy
    }

    /// Decides `category` for `subject` under `context`.
    ///
    /// Order (todo.md Phase 8 Policy): provider answer → choice
    /// validation → safety override → confidence threshold →
    /// deterministic fallback on provider failure.
    pub fn decide(&self, category: DecisionCategory, subject: &str, context: &str) -> Judgement {
        let request = category.ask(subject, context);
        let provider_name = self.provider.name().to_string();

        let started = Instant::now();
        let outcome = self.provider.decide(&request);
        let fallback_latency = started.elapsed();

        let judgement = match outcome {
            Ok(outcome) => {
                if !request.choices.contains(&outcome.decision.choice) {
                    self.policy.fallback(
                        category,
                        &request,
                        outcome.latency,
                        format!(
                            "provider returned choice `{}` outside allowed set",
                            outcome.decision.choice
                        ),
                    )
                } else {
                    self.policy.apply(category, outcome)
                }
            }
            Err(error) => {
                self.policy
                    .fallback(category, &request, fallback_latency, error.to_string())
            }
        };

        // A safety override must also win over the fallback path.
        let judgement = if judgement.source == DecisionSource::Fallback {
            if let Some(over) = self.policy.override_for(category) {
                if category.choices().contains(&over.choice.as_str()) {
                    Judgement {
                        choice: over.choice.clone(),
                        confidence: 1.0,
                        source: DecisionSource::SafetyOverride,
                        error: None,
                        ..judgement
                    }
                } else {
                    judgement
                }
            } else {
                judgement
            }
        } else {
            judgement
        };

        self.log.record(DecisionRecord {
            category,
            subject: subject.to_string(),
            question: request.question.clone(),
            choices: request.choices.clone(),
            provider: provider_name,
            judgement: judgement.clone(),
        });

        judgement
    }
}
