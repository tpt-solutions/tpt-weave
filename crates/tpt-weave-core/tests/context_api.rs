//! Context types exercised through the public API (levels, ids, candidates).

use tpt_weave_core::{
    ContextCandidate, ContextId, ContextLevel, ContextRequest, ContextSource, SymbolId,
    SymbolKind, TokenAccounting, RepositoryId,
};

#[test]
fn levels_order_and_numbering() {
    assert!(ContextLevel::Metadata < ContextLevel::Full);
    assert_eq!(ContextLevel::Skeleton.as_number(), 3);
    assert_eq!(
        ContextLevel::from_number(4),
        Some(ContextLevel::Implementation)
    );
    assert_eq!(ContextLevel::from_number(6), None);
    assert_eq!(ContextLevel::Signatures.to_string(), "signatures");
    assert_eq!(ContextLevel::ALL.len(), 6);
}

#[test]
fn candidate_ids_depend_on_source_and_level_not_estimate() {
    let file = |level, tokens| {
        ContextCandidate::new(ContextSource::File("src/lib.rs".into()), level, tokens)
    };
    let skeleton_a = file(ContextLevel::Skeleton, 100);
    let skeleton_b = file(ContextLevel::Skeleton, 999);
    let full = file(ContextLevel::Full, 100);

    assert_eq!(skeleton_a.id, skeleton_b.id);
    assert_ne!(skeleton_a.id, full.id);
    assert!(skeleton_a.id.as_str().starts_with("ctx:"));
}

#[test]
fn deterministic_ids_are_repeatable() {
    let a = ContextId::deterministic(&["tpt-cv", "file:src/lib.rs", "3"]);
    let b = ContextId::deterministic(&["tpt-cv", "file:src/lib.rs", "3"]);
    assert_eq!(a, b);
}

#[test]
fn request_defaults_and_builders() {
    let request = ContextRequest::new("tpt-cv", "fix the image resampling bug");
    assert_eq!(request.budget_tokens, 12_000);
    assert_eq!(request.max_level, ContextLevel::Skeleton);
    assert!(request.include_dependencies);

    let request = request
        .with_budget_tokens(4_000)
        .with_max_level(ContextLevel::Signatures)
        .without_dependencies();
    assert_eq!(request.budget_tokens, 4_000);
    assert_eq!(request.max_level, ContextLevel::Signatures);
    assert!(!request.include_dependencies);
}

#[test]
fn symbol_sources_get_distinct_keys() {
    let symbol = SymbolId {
        repository: RepositoryId::new("tpt-cv"),
        package: "tpt-cv".into(),
        module: "image".into(),
        name: "PixelBuffer".into(),
        kind: SymbolKind::Struct,
    };
    let from_symbol =
        ContextCandidate::new(ContextSource::Symbol(symbol), ContextLevel::Signatures, 50);
    let from_file =
        ContextCandidate::new(ContextSource::File("image/pixel.rs".into()), ContextLevel::Signatures, 50);
    assert_ne!(from_symbol.id, from_file.id);
    assert_eq!(
        from_symbol.source.key(),
        "symbol:tpt-cv/tpt-cv::image::PixelBuffer#struct"
    );
}

#[test]
fn accounting_roundtrips_through_json() {
    let accounting = TokenAccounting::new(48_231, 7_312, 612, 128, true);
    let json = serde_json::to_string(&accounting).unwrap();
    let back: TokenAccounting = serde_json::from_str(&json).unwrap();
    assert_eq!(back.saved_tokens, 40_919);
    assert!(back.cache_hit);
}
