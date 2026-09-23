//! Graph nodes, edges and queries (spec.md section 11, todo.md Phase 3).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tpt_weave_core::{RepositoryId, Revision, SymbolKind, SCHEMA_VERSION};
use tpt_weave_index::DependencyKind;
use tpt_weave_rust::SymbolRecord;

/// One crate (crate graph node).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateNode {
    pub name: String,
    pub version: String,
    pub manifest_path: PathBuf,
    pub is_workspace_member: bool,
}

/// One module (module graph node). `path` is `::`-separated within the
/// package; there is no node for the crate root itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleNode {
    pub package: String,
    /// e.g. `image::resample`.
    pub path: String,
    /// Parent module path, or `None` for a top-level module.
    pub parent: Option<String>,
}

/// Kind of resolved reference edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    /// `foo(..)` — a caller/callee edge.
    Call,
    /// `x.bar(..)` — a caller/callee edge.
    MethodCall,
    /// A type/bound relationship (type relationships).
    Type,
    /// An impl block pointing at its trait (trait implementations).
    TraitImpl,
    /// Any other resolved identifier (general references).
    Path,
}

/// A resolved reference between two symbols (canonical keys).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceEdge {
    pub from: String,
    pub to: String,
    pub kind: ReferenceKind,
}

/// One declared dependency (dependency graph edge). Edges with
/// `internal = true` also form the crate graph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from_package: String,
    /// Package name as declared (may be outside this graph).
    pub to_package: String,
    pub kind: DependencyKind,
    pub optional: bool,
    pub target: Option<String>,
    /// The dependency resolves to a package inside this graph.
    pub internal: bool,
    /// Registered cross-repository link (spec.md section 11).
    pub repository: Option<tpt_weave_core::RepositoryId>,
}

/// Registered package-name to repository mapping (cross-repository graph).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalLink {
    pub package: String,
    pub repository: tpt_weave_core::RepositoryId,
}

/// The repository graph (spec.md section 11).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RepositoryGraph {
    /// Schema the graph was written with
    /// ([`tpt_weave_core::SCHEMA_VERSION`]); drives invalidation.
    pub schema: u32,
    pub repository: tpt_weave_core::RepositoryId,
    /// Git revision the graph was built from (invalidation key).
    pub revision: tpt_weave_core::Revision,
    /// Crate graph nodes.
    pub crates: Vec<CrateNode>,
    /// Module graph nodes.
    pub modules: Vec<ModuleNode>,
    /// The symbol table (every extracted [`SymbolRecord`]).
    pub symbols: Vec<SymbolRecord>,
    /// Resolved references (callers/callees, types, traits, paths).
    pub references: Vec<ReferenceEdge>,
    /// Dependency graph edges.
    pub dependencies: Vec<DependencyEdge>,
    /// Registered cross-repository links.
    pub external_links: Vec<ExternalLink>,
    /// Mentions that matched no symbol (dropped deterministically;
    /// diagnostics for resolver tuning).
    pub unresolved_mentions: u32,
}

impl RepositoryGraph {
    /// Symbol-table lookup by canonical key.
    pub fn symbol(&self, key: &str) -> Option<&SymbolRecord> {
        self.symbols.iter().find(|s| s.id.canonical_key() == key)
    }

    /// All symbols whose id equals this exact name (methods are
    /// `Type::method`; use [`Self::find_methods_by_name`] for bare names).
    pub fn find_symbols(&self, name: &str) -> Vec<&SymbolRecord> {
        self.symbols.iter().filter(|s| s.id.name == name).collect()
    }

    /// Methods whose `Type::method` id ends with `::name`.
    pub fn find_methods_by_name(&self, name: &str) -> Vec<&SymbolRecord> {
        let suffix = format!("::{name}");
        self.symbols
            .iter()
            .filter(|s| s.id.kind == SymbolKind::Method && s.id.name.ends_with(&suffix))
            .collect()
    }

    /// Symbols defined in one module of a package (`""` = crate root).
    pub fn symbols_in_module(&self, package: &str, module: &str) -> Vec<&SymbolRecord> {
        self.symbols
            .iter()
            .filter(|s| s.id.package == package && s.id.module == module)
            .collect()
    }

    /// Module graph nodes of a package.
    pub fn modules_of(&self, package: &str) -> Vec<&ModuleNode> {
        self.modules.iter().filter(|m| m.package == package).collect()
    }

    /// Full dependency graph edges of a package (internal + external).
    pub fn dependencies_of(&self, package: &str) -> Vec<&DependencyEdge> {
        self.dependencies
            .iter()
            .filter(|e| e.from_package == package)
            .collect()
    }

    /// Crate graph: only dependencies that resolve inside this graph.
    pub fn crate_dependencies(&self, package: &str) -> Vec<&DependencyEdge> {
        self.dependencies
            .iter()
            .filter(|e| e.from_package == package && e.internal)
            .collect()
    }

    /// Symbols this one calls (`Call` / `MethodCall` targets).
    pub fn callees_of(&self, key: &str) -> Vec<&SymbolRecord> {
        let keys: Vec<&str> = self
            .references
            .iter()
            .filter(|e| {
                e.from == key && matches!(e.kind, ReferenceKind::Call | ReferenceKind::MethodCall)
            })
            .map(|e| e.to.as_str())
            .collect();
        keys.iter().filter_map(|k| self.symbol(k)).collect()
    }

    /// Symbols that call this one (edges point *at* it).
    pub fn callers_of(&self, key: &str) -> Vec<&SymbolRecord> {
        let mut keys: Vec<&str> = Vec::new();
        for edge in &self.references {
            if edge.to == key
                && matches!(edge.kind, ReferenceKind::Call | ReferenceKind::MethodCall)
                && !keys.contains(&edge.from.as_str())
            {
                keys.push(&edge.from);
            }
        }
        keys.iter().filter_map(|k| self.symbol(k)).collect()
    }

    /// Outgoing type relationships (`Type` edges) of a symbol.
    pub fn type_relations_of(&self, key: &str) -> Vec<&SymbolRecord> {
        let keys: Vec<&str> = self
            .references
            .iter()
            .filter(|e| e.from == key && e.kind == ReferenceKind::Type)
            .map(|e| e.to.as_str())
            .collect();
        keys.iter().filter_map(|k| self.symbol(k)).collect()
    }

    /// Impl blocks that implement the given trait (trait implementations).
    pub fn trait_impls_of(&self, trait_key: &str) -> Vec<&SymbolRecord> {
        let keys: Vec<&str> = self
            .references
            .iter()
            .filter(|e| e.to == trait_key && e.kind == ReferenceKind::TraitImpl)
            .map(|e| e.from.as_str())
            .collect();
        keys.iter().filter_map(|k| self.symbol(k)).collect()
    }

    /// Every resolved reference target of a symbol with its kind.
    pub fn references_of(&self, key: &str) -> Vec<(&SymbolRecord, ReferenceKind)> {
        self.references
            .iter()
            .filter(|e| e.from == key)
            .filter_map(|e| self.symbol(&e.to).map(|s| (s, e.kind)))
            .collect()
    }

    /// Impl blocks of a type (`impl PixelBuffer` or
    /// `impl<..> Trait for PixelBuffer`).
    pub fn impls_of(&self, type_name: &str) -> Vec<&SymbolRecord> {
        let trait_form = format!("<{type_name} as ");
        self.symbols
            .iter()
            .filter(|s| {
                s.id.kind == SymbolKind::Impl
                    && (s.id.name == type_name || s.id.name.starts_with(&trait_form))
            })
            .collect()
    }

    /// Public API surface of a package (plain `pub` symbols).
    pub fn public_api(&self, package: &str) -> Vec<&SymbolRecord> {
        self.symbols
            .iter()
            .filter(|s| s.id.package == package && s.visibility.is_public())
            .collect()
    }

    /// Public API relationships: reference edges whose endpoints are both
    /// plain-`pub` symbols of the same package.
    pub fn public_api_edges(&self, package: &str) -> Vec<&ReferenceEdge> {
        let is_pub = |key: &str| {
            self.symbol(key)
                .map(|s| s.visibility.is_public() && s.id.package == package)
                .unwrap_or(false)
        };
        self.references
            .iter()
            .filter(|e| is_pub(&e.from) && is_pub(&e.to))
            .collect()
    }

    /// Repositories reachable through registered cross-repository links
    /// (spec.md section 11).
    pub fn linked_repositories(&self) -> Vec<RepositoryId> {
        let mut repos: Vec<RepositoryId> = self
            .external_links
            .iter()
            .map(|l| l.repository.clone())
            .chain(self.dependencies.iter().filter_map(|e| e.repository.clone()))
            .collect();
        repos.sort();
        repos.dedup();
        repos
    }

    /// Graph invalidation (todo.md Phase 3): stale when the schema or the
    /// revision it was built from no longer matches.
    pub fn is_stale(&self, revision: &Revision) -> bool {
        self.schema != SCHEMA_VERSION || self.revision.sha != revision.sha
    }
}


