//! Working-tree loading for CLI commands: cargo index, git revision,
//! parsed sources and the symbol graph (shared shape with the MCP
//! workspace, todo.md Phase 9 / Phase 10).

use std::path::{Path, PathBuf};
use tpt_weave_context::{LazySources, SourceProvider};
use tpt_weave_core::{
    Manifest, PrivacyConfig, RepositoryRegistry, Revision, discover_repository_root, manifest_path,
};
use tpt_weave_graph::{GraphBuilder, RepositoryGraph, graph_path};
use tpt_weave_index::{CargoIndex, GitRepository};
use tpt_weave_rust::{ParseCache, ParseCacheStats, ParseFileInput};

use crate::error::CliError;

/// Placeholder revision for non-git trees (matches the MCP workspace).
pub const NO_REVISION: &str = "0000000000000000000000000000000000000000";

/// A working tree ready for index/query commands.
#[derive(Debug)]
pub struct Workspace {
    root: PathBuf,
    graph: RepositoryGraph,
    sources: LazySources,
    git: Option<GitRepository>,
    manifest: Option<Manifest>,
    repository_name: String,
    parse_cache: ParseCacheStats,
    registry: Option<RepositoryRegistry>,
}

impl Workspace {
    /// Canonicalises `root` for path-based commands.
    pub fn canonical_root(root: impl AsRef<Path>) -> Result<PathBuf, CliError> {
        discover_repository_root(root).map_err(Into::into)
    }

    /// Returns `true` when the local manifest is newer than the persisted
    /// graph. Manifest changes can alter privacy exclusions even when Git
    /// reports a clean tree.
    pub fn manifest_newer_than_graph(root: &Path, graph_path: &Path) -> bool {
        let Ok(manifest) = manifest_path(root).metadata() else {
            return false;
        };
        let Ok(graph) = graph_path.metadata() else {
            return false;
        };
        manifest
            .modified()
            .ok()
            .zip(graph.modified().ok())
            .is_some_and(|(manifest_time, graph_time)| manifest_time > graph_time)
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

        let privacy = manifest
            .as_ref()
            .map(|m| m.privacy.clone())
            .unwrap_or_default();
        let registry = RepositoryRegistry::load_standard(&root)
            .map_err(|error| CliError::internal(error.to_string()))?;
        let cargo = CargoIndex::load(&root)?;
        let previous_graph = RepositoryGraph::load(graph_path(&root)).ok();
        let sources = LazySources::open_with_privacy(&root, &privacy)
            .map_err(|e| CliError::internal(format!("failed to load sources: {e}")))?;

        let mut builder = GraphBuilder::new(&repository_name, revision, cargo.clone());
        // Cross-repository registration: TPT-named path dependencies become
        // external links (spec.md section 11 / section 28 adopt step).
        for (package, repository) in cargo.discover_tpt_dependencies() {
            let repository = registry
                .as_ref()
                .and_then(|registry| registry.get(&package))
                .map(|entry| entry.name.clone())
                .unwrap_or(repository);
            builder = builder.link_cross_repository(package, repository);
        }
        let repository_id = tpt_weave_core::RepositoryId::new(&repository_name);
        let mut parse_inputs = Vec::new();
        for package in cargo.workspace_packages() {
            let package_root = package
                .manifest_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| root.clone());
            let package_root = package_root.canonicalize().unwrap_or(package_root);
            for (relative, text) in collect_rust_files(&package_root, &root, &privacy)? {
                parse_inputs.push(ParseFileInput {
                    repository: repository_id.clone(),
                    package: package.name.clone(),
                    path: relative,
                    module_prefix: Vec::new(),
                    source: text,
                });
            }
        }
        let mut parse_cache = ParseCache::for_repository(&root);
        let parsed_files = parse_cache.parse_files(parse_inputs);
        let parse_stats = parse_cache.stats();
        let parsed_any = !parsed_files.is_empty();
        for parsed in parsed_files {
            builder = builder.add_file(parsed.package, parsed.file);
        }
        if !parsed_any {
            return Err(CliError::internal(
                "no parseable Rust sources found under workspace packages",
            ));
        }
        let graph = match previous_graph {
            Some(previous) => builder.build_incremental(&previous, parse_cache.changed_paths()),
            None => builder.build(),
        };
        Ok(Self {
            root,
            graph,
            sources,
            git,
            manifest,
            repository_name,
            parse_cache: parse_stats,
            registry,
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
        if Self::manifest_newer_than_graph(&root, &path) {
            return Err(CliError::stale(
                "manifest changed since index; re-run `tpt-weave index`",
            ));
        }
        let graph = RepositoryGraph::load(&path)?;
        // Discover once and reuse: each `git` invocation is a process spawn
        // (expensive, particularly on Windows), so avoid discovering twice
        // just to get the same repository root a second time.
        let git = GitRepository::discover(&root).ok();
        // Conservative invalidation: HEAD moved since the graph was written.
        if let Some(git) = &git {
            if let Ok(current) = git.current_revision() {
                if current.sha != graph.revision.sha {
                    return Err(CliError::stale(
                        "index is stale (HEAD moved); re-run `tpt-weave index`",
                    ));
                }
            }
        }
        let privacy = manifest
            .as_ref()
            .map(|m| m.privacy.clone())
            .unwrap_or_default();
        let registry = RepositoryRegistry::load_standard(&root)
            .map_err(|error| CliError::internal(error.to_string()))?;
        let sources = LazySources::open_with_privacy(&root, &privacy)
            .map_err(|e| CliError::internal(format!("failed to load sources: {e}")))?;
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
            parse_cache: ParseCacheStats::default(),
            registry,
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

    /// File sources, loaded lazily from the repository root.
    pub fn sources(&self) -> &dyn SourceProvider {
        &self.sources
    }

    /// Parse-cache counters from the most recent graph build.
    pub fn parse_cache_stats(&self) -> ParseCacheStats {
        self.parse_cache
    }

    /// Git handle when the tree is a repository.
    pub fn git(&self) -> Option<&GitRepository> {
        self.git.as_ref()
    }

    /// Loaded manifest, when `.tpt-weave/manifest.toml` exists.
    pub fn manifest(&self) -> Option<&Manifest> {
        self.manifest.as_ref()
    }

    /// Optional standard repository registry loaded from the working tree.
    pub fn registry(&self) -> Option<&RepositoryRegistry> {
        self.registry.as_ref()
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

/// Compatibility wrapper for the shared [`CargoIndex`] discovery method.
pub fn discover_tpt_dependencies(cargo: &tpt_weave_index::CargoIndex) -> Vec<(String, String)> {
    cargo.discover_tpt_dependencies()
}

/// Collects `.rs` files under `package_root`, keyed by path relative to
/// `repo_root` (`/`-separated). Skips `target/`, `.git/`, `.tpt-weave/`.
fn collect_rust_files(
    package_root: &Path,
    repo_root: &Path,
    privacy: &PrivacyConfig,
) -> Result<Vec<(String, String)>, CliError> {
    let mut out = Vec::new();
    walk(package_root, repo_root, privacy, &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn walk(
    dir: &Path,
    repo_root: &Path,
    privacy: &PrivacyConfig,
    out: &mut Vec<(String, String)>,
) -> Result<(), CliError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| CliError::internal(format!("{}: {e}", dir.display())))?;
    for entry in entries {
        let entry = entry.map_err(|e| CliError::internal(format!("{}: {e}", dir.display())))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "target" || name == ".git" || name == ".tpt-weave" {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|e| CliError::internal(format!("{}: {e}", entry.path().display())))?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            walk(&path, repo_root, privacy, out)?;
        } else if file_type.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
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
            if privacy.is_excluded(&relative)
                || privacy.is_private_path(&relative)
                || privacy.is_private_path(&path.to_string_lossy())
            {
                continue;
            }
            let text = std::fs::read_to_string(&path)
                .map_err(|e| CliError::internal(format!("{}: {e}", path.display())))?;
            out.push((relative, text));
        }
    }
    Ok(())
}
