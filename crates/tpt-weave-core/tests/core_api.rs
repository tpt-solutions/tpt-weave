//! End-to-end context flow through the public core API.

use tpt_weave_core::{
    ContextCandidate, ContextLevel, ContextRequest, ContextResponse, ContextSource, Manifest,
    RepositoryId, Revision, SymbolId, SymbolKind, TokenAccounting, FileRecord,
};

#[test]
fn context_flow_from_request_to_response() {
    // Task: spec.md section 29 example — 87.2% reduction target.
    let request = ContextRequest::new("tpt-cv", "fix the image resampling bug");
    assert_eq!(request.budget_tokens, 12_000);
    assert_eq!(request.max_level, ContextLevel::Skeleton);

    let resample = SymbolId {
        repository: RepositoryId::new("tpt-cv"),
        package: "tpt-cv".into(),
        module: "image::resample".into(),
        name: "resample".into(),
        kind: SymbolKind::Function,
    };

    let file = ContextCandidate::new(
        ContextSource::File("src/image/resample.rs".into()),
        ContextLevel::Skeleton,
        6_400,
    );
    let symbol = ContextCandidate::new(
        ContextSource::Symbol(resample),
        ContextLevel::Signatures,
        1_442,
    )
    .with_relationship(file.id.clone());

    let tokens = TokenAccounting::new(61_420, 7_842, 0, 0, false);
    assert!(
        (tokens.gross_reduction() * 100.0 - 87.2).abs() < 0.05,
        "expected 87.2% reduction, got {:.1}%",
        tokens.gross_reduction() * 100.0
    );

    let make_response = || {
        ContextResponse::new(
            RepositoryId::new("tpt-cv"),
            vec![file.clone(), symbol.clone()],
            tokens.clone(),
            Some("typesafe/jev-1.13".to_string()),
        )
    };

    let first = make_response();
    assert_eq!(first.context_id, make_response().context_id);
    assert_eq!(first.total_tokens(), 7_842);
    assert_eq!(first.decision_provider.as_deref(), Some("typesafe/jev-1.13"));

    // The response must round-trip through JSON (index artefacts are JSON).
    let json = serde_json::to_string(&first).expect("serialise");
    let back: ContextResponse = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(back, first);
}

#[test]
fn level_and_revision_helpers() {
    assert_eq!(tpt_weave_core::ContextLevel::from_number(5).unwrap().name(), "full");
    let revision = Revision::new("abcdef0123456789abcdef0123456789abcdef01").with_branch("main");
    assert_eq!(revision.short(), "abcdef012345");

    let file = FileRecord::new("src\\lib.rs", 10, 2, "deadbeef");
    assert_eq!(file.path, "src/lib.rs");

    let manifest = Manifest::new("tpt-cv");
    manifest.validate().expect("default manifest is valid");
}
