//! Extraction of every symbol kind requested by todo.md Phase 2.

use tpt_weave_core::{RepositoryId, SymbolKind};
use tpt_weave_rust::{parse_file, FileInput, ParsedFile, SymbolRecord, Visibility};

const SAMPLE: &str = r#"
#[derive(Clone, Debug)]
pub struct PixelBuffer<T> where T: Copy {
    pub pixels: Vec<T>,
}

pub enum Filter {
    Nearest,
    Bilinear,
}

pub trait Sampler {
    fn sample(&self, x: u32) -> u32;
}

impl<T: Copy> PixelBuffer<T> {
    pub fn view_mut(&mut self) -> &mut [T] {
        &mut self.pixels
    }

    fn private_helper() {}
}

impl Sampler for PixelBuffer<u8> {
    fn sample(&self, x: u32) -> u32 {
        x
    }
}

pub mod image {
    pub fn resize(width: usize) -> usize {
        width
    }
}

const MAX_DIMENSION: usize = 4096;
pub static DEFAULT_FILTER: u8 = 0;
pub type Pixel = u8;

macro_rules! clamp {
    ($v:expr) => { $v };
}
"#;

fn parsed_sample() -> ParsedFile {
    let repository = RepositoryId::new("tpt-cv");
    let input = FileInput {
        repository: &repository,
        package: "tpt-cv",
        path: "src/lib.rs",
        module_prefix: &[],
    };
    parse_file(&input, SAMPLE).expect("sample parses")
}

fn get<'a>(file: &'a ParsedFile, name: &str, kind: SymbolKind) -> &'a SymbolRecord {
    file.symbols
        .iter()
        .find(|s| s.id.name == name && s.id.kind == kind)
        .unwrap_or_else(|| panic!("missing symbol {name} ({kind:?})"))
}

#[test]
fn extracts_every_requested_item_kind() {
    let file = parsed_sample();

    get(&file, "PixelBuffer", SymbolKind::Struct);
    get(&file, "Filter", SymbolKind::Enum);
    get(&file, "Sampler", SymbolKind::Trait);
    get(&file, "PixelBuffer", SymbolKind::Impl); // inherent impl
    get(&file, "<PixelBuffer as Sampler>", SymbolKind::Impl);
    get(&file, "PixelBuffer::view_mut", SymbolKind::Method);
    get(&file, "PixelBuffer::private_helper", SymbolKind::Method);
    get(&file, "Sampler::sample", SymbolKind::Method); // trait declaration
    get(&file, "PixelBuffer::sample", SymbolKind::Method); // trait impl
    get(&file, "image", SymbolKind::Module);
    get(&file, "resize", SymbolKind::Function);
    get(&file, "MAX_DIMENSION", SymbolKind::Constant);
    get(&file, "DEFAULT_FILTER", SymbolKind::Constant);
    get(&file, "Pixel", SymbolKind::TypeAlias);
    get(&file, "clamp", SymbolKind::Macro);

    // Exactly the items above — no field/variant noise.
    assert_eq!(file.symbols.len(), 15, "{:#?}", file.symbols);
}

#[test]
fn records_visibility_attributes_locations_and_parents() {
    let file = parsed_sample();

    let pixel = get(&file, "PixelBuffer", SymbolKind::Struct);
    assert_eq!(pixel.visibility, Visibility::Public);
    assert!(pixel.line >= 3, "name line recorded: {}", pixel.line);
    assert!(pixel.column >= 1);
    assert!(
        pixel
            .attributes
            .iter()
            .any(|a| a.starts_with("#[derive(") && a.contains("Clone")),
        "attributes: {:?}",
        pixel.attributes
    );
    assert!(pixel.parent.is_none(), "top-level item has no parent");

    let helper = get(&file, "PixelBuffer::private_helper", SymbolKind::Method);
    assert_eq!(helper.visibility, Visibility::Inherited);
    assert!(helper.parent.is_some(), "impl member records its impl");

    let resize = get(&file, "resize", SymbolKind::Function);
    assert_eq!(resize.module(), "image");
    assert_eq!(resize.visibility, Visibility::Public);
    assert_eq!(resize.file, "src/lib.rs");

    // Line ordering follows the file.
    let sampler = get(&file, "Sampler", SymbolKind::Trait);
    assert!(pixel.line < sampler.line);
}
