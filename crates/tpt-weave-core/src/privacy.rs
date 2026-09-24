//! Secret detection/redaction and remote-transfer audit primitives.

use crate::config::PrivacyConfig;
use serde::{Deserialize, Serialize};

/// Replacement inserted for a detected secret value.
pub const REDACTION_MARKER: &str = "[REDACTED]";

/// Broad classes of sensitive material recognised by the conservative scanner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretKind {
    /// Environment or configuration assignment.
    Environment,
    /// Password, token, credential, or private-key material.
    Credential,
    /// API key or bearer token.
    ApiKey,
    /// Certificate or PEM-encoded private key.
    Certificate,
}

/// A content-free location of a detected secret.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretFinding {
    pub kind: SecretKind,
    /// One-based source line.
    pub line: u32,
    /// One-based source column.
    pub column: u32,
}

/// Redacted text plus metadata that never contains the original value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Redaction {
    pub text: String,
    pub findings: Vec<SecretFinding>,
}

impl Redaction {
    /// Whether at least one value was replaced.
    pub fn was_redacted(&self) -> bool {
        !self.findings.is_empty()
    }

    /// Number of detected values.
    pub fn count(&self) -> usize {
        self.findings.len()
    }
}

impl PrivacyConfig {
    /// Redacts environment assignments, credentials, API-key-like literals,
    /// bearer tokens, JWTs, certificates, and private-key PEM blocks.
    pub fn redact(&self, text: &str) -> Redaction {
        redact_secrets(text)
    }
}

/// Scan and redact `text` without retaining secret values in the result.
pub fn redact_secrets(text: &str) -> Redaction {
    let mut output = String::with_capacity(text.len());
    let mut findings = Vec::new();
    let mut in_pem = false;

    for (index, raw_line) in text.split_inclusive('\n').enumerate() {
        let line_number = u32::try_from(index + 1).unwrap_or(u32::MAX);
        let newline = if raw_line.ends_with("\r\n") {
            "\r\n"
        } else if raw_line.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let line = raw_line.strip_suffix(newline).unwrap_or(raw_line);
        let detected_kind = pem_kind_for(line);

        if in_pem || detected_kind.is_some() {
            if !in_pem {
                findings.push(SecretFinding {
                    kind: detected_kind.unwrap_or(SecretKind::Certificate),
                    line: line_number,
                    column: 1,
                });
                in_pem = true;
            }
            output.push_str(REDACTION_MARKER);
            output.push_str(newline);
            if line.to_ascii_uppercase().contains("-----END ") {
                in_pem = false;
            }
            continue;
        }

        let (redacted, mut line_findings) = redact_line(line, line_number);
        findings.append(&mut line_findings);
        output.push_str(&redacted);
        output.push_str(newline);
    }

    Redaction {
        text: output,
        findings,
    }
}

fn redact_line(line: &str, line_number: u32) -> (String, Vec<SecretFinding>) {
    let mut findings = Vec::new();
    let mut redacted = line.to_string();

    if let Some((delimiter, key)) = sensitive_assignment(&redacted) {
        let kind = assignment_kind(&key);
        findings.push(SecretFinding {
            kind,
            line: line_number,
            column: u32::try_from(delimiter + 2).unwrap_or(u32::MAX),
        });
        redacted.truncate(delimiter + 1);
        redacted.push_str(REDACTION_MARKER);
    }

    for (prefix, kind, minimum) in [
        ("bearer ", SecretKind::ApiKey, 4),
        ("sk_live_", SecretKind::ApiKey, 8),
        ("sk-", SecretKind::ApiKey, 8),
        ("ghp_", SecretKind::ApiKey, 8),
        ("github_pat_", SecretKind::ApiKey, 8),
        ("xoxb-", SecretKind::ApiKey, 8),
        ("xoxp-", SecretKind::ApiKey, 8),
        ("akia", SecretKind::ApiKey, 12),
        ("aiza", SecretKind::ApiKey, 20),
        ("eyj", SecretKind::ApiKey, 20),
    ] {
        redacted =
            replace_literal_prefix(&redacted, prefix, kind, minimum, line_number, &mut findings);
    }

    (redacted, findings)
}

fn sensitive_assignment(line: &str) -> Option<(usize, String)> {
    let equals = line.find('=');
    let colon = line.find(':');
    let delimiter = match (equals, colon) {
        (Some(left), Some(right)) if left <= right => left,
        (Some(left), _) => left,
        (None, Some(right)) => right,
        (None, None) => return None,
    };
    let key = line[..delimiter].to_ascii_lowercase();
    let sensitive = [
        "password",
        "passwd",
        "secret",
        "api_key",
        "apikey",
        "access_key",
        "private_key",
        "credential",
        "auth",
        "bearer",
        "token",
    ]
    .iter()
    .any(|marker| key.contains(marker));
    sensitive.then_some((delimiter, key))
}

fn assignment_kind(key: &str) -> SecretKind {
    if ["api", "key", "token", "bearer", "auth"]
        .iter()
        .any(|part| key.contains(part))
    {
        SecretKind::ApiKey
    } else if key.contains("env") {
        SecretKind::Environment
    } else {
        SecretKind::Credential
    }
}

fn replace_literal_prefix(
    line: &str,
    prefix: &str,
    kind: SecretKind,
    minimum: usize,
    line_number: u32,
    findings: &mut Vec<SecretFinding>,
) -> String {
    let lower = line.to_ascii_lowercase();
    let mut output = String::with_capacity(line.len());
    let mut cursor = 0;
    while cursor < line.len() {
        let Some(relative) = lower[cursor..].find(prefix) else {
            break;
        };
        let start = cursor + relative;
        let value_start = start + prefix.len();
        let end = token_end(line, value_start);
        if end - value_start < minimum {
            output.push_str(&line[cursor..value_start]);
            cursor = value_start;
            continue;
        }
        output.push_str(&line[cursor..value_start]);
        output.push_str(REDACTION_MARKER);
        findings.push(SecretFinding {
            kind,
            line: line_number,
            column: u32::try_from(value_start + 1).unwrap_or(u32::MAX),
        });
        cursor = end;
    }
    output.push_str(&line[cursor..]);
    output
}

fn token_end(text: &str, start: usize) -> usize {
    text.as_bytes()[start..]
        .iter()
        .position(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')))
        .map(|offset| start + offset)
        .unwrap_or(text.len())
}

fn pem_kind_for(line: &str) -> Option<SecretKind> {
    let upper = line.to_ascii_uppercase();
    if !upper.contains("-----BEGIN ") {
        return None;
    }
    [
        "PRIVATE KEY",
        "CERTIFICATE",
        "EC PARAMETERS",
        "OPENSSH PRIVATE KEY",
    ]
    .iter()
    .any(|marker| upper.contains(marker))
    .then_some(SecretKind::Certificate)
}
