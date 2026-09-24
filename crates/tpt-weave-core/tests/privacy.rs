//! Phase 16 secret redaction and private-path policy tests.

use tpt_weave_core::{PrivacyConfig, REDACTION_MARKER, SecretKind, redact_secrets};

#[test]
fn redacts_assignments_tokens_and_private_key_blocks() {
    let input = concat!(
        "OPENROUTER_API_KEY=super-secret-value\n",
        "password: \"correct horse battery staple\"\n",
        "Authorization: Bearer abcdef0123456789\n",
        "-----BEGIN RSA PRIVATE KEY-----\n",
        "private-material\n",
        "-----END RSA PRIVATE KEY-----\n",
    );
    let redacted = redact_secrets(input);
    assert!(redacted.was_redacted());
    assert!(redacted.count() >= 4);
    assert!(!redacted.text.contains("super-secret-value"));
    assert!(!redacted.text.contains("correct horse"));
    assert!(!redacted.text.contains("abcdef0123456789"));
    assert!(!redacted.text.contains("private-material"));
    assert!(redacted.text.contains(REDACTION_MARKER));
    assert!(
        redacted
            .findings
            .iter()
            .any(|f| f.kind == SecretKind::Certificate)
    );
}

#[test]
fn private_paths_extend_exclusions_without_changing_default_local_only_mode() {
    let mut privacy = PrivacyConfig::default();
    assert!(!privacy.remote_decisions);
    assert!(privacy.is_excluded(".env"));
    assert!(privacy.is_excluded("secrets/token.rs"));
    assert!(!privacy.is_excluded("src/main.rs"));

    privacy.private_paths.push("C:/private/client".to_string());
    assert!(privacy.is_private_path("C:/private/client/src/lib.rs"));
    assert!(privacy.is_private_path("C:\\private\\client\\src\\lib.rs"));
    assert!(!privacy.is_private_path("C:/public/client/src/lib.rs"));
}

#[test]
fn manifest_serialisation_keeps_private_paths_empty_by_default() {
    let privacy = PrivacyConfig::default();
    assert!(privacy.private_paths.is_empty());
    privacy.validate().expect("default privacy policy is valid");
}
