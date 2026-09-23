//! Signature rendering, error reporting and module-prefix behaviour.

use tpt_weave_core::{RepositoryId, SymbolKind};
use tpt_weave_rust::{FileInput, ParsedFile, SymbolRecord, parse_file};

fn parsed(source: &str, prefix: &[&str]) -> ParsedFile {
    let repository = RepositoryId::new("tpt-cv");
    let input = FileInput {
        repository: &repository,
        package: "tpt-cv",
        path: "src/lib.rs",
        module_prefix: prefix,
    };
    parse_file(&input, source).expect("parses")
}

fn get<'a>(file: &'a ParsedFile, name: &str, kind: SymbolKind) -> &'a SymbolRecord {
    file.symbols
        .iter()
        .find(|s| s.id.name == name && s.id.kind == kind)
        .unwrap_or_else(|| panic!("missing symbol {name} ({kind:?})"))
}

const SAMPLE: &str = r#"
pub struct PixelBuffer<T> where T: Copy { pub pixels: Vec<T> }

impl<T: Copy> PixelBuffer<T> {
    pub fn view_mut(&mut self) -> &mut [T] { &mut self.pixels }
}

impl Sampler for PixelBuffer<u8> {
    fn sample(&self, x: u32) -> u32 { x }
}

pub mod image {
    pub fn resize(width: usize) -> usize { width }
}

pub static DEFAULT_FILTER: u8 = 0;
macro_rules! clamp { ($v:expr) => { $v }; }
"#;

#[test]
fn signatures_are_single_line_and_bodies_stripped() {
    let file = parsed(SAMPLE, &[]);

    let st = &get(&file, "PixelBuffer", SymbolKind::Struct).signature;
    assert!(st.contains("struct PixelBuffer<T>"), "{st}");
    assert!(st.contains("where T: Copy"), "{st}");
    assert_eq!(
        st.matches("where").count(),
        1,
        "where clause rendered exactly once: {st}"
    );

    let view = &get(&file, "PixelBuffer::view_mut", SymbolKind::Method).signature;
    assert!(view.contains("fn view_mut("), "{view}");
    assert!(view.contains("-> &mut[T]"), "{view}");
    assert!(!view.contains('{'), "no body: {view}");

    let sample = &get(&file, "PixelBuffer::sample", SymbolKind::Method).signature;
    assert!(sample.contains("fn sample("), "{sample}");
    assert!(!sample.contains('{'), "no body: {sample}");

    let resize = &get(&file, "resize", SymbolKind::Function).signature;
    assert!(
        resize.contains("fn resize(width: usize) -> usize"),
        "{resize}"
    );
    assert!(!resize.contains('{'), "{resize}");

    let impl_sig = &get(&file, "<PixelBuffer as Sampler>", SymbolKind::Impl).signature;
    assert!(impl_sig.contains("impl"), "{impl_sig}");
    assert!(
        impl_sig.contains("Sampler for PixelBuffer<u8>"),
        "{impl_sig}"
    );

    let stat = &get(&file, "DEFAULT_FILTER", SymbolKind::Constant).signature;
    assert!(stat.contains("pub static DEFAULT_FILTER: u8"), "{stat}");

    let mac = &get(&file, "clamp", SymbolKind::Macro).signature;
    assert!(mac.contains("macro_rules!"), "{mac}");
    assert!(!mac.contains('$'), "macro rules body stripped: {mac}");
}

#[test]
fn nested_modules_build_the_module_path() {
    let file = parsed("mod outer { mod inner { pub fn deep() {} } }", &[]);
    let deep = get(&file, "deep", SymbolKind::Function);
    assert_eq!(deep.module(), "outer::inner");
    let outer = get(&file, "outer", SymbolKind::Module);
    let inner = get(&file, "inner", SymbolKind::Module);
    assert!(outer.parent.is_none());
    assert_eq!(
        inner.parent.as_deref(),
        Some(outer.id.canonical_key().as_str())
    );

    // An indexer-supplied prefix shifts whole files (out-of-line mods).
    let file = parsed("pub fn resize() {}", &["image"]);
    assert_eq!(get(&file, "resize", SymbolKind::Function).module(), "image");
}

#[test]
fn parse_errors_report_a_location() {
    let repository = RepositoryId::new("demo");
    let input = FileInput {
        repository: &repository,
        package: "demo",
        path: "src/bad.rs",
        module_prefix: &[],
    };
    let err = parse_file(&input, "fn broken( {").expect_err("must fail");
    assert!(err.line >= 1, "line recorded: {}", err.line);
    assert!(!err.message.is_empty());
}
