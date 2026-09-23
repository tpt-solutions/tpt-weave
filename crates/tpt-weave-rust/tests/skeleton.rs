//! Skeleton generation (todo.md Phase 4).

use tpt_weave_core::{RepositoryId, SymbolId, SymbolKind};
use tpt_weave_rust::{BODY_PLACEHOLDER, FileInput, parse_file, skeleton, skeleton_with};

const SRC: &str = r#"//! Crate docs.
#[derive(Clone)]
pub struct Pixel {
    pub value: u8,
}

pub fn make() -> Pixel {
    Pixel { value: 0 }
}

pub trait Draw {
    fn draw(&self);
    fn redraw(&self) {
        self.draw();
    }
}

impl Draw for Pixel {
    fn draw(&self) {
        let p = make();
        let _ = p;
    }
}

pub mod inner {
    pub fn outer() {
        crate::make();
    }
}
"#;

fn input<'a>(repository: &'a RepositoryId, path: &'a str, prefix: &'a [&'a str]) -> FileInput<'a> {
    FileInput {
        repository,
        package: "demo",
        path,
        module_prefix: prefix,
    }
}

fn repository() -> RepositoryId {
    RepositoryId::new("demo-repo")
}

#[test]
fn removes_bodies_preserving_signatures_types_impls_and_modules() {
    let repo = repository();
    let sk = skeleton(&input(&repo, "src/lib.rs", &[]), SRC).expect("skeleton");

    // Function bodies removed (make, redraw, draw, outer — the bodyless
    // trait declaration has no body to remove).
    assert_eq!(sk.removed_bodies, 4);
    assert!(!sk.text.contains("Pixel { value: 0 }"));
    assert!(!sk.text.contains("self.draw()"));
    assert!(!sk.text.contains("crate::make()"));
    assert!(!sk.text.contains("let p = make()"));
    assert!(sk.text.contains(BODY_PLACEHOLDER));

    // Signatures, type definitions, attributes and impl relationships
    // preserved verbatim.
    assert!(sk.text.contains("pub fn make() -> Pixel"));
    assert!(sk.text.contains("pub struct Pixel"));
    assert!(sk.text.contains("#[derive(Clone)]"));
    assert!(sk.text.contains("pub trait Draw"));
    assert!(sk.text.contains("fn draw(&self);"));
    assert!(sk.text.contains("impl Draw for Pixel"));
    assert!(sk.text.contains("pub mod inner"));
    assert!(sk.text.contains("pub fn outer()"));

    // Line/source references preserved as markers.
    assert!(sk.text.contains("// src/lib.rs:2"), "struct marker");
    assert!(sk.text.contains("// src/lib.rs:7"), "make marker");

    // The rendered skeleton is still valid Rust.
    syn::parse_file(&sk.text).expect("skeleton parses");
    assert_eq!(sk.path, "src/lib.rs");
}

#[test]
fn keeps_selected_bodies_and_removes_the_rest() {
    let repo = repository();
    let keep_make = |id: &SymbolId| id.name == "make" && id.kind == SymbolKind::Function;
    let sk = skeleton_with(&input(&repo, "src/lib.rs", &[]), SRC, keep_make).expect("skeleton");

    // Selected body kept, everything else still stripped.
    assert!(sk.text.contains("Pixel { value: 0 }"));
    assert!(!sk.text.contains("crate::make()"));
    assert!(!sk.text.contains("let p = make()"));
    assert_eq!(sk.removed_bodies, 3);
    syn::parse_file(&sk.text).expect("skeleton parses");
}

#[test]
fn applies_module_prefix_to_selection_keys() {
    let repo = repository();
    let prefix = ["image"];
    let sk = skeleton_with(
        &input(&repo, "src/image.rs", &prefix),
        "pub fn resample() -> u8 { 0 }\n",
        |id| id.module == "image" && id.name == "resample",
    )
    .expect("skeleton");
    assert!(sk.text.contains("{ 0 }"), "prefix-matched body kept");
    assert_eq!(sk.removed_bodies, 0);
}

#[test]
fn reports_parse_errors_with_a_location() {
    let repo = repository();
    let err = skeleton(&input(&repo, "src/lib.rs", &[]), "fn (").expect_err("fails");
    assert!(err.line >= 1);
    assert!(!err.message.is_empty());
}

#[test]
fn keep_callback_ids_match_parse_file_records() {
    // Selections derived from the symbol table must match the ids the
    // skeleton walker constructs, otherwise expanded bodies silently vanish.
    let repo = repository();
    let input = input(&repo, "src/lib.rs", &[]);
    let parsed = parse_file(&input, SRC).expect("parses");
    let keys: Vec<String> = parsed
        .symbols
        .iter()
        .map(|s| s.id.canonical_key())
        .collect();

    let mut seen: Vec<String> = Vec::new();
    let sk = skeleton_with(&input, SRC, |id: &SymbolId| {
        seen.push(id.canonical_key());
        keys.contains(&id.canonical_key())
    })
    .expect("skeleton");
    assert_eq!(
        sk.removed_bodies, 0,
        "every body-bearing id matches a symbol record"
    );
    assert_eq!(seen.len(), 4, "four bodies visited: {seen:?}");
}
