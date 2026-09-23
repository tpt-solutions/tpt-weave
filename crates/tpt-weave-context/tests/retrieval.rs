//! Deterministic context retrieval (todo.md Phase 5).

mod common;

use common::{build, key_of, sources};
use tpt_weave_context::{RepositoryOverview, Retriever};
use tpt_weave_core::{ContextLevel, ContextRequest, ContextSource};

#[test]
fn summarises_the_repository_overview() {
    let graph = build();
    let overview = RepositoryOverview::of(&graph);

    assert_eq!(overview.repository.as_str(), "demo-repo");
    assert_eq!(overview.revision.short(), "111111111111");
    assert_eq!(overview.crates, vec!["demo", "tpt-math"]);
    assert_eq!(overview.files, 3);
    assert!(overview.symbols > 0);
    assert!(overview.public_symbols > 0);
    assert!(overview.modules >= 2, "inner + tests modules");
    assert_eq!(overview.dependencies, 3, "tpt-math, serde, tpt-cv");
    assert_eq!(
        overview.external_repositories.iter().map(|r| r.as_str()).collect::<Vec<_>>(),
        vec!["tpt-cv"]
    );

    let text = overview.text();
    assert!(text.starts_with("demo-repo@111111111111"));
    assert!(text.contains("symbols="));
    assert!(overview.token_estimate() > 0);

    // Deterministic.
    assert_eq!(overview, RepositoryOverview::of(&graph));
}

#[test]
fn supports_each_lookup() {
    let graph = build();
    let sources = sources();
    let retriever = Retriever::new(&graph, &sources);

    // File lookup.
    let all = retriever.find_files("");
    assert_eq!(all.len(), 3);
    let lib = retriever.find_files("LIB.RS");
    assert!(
        lib.contains(&"src/lib.rs".to_string()),
        "matches crate-local lib, got {lib:?}"
    );
    let math_lib = retriever.find_files("TPT-MATH/SRC");
    assert_eq!(math_lib, vec!["crates/tpt-math/src/lib.rs".to_string()]);
    assert!(retriever.find_files("nope").is_empty());

    // Symbol lookup (methods and module paths).
    let make = retriever.find_symbols("make");
    assert!(make.iter().any(|r| r.id.name == "make"));
    let methods = retriever.find_symbols("draw");
    assert!(methods.iter().any(|r| r.id.name == "Pixel::draw"));
    assert!(retriever.find_symbols("").is_empty());

    // Reference lookup.
    let consume = key_of(&graph, "consume");
    let refs = retriever.find_references(&consume);
    assert!(refs.iter().any(|(s, kind)| {
        s.id.name == "helper" && *kind == tpt_weave_graph::ReferenceKind::Call
    }));

    // Dependency lookup.
    let deps = retriever.find_dependencies("demo");
    assert_eq!(deps.len(), 3);
    assert!(deps.iter().any(|e| e.to_package == "tpt-math"));

    // Related-symbol lookup.
    let pixel = key_of(&graph, "Pixel");
    let related = retriever.find_related(&pixel).expect("related");
    let names: Vec<&str> = related.iter().map(|r| r.id.name.as_str()).collect();
    assert!(names.contains(&"Pixel"));
    assert!(names.contains(&"<Pixel as Draw>"));

    // Changed-file lookup.
    let changed = vec!["src/lib.rs".to_string()];
    let touched = retriever.find_changed(&changed);
    assert!(touched.iter().any(|r| r.id.name == "make"));
    assert!(touched.iter().all(|r| r.file == "src/lib.rs"));
    // Windows separators normalise.
    assert_eq!(
        retriever.find_changed(&["src\\lib.rs".to_string()]).len(),
        touched.len()
    );

    // Test lookup.
    let make_key = key_of(&graph, "make");
    let tests = retriever.find_tests(&make_key).expect("tests");
    assert!(tests.iter().any(|r| r.id.name == "test_make"));

    let err = retriever.find_related("missing").expect_err("unknown");
    assert!(matches!(err, tpt_weave_context::ContextError::UnknownSymbol(_)));
}

#[test]
fn retrieves_budgeted_deterministic_context() {
    let graph = build();
    let sources = sources();
    let retriever = Retriever::new(&graph, &sources);

    let request = ContextRequest::new("demo-repo", "fix the make pixel bug");
    let response = retriever
        .retrieve(&request, &[])
        .expect("retrieval");

    // Deterministic: identical request → identical response.
    let again = retriever.retrieve(&request, &[]).expect("retrieval");
    assert_eq!(response, again);
    assert_eq!(response.context_id, again.context_id);
    assert!(response.decision_provider.is_none(), "deterministic only");

    // Overview always present.
    assert!(
        response
            .candidates
            .iter()
            .any(|c| c.source == ContextSource::Repository)
    );

    // Budget respected.
    assert!(response.total_tokens() <= u64::from(request.budget_tokens));
    assert!(response.tokens.selected_tokens <= response.tokens.raw_tokens);

    // Ordering: scores are non-increasing.
    for pair in response.candidates.windows(2) {
        let a = pair[0].score.unwrap_or(0.0);
        let b = pair[1].score.unwrap_or(0.0);
        assert!(a >= b, "scores ordered: {a} < {b}");
    }

    // Relevance: the matched file is present, and symbol anchors are
    // deduplicated against their file at signature depth or deeper.
    let file = response
        .candidates
        .iter()
        .find(|c| matches!(&c.source, ContextSource::File(path) if path == "src/lib.rs"))
        .expect("src/lib.rs selected");
    assert!(file.level >= ContextLevel::Signatures);
    assert!(file.score.unwrap_or(0.0) > 0.0);
    let lib_has_symbol_anchor = response.candidates.iter().any(|c| {
        matches!(&c.source, ContextSource::Symbol(id) if id.name == "make")
    });
    assert!(
        !lib_has_symbol_anchor || file.level < ContextLevel::Signatures,
        "symbol anchor should be subsumed by its file"
    );

    // No duplicate (source key, level) pairs survive.
    let mut seen = std::collections::BTreeSet::new();
    for candidate in &response.candidates {
        let key = (candidate.source.key(), candidate.level);
        assert!(seen.insert(key.clone()), "duplicate candidate {key:?}");
    }

    // Scores are within the documented range.
    for candidate in &response.candidates {
        let score = candidate.score.expect("scored");
        assert!((0.0..=1.0).contains(&score));
    }
}

#[test]
fn boosts_changed_files_and_honours_options() {
    let graph = build();
    let sources = sources();
    let retriever = Retriever::new(&graph, &sources);

    // Empty task: only overview + changed files.
    let request = ContextRequest::new("demo-repo", "");
    let changed = vec!["tests/integration.rs".to_string()];
    let response = retriever.retrieve(&request, &changed).expect("retrieval");
    let paths: Vec<&str> = response
        .candidates
        .iter()
        .filter_map(|c| match &c.source {
            ContextSource::File(path) => Some(path.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(paths, vec!["tests/integration.rs"]);
    assert!(
        response
            .candidates
            .iter()
            .any(|c| c.source == ContextSource::Repository)
    );

    // Changed file outranks an unchanged file with the same word hits.
    let request = ContextRequest::new("demo-repo", "consume pixel");
    let with_boost = retriever.retrieve(&request, &changed).expect("boost");
    let without = retriever.retrieve(&request, &[]).expect("no boost");
    let boosted_score = with_boost
        .candidates
        .iter()
        .find_map(|c| match &c.source {
            ContextSource::File(path) if path == "tests/integration.rs" => c.score,
            _ => None,
        })
        .expect("changed file present");
    let plain_score = without
        .candidates
        .iter()
        .find_map(|c| match &c.source {
            ContextSource::File(path) if path == "tests/integration.rs" => c.score,
            _ => None,
        })
        .expect("file present without boost");
    assert!(boosted_score > plain_score, "{boosted_score} <= {plain_score}");

    // Dependencies are opt-in.
    let with_deps = ContextRequest::new("demo-repo", "tpt-math square");
    let response = retriever.retrieve(&with_deps, &[]).expect("deps on");
    assert!(
        response
            .candidates
            .iter()
            .any(|c| matches!(&c.source, ContextSource::Dependency(p) if p == "tpt-math"))
    );
    let without_deps = ContextRequest::new("demo-repo", "tpt-math square").without_dependencies();
    let response = retriever.retrieve(&without_deps, &[]).expect("deps off");
    assert!(
        !response
            .candidates
            .iter()
            .any(|c| matches!(c.source, ContextSource::Dependency(_)))
    );
}

#[test]
fn tiny_budget_still_delivers_the_overview() {
    let graph = build();
    let sources = sources();
    let retriever = Retriever::new(&graph, &sources);
    let overview_tokens = RepositoryOverview::of(&graph).token_estimate();

    // Exactly enough budget for the overview: it is admitted first and
    // larger candidates cannot crowd it out.
    let request = ContextRequest::new("demo-repo", "make")
        .with_budget_tokens(overview_tokens);
    let response = retriever.retrieve(&request, &[]).expect("retrieval");
    assert!(response.total_tokens() <= u64::from(overview_tokens));
    assert!(
        response
            .candidates
            .iter()
            .any(|c| c.source == ContextSource::Repository),
        "overview fits a budget sized for it"
    );

    // Zero budget delivers nothing and never exceeds the ceiling.
    let request = ContextRequest::new("demo-repo", "make").with_budget_tokens(0);
    let response = retriever.retrieve(&request, &[]).expect("retrieval");
    assert!(response.candidates.is_empty());
    assert_eq!(response.tokens.selected_tokens, 0);
}
