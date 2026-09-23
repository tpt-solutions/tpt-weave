//! Working-tree loading for CLI commands: cargo index, git revision,
//! parsed sources and the symbol graph (shared shape with the MCP
//! workspace, todo.md Phase 9 / Phase 10).

use std::path::{Path, PathBuf};
use tpt_weave_context::Sources;
use tpt_weave_core::{Manifest, Revision, manifest_path};
use tpt_weave_graph::{GraphBuilder, RepositoryGraph, graph_path};
use tpt_weave_index::{CargoIndex, GitRepository};
use tpt_weave_rust::{FileInput, parse_file};

use crate::error::CliError;

/// Placeholder revision for non-git trees (matches the MCP workspace).
pub const NO_REVISION: &str = "0000000000000000000000000000000000000000";

/// A working tree ready for index/query commands.
#[derive(Debug)]
pub struct Workspace {
    root: PathBuf,
    graph: RepositoryGraph,
    sources: Sources,
    git: Option<GitRepository>,
    manifest: Option<Manifest>,
    repository_name: String,
}

impl Workspace {
    /// Canonicalises `root` for path-based commands.
    pub fn canonical_root(root: impl AsRef<Path>) -> Result<PathBuf, CliError> {
        let root = root.as_ref();
        root.canonicalize()
            .map_err(|e| CliError::internal(format!("{}: {e}", root.display())))
    }

    /// Loads (and rebuilds) the graph from the working tree. Used by
    /// `index` / `adopt`.
    pub fn build(root: impl AsRef<Path>) -> Result<Self, CliError> {
        let root = Self::canonical_root(root)?;
        let manifest_path = manifest_path(&root);
        let manifest = if manifest_path.exists() {
            Some(Manifest::load(&manifest_path)?)
        } else {
            None
        };
        let repository_name = manifest
            .as_ref()
            .map(|m| m.repository.clone())
            .unwrap_or_else(|| {
                root.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "workspace".to_string())
            });

        let git = GitRepository::discover(&root).ok();
        let revision = match &git {
            Some(repo) => repo.current_revision()?,
            None => Revision::new(NO_REVISION),
        };

        let cargo = CargoIndex::load(&root)?;
        let sources = Sources::load_dir(&root)
            .map_err(|e| CliError::internal(format!("failed to load sources: {e}")))?;

        let mut builder = GraphBuilder::new(&repository_name, revision, cargo.clone());
        // Cross-repository registration: TPT-named path dependencies become
        // external links (spec.md section 11 / section 28 adopt step).
        for (package, repository) in discover_tpt_dependencies(&cargo) {
            builder = builder.link_cross_repository(package, repository);
        }
        let mut parsed_any = false;
        for package in cargo.workspace_packages() {
            let package_root = package
                .manifest_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| root.clone());
            let package_root = package_root.canonicalize().unwrap_or(package_root);
            for (relative, text) in collect_rust_files(&package_root, &root)? {
                let input = FileInput {
                    repository: &tpt_weave_core::RepositoryId::new(&repository_name),
                    package: &package.name,
                    path: &relative,
                    module_prefix: &[],
                };
                if let Ok(parsed) = parse_file(&input, &text) {
                    builder = builder.add_file(&package.name, parsed);
                    parsed_any = true;
                }
            }
        }
        if !parsed_any {
            return Err(CliError::internal(
                "no parseable Rust sources found under workspace packages",
            ));
        }
        let graph = builder.build();
        Ok(Self {
            root,
            graph,
            sources,
            git,
            manifest,
            repository_name,
        })
    }

    /// Loads the persisted index (`.tpt-weave/graph.json`) without
    /// rebuilding. Fails with exit 4 when missing or schema-mismatched.
    pub fn load_index(root: impl AsRef<Path>) -> Result<Self, CliError> {
        let root = Self::canonical_root(root)?;
        let manifest_path = manifest_path(&root);
        let manifest = if manifest_path.exists() {
            Some(Manifest::load(&manifest_path)?)
        } else {
            None
        };
        let path = graph_path(&root);
        if !path.exists() {
            return Err(CliError::stale(
                "no index found; run `tpt-weave index` (or `tpt-weave adopt`)",
            ));
        }
        let graph = RepositoryGraph::load(&path)?;
        // Conservative invalidation: HEAD moved since the graph was written.
        if let Ok(git) = GitRepository::discover(&root) {
            if let Ok(current) = git.current_revision() {
                if current.sha != graph.revision.sha {
                    return Err(CliError::stale(
                        "index is stale (HEAD moved); re-run `tpt-weave index`",
                    ));
                }
            }
        }
        let sources = Sources::load_dir(&root)
            .map_err(|e| CliError::internal(format!("failed to load sources: {e}")))?;
        let git = GitRepository::discover(&root).ok();
        let repository_name = manifest
            .as_ref()
            .map(|m| m.repository.clone())
            .unwrap_or_else(|| graph.repository.as_str().to_string());
        Ok(Self {
            root,
            graph,
            sources,
            git,
            manifest,
            repository_name,
        })
    }

    /// Builds the graph and writes `.tpt-weave/graph.json`.
    pub fn index_and_save(root: impl AsRef<Path>) -> Result<Self, CliError> {
        let workspace = Self::build(root)?;
        workspace.save_index()?;
        Ok(workspace)
    }

    /// Persists the in-memory graph to `.tpt-weave/graph.json`.
    pub fn save_index(&self) -> Result<PathBuf, CliError> {
        let path = graph_path(&self.root);
        self.graph.save(&path)?;
        Ok(path)
    }

    /// Repository work-tree root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The (possibly rebuilt) symbol graph.
    pub fn graph(&self) -> &RepositoryGraph {
        &self.graph
    }

    /// In-memory file sources.
    pub fn sources(&self) -> &Sources {
        &self.sources
    }

    /// Git handle when the tree is a repository.
    pub fn git(&self) -> Option<&GitRepository> {
        self.git.as_ref()
    }

    /// Loaded manifest, when `.tpt-weave/manifest.toml` exists.
    pub fn manifest(&self) -> Option<&Manifest> {
        self.manifest.as_ref()
    }

    /// Repository display name (manifest `repository` or directory name).
    pub fn repository_name(&self) -> &str {
        &self.repository_name
    }

    /// Paths changed relative to HEAD (empty outside git).
    pub fn changed_files(&self) -> Vec<String> {
        self.git
            .as_ref()
            .and_then(|git| git.changed_files().ok())
            .unwrap_or_default()
    }
}

/// Sorted `(package, repository)` pairs for `tpt-*` non-member dependencies.
pub fn discover_tpt_dependencies(cargo: &tpt_weave_index::CargoIndex) -> Vec<(String, String)> {
    let members: std::collections::BTreeSet<&str> =
        cargo.workspace_members.iter().map(String::as_str).collect();
    let mut links: std::collections::BTreeSet<(String, String)> = std::collections::BTreeSet::new();
    for package in cargo.workspace_packages() {
        for dep in &package.dependencies {
            if dep.name.starts_with("tpt-") && !members.contains(dep.name.as_str()) {
                links.insert((dep.name.clone(), dep.name.clone()));
            }
        }
    }
    links.into_iter().collect()
}

/// Collects `.rs` files under `package_root`, keyed by path relative to
/// `repo_root` (`/`-separated). Skips `target/`, `.git/`, `.tpt-weave/`.
fn collect_rust_files(
    package_root: &Path,
    repo_root: &Path,
) -> Result<Vec<(String, String)>, CliError> {
    let mut out = Vec::new();
    walk(package_root, repo_root, &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn walk(dir: &Path, repo_root: &Path, out: &mut Vec<(String, String)>) -> Result<(), CliError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| CliError::internal(format!("{}: {e}", dir.display())))?;
    for entry in entries {
        let entry = entry.map_err(|e| CliError::internal(format!("{}: {e}", dir.display())))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "target" || name == ".git" || name == ".tpt-weave" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            walk(&path, repo_root, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let relative = match path.strip_prefix(repo_root) {
                Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
                Err(_) => match path.canonicalize() {
                    Ok(canon) => match canon.strip_prefix(repo_root) {
                        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
                        Err(_) => path.to_string_lossy().replace('\\', "/"),
                    },
                    Err(_) => path.to_string_lossy().replace('\\', "/"),
                },
            };
            let text = std::fs::read_to_string(&path)
                .map_err(|e| CliError::internal(format!("{}: {e}", path.display())))?;
            out.push((relative, text));
        }
    }
    Ok(())
}
