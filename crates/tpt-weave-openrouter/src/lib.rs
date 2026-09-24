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
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tpt_weave_core::{JevConfig, MANIFEST_DIR, PrivacyConfig, ProviderConfig};
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

/// Default audit file name inside `.tpt-weave/`.
pub const AUDIT_FILE: &str = "privacy-audit.jsonl";

/// One content-free audit record for a remote decision request.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RemoteAuditEntry {
    pub timestamp_ms: u64,
    pub repository: String,
    pub provider: String,
    pub item_count: u64,
    pub token_count: u64,
    pub redactions: u64,
}

/// Append-only JSONL audit log for remote decision calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteAuditLog {
    path: PathBuf,
    repository: String,
    provider: String,
}

impl RemoteAuditLog {
    /// Creates a log at `path` for one repository/provider pair.
    pub fn new(
        path: impl Into<PathBuf>,
        repository: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            repository: repository.into(),
            provider: provider.into(),
        }
    }

    /// Creates the standard audit log under a repository's `.tpt-weave/`
    /// directory.
    pub fn for_repository(
        repository_root: &Path,
        repository: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        Self::new(
            repository_root.join(MANIFEST_DIR).join(AUDIT_FILE),
            repository,
            provider,
        )
    }

    /// The audit file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends one request summary without storing request content.
    pub fn record(
        &self,
        item_count: u64,
        token_count: u64,
        redactions: usize,
    ) -> std::io::Result<RemoteAuditEntry> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or_default();
        let entry = RemoteAuditEntry {
            timestamp_ms,
            repository: self.repository.clone(),
            provider: self.provider.clone(),
            item_count,
            token_count,
            redactions: redactions as u64,
        };
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        serde_json::to_writer(&mut file, &entry)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        file.write_all(b"\n")?;
        Ok(entry)
    }
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
    privacy: PrivacyConfig,
    audit: Option<RemoteAuditLog>,
}

impl OpenRouterProvider {
    /// Builds a client with an explicit API key (tests, custom loaders).
    /// This explicit constructor enables remote decisions; manifest-driven
    /// callers should use [`Self::from_config_with_privacy`].
    pub fn new(
        endpoint: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
        timeout: Duration,
        max_retries: u32,
    ) -> Result<Self, ProviderError> {
        Self::new_with_privacy(
            endpoint,
            model,
            api_key,
            timeout,
            max_retries,
            PrivacyConfig {
                remote_decisions: true,
                ..PrivacyConfig::default()
            },
            None,
        )
    }

    /// Builds a client with an explicit privacy policy and optional audit log.
    pub fn new_with_privacy(
        endpoint: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
        timeout: Duration,
        max_retries: u32,
        privacy: PrivacyConfig,
        audit: Option<RemoteAuditLog>,
    ) -> Result<Self, ProviderError> {
        if !privacy.remote_decisions {
            return Err(ProviderError::Privacy {
                detail: "remote decisions are disabled by privacy policy".to_string(),
            });
        }
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
            privacy,
            audit,
        })
    }

    /// Builds a client from manifest configuration, reading the API key
    /// from the configured environment variable name. This compatibility
    /// constructor explicitly enables remote decisions; new integrations
    /// should pass the manifest's privacy policy to
    /// [`Self::from_config_with_privacy`].
    pub fn from_config(jev: &JevConfig, provider: &ProviderConfig) -> Result<Self, ProviderError> {
        Self::from_config_with_privacy(
            jev,
            provider,
            &PrivacyConfig {
                remote_decisions: true,
                ..PrivacyConfig::default()
            },
            None,
        )
    }

    /// Builds a client from manifest configuration and privacy policy.
    pub fn from_config_with_privacy(
        jev: &JevConfig,
        provider: &ProviderConfig,
        privacy: &PrivacyConfig,
        audit: Option<RemoteAuditLog>,
    ) -> Result<Self, ProviderError> {
        jev.validate().map_err(|err| ProviderError::Config {
            detail: err.to_string(),
        })?;
        provider.validate().map_err(|err| ProviderError::Config {
            detail: err.to_string(),
        })?;
        if !privacy.remote_decisions {
            return Err(ProviderError::Privacy {
                detail: "remote decisions are disabled by privacy policy".to_string(),
            });
        }
        let api_key = std::env::var(&provider.api_key_env)
            .ok()
            .filter(|key| !key.trim().is_empty())
            .ok_or_else(|| ProviderError::MissingApiKey {
                env: provider.api_key_env.clone(),
            })?;
        Self::new_with_privacy(
            provider.endpoint.clone(),
            jev.model.clone(),
            api_key,
            Duration::from_millis(jev.timeout_ms),
            jev.max_retries,
            privacy.clone(),
            audit,
        )
    }

    /// Builds a client from manifest configuration and installs the standard
    /// content-free audit log for `repository_root`.
    pub fn from_manifest(
        jev: &JevConfig,
        provider: &ProviderConfig,
        privacy: &PrivacyConfig,
        repository_root: &Path,
        repository: &str,
    ) -> Result<Self, ProviderError> {
        let audit = RemoteAuditLog::for_repository(repository_root, repository, &provider.name);
        Self::from_config_with_privacy(jev, provider, privacy, Some(audit))
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

    /// Returns a copy of `request` safe for a remote provider.
    pub fn redact_request(&self, request: &DecisionRequest) -> (DecisionRequest, usize) {
        let question = self.privacy.redact(&request.question);
        let context = self.privacy.redact(&request.context);
        let mut choices = Vec::with_capacity(request.choices.len());
        let mut redactions = question.count() + context.count();
        for choice in &request.choices {
            let redacted = self.privacy.redact(choice);
            redactions += redacted.count();
            choices.push(redacted.text);
        }
        (
            DecisionRequest {
                question: question.text,
                choices,
                context: context.text,
            },
            redactions,
        )
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
        if !self.privacy.remote_decisions {
            return Err(ProviderError::Privacy {
                detail: "remote decisions are disabled by privacy policy".to_string(),
            });
        }
        let (safe_request, redactions) = self.redact_request(request);
        if let Some(audit) = &self.audit {
            let material = format!(
                "{}\n{}\n{}",
                safe_request.question,
                safe_request.context,
                safe_request.choices.join("\n")
            );
            let token_count = material.chars().count().div_ceil(4) as u64;
            audit
                .record(1, token_count, redactions)
                .map_err(|error| ProviderError::Privacy {
                    detail: format!("remote audit failed: {error}"),
                })?;
        }
        let body = Self::request_body(&safe_request, &self.model);
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
