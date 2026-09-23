//! OpenRouter Decisions API client (JEv; todo.md Phase 8 "OpenRouter",
//! spec.md sections 4 and 18).
//!
//! Wire format (OpenRouter `POST /api/alpha/decisions`, verified against
//! the public API reference):
//!
//! - request: `{ model, state, questions: { answer: { type: "choice",
//!   instructions, criteria } } }`;
//! - response: `{ answers: { answer: { type: "choice", choice,
//!   confidence, probabilities } }, usage: { input_tokens,
//!   output_tokens, cost } }`.
//!
//! Retries cover transport failures and retryable statuses (429, 5xx);
//! API-key and validation errors fail immediately.

#![forbid(unsafe_code)]

use serde::Deserialize;
use std::time::Duration;
use tpt_weave_core::{JevConfig, ProviderConfig};
use tpt_weave_decisions::{
    Decision, DecisionOutcome, DecisionProvider, DecisionRequest, ProviderError,
};

/// Default question key inside the `questions` map.
pub const QUESTION_KEY: &str = "answer";

/// User-agent / `X-Title` attribution header value.
pub const APP_TITLE: &str = "tpt-weave";

/// Confidence assumed when the provider omits it: mid-range, so the
/// policy threshold still treats the answer as untrusted.
pub const DEFAULT_CONFIDENCE: f32 = 0.5;

/// One parsed decision answer plus usage (todo.md Phase 8: token usage,
/// confidence).
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedAnswer {
    /// The selected choice.
    pub choice: String,
    /// The reported (or defaulted) confidence.
    pub confidence: f32,
    /// `usage.input_tokens`.
    pub input_tokens: u32,
    /// `usage.output_tokens`.
    pub output_tokens: u32,
    /// `usage.cost` in USD, when present.
    pub cost_usd: Option<f64>,
    /// Response id, when present (for evaluation logs).
    pub response_id: Option<String>,
    /// The dated model snapshot that served the request.
    pub served_model: Option<String>,
}

/// Blocking OpenRouter Decisions API client (spec.md section 18:
/// provider-neutral interface implemented for OpenRouter).
pub struct OpenRouterProvider {
    client: reqwest::blocking::Client,
    endpoint: String,
    api_key: String,
    model: String,
    max_retries: u32,
    backoff: Duration,
}

impl OpenRouterProvider {
    /// Builds a client with an explicit API key (tests, custom loaders).
    pub fn new(
        endpoint: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
        timeout: Duration,
        max_retries: u32,
    ) -> Result<Self, ProviderError> {
        let endpoint = endpoint.into();
        if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
            return Err(ProviderError::Config {
                detail: format!("endpoint must be http(s): {endpoint}"),
            });
        }
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(ProviderError::Config {
                detail: "api key must not be empty".to_string(),
            });
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .user_agent(APP_TITLE)
            .build()
            .map_err(|err| ProviderError::Config {
                detail: format!("failed to build http client: {err}"),
            })?;
        Ok(Self {
            client,
            endpoint,
            api_key,
            model: model.into(),
            max_retries,
            backoff: Duration::from_millis(250),
        })
    }

    /// Builds a client from manifest configuration, reading the API key
    /// from the configured environment variable name (todo.md: API key
    /// configuration; `docs/decisions.md` section 5: keys are never
    /// stored in the manifest).
    pub fn from_config(jev: &JevConfig, provider: &ProviderConfig) -> Result<Self, ProviderError> {
        jev.validate().map_err(|err| ProviderError::Config {
            detail: err.to_string(),
        })?;
        provider.validate().map_err(|err| ProviderError::Config {
            detail: err.to_string(),
        })?;
        let api_key = std::env::var(&provider.api_key_env)
            .ok()
            .filter(|key| !key.trim().is_empty())
            .ok_or_else(|| ProviderError::MissingApiKey {
                env: provider.api_key_env.clone(),
            })?;
        Self::new(
            provider.endpoint.clone(),
            jev.model.clone(),
            api_key,
            Duration::from_millis(jev.timeout_ms),
            jev.max_retries,
        )
    }

    /// Overrides the retry backoff (tests use zero).
    pub fn with_backoff(mut self, backoff: Duration) -> Self {
        self.backoff = backoff;
        self
    }

    /// The configured model id (e.g. `typesafe/jev-1.13`).
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Builds the Decisions API request body for one choice question.
    pub fn request_body(request: &DecisionRequest, model: &str) -> serde_json::Value {
        let criteria: serde_json::Map<String, serde_json::Value> = request
            .choices
            .iter()
            .map(|choice| (choice.clone(), serde_json::Value::String(choice.clone())))
            .collect();
        serde_json::json!({
            "model": model,
            "state": request.context,
            "questions": {
                QUESTION_KEY: {
                    "type": "choice",
                    "instructions": request.question,
                    "criteria": criteria,
                }
            }
        })
    }

    /// Parses a Decisions API response body (todo.md Phase 8: provider
    /// errors surface as [`ProviderError::InvalidResponse`]).
    pub fn parse_response(body: &str) -> Result<ParsedAnswer, ProviderError> {
        #[derive(Deserialize)]
        struct Usage {
            input_tokens: u32,
            output_tokens: u32,
            #[serde(default)]
            cost: Option<f64>,
        }

        #[derive(Deserialize)]
        struct Answer {
            #[serde(rename = "type")]
            kind: String,
            #[serde(default)]
            choice: Option<String>,
            #[serde(default)]
            confidence: Option<f64>,
        }

        #[derive(Deserialize)]
        struct Wire {
            answers: std::collections::BTreeMap<String, Answer>,
            usage: Usage,
            #[serde(default)]
            id: Option<String>,
            #[serde(default)]
            model: Option<String>,
        }

        let wire: Wire =
            serde_json::from_str(body).map_err(|err| ProviderError::InvalidResponse {
                detail: format!("unparseable body: {err}"),
            })?;
        let answer =
            wire.answers
                .get(QUESTION_KEY)
                .ok_or_else(|| ProviderError::InvalidResponse {
                    detail: format!("missing answers.{QUESTION_KEY}"),
                })?;
        if answer.kind != "choice" {
            return Err(ProviderError::InvalidResponse {
                detail: format!("expected a choice answer, got `{}`", answer.kind),
            });
        }
        let choice = answer
            .choice
            .clone()
            .ok_or_else(|| ProviderError::InvalidResponse {
                detail: "choice answer missing `choice`".to_string(),
            })?;
        if choice.is_empty() {
            return Err(ProviderError::InvalidResponse {
                detail: "choice answer is empty".to_string(),
            });
        }
        let confidence = answer
            .confidence
            .map(|c| c as f32)
            .unwrap_or(DEFAULT_CONFIDENCE)
            .clamp(0.0, 1.0);
        Ok(ParsedAnswer {
            choice,
            confidence,
            input_tokens: wire.usage.input_tokens,
            output_tokens: wire.usage.output_tokens,
            cost_usd: wire.usage.cost,
            response_id: wire.id,
            served_model: wire.model,
        })
    }
}

impl DecisionProvider for OpenRouterProvider {
    fn name(&self) -> &str {
        "openrouter"
    }

    fn decide(&self, request: &DecisionRequest) -> Result<DecisionOutcome, ProviderError> {
        let body = Self::request_body(request, &self.model);
        let mut attempts = 0u32;

        loop {
            attempts += 1;
            let started = std::time::Instant::now();
            let send = self
                .client
                .post(&self.endpoint)
                .bearer_auth(&self.api_key)
                .header("Content-Type", "application/json")
                .header("X-Title", APP_TITLE)
                .json(&body)
                .send();

            match send {
                Err(err) => {
                    if attempts <= self.max_retries {
                        std::thread::sleep(self.backoff.saturating_mul(attempts - 1));
                        continue;
                    }
                    return if err.is_timeout() {
                        Err(ProviderError::Timeout { attempts })
                    } else {
                        Err(ProviderError::Transport {
                            detail: err.to_string(),
                            attempts,
                        })
                    };
                }
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        let text = response.text().map_err(|err| ProviderError::Transport {
                            detail: err.to_string(),
                            attempts,
                        })?;
                        let parsed = Self::parse_response(&text)?;
                        return Ok(DecisionOutcome {
                            decision: Decision {
                                choice: parsed.choice,
                                confidence: parsed.confidence,
                            },
                            latency: started.elapsed(),
                            input_tokens: parsed.input_tokens,
                            output_tokens: parsed.output_tokens,
                            cost_usd: parsed.cost_usd,
                        });
                    }

                    let code = status.as_u16();
                    let message = extract_error_message(response);
                    if is_retryable(code) && attempts <= self.max_retries {
                        std::thread::sleep(self.backoff.saturating_mul(attempts - 1));
                        continue;
                    }
                    return Err(ProviderError::Http {
                        status: code,
                        message,
                    });
                }
            }
        }
    }
}

/// Retryable statuses: rate limits, transient server/provider failures
/// (spec.md section 18: timeouts, retries, provider errors).
fn is_retryable(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 524 | 529)
}

/// Reads `error.message` from an OpenRouter error body, else the first
/// 200 characters of the body, else the status text.
fn extract_error_message(response: reqwest::blocking::Response) -> String {
    let status = response.status();
    let text = response.text().unwrap_or_default();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(message) = value
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(|message| message.as_str())
        {
            return message.to_string();
        }
    }
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return status.to_string();
    }
    trimmed.chars().take(200).collect()
}
