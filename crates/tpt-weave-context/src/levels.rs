//! Hierarchical representation rendering (todo.md Phase 4, spec.md section 9).

use crate::ContextError;
use crate::sources::SourceProvider;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use tpt_weave_core::{ContextLevel, SymbolKind};
use tpt_weave_graph::RepositoryGraph;
use tpt_weave_rust::{FileInput, SymbolRecord, skeleton, skeleton_with};

/// Which symbols receive a full implementation at level 4 (canonical keys).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection(BTreeSet<String>);

impl Selection {
    /// An empty selection (level 4 then equals level 3).
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one canonical key (chainable).
    pub fn with(mut self, key: impl Into<String>) -> Self {
        self.0.insert(key.into());
        self
    }

    /// Adds one canonical key in place.
    pub fn insert(&mut self, key: impl Into<String>) {
        self.0.insert(key.into());
    }

    /// Whether the canonical key is selected.
    pub fn contains(&self, key: &str) -> bool {
        self.0.contains(key)
    }

    /// Number of selected keys.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> FromIterator<&'a SymbolRecord> for Selection {
    fn from_iter<T: IntoIterator<Item = &'a SymbolRecord>>(iter: T) -> Self {
        let mut selection = Selection::new();
        for record in iter {
            selection.insert(record.id.canonical_key());
        }
        selection
    }
}

/// One hierarchical representation of one file (spec.md section 9).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileRepresentation {
    /// Repository-relative path of the file.
    pub path: String,
    /// Level this text was rendered at.
    pub level: ContextLevel,
    /// Rendered text.
    pub text: String,
    /// Deterministic token estimate (`ceil(chars / 4)`).
    pub token_estimate: u32,
}

/// Deterministic token estimate: one token per four characters, rounded up.
pub fn estimate_tokens(text: &str) -> u32 {
    let chars = text.chars().count().div_ceil(4);
    u32::try_from(chars).unwrap_or(u32::MAX)
}

/// Renders one file at one context level (spec.md section 9).
///
/// Levels 1, 2 and the listing half of level 0 need only the graph; levels
/// 0, 3, 4 and 5 additionally need the file's source.
pub fn represent_file(
    graph: &RepositoryGraph,
    sources: &dyn SourceProvider,
    path: &str,
    level: ContextLevel,
    selection: &Selection,
) -> Result<FileRepresentation, ContextError> {
    let records = file_records(graph, path);
    let package = records
        .first()
        .map(|record| record.id.package.clone())
        .unwrap_or_default();
    let prefix_owned = derive_module_prefix(&records);
    let prefix: Vec<&str> = prefix_owned.iter().map(String::as_str).collect();
    let input = FileInput {
        repository: &graph.repository,
        package: &package,
        path,
        module_prefix: &prefix,
    };

    let text = match level {
        ContextLevel::Metadata => {
            let source = source_of(sources, path)?;
            render_metadata(path, &prefix_owned, &records, source, graph)
        }
        ContextLevel::Symbols => listing(&records, display_name),
        ContextLevel::Signatures => listing(&records, |record| record.signature.clone()),
        ContextLevel::Skeleton => {
            skeleton(&input, source_of(sources, path)?)
                .map_err(|error| map_parse(path, error))?
                .text
        }
        ContextLevel::Implementation => {
            skeleton_with(&input, source_of(sources, path)?, |id| {
                selection.contains(&id.canonical_key())
            })
            .map_err(|error| map_parse(path, error))?
            .text
        }
        ContextLevel::Full => source_of(sources, path)?.to_string(),
    };

    Ok(FileRepresentation {
        path: path.to_string(),
        level,
        token_estimate: estimate_tokens(&text),
        text,
    })
}

/// Symbols of one file ordered by source position.
pub(crate) fn file_records<'a>(graph: &'a RepositoryGraph, path: &str) -> Vec<&'a SymbolRecord> {
    let mut records: Vec<&SymbolRecord> = graph
        .symbols
        .iter()
        .filter(|record| record.file == path)
        .collect();
    records.sort_by_key(|record| (record.line, record.column));
    records
}

/// Module path of a file's first top-level symbol (out-of-line `mod` chain).
fn derive_module_prefix(records: &[&SymbolRecord]) -> Vec<String> {
    let Some(top) = records
        .iter()
        .filter(|record| record.parent.is_none())
        .min_by_key(|record| (record.line, record.column))
    else {
        return Vec::new();
    };
    top.id
        .module
        .split("::")
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

fn source_of<'a>(sources: &'a dyn SourceProvider, path: &str) -> Result<&'a str, ContextError> {
    sources
        .source(path)
        .ok_or_else(|| ContextError::MissingSource(path.to_string()))
}

fn map_parse(path: &str, error: tpt_weave_rust::ParseError) -> ContextError {
    ContextError::Parse {
        path: path.to_string(),
        error,
    }
}

/// Level 1-style name: `make()`, `Pixel::draw()`, `mod inner`, `impl Draw
/// for Pixel`, or the plain name otherwise.
fn display_name(record: &SymbolRecord) -> String {
    match record.id.kind {
        SymbolKind::Function | SymbolKind::Method => format!("{}()", record.id.name),
        SymbolKind::Module => format!("mod {}", record.id.name),
        SymbolKind::Impl => format!("impl {}", record.id.name),
        _ => record.id.name.clone(),
    }
}

fn listing(records: &[&SymbolRecord], render: impl Fn(&SymbolRecord) -> String) -> String {
    let mut text = String::new();
    for record in records {
        text.push_str(&render(record));
        text.push('\n');
    }
    text
}

/// Level 0 metadata (spec.md section 9).
fn render_metadata(
    path: &str,
    module_prefix: &[String],
    records: &[&SymbolRecord],
    source: &str,
    graph: &RepositoryGraph,
) -> String {
    let exports = records
        .iter()
        .filter(|record| record.visibility.is_public())
        .count();
    let own_keys: BTreeSet<String> = records
        .iter()
        .map(|record| record.id.canonical_key())
        .collect();
    let mut external: BTreeSet<String> = BTreeSet::new();
    for record in records {
        let key = record.id.canonical_key();
        for edge in graph.references.iter().filter(|edge| edge.from == key) {
            if own_keys.contains(&edge.to) {
                continue;
            }
            if let Some(target) = graph.symbol(&edge.to) {
                if target.file != path {
                    external.insert(target.id.canonical_key());
                }
            }
        }
    }
    format!(
        "{path}\nmodule={}\nLOC={}\nexports={exports}\ndependencies={}\n",
        module_prefix.join("::"),
        source.lines().count(),
        external.len(),
    )
}
