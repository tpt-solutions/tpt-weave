//! Hierarchical representation levels 0-5 (todo.md Phase 4).

mod common;

use common::{build, key_of, sources, SRC_LIB};
use tpt_weave_context::{estimate_tokens, represent_file, Selection};
use tpt_weave_core::ContextLevel;

#[test]
fn renders_all_six_levels_deterministically() {
    let graph = build();
    let sources = sources();
    let path = "src/lib.rs";

    let metadata = represent_file(&graph, &sources, path, ContextLevel::Metadata, &Selection::new())
        .expect("level 0");
    let symbols = represent_file(&graph, &sources, path, ContextLevel::Symbols, &Selection::new())
        .expect("level 1");
    let signatures =
        represent_file(&graph, &sources, path, ContextLevel::Signatures, &Selection::new())
            .expect("level 2");
    let skeleton =
        represent_file(&graph, &sources, path, ContextLevel::Skeleton, &Selection::new())
            .expect("level 3");
    let full = represent_file(&graph, &sources, path, ContextLevel::Full, &Selection::new())
        .expect("level 5");

    // Level 0: metadata (spec.md section 9 example shape).
    assert!(metadata.text.starts_with("src/lib.rs\n"));
    assert!(metadata.text.contains("module=\n"), "crate-root module");
    assert!(metadata.text.contains("LOC="));
    assert!(metadata.text.contains("exports="));
    assert!(metadata.text.contains("dependencies="));
    assert_eq!(metadata.level, ContextLevel::Metadata);

    // Level 1: symbol listing.
    assert!(symbols.text.contains("make()\n"));
    assert!(symbols.text.contains("Pixel\n"));
    assert!(symbols.text.contains("Draw\n"));
    assert!(symbols.text.contains("mod inner\n"));
    assert!(!symbols.text.contains("{"), "no bodies at level 1");

    // Level 2: signatures.
    assert!(signatures.text.contains("pub fn make() -> Pixel"));
    assert!(signatures.text.contains("pub struct Pixel"));
    assert!(!signatures.text.contains("value: 0"), "no bodies at level 2");

    // Level 3: skeleton — structure without bodies, with source markers.
    assert!(skeleton.text.contains("pub fn make() -> Pixel"));
    assert!(skeleton.text.contains("impl Draw for Pixel"));
    assert!(skeleton.text.contains("pub mod inner"));
    assert!(skeleton.text.contains("// src/lib.rs:"));
    assert!(!skeleton.text.contains("value: 0"));
    assert!(!skeleton.text.contains("helper(p.value)"));
    syn_parse(&skeleton.text);

    // Level 5: complete source.
    assert_eq!(full.text, SRC_LIB);

    // Deterministic: identical renders.
    let again =
        represent_file(&graph, &sources, path, ContextLevel::Skeleton, &Selection::new())
            .expect("level 3");
    assert_eq!(skeleton.text, again.text);
    assert_eq!(skeleton.token_estimate, again.token_estimate);
}

#[test]
fn level_4_keeps_only_selected_bodies() {
    let graph = build();
    let sources = sources();
    let path = "src/lib.rs";
    let make = key_of(&graph, "make");

    let selection = Selection::new().with(make);
    let targeted =
        represent_file(&graph, &sources, path, ContextLevel::Implementation, &selection)
            .expect("level 4");

    // Selected body kept; every other body still stripped.
    assert!(targeted.text.contains("Pixel { value: 0 }"));
    assert!(!targeted.text.contains("helper(p.value)"));
    assert!(!targeted.text.contains("crate::make()"));
    assert!(targeted.text.contains("// src/lib.rs:"));
    syn_parse(&targeted.text);

    // Empty selection at level 4 equals the skeleton.
    let skeleton =
        represent_file(&graph, &sources, path, ContextLevel::Skeleton, &Selection::new())
            .expect("level 3");
    let empty = represent_file(
        &graph,
        &sources,
        path,
        ContextLevel::Implementation,
        &Selection::new(),
    )
    .expect("level 4 empty");
    assert_eq!(empty.text, skeleton.text);
}

#[test]
fn token_estimates_increase_with_levels() {
    let graph = build();
    let sources = sources();
    let path = "src/lib.rs";
    let render = |level: ContextLevel, selection: &Selection| {
        represent_file(&graph, &sources, path, level, selection)
            .unwrap_or_else(|error| panic!("{level}: {error}"))
            .token_estimate
    };
    let selection = Selection::new().with(key_of(&graph, "make"));

    let level0 = render(ContextLevel::Metadata, &Selection::new());
    let level1 = render(ContextLevel::Symbols, &Selection::new());
    let level2 = render(ContextLevel::Signatures, &Selection::new());
    let level3 = render(ContextLevel::Skeleton, &Selection::new());
    let level4 = render(ContextLevel::Implementation, &selection);
    let level5 = render(ContextLevel::Full, &Selection::new());

    assert!(level0 < level1, "{level0} < {level1}");
    assert!(level1 < level2, "{level1} < {level2}");
    assert!(level3 < level4, "{level3} < {level4}");
    // Skeletons carry `// path:line` markers, so they are not guaranteed to
    // be shorter than the raw source for very small files; the full source
    // must still dominate the pure listings.
    assert!(level5 > level2, "{level5} > {level2}");
    assert!(level5 > 0);
    assert_eq!(estimate_tokens(""), 0);
    assert_eq!(estimate_tokens("abcd"), 1);
    assert_eq!(estimate_tokens("abcde"), 2);
}

#[test]
fn missing_sources_fail_only_for_levels_that_need_them() {
    let graph = build();
    let empty = tpt_weave_context::Sources::new();
    let path = "src/lib.rs";

    // Levels 1-2 need no source text.
    represent_file(&graph, &empty, path, ContextLevel::Symbols, &Selection::new())
        .expect("symbols ok");
    represent_file(
        &graph,
        &empty,
        path,
        ContextLevel::Signatures,
        &Selection::new(),
    )
    .expect("signatures ok");

    // Levels 0 and 3-5 do.
    for level in [
        ContextLevel::Metadata,
        ContextLevel::Skeleton,
        ContextLevel::Implementation,
        ContextLevel::Full,
    ] {
        let err = represent_file(&graph, &empty, path, level, &Selection::new())
            .expect_err("missing source");
        assert_eq!(
            err,
            tpt_weave_context::ContextError::MissingSource(path.to_string())
        );
    }
}

fn syn_parse(text: &str) {
    syn::parse_file(text).expect("representation parses as Rust");
}
