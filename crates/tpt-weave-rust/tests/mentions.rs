//! Mention extraction feeding the graph (todo.md Phase 3).

use tpt_weave_core::{RepositoryId, SymbolKind};
use tpt_weave_rust::{parse_file, FileInput, MentionKind, ParsedFile};

fn parsed(source: &str) -> ParsedFile {
    let repository = RepositoryId::new("demo");
    let input = FileInput {
        repository: &repository,
        package: "demo",
        path: "src/lib.rs",
        module_prefix: &[],
    };
    parse_file(&input, source).expect("parses")
}

const SAMPLE: &str = r#"
pub struct Pixel { pub value: u8 }
pub trait Draw { fn draw(&self); }

pub fn make() -> Pixel {
    Pixel { value: 0 }
}

pub fn consume(p: Pixel) -> u8 {
    let doubled: Wrapper<u8> = Wrapper(p.value);
    helper(doubled.0)
}

fn helper(v: u8) -> u8 { v }

struct Wrapper<T>(T);

impl Draw for Pixel {
    fn draw(&self) { make(); }
}
"#;

fn mentions_of(file: &ParsedFile, from_name: &str) -> Vec<(String, MentionKind)> {
    let from = file
        .symbols
        .iter()
        .find(|s| s.id.name == from_name)
        .unwrap_or_else(|| panic!("missing symbol {from_name}"))
        .id
        .canonical_key();
    file.mentions
        .iter()
        .filter(|m| m.from == from)
        .map(|m| (m.name.clone(), m.kind))
        .collect()
}

#[test]
fn captures_calls_method_calls_and_type_paths() {
    let file = parsed(SAMPLE);

    // Return type + struct literal in make().
    let make = mentions_of(&file, "make");
    assert!(
        make.contains(&("Pixel".to_string(), MentionKind::Path)),
        "make mentions: {make:?}"
    );

    // consume: parameter/annotation types are paths; helper(..) is a call.
    let consume = mentions_of(&file, "consume");
    assert!(
        consume.contains(&("helper".to_string(), MentionKind::Call)),
        "consume mentions: {consume:?}"
    );
    assert!(
        consume.contains(&("Pixel".to_string(), MentionKind::Path)),
        "consume mentions: {consume:?}"
    );
    assert!(
        consume.contains(&("Wrapper".to_string(), MentionKind::Call)),
        "consume mentions: {consume:?}"
    );

    // Trait method body: make() is a direct call.
    let draw = mentions_of(&file, "Pixel::draw");
    assert!(
        draw.contains(&("make".to_string(), MentionKind::Call)),
        "Pixel::draw mentions: {draw:?}"
    );
}

#[test]
fn records_trait_impl_path_and_generic_bounds() {
    let file = parsed(SAMPLE);

    // The impl mentions its trait (becomes a TraitImpl edge in the graph).
    let impl_key = file
        .symbols
        .iter()
        .find(|s| s.id.kind == SymbolKind::Impl)
        .expect("impl symbol");
    let impl_mentions: Vec<(String, MentionKind)> = file
        .mentions
        .iter()
        .filter(|m| m.from == impl_key.id.canonical_key())
        .map(|m| (m.name.clone(), m.kind))
        .collect();
    assert!(
        impl_mentions.contains(&("Draw".to_string(), MentionKind::Path)),
        "impl mentions: {impl_mentions:?}"
    );

    // The trait bound in `impl Draw for Pixel` sits on the trait itself via
    // `T: ..`-style generics: check a bound mention on a generic struct.
    let bounded = parsed("pub struct Guarded<T: Draw> { pub inner: T }");
    let guarded = bounded
        .symbols
        .iter()
        .find(|s| s.id.name == "Guarded")
        .expect("Guarded");
    let mentions: Vec<(String, MentionKind)> = bounded
        .mentions
        .iter()
        .filter(|m| m.from == guarded.id.canonical_key())
        .map(|m| (m.name.clone(), m.kind))
        .collect();
    assert!(
        mentions.contains(&("Draw".to_string(), MentionKind::Path)),
        "Guarded mentions: {mentions:?}"
    );
}

#[test]
fn dedupes_names_keeping_the_most_specific_kind() {
    // `helper` appears as a call and (via `u8`-style paths) plain: use a
    // name seen both ways — `Pixel` appears as return type and literal.
    let file = parsed(SAMPLE);
    let make = mentions_of(&file, "make");
    let pixel_entries = make
        .iter()
        .filter(|(name, _)| name == "Pixel")
        .count();
    assert_eq!(pixel_entries, 1, "one deduped mention per name: {make:?}");
}
