//! The repository symbol graph (todo.md Phase 3; spec.md section 11).
//!
//! Built deterministically from a [`tpt_weave_index::CargoIndex`] plus files
//! parsed by [`tpt_weave_rust::parse_file`]:
//!
//! - symbol table, module graph, crate graph, dependency graph;
//! - resolved reference, caller/callee, type and trait-impl relationships;
//! - public API queries and cross-repository links (spec.md section 11);
//! - JSON persistence with schema + revision invalidation (spec.md
//!   section 21: conservative invalidation).

#![forbid(unsafe_code)]

pub mod builder;
pub mod model;
pub mod persistence;

pub use builder::GraphBuilder;
pub use model::{
    CrateNode, DependencyEdge, ExternalLink, ModuleNode, ReferenceEdge, ReferenceKind,
    RepositoryGraph,
};
pub use persistence::{graph_path, GraphError, GRAPH_FILE};
