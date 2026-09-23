//! Expansion: choosing *which* bodies to reveal when moving up a level
//! (todo.md Phase 4 "Expansion").

use crate::levels::{file_records, represent_file, FileRepresentation, Selection};
use crate::sources::SourceProvider;
use crate::ContextError;
use std::collections::BTreeSet;
use tpt_weave_core::{ContextLevel, SymbolKind};
use tpt_weave_graph::{ReferenceKind, RepositoryGraph};
use tpt_weave_rust::SymbolRecord;

/// Result of an expansion: the involved symbols plus file representations
/// rendered at the requested level.
#[derive(Clone, Debug, PartialEq)]
pub struct Expansion {
    /// Canonical key (or package/module name) that was expanded.
    pub subject: String,
    /// Level the files were rendered at.
    pub level: ContextLevel,
    /// Cross-repository link target, for dependency expansions.
    pub repository: Option<tpt_weave_core::RepositoryId>,
    /// Symbols involved in the expansion (source order).
    pub symbols: Vec<SymbolRecord>,
    /// File representations, sorted by path.
    pub files: Vec<FileRepresentation>,
}

impl Expansion {
    /// Total token estimate across all file representations.
    pub fn total_tokens(&self) -> u32 {
        self.files
            .iter()
            .map(|file| file.token_estimate)
            .fold(0u32, u32::saturating_add)
    }
}

/// Expands one symbol: its file rendered at `level` with only this symbol's
/// body kept at level 4.
pub fn expand_symbol(
    graph: &RepositoryGraph,
    sources: &dyn SourceProvider,
    key: &str,
    level: ContextLevel,
) -> Result<Expansion, ContextError> {
    let record = graph
        .symbol(key)
        .ok_or_else(|| ContextError::UnknownSymbol(key.to_string()))?;
    let selection = selection_for(level, std::iter::once(record));
    let file = represent_file(graph, sources, &record.file, level, &selection)?;
    Ok(Expansion {
        subject: key.to_string(),
        level,
        repository: None,
        symbols: vec![record.clone()],
        files: vec![file],
    })
}

/// Expands a module (and its submodules) across every file that defines
/// symbols in it; at level 4 all of the module's bodies are kept.
pub fn expand_module(
    graph: &RepositoryGraph,
    sources: &dyn SourceProvider,
    package: &str,
    module: &str,
    level: ContextLevel,
) -> Result<Expansion, ContextError> {
    let members: Vec<&SymbolRecord> = graph
        .symbols
        .iter()
        .filter(|record| {
            record.id.package == package
                && (module.is_empty()
                    || record.id.module == module
                    || record
                        .id
                        .module
                        .starts_with(&format!("{}::", module)))
        })
        .collect();
    if members.is_empty() {
        return Err(ContextError::UnknownModule(format!(
            "{package}::{module}"
        )));
    }
    let selection = selection_for(level, members.iter().copied());
    let files = represent_files(graph, sources, &members, level, &selection)?;
    Ok(Expansion {
        subject: format!("{package}::{module}"),
        level,
        repository: None,
        symbols: sort_records(members),
        files,
    })
}

/// Expands the public API of a dependency package. Files are included from
/// level 3 up when their sources are available (external packages often
/// have none); missing sources are skipped rather than failing.
pub fn expand_dependency(
    graph: &RepositoryGraph,
    sources: &dyn SourceProvider,
    package: &str,
    level: ContextLevel,
) -> Result<Expansion, ContextError> {
    let known = graph.crates.iter().any(|node| node.name == package)
        || graph.symbols.iter().any(|record| record.id.package == package)
        || graph
            .dependencies
            .iter()
            .any(|edge| edge.to_package == package);
    if !known {
        return Err(ContextError::UnknownPackage(package.to_string()));
    }

    let api: Vec<&SymbolRecord> = graph
        .public_api(package)
        .into_iter()
        .filter(|record| record.id.kind != SymbolKind::Module)
        .collect();
    let repository = graph
        .external_links
        .iter()
        .find(|link| link.package == package)
        .map(|link| link.repository.clone());

    let mut files = Vec::new();
    if level >= ContextLevel::Skeleton {
        let selection = selection_for(level, api.iter().copied());
        for path in distinct_paths(&api) {
            match represent_file(graph, sources, &path, level, &selection) {
                Ok(file) => files.push(file),
                // External packages may not have sources indexed here.
                Err(ContextError::MissingSource(_)) => continue,
                Err(error) => return Err(error),
            }
        }
    }

    Ok(Expansion {
        subject: package.to_string(),
        level,
        repository,
        symbols: sort_records(api),
        files,
    })
}

/// Expands the tests related to a symbol: colocated `#[test]` fns in the
/// same file, tests that call it, and test files named after its file.
/// At level 4 the tests' bodies (and the subject's) are kept.
pub fn expand_test(
    graph: &RepositoryGraph,
    sources: &dyn SourceProvider,
    key: &str,
    level: ContextLevel,
) -> Result<Expansion, ContextError> {
    let tests = test_keys(graph, key)?;
    let test_records: Vec<&SymbolRecord> = graph
        .symbols
        .iter()
        .filter(|record| tests.contains(&record.id.canonical_key()))
        .collect();

    // Keep the subject's body alongside the tests at level 4.
    let mut selection = selection_for(level, test_records.iter().copied());
    selection.insert(key.to_string());

    let files = represent_files(graph, sources, &test_records, level, &selection)?;
    Ok(Expansion {
        subject: key.to_string(),
        level,
        repository: None,
        symbols: sort_records(test_records),
        files,
    })
}

/// Canonical keys of the tests related to a symbol (not including the
/// subject itself). See [`expand_test`] for the selection rules.
pub fn test_keys(graph: &RepositoryGraph, key: &str) -> Result<BTreeSet<String>, ContextError> {
    let subject = graph
        .symbol(key)
        .ok_or_else(|| ContextError::UnknownSymbol(key.to_string()))?;

    let mut tests: BTreeSet<String> = BTreeSet::new();

    // 1. Colocated unit tests in the subject's own file.
    for record in file_records(graph, &subject.file) {
        if is_test_symbol(record) {
            tests.insert(record.id.canonical_key());
        }
    }

    // 2. Tests that call the subject.
    for record in graph.callers_of(key) {
        if is_test_symbol(record) {
            tests.insert(record.id.canonical_key());
        }
    }

    // 3. Test files named after the subject's file (src/foo.rs ↔ tests/foo.rs).
    let subject_stem = file_stem(&subject.file);
    if !subject_stem.is_empty() {
        for record in &graph.symbols {
            if is_test_path(&record.file)
                && is_test_symbol(record)
                && file_stem(&record.file) == subject_stem
            {
                tests.insert(record.id.canonical_key());
            }
        }
    }
    Ok(tests)
}

/// Expands a symbol together with everything related to it: callers,
/// callees, type relations, trait implementations, impl blocks and (for
/// impls) the implemented trait. At level 4 all of their bodies are kept.
pub fn expand_related(
    graph: &RepositoryGraph,
    sources: &dyn SourceProvider,
    key: &str,
    level: ContextLevel,
) -> Result<Expansion, ContextError> {
    let related = related_keys(graph, key)?;
    let records: Vec<&SymbolRecord> = graph
        .symbols
        .iter()
        .filter(|record| related.contains(&record.id.canonical_key()))
        .collect();
    let selection = selection_for(level, records.iter().copied());
    let files = represent_files(graph, sources, &records, level, &selection)?;
    Ok(Expansion {
        subject: key.to_string(),
        level,
        repository: None,
        symbols: sort_records(records),
        files,
    })
}

/// Canonical keys related to a symbol: the symbol itself plus callers,
/// callees, type relations, trait implementations, impl blocks, structural
/// parents/children and trait↔impl links (fixpoint closure).
pub fn related_keys(
    graph: &RepositoryGraph,
    key: &str,
) -> Result<BTreeSet<String>, ContextError> {
    graph
        .symbol(key)
        .ok_or_else(|| ContextError::UnknownSymbol(key.to_string()))?;

    let mut related: BTreeSet<String> = BTreeSet::new();
    related.insert(key.to_string());
    fn add_all(related: &mut BTreeSet<String>, records: Vec<&SymbolRecord>) {
        for record in records {
            related.insert(record.id.canonical_key());
        }
    }
    add_all(&mut related, graph.callees_of(key));
    add_all(&mut related, graph.callers_of(key));
    add_all(&mut related, graph.type_relations_of(key));
    add_all(&mut related, graph.trait_impls_of(key));

    // Symbols that reference the subject as a type or trait implementation.
    for edge in graph.references.iter().filter(|edge| {
        edge.to == key && matches!(edge.kind, ReferenceKind::Type | ReferenceKind::TraitImpl)
    }) {
        related.insert(edge.from.clone());
    }
    // For type subjects: their inherent and trait impl blocks.
    if let Some(subject) = graph.symbol(key) {
        if matches!(
            subject.id.kind,
            SymbolKind::Struct | SymbolKind::Enum | SymbolKind::TypeAlias
        ) {
            add_all(&mut related, graph.impls_of(&subject.id.name));
        }
    }

    // Structural closure: parents, children, and trait↔impl links, until
    // nothing new appears (structure is shallow, so this terminates fast).
    loop {
        let snapshot: Vec<String> = related.iter().cloned().collect();
        let mut changed = false;
        for member in &snapshot {
            let Some(record) = graph.symbol(member) else {
                continue;
            };
            // Enclosing impl/trait/module.
            if let Some(parent) = &record.parent {
                changed |= related.insert(parent.clone());
            }
            // Members of this impl/trait/module.
            for child in graph
                .symbols
                .iter()
                .filter(|child| child.parent.as_deref() == Some(member.as_str()))
            {
                changed |= related.insert(child.id.canonical_key());
            }
            // Trait implemented by this impl → the trait.
            for edge in graph
                .references
                .iter()
                .filter(|edge| edge.from == *member && edge.kind == ReferenceKind::TraitImpl)
            {
                changed |= related.insert(edge.to.clone());
            }
            // Implementations of this trait → the impls (and their members
            // arrive via the child rule above).
            add_all(&mut related, graph.trait_impls_of(member));
        }
        if !changed {
            break;
        }
    }
    Ok(related)
}

/// A `#[test]`-annotated symbol.
pub fn is_test_symbol(record: &SymbolRecord) -> bool {
    record
        .attributes
        .iter()
        .any(|attr| attr == "#[test]" || attr.ends_with("::test]"))
}

/// A path inside a `tests/` directory.
pub fn is_test_path(path: &str) -> bool {
    path.starts_with("tests/") || path.contains("/tests/")
}

fn file_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Selection used for rendering: empty below level 4, otherwise the
/// records relevant to the expansion.
fn selection_for<'a>(
    level: ContextLevel,
    records: impl IntoIterator<Item = &'a SymbolRecord>,
) -> Selection {
    if level >= ContextLevel::Implementation {
        records.into_iter().collect()
    } else {
        Selection::new()
    }
}

/// Renders every distinct file of `records` at `level`, sorted by path.
fn represent_files(
    graph: &RepositoryGraph,
    sources: &dyn SourceProvider,
    records: &[&SymbolRecord],
    level: ContextLevel,
    selection: &Selection,
) -> Result<Vec<FileRepresentation>, ContextError> {
    let mut files = Vec::new();
    for path in distinct_paths(records) {
        files.push(represent_file(graph, sources, &path, level, selection)?);
    }
    Ok(files)
}

fn distinct_paths(records: &[&SymbolRecord]) -> Vec<String> {
    let mut paths: Vec<String> = records.iter().map(|record| record.file.clone()).collect();
    paths.sort();
    paths.dedup();
    paths
}

fn sort_records(records: Vec<&SymbolRecord>) -> Vec<SymbolRecord> {
    let mut records: Vec<SymbolRecord> = records.into_iter().cloned().collect();
    records.sort_by(|a, b| {
        (a.file.as_str(), a.line, a.column).cmp(&(b.file.as_str(), b.line, b.column))
    });
    records
}
