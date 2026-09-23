//! Expansion of symbols, modules, dependencies, tests and related
//! implementations (todo.md Phase 4 "Expansion").

mod common;

use common::{build, key_of, sources};
use tpt_weave_context::{
    ContextError, expand_dependency, expand_module, expand_related, expand_symbol, expand_test,
};
use tpt_weave_core::ContextLevel;

#[test]
fn expands_a_symbol_to_targeted_implementation() {
    let graph = build();
    let sources = sources();
    let make = key_of(&graph, "make");

    let expansion =
        expand_symbol(&graph, &sources, &make, ContextLevel::Implementation).expect("expansion");
    assert_eq!(expansion.subject, make);
    assert_eq!(expansion.level, ContextLevel::Implementation);
    assert_eq!(expansion.symbols.len(), 1);
    assert_eq!(expansion.files.len(), 1);
    assert_eq!(expansion.files[0].path, "src/lib.rs");
    assert!(
        expansion.files[0].text.contains("Pixel { value: 0 }"),
        "selected body kept"
    );
    assert!(!expansion.files[0].text.contains("helper(p.value)"));
    assert!(expansion.total_tokens() > 0);

    // Below level 4 the file is a plain listing/skeleton with no bodies.
    let skeleton = expand_symbol(&graph, &sources, &make, ContextLevel::Skeleton).expect("level 3");
    assert!(!skeleton.files[0].text.contains("value: 0"));

    let err =
        expand_symbol(&graph, &sources, "missing", ContextLevel::Full).expect_err("unknown symbol");
    assert!(matches!(err, ContextError::UnknownSymbol(_)));
}

#[test]
fn expands_a_module_across_its_files() {
    let graph = build();
    let sources = sources();

    let expansion = expand_module(
        &graph,
        &sources,
        "demo",
        "inner",
        ContextLevel::Implementation,
    )
    .expect("expansion");
    assert_eq!(expansion.subject, "demo::inner");
    assert!(expansion.symbols.iter().any(|s| s.id.name == "outer"));
    assert_eq!(expansion.files.len(), 1);
    assert!(
        expansion.files[0].text.contains("crate::make();"),
        "module body kept"
    );

    // Submodules are included in a parent expansion.
    let root = expand_module(&graph, &sources, "demo", "", ContextLevel::Signatures).expect("root");
    assert!(root.symbols.iter().any(|s| s.id.name == "make"));
    assert!(root.symbols.iter().any(|s| s.id.name == "outer"));

    let err = expand_module(&graph, &sources, "demo", "nope", ContextLevel::Skeleton)
        .expect_err("unknown module");
    assert!(matches!(err, ContextError::UnknownModule(_)));
}

#[test]
fn expands_a_dependency_public_api() {
    let graph = build();
    let sources = sources();

    // Internal dependency: API symbols + files at source levels.
    let internal = expand_dependency(&graph, &sources, "tpt-math", ContextLevel::Implementation)
        .expect("tpt-math");
    assert_eq!(internal.subject, "tpt-math");
    assert!(internal.symbols.iter().any(|s| s.id.name == "square"));
    assert!(internal.repository.is_none());
    assert_eq!(internal.files.len(), 1);
    assert!(internal.files[0].text.contains("x * x"));

    // Below level 3 only the API listing is produced.
    let api = expand_dependency(&graph, &sources, "tpt-math", ContextLevel::Signatures)
        .expect("signatures");
    assert!(api.files.is_empty());
    assert!(api.symbols.iter().any(|s| s.signature.contains("square")));

    // External linked package: repository recorded; no sources indexed.
    let external =
        expand_dependency(&graph, &sources, "tpt-cv", ContextLevel::Skeleton).expect("tpt-cv");
    assert_eq!(
        external.repository.as_ref().map(|r| r.as_str()),
        Some("tpt-cv")
    );
    assert!(external.symbols.is_empty());
    assert!(external.files.is_empty());

    // Unrelated package names fail.
    let err = expand_dependency(&graph, &sources, "nope", ContextLevel::Symbols)
        .expect_err("unknown package");
    assert!(matches!(err, ContextError::UnknownPackage(_)));
}

#[test]
fn expands_related_tests_for_a_symbol() {
    let graph = build();
    let sources = sources();
    let make = key_of(&graph, "make");
    let helper = key_of(&graph, "helper");

    // Callers that are `#[test]` (test_make calls make) plus colocated
    // tests in the same file; the subject body comes along at level 4.
    let tests = expand_test(&graph, &sources, &make, ContextLevel::Implementation).expect("tests");
    assert!(
        tests.symbols.iter().any(|s| s.id.name == "test_make"),
        "colocated/calling test found: {:?}",
        tests.symbols.iter().map(|s| &s.id.name).collect::<Vec<_>>()
    );
    assert_eq!(tests.files.len(), 1);
    assert!(tests.files[0].text.contains("assert_eq!(p.value, 0);"));
    assert!(
        tests.files[0].text.contains("Pixel { value: 0 }"),
        "subject body kept alongside tests"
    );
    assert!(!tests.files[0].text.contains("helper(p.value)"));

    // helper has no direct test callers, but the colocated rule finds the
    // unit tests in its file.
    let colocated =
        expand_test(&graph, &sources, &helper, ContextLevel::Signatures).expect("colocated tests");
    assert!(colocated.symbols.iter().any(|s| s.id.name == "test_make"));

    // Integration tests calling consume live in tests/.
    let consume = key_of(&graph, "consume");
    let integration = expand_test(&graph, &sources, &consume, ContextLevel::Implementation)
        .expect("integration tests");
    assert!(
        integration
            .symbols
            .iter()
            .any(|s| s.id.name == "integration_consume"),
        "tests/ file test found"
    );

    let err = expand_test(&graph, &sources, "missing", ContextLevel::Skeleton)
        .expect_err("unknown symbol");
    assert!(matches!(err, ContextError::UnknownSymbol(_)));
}

#[test]
fn expands_related_implementations() {
    let graph = build();
    let sources = sources();
    let pixel = key_of(&graph, "Pixel");
    let draw = key_of(&graph, "Draw");
    let make = key_of(&graph, "make");

    // Type expansion pulls in its trait impls and everything referencing it.
    let related = expand_related(&graph, &sources, &pixel, ContextLevel::Implementation)
        .expect("related type");
    let names: Vec<&str> = related.symbols.iter().map(|s| s.id.name.as_str()).collect();
    assert!(names.contains(&"Pixel"));
    assert!(names.contains(&"<Pixel as Draw>"), "impl found: {names:?}");
    assert!(names.contains(&"Draw"), "trait found: {names:?}");
    assert!(names.contains(&"make"), "caller/ref found: {names:?}");
    assert!(
        related.files[0].text.contains("make();"),
        "impl body kept at level 4"
    );

    // Function expansion pulls in callers, callees and type relations.
    let call_related =
        expand_related(&graph, &sources, &make, ContextLevel::Skeleton).expect("related fn");
    let call_names: Vec<&str> = call_related
        .symbols
        .iter()
        .map(|s| s.id.name.as_str())
        .collect();
    assert!(call_names.contains(&"make"));
    assert!(call_names.contains(&"outer"), "caller: {call_names:?}");
    assert!(
        call_names.contains(&"Pixel::draw"),
        "caller: {call_names:?}"
    );
    assert!(call_names.contains(&"Pixel"), "type relation");
    assert!(call_names.contains(&"Draw"), "via related impl");

    // Callees are included for a function that has any.
    let consume_key = key_of(&graph, "consume");
    let consume_related =
        expand_related(&graph, &sources, &consume_key, ContextLevel::Skeleton).expect("consume");
    let consume_names: Vec<&str> = consume_related
        .symbols
        .iter()
        .map(|s| s.id.name.as_str())
        .collect();
    assert!(
        consume_names.contains(&"helper"),
        "callee: {consume_names:?}"
    );

    // Trait expansion includes its implementations.
    let trait_related =
        expand_related(&graph, &sources, &draw, ContextLevel::Signatures).expect("related trait");
    assert!(
        trait_related
            .symbols
            .iter()
            .any(|s| s.id.name == "<Pixel as Draw>")
    );

    let err = expand_related(&graph, &sources, "missing", ContextLevel::Full)
        .expect_err("unknown symbol");
    assert!(matches!(err, ContextError::UnknownSymbol(_)));
}
