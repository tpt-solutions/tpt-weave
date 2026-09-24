//! Hierarchical context representations, expansion and deterministic
//! retrieval (todo.md Phases 4–5, spec.md sections 9–10).
//!
//! Levels 0-5 of [`ContextLevel`] are rendered from a
//! [`RepositoryGraph`](tpt_weave_graph::RepositoryGraph) plus file sources:
//!
//! - level 0 metadata, level 1 symbol listing, level 2 signatures are
//!   synthesised from the symbol table;
//! - level 3 skeleton and level 4 targeted implementation come from
//!   [`tpt_weave_rust::skeleton_with`];
//! - level 5 is the complete source.
//!
//! Expansion ([`expand`]) walks the graph to decide *which* bodies to keep
//! when moving to a higher level: symbols, modules, dependencies, tests and
//! related implementations.

#![forbid(unsafe_code)]

pub mod expand;
pub mod levels;
pub mod retrieval;
pub mod sources;

pub use expand::{
    Expansion, expand_dependency, expand_module, expand_related, expand_symbol, expand_test,
    related_keys, test_keys,
};
pub use levels::{FileRepresentation, Selection, estimate_tokens, represent_file};
pub use retrieval::{RepositoryOverview, Retriever};
pub use sources::{LazySources, SourceProvider, Sources};

use std::fmt;

/// Failure to produce a representation or expansion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextError {
    /// No symbol with that canonical key exists in the graph.
    UnknownSymbol(String),
    /// No package with that name exists in the graph.
    UnknownPackage(String),
    /// No module with that path exists in the package.
    UnknownModule(String),
    /// The requested file has no source available.
    MissingSource(String),
    /// The file's source failed to parse during skeleton rendering.
    Parse {
        path: String,
        error: tpt_weave_rust::ParseError,
    },
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContextError::UnknownSymbol(key) => write!(f, "unknown symbol: {key}"),
            ContextError::UnknownPackage(package) => write!(f, "unknown package: {package}"),
            ContextError::UnknownModule(module) => write!(f, "unknown module: {module}"),
            ContextError::MissingSource(path) => write!(f, "missing source: {path}"),
            ContextError::Parse { path, error } => write!(f, "{path}: {error}"),
        }
    }
}

impl std::error::Error for ContextError {}

impl From<tpt_weave_rust::ParseError> for ContextError {
    fn from(error: tpt_weave_rust::ParseError) -> Self {
        // Callers that know the path wrap this via `map_parse`; bare
        // conversions keep the location information.
        ContextError::Parse {
            path: String::new(),
            error,
        }
    }
}
