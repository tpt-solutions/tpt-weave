//! Loaded workspace state for MCP tools (todo.md Phase 9).
//!
//! Builds the deterministic graph + sources from a working tree, and
//! refreshes when the git revision changes (spec.md section 21:
//! conservative invalidation).

use std::path::{Path, PathBuf};
use tpt_weave_context::{LazySources, SourceProvider};
use tpt_weave_core::{
    Manifest, PrivacyConfig, RepositoryRegistry, Revision, discover_repository_root,
    hash::fnv1a64_hex, manifest_path,
};
use tpt_weave_graph::{GraphBuilder, RepositoryGraph, graph_path};
use tpt_weave_index::{CargoIndex, GitRepository};
use tpt_weave_rust::{ParseCache, ParseFileInput};

/// Placeholder revision for non-git trees.
pub const NO_REVISION: &str = "0000000000000000000000000000000000000000";

/// Failure to load or refresh a workspace.
#[derive(Debug)]
pub enum WorkspaceError {
    /// `cargo metadata` failed (not a Cargo project, cargo missing, ...).
    Index(String),
    /// Git discovery/revision failed and no fallback applied.
    Git(String),
    /// Filesystem walk or read failed.
    Io(String),
    /// No parseable Rust sources / packages found.
    Empty,
}

impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceError::Index(msg) => write!(f, "index error: {msg}"),
            WorkspaceError::Git(msg) => write!(f, "git error: {msg}"),
            WorkspaceError::Io(msg) => write!(f, "io error: {msg}"),
            WorkspaceError::Empty => write!(f, "no packages or sources found"),
        }
    }
}

impl std::error::Error for WorkspaceError {}

/// A loaded repository: graph + lazily-read sources + git handle.
#[derive(Debug)]
pub struct Workspace {
    root: PathBuf,
    graph: RepositoryGraph,
    sources: LazySources,
    git: Option<GitRepository>,
    repository_name: String,
    worktree_state: String,
    registry: Option<RepositoryRegistry>,
}

impl Workspace {
    /// Loads a working tree: cargo index, git revision, parsed sources,
    /// symbol graph.
    pub fn load(root: impl AsRef<Path>) -> Result<Self, WorkspaceError> {
        let root = discover_repository_root(root)
            .map_err(|e: tpt_weave_core::ConfigError| WorkspaceError::Io(e.to_string()))?;
        let repository_name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "workspace".to_string());
        let manifest_file = manifest_path(&root);
        let privacy = if manifest_file.exists() {
            Manifest::load(&manifest_file)
                .map_err(|error| WorkspaceError::Index(error.to_string()))?
                .privacy
        } else {
            PrivacyConfig::default()
        };
        let registry = RepositoryRegistry::load_standard(&root)
            .map_err(|error| WorkspaceError::Index(error.to_string()))?;

        let git = GitRepository::discover(&root).ok();
        let revision = match &git {
            Some(repo) => repo
                .current_revision()
                .map_err(|e| WorkspaceError::Git(e.to_string()))?,
            None => Revision::new(NO_REVISION),
        };
        let worktree_state = worktree_state(&root, git.as_ref())?;
        let cargo = CargoIndex::load(&root).map_err(|e| WorkspaceError::Index(e.to_string()))?;
        let previous_graph = RepositoryGraph::load(graph_path(&root)).ok();
        let sources = LazySources::open_with_privacy(&root, &privacy)
            .map_err(|e| WorkspaceError::Io(e.to_string()))?;

        let mut builder = GraphBuilder::new(&repository_name, revision, cargo.clone());
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
            // Canonicalize so `strip_prefix(root)` matches Windows `\\?\` forms.
            let package_root = package_root.canonicalize().unwrap_or(package_root);
            for (relative, text) in collect_rust_files(&package_root, &root, &privacy)
                .map_err(|e| WorkspaceError::Io(e.to_string()))?
            {
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
        let parsed_any = !parsed_files.is_empty();
        for parsed in parsed_files {
            builder = builder.add_file(parsed.package, parsed.file);
        }
        if !parsed_any {
            return Err(WorkspaceError::Empty);
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
            repository_name,
            worktree_state,
            registry,
        })
    }

    /// Repository work-tree root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The loaded symbol graph.
    pub fn graph(&self) -> &RepositoryGraph {
        &self.graph
    }

    /// Lazily-read repository file sources.
    pub fn sources(&self) -> &dyn SourceProvider {
        &self.sources
    }

    /// Optional standard repository registry loaded from the working tree.
    pub fn registry(&self) -> Option<&RepositoryRegistry> {
        self.registry.as_ref()
    }

    /// Git handle, when the tree is a git repository.
    pub fn git(&self) -> Option<&GitRepository> {
        self.git.as_ref()
    }

    /// Repository display name.
    pub fn repository_name(&self) -> &str {
        &self.repository_name
    }

    /// Current HEAD revision, if git is available.
    pub fn current_revision(&self) -> Result<Option<Revision>, WorkspaceError> {
        match &self.git {
            Some(repo) => repo
                .current_revision()
                .map(Some)
                .map_err(|e| WorkspaceError::Git(e.to_string())),
            None => Ok(None),
        }
    }

    /// Rebuilds the graph/sources when HEAD or the working-tree status changed
    /// since load. Returns `true` when rebuilt. The status fingerprint avoids
    /// rebuilding for every tool call while the tree remains dirty.
    pub fn refresh_if_stale(&mut self) -> Result<bool, WorkspaceError> {
        let Some(current) = self.current_revision()? else {
            return Ok(false);
        };
        let current_worktree = worktree_state(&self.root, self.git.as_ref())?;
        if current.sha == self.graph.revision.sha && current_worktree == self.worktree_state {
            return Ok(false);
        }
        let rebuilt = Self::load(&self.root)?;
        *self = rebuilt;
        Ok(true)
    }
}

fn worktree_state(root: &Path, git: Option<&GitRepository>) -> Result<String, WorkspaceError> {
    let git_state = git
        .map(|repo| {
            repo.worktree_fingerprint()
                .map_err(|error| WorkspaceError::Git(error.to_string()))
        })
        .transpose()?
        .unwrap_or_else(|| "non-git".to_string());
    let manifest = std::fs::read(manifest_path(root)).unwrap_or_default();
    let mut material = git_state.into_bytes();
    material.push(0xff);
    material.extend_from_slice(&manifest);
    Ok(fnv1a64_hex(&material))
}

/// Collects `.rs` files under `package_root`, keyed by path relative to
/// `repo_root` (`/`-separated). Skips `target/`, `.git/`, `.tpt-weave/`.
fn collect_rust_files(
    package_root: &Path,
    repo_root: &Path,
    privacy: &PrivacyConfig,
) -> std::io::Result<Vec<(String, String)>> {
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
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "target" || name == ".git" || name == ".tpt-weave" {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            walk(&path, repo_root, privacy, out)?;
        } else if file_type.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            let relative = match path.strip_prefix(repo_root) {
                Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
                Err(_) => {
                    // Windows: `path` may lack the `\\?\` prefix that
                    // `repo_root` has after canonicalize.
                    match path.canonicalize() {
                        Ok(canon) => match canon.strip_prefix(repo_root) {
                            Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
                            Err(_) => path.to_string_lossy().replace('\\', "/"),
                        },
                        Err(_) => path.to_string_lossy().replace('\\', "/"),
                    }
                }
            };
            if privacy.is_excluded(&relative)
                || privacy.is_private_path(&relative)
                || privacy.is_private_path(&path.to_string_lossy())
            {
                continue;
            }
            let text = std::fs::read_to_string(&path)?;
            out.push((relative, text));
        }
    }
    Ok(())
}
