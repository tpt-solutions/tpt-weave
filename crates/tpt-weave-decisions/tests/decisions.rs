//! Phase 8 decision provider/policy/engine tests (todo.md Phase 8).

use std::time::Duration;
use tpt_weave_core::{JevConfig, PrivacyConfig, ProviderConfig};
use tpt_weave_decisions::{
    Decision, DecisionCategory, DecisionEngine, DecisionOutcome, DecisionProvider, DecisionRequest,
    DecisionSource, DeterministicFallbackProvider, FALLBACK_CONFIDENCE, MockProvider,
    ProviderError, SafetyOverride,
};

fn outcome(choice: &str, confidence: f32) -> DecisionOutcome {
    DecisionOutcome {
        decision: Decision {
            choice: choice.to_string(),
            confidence,
        },
        latency: Duration::from_millis(3),
        input_tokens: 10,
        output_tokens: 5,
        cost_usd: Some(0.001),
    }
}

struct FailingProvider;

impl DecisionProvider for FailingProvider {
    fn name(&self) -> &str {
        "failing"
    }

    fn decide(&self, _request: &DecisionRequest) -> Result<DecisionOutcome, ProviderError> {
        Err(ProviderError::Timeout { attempts: 3 })
    }
}

struct InvalidChoiceProvider;

impl DecisionProvider for InvalidChoiceProvider {
    fn name(&self) -> &str {
        "invalid"
    }

    fn decide(&self, _request: &DecisionRequest) -> Result<DecisionOutcome, ProviderError> {
        Ok(outcome("not-a-choice", 0.99))
    }
}

#[test]
fn every_category_has_unique_choices_and_a_safe_choice_in_the_set() {
    for category in DecisionCategory::ALL {
        let choices = category.choices();
        assert!(!choices.is_empty(), "{category} must offer choices");
        assert!(
            choices.contains(&category.safe_choice()),
            "{category} safe choice `{}` not in {:?}",
            category.safe_choice(),
            choices
        );
        let mut seen = choices.to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            seen.len(),
            choices.len(),
            "{category} choices must be unique"
        );
    }
}

#[test]
fn ask_builds_a_request_with_every_choice() {
    let request = DecisionCategory::Relevance.ask("src/lib.rs", "fix the bug");
    assert_eq!(request.choices, ["relevant", "irrelevant"]);
    assert!(request.question.contains("src/lib.rs"));
    assert_eq!(request.context, "fix the bug");
}

#[test]
fn mock_provider_scripts_then_repeats_and_records_requests() {
    let mock = MockProvider::with_script(
        Decision {
            choice: "relevant".to_string(),
            confidence: 0.9,
        },
        [Decision {
            choice: "irrelevant".to_string(),
            confidence: 0.8,
        }],
    );
    let request = DecisionCategory::Relevance.ask("a", "ctx");
    let first = mock.decide(&request).unwrap();
    let second = mock.decide(&request).unwrap();
    let third = mock.decide(&request).unwrap();
    assert_eq!(first.decision.choice, "irrelevant");
    assert_eq!(second.decision.choice, "relevant");
    assert_eq!(third.decision.choice, "relevant");
    assert_eq!(mock.requests().len(), 3);
}

#[test]
fn deterministic_fallback_is_stable_and_within_choices() {
    let provider = DeterministicFallbackProvider::new();
    let request = DecisionCategory::RetrievalDepth.ask("heavy.rs", "ctx");
    let first = provider.choose(&request);
    let second = provider.choose(&request);
    assert_eq!(first, second);
    assert!(request.choices.contains(&first.choice));
    assert_eq!(first.confidence, FALLBACK_CONFIDENCE);
    assert!(first.confidence < 0.70);
}

#[test]
fn policy_accepts_confident_provider_answers() {
    let policy = tpt_weave_decisions::DecisionPolicy::new(0.70);
    let judgement = policy.apply(DecisionCategory::Relevance, outcome("relevant", 0.9));
    assert_eq!(judgement.choice, "relevant");
    assert_eq!(judgement.source, DecisionSource::Provider);
    assert_eq!(judgement.confidence, 0.9);
    assert_eq!(judgement.latency_ms, 3);
    assert_eq!(judgement.input_tokens, 10);
    assert_eq!(judgement.output_tokens, 5);
    assert_eq!(judgement.cost_usd, Some(0.001));
}

#[test]
fn policy_replaces_low_confidence_with_safe_choice() {
    let policy = tpt_weave_decisions::DecisionPolicy::new(0.70);
    let judgement = policy.apply(DecisionCategory::Relevance, outcome("irrelevant", 0.4));
    assert_eq!(judgement.choice, "relevant");
    assert_eq!(judgement.source, DecisionSource::LowConfidence);
    assert_eq!(judgement.confidence, 0.4);
}

#[test]
fn safety_override_beats_provider_and_low_confidence_paths() {
    let policy = tpt_weave_decisions::DecisionPolicy::new(0.70)
        .with_safety_override(DecisionCategory::Relevance, "irrelevant");
    let pass = policy.apply(DecisionCategory::Relevance, outcome("irrelevant", 0.9));
    assert_eq!(pass.source, DecisionSource::SafetyOverride);
    assert_eq!(pass.confidence, 1.0);
    let low = policy.apply(DecisionCategory::Relevance, outcome("relevant", 0.1));
    assert_eq!(low.choice, "irrelevant");
    assert_eq!(low.source, DecisionSource::SafetyOverride);
}

#[test]
fn policy_from_jev_uses_configured_threshold() {
    let jev = JevConfig {
        min_confidence: 0.5,
        ..Default::default()
    };
    let policy = tpt_weave_decisions::DecisionPolicy::from_jev(&jev);
    assert_eq!(policy.min_confidence, 0.5);
    let judgement = policy.apply(DecisionCategory::Relevance, outcome("relevant", 0.6));
    assert_eq!(judgement.source, DecisionSource::Provider);
}

#[test]
fn engine_passes_confident_answers_through_and_logs_them() {
    let engine = DecisionEngine::new(
        MockProvider::answering("keep", 0.95),
        tpt_weave_decisions::DecisionPolicy::new(0.70),
    );
    let judgement = engine.decide(DecisionCategory::Expansion, "mod.rs", "ctx");
    assert_eq!(judgement.choice, "keep");
    assert_eq!(judgement.source, DecisionSource::Provider);
    let entries = engine.log().entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].category, DecisionCategory::Expansion);
    assert_eq!(entries[0].provider, "mock");
    assert!(entries[0].question.contains("mod.rs"));
}

#[test]
fn engine_falls_back_when_the_provider_fails() {
    let engine = DecisionEngine::new(
        FailingProvider,
        tpt_weave_decisions::DecisionPolicy::new(0.70),
    );
    let judgement = engine.decide(DecisionCategory::Relevance, "src.rs", "ctx");
    assert_eq!(judgement.source, DecisionSource::Fallback);
    let error = judgement
        .error
        .expect("fallback records the provider error");
    assert!(error.contains("timeout"), "{error}");
    assert!(
        DecisionCategory::Relevance
            .choices()
            .contains(&judgement.choice.as_str())
    );
    assert_eq!(engine.log().entries().len(), 1);
}

#[test]
fn engine_rejects_out_of_set_choices_and_falls_back_deterministically() {
    let engine = DecisionEngine::new(
        InvalidChoiceProvider,
        tpt_weave_decisions::DecisionPolicy::new(0.70),
    );
    let judgement = engine.decide(DecisionCategory::Relevance, "src.rs", "ctx");
    assert_eq!(judgement.source, DecisionSource::Fallback);
    assert!(
        judgement
            .error
            .as_deref()
            .unwrap_or("")
            .contains("not-a-choice")
    );
    assert!(
        DecisionCategory::Relevance
            .choices()
            .contains(&judgement.choice.as_str())
    );
}

#[test]
fn engine_safety_override_wins_even_when_the_provider_fails() {
    let engine = DecisionEngine::new(
        FailingProvider,
        tpt_weave_decisions::DecisionPolicy::new(0.70)
            .with_safety_override(DecisionCategory::Relevance, "irrelevant"),
    );
    let judgement = engine.decide(DecisionCategory::Relevance, "src.rs", "ctx");
    assert_eq!(judgement.choice, "irrelevant");
    assert_eq!(judgement.source, DecisionSource::SafetyOverride);
    assert_eq!(judgement.error, None);
}

#[test]
fn decision_log_writes_jsonl() {
    let engine = DecisionEngine::new(
        MockProvider::answering("relevant", 0.9),
        tpt_weave_decisions::DecisionPolicy::new(0.70),
    );
    engine.decide(DecisionCategory::Relevance, "a.rs", "ctx");
    engine.decide(DecisionCategory::Expansion, "b.rs", "ctx");
    let path = std::env::temp_dir().join(format!(
        "tpt-weave-decisions-test-{}.jsonl",
        std::process::id()
    ));
    let written = engine.log().write_jsonl(&path).unwrap();
    assert_eq!(written, 2);
    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    for line in lines {
        let value: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(value.get("category").is_some());
        assert!(value.get("judgement").is_some());
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn policy_serde_round_trips() {
    let policy = tpt_weave_decisions::DecisionPolicy::new(0.42)
        .with_safety_override(DecisionCategory::ContextEviction, "keep");
    let json = serde_json::to_string(&policy).unwrap();
    let back: tpt_weave_decisions::DecisionPolicy = serde_json::from_str(&json).unwrap();
    assert_eq!(back, policy);
    assert_eq!(
        back.override_for(DecisionCategory::ContextEviction),
        Some(&SafetyOverride {
            category: DecisionCategory::ContextEviction,
            choice: "keep".to_string(),
        })
    );
}

#[test]
fn config_built_from_jev_and_provider_config_reads_key_from_env_name() {
    // A definitely-unset variable name avoids mutating the process
    // environment (edition 2024 makes set_var unsafe).
    let jev = JevConfig::default();
    let provider = ProviderConfig {
        api_key_env: "TPT_WEAVE_TEST_NO_SUCH_KEY_VAR".to_string(),
        ..Default::default()
    };
    let privacy = PrivacyConfig {
        remote_decisions: true,
        ..PrivacyConfig::default()
    };
    let err = tpt_weave_openrouter_shim(&jev, &provider, &privacy);
    assert!(
        err.contains("missing API key"),
        "expected missing-key error, got: {err}"
    );
}

fn tpt_weave_openrouter_shim(
    jev: &JevConfig,
    provider: &ProviderConfig,
    privacy: &PrivacyConfig,
) -> String {
    use tpt_weave_openrouter::OpenRouterProvider;
    OpenRouterProvider::from_config_with_privacy(jev, provider, privacy, None)
        .err()
        .map(|err| err.to_string())
        .unwrap_or_else(|| "unexpected success".to_string())
}
