//! Phase 8 OpenRouter client tests (todo.md Phase 8 "OpenRouter"):
//! request/response shapes, retries, provider errors — against a local
//! scripted HTTP server, never the live API.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tpt_weave_core::{JevConfig, PrivacyConfig, ProviderConfig};
use tpt_weave_decisions::{DecisionCategory, DecisionProvider, ProviderError};
use tpt_weave_openrouter::{OpenRouterProvider, RemoteAuditLog};

const SUCCESS_BODY: &str = r#"{
  "id": "dec_abc123",
  "model": "typesafe/jev-1.13",
  "provider": "Typesafe",
  "answers": {
    "answer": {
      "type": "choice",
      "choice": "relevant",
      "confidence": 0.91,
      "probabilities": { "relevant": 0.91, "irrelevant": 0.09 }
    }
  },
  "usage": { "input_tokens": 42, "output_tokens": 3, "cost": 0.0001 }
}"#;

fn respond(stream: &mut TcpStream, status: u16, body: &str) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

/// Drains a request body (headers + content-length) enough for the
/// server thread to finish writing the response.
fn read_request_head(stream: &mut TcpStream) -> String {
    let mut buf = [0u8; 8192];
    let mut head = String::new();
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 {
            break;
        }
        head.push_str(&String::from_utf8_lossy(&buf[..n]));
        if head.contains("\r\n\r\n") {
            break;
        }
    }
    head
}

/// Scripted HTTP server returning `responses` in order (final repeats),
/// counting request hits.
struct Scripted {
    endpoint: String,
    hits: Arc<AtomicUsize>,
}

impl Scripted {
    fn start(responses: Vec<(u16, &'static str)>) -> Self {
        assert!(!responses.is_empty());
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&hits);
        std::thread::spawn(move || {
            let queue = responses;
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let head = read_request_head(&mut stream);
                if head.is_empty() {
                    continue;
                }
                let n = counter.fetch_add(1, Ordering::SeqCst);
                let index = if queue.len() == 1 {
                    0
                } else {
                    n.min(queue.len() - 1)
                };
                let (status, body) = queue[index];
                let _ = respond(&mut stream, status, body);
            }
        });
        Self { endpoint, hits }
    }
}

fn provider(endpoint: &str) -> OpenRouterProvider {
    OpenRouterProvider::new(
        endpoint,
        "typesafe/jev-1.13",
        "test-key",
        Duration::from_secs(2),
        2,
    )
    .unwrap()
    .with_backoff(Duration::ZERO)
}

fn request() -> tpt_weave_decisions::DecisionRequest {
    DecisionCategory::Relevance.ask("src/lib.rs", "fix the bug")
}

#[test]
fn redacts_remote_request_content_before_transport() {
    let provider = provider("http://127.0.0.1:1");
    let request = tpt_weave_decisions::DecisionRequest {
        question: "Is this relevant?".to_string(),
        choices: vec!["relevant".to_string(), "irrelevant".to_string()],
        context: "OPENROUTER_API_KEY=super-secret-value".to_string(),
    };
    let (safe, count) = provider.redact_request(&request);
    assert!(count > 0);
    assert!(!safe.context.contains("super-secret-value"));
    assert!(safe.context.contains("[REDACTED]"));
}

#[test]
fn local_only_privacy_rejects_remote_provider() {
    let error = match OpenRouterProvider::new_with_privacy(
        "http://127.0.0.1:1",
        "typesafe/jev-1.13",
        "test-key",
        Duration::from_secs(1),
        0,
        PrivacyConfig::default(),
        None,
    ) {
        Ok(_) => panic!("local-only policy must reject remote transport"),
        Err(error) => error,
    };
    assert!(matches!(error, ProviderError::Privacy { .. }));
}

#[test]
fn remote_audit_log_is_content_free() {
    let server = Scripted::start(vec![(200, SUCCESS_BODY)]);
    let path = std::env::temp_dir().join(format!(
        "tpt-weave-remote-audit-{}.jsonl",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let audit = RemoteAuditLog::new(&path, "demo", "openrouter");
    let provider = OpenRouterProvider::new_with_privacy(
        &server.endpoint,
        "typesafe/jev-1.13",
        "test-key",
        Duration::from_secs(2),
        0,
        PrivacyConfig {
            remote_decisions: true,
            ..PrivacyConfig::default()
        },
        Some(audit),
    )
    .unwrap();
    let request = tpt_weave_decisions::DecisionRequest {
        question: "Is this relevant?".to_string(),
        choices: vec!["relevant".to_string(), "irrelevant".to_string()],
        context: "password=do-not-log-this".to_string(),
    };
    provider.decide(&request).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("do-not-log-this"));
    let entry: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(entry["repository"], "demo");
    assert_eq!(entry["redactions"], 1);
    let _ = std::fs::remove_file(path);
}

#[test]
fn request_body_matches_the_decisions_api_shape() {
    let body = OpenRouterProvider::request_body(&request(), "typesafe/jev-1.13");
    assert_eq!(body["model"], "typesafe/jev-1.13");
    assert_eq!(body["state"], "fix the bug");
    let question = &body["questions"]["answer"];
    assert_eq!(question["type"], "choice");
    assert!(
        question["instructions"]
            .as_str()
            .unwrap()
            .contains("src/lib.rs")
    );
    assert_eq!(question["criteria"]["relevant"], "relevant");
    assert_eq!(question["criteria"]["irrelevant"], "irrelevant");
}

#[test]
fn parse_response_extracts_choice_confidence_and_usage() {
    let parsed = OpenRouterProvider::parse_response(SUCCESS_BODY).unwrap();
    assert_eq!(parsed.choice, "relevant");
    assert_eq!(parsed.confidence, 0.91);
    assert_eq!(parsed.input_tokens, 42);
    assert_eq!(parsed.output_tokens, 3);
    assert_eq!(parsed.cost_usd, Some(0.0001));
    assert_eq!(parsed.response_id.as_deref(), Some("dec_abc123"));
    assert_eq!(parsed.served_model.as_deref(), Some("typesafe/jev-1.13"));
}

#[test]
fn parse_response_defaults_missing_confidence_to_midrange() {
    let body = r#"{
      "answers": { "answer": { "type": "choice", "choice": "keep" } },
      "usage": { "input_tokens": 1, "output_tokens": 1 }
    }"#;
    let parsed = OpenRouterProvider::parse_response(body).unwrap();
    assert_eq!(parsed.confidence, 0.5);
    assert_eq!(parsed.cost_usd, None);
}

#[test]
fn parse_response_rejects_non_choice_and_missing_answers() {
    let non_choice = r#"{
      "answers": { "answer": { "type": "noul", "value": "x" } },
      "usage": { "input_tokens": 1, "output_tokens": 1 }
    }"#;
    assert!(matches!(
        OpenRouterProvider::parse_response(non_choice),
        Err(ProviderError::InvalidResponse { .. })
    ));
    let missing = r#"{ "answers": {}, "usage": { "input_tokens": 1, "output_tokens": 1 } }"#;
    assert!(matches!(
        OpenRouterProvider::parse_response(missing),
        Err(ProviderError::InvalidResponse { .. })
    ));
    assert!(matches!(
        OpenRouterProvider::parse_response("not json"),
        Err(ProviderError::InvalidResponse { .. })
    ));
}

#[test]
fn decide_records_latency_tokens_and_confidence() {
    let server = Scripted::start(vec![(200, SUCCESS_BODY)]);
    let provider = provider(&server.endpoint);
    let outcome = provider.decide(&request()).unwrap();
    assert_eq!(outcome.decision.choice, "relevant");
    assert_eq!(outcome.decision.confidence, 0.91);
    assert_eq!(outcome.input_tokens, 42);
    assert_eq!(outcome.output_tokens, 3);
    assert_eq!(outcome.cost_usd, Some(0.0001));
    // Latency is recorded (may be ~0 on loopback but must not panic).
    let _ = outcome.latency.as_millis();
    assert_eq!(server.hits.load(Ordering::SeqCst), 1);
}

#[test]
fn retries_server_errors_then_succeeds() {
    let server = Scripted::start(vec![
        (500, r#"{"error":{"code":500,"message":"internal"}}"#),
        (200, SUCCESS_BODY),
    ]);
    let provider = provider(&server.endpoint);
    let outcome = provider.decide(&request()).unwrap();
    assert_eq!(outcome.decision.choice, "relevant");
    assert_eq!(server.hits.load(Ordering::SeqCst), 2);
}

#[test]
fn respects_max_retries_on_repeated_server_errors() {
    let server = Scripted::start(vec![(500, r#"{"error":{"code":500,"message":"boom"}}"#)]);
    let provider = provider(&server.endpoint); // max_retries = 2 → 3 attempts
    let err = provider.decide(&request()).unwrap_err();
    match err {
        ProviderError::Http { status, message } => {
            assert_eq!(status, 500);
            assert!(message.contains("boom"), "{message}");
        }
        other => panic!("expected http error, got {other:?}"),
    }
    assert_eq!(server.hits.load(Ordering::SeqCst), 3);
}

#[test]
fn does_not_retry_client_errors() {
    let server = Scripted::start(vec![(
        400,
        r#"{"error":{"code":400,"message":"bad request"}}"#,
    )]);
    let provider = provider(&server.endpoint);
    let err = provider.decide(&request()).unwrap_err();
    assert!(matches!(err, ProviderError::Http { status: 400, .. }));
    assert_eq!(server.hits.load(Ordering::SeqCst), 1);
}

#[test]
fn rejects_invalid_endpoints_and_empty_keys() {
    assert!(matches!(
        OpenRouterProvider::new("ftp://x", "m", "k", Duration::from_secs(1), 0),
        Err(ProviderError::Config { .. })
    ));
    assert!(matches!(
        OpenRouterProvider::new("http://127.0.0.1:1", "m", "  ", Duration::from_secs(1), 0),
        Err(ProviderError::Config { .. })
    ));
}

#[test]
fn from_config_reads_endpoint_model_and_key_env() {
    // Definitely-unset env name; edition 2024 forbids safe set_var.
    let jev = JevConfig::default();
    let provider_cfg = ProviderConfig {
        endpoint: "http://127.0.0.1:1".to_string(),
        api_key_env: "TPT_WEAVE_TEST_NO_SUCH_KEY_VAR".to_string(),
        ..Default::default()
    };
    let privacy = PrivacyConfig {
        remote_decisions: true,
        ..PrivacyConfig::default()
    };
    let err =
        match OpenRouterProvider::from_config_with_privacy(&jev, &provider_cfg, &privacy, None) {
            Ok(_) => panic!("expected missing API key error"),
            Err(err) => err,
        };
    assert!(
        matches!(err, ProviderError::MissingApiKey { .. }),
        "{err:?}"
    );
}

#[test]
fn from_config_fails_closed_for_local_only_privacy() {
    let jev = JevConfig::default();
    let provider_cfg = ProviderConfig {
        api_key_env: "TPT_WEAVE_TEST_NO_SUCH_KEY_VAR".to_string(),
        ..Default::default()
    };
    let err = match OpenRouterProvider::from_config(&jev, &provider_cfg) {
        Ok(_) => panic!("expected local-only policy error"),
        Err(err) => err,
    };
    assert!(matches!(err, ProviderError::Privacy { .. }), "{err:?}");
}
