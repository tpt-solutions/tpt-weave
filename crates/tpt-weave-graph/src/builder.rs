//! Deterministic graph construction (todo.md Phase 3).

use crate::model::{
    CrateNode, DependencyEdge, ExternalLink, ModuleNode, ReferenceEdge, ReferenceKind,
    RepositoryGraph,
};
use std::collections::BTreeMap;
use tpt_weave_core::{RepositoryId, Revision, SCHEMA_VERSION, SymbolKind};
use tpt_weave_index::CargoIndex;
use tpt_weave_rust::{Mention, MentionKind, ParsedFile, SymbolRecord};

/// Accumulates graph inputs, then builds a [`RepositoryGraph`].
///
/// Construction order is deterministic: inputs are processed in insertion
/// order and outputs are sorted before serialisation.
#[derive(Debug)]
pub struct GraphBuilder {
    repository: RepositoryId,
    revision: Revision,
    cargo: CargoIndex,
    files: Vec<(String, ParsedFile)>,
    external_links: Vec<ExternalLink>,
}

impl GraphBuilder {
    /// Starts building the graph for one repository at one revision.
    pub fn new(repository: impl Into<String>, revision: Revision, cargo: CargoIndex) -> Self {
        Self {
            repository: RepositoryId::new(repository),
            revision,
            cargo,
            files: Vec::new(),
            external_links: Vec::new(),
        }
    }

    /// Adds a parsed source file belonging to `package`.
    pub fn add_file(mut self, package: impl Into<String>, parsed: ParsedFile) -> Self {
        self.files.push((package.into(), parsed));
        self
    }

    /// Registers a cross-repository link for a dependency package name
    /// (todo.md Phase 3: "Add cross-repository graph support").
    pub fn link_cross_repository(
        mut self,
        package: impl Into<String>,
        repository: impl Into<String>,
    ) -> Self {
        let package = package.into();
        let repository = RepositoryId::new(repository);
        match self
            .external_links
            .iter_mut()
            .find(|l| l.package == package)
        {
            Some(existing) => existing.repository = repository,
            None => self.external_links.push(ExternalLink {
                package,
                repository,
            }),
        }
        self
    }

    /// Builds the [`RepositoryGraph`] deterministically from the inputs.
    pub fn build(self) -> RepositoryGraph {
        // Crate graph nodes.
        let crates: Vec<CrateNode> = self
            .cargo
            .packages
            .iter()
            .map(|p| CrateNode {
                name: p.name.clone(),
                version: p.version.clone(),
                manifest_path: p.manifest_path.clone(),
                is_workspace_member: p.is_workspace_member,
            })
            .collect();
        let known_packages: Vec<&str> = self
            .cargo
            .packages
            .iter()
            .map(|p| p.name.as_str())
            .collect();

        // Symbol table (all files) + module graph nodes.
        let mut symbols: Vec<SymbolRecord> = Vec::new();
        for (_, file) in &self.files {
            symbols.extend(file.symbols.iter().cloned());
        }
        let mut modules: Vec<ModuleNode> = Vec::new();
        for symbol in &symbols {
            if symbol.id.kind != SymbolKind::Module {
                continue;
            }
            let path = if symbol.id.module.is_empty() {
                symbol.id.name.clone()
            } else {
                format!("{}::{}", symbol.id.module, symbol.id.name)
            };
            if modules
                .iter()
                .any(|m| m.package == symbol.id.package && m.path == path)
            {
                continue;
            }
            let parent = path.rsplit_once("::").map(|(head, _)| head.to_string());
            modules.push(ModuleNode {
                package: symbol.id.package.clone(),
                path,
                parent,
            });
        }

        // Dependency graph edges (internal + registered cross-repo links).
        let mut dependencies = Vec::new();
        for pkg in &self.cargo.packages {
            for dep in &pkg.dependencies {
                let repository = self
                    .external_links
                    .iter()
                    .find(|l| l.package == dep.name)
                    .map(|l| l.repository.clone());
                dependencies.push(DependencyEdge {
                    from_package: pkg.name.clone(),
                    to_package: dep.name.clone(),
                    kind: dep.kind,
                    optional: dep.optional,
                    target: dep.target.clone(),
                    internal: known_packages.contains(&dep.name.as_str()),
                    repository,
                });
            }
        }
        dependencies.sort_by(|a, b| {
            a.from_package
                .cmp(&b.from_package)
                .then_with(|| a.to_package.cmp(&b.to_package))
        });
        dependencies
            .dedup_by(|a, b| a.from_package == b.from_package && a.to_package == b.to_package);

        // Resolve mentions into reference edges.
        let key_index: BTreeMap<String, usize> = symbols
            .iter()
            .enumerate()
            .map(|(index, s)| (s.id.canonical_key(), index))
            .collect();
        let mut references: Vec<ReferenceEdge> = Vec::new();
        let mut unresolved: u32 = 0;
        for (_, file) in &self.files {
            for mention in &file.mentions {
                let Some(&from_index) = key_index.get(&mention.from) else {
                    continue;
                };
                let origin = &symbols[from_index];
                let resolved = resolve_candidates(&symbols, origin, mention);
                if resolved.is_empty() {
                    unresolved += 1;
                    continue;
                }
                for to_index in resolved {
                    let target = &symbols[to_index];
                    let kind = match mention.kind {
                        MentionKind::Call => ReferenceKind::Call,
                        MentionKind::MethodCall => ReferenceKind::MethodCall,
                        MentionKind::Path => {
                            if origin.id.kind == SymbolKind::Impl
                                && target.id.kind == SymbolKind::Trait
                            {
                                ReferenceKind::TraitImpl
                            } else if target.id.kind == SymbolKind::Module {
                                ReferenceKind::Path
                            } else {
                                ReferenceKind::Type
                            }
                        }
                    };
                    references.push(ReferenceEdge {
                        from: origin.id.canonical_key(),
                        to: target.id.canonical_key(),
                        kind,
                    });
                }
            }
        }
        references.sort_by(|a, b| {
            a.from
                .cmp(&b.from)
                .then_with(|| a.to.cmp(&b.to))
                .then_with(|| a.kind.cmp(&b.kind))
        });
        references.dedup_by(|a, b| a.from == b.from && a.to == b.to && a.kind == b.kind);
        modules.sort_by(|a, b| a.package.cmp(&b.package).then_with(|| a.path.cmp(&b.path)));
        symbols.sort_by_key(|a| a.id.canonical_key());

        RepositoryGraph {
            schema: SCHEMA_VERSION,
            repository: self.repository,
            revision: self.revision,
            crates,
            modules,
            symbols,
            references,
            dependencies,
            external_links: self.external_links,
            unresolved_mentions: unresolved,
        }
    }
}

/// Resolves one mention against the symbol table with deterministic
/// tiering: same module → same package → any package, then kind
/// preference (calls prefer functions, method calls prefer methods, paths
/// prefer types/traits/modules).
fn resolve_candidates(
    symbols: &[SymbolRecord],
    origin: &SymbolRecord,
    mention: &Mention,
) -> Vec<usize> {
    let method_suffix = format!("::{}", mention.name);
    let mut candidates: Vec<usize> = symbols
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            let simple_method = s.id.kind == SymbolKind::Method
                && s.id.name.ends_with(&method_suffix)
                && s.id.name.len() > method_suffix.len();
            (s.id.name == mention.name || simple_method) && s.id.canonical_key() != mention.from
        })
        .map(|(index, _)| index)
        .collect();
    if candidates.is_empty() {
        return Vec::new();
    }

    let same_module: Vec<usize> = candidates
        .iter()
        .copied()
        .filter(|&i| {
            symbols[i].id.package == origin.id.package && symbols[i].id.module == origin.id.module
        })
        .collect();
    let same_package: Vec<usize> = candidates
        .iter()
        .copied()
        .filter(|&i| symbols[i].id.package == origin.id.package)
        .collect();
    let tier = if !same_module.is_empty() {
        same_module
    } else if !same_package.is_empty() {
        same_package
    } else {
        candidates.sort_unstable();
        candidates
    };

    let preferred: Vec<usize> = tier
        .iter()
        .copied()
        .filter(|&i| match mention.kind {
            MentionKind::Call => symbols[i].id.kind == SymbolKind::Function,
            MentionKind::MethodCall => symbols[i].id.kind == SymbolKind::Method,
            MentionKind::Path => matches!(
                symbols[i].id.kind,
                SymbolKind::Struct
                    | SymbolKind::Enum
                    | SymbolKind::Trait
                    | SymbolKind::TypeAlias
                    | SymbolKind::Module
                    | SymbolKind::Constant
            ),
        })
        .collect();
    if preferred.is_empty() {
        tier
    } else {
        preferred
    }
}
