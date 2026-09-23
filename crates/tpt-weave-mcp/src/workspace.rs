//! Loaded workspace state for MCP tools (todo.md Phase 9).
//!
//! Builds the deterministic graph + sources from a working tree, and
//! refreshes when the git revision changes (spec.md section 21:
//! conservative invalidation).

use std::path::{Path, PathBuf};
use tpt_weave_context::Sources;
use tpt_weave_core::Revision;
use tpt_weave_graph::{GraphBuilder, RepositoryGraph};
use tpt_weave_index::{CargoIndex, GitRepository};
use tpt_weave_rust::{FileInput, parse_file};

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

/// A loaded repository: graph + in-memory sources + git handle.
#[derive(Debug)]
pub struct Workspace {
    root: PathBuf,
    graph: RepositoryGraph,
    sources: Sources,
    git: Option<GitRepository>,
    repository_name: String,
}

impl Workspace {
    /// Loads a working tree: cargo index, git revision, parsed sources,
    /// symbol graph.
    pub fn load(root: impl AsRef<Path>) -> Result<Self, WorkspaceError> {
        let root = root
            .as_ref()
            .canonicalize()
            .map_err(|e| WorkspaceError::Io(format!("{}: {e}", root.as_ref().display())))?;
        let repository_name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "workspace".to_string());

        let git = GitRepository::discover(&root).ok();
        let revision = match &git {
            Some(repo) => repo
                .current_revision()
                .map_err(|e| WorkspaceError::Git(e.to_string()))?,
            None => Revision::new(NO_REVISION),
        };

        let cargo = CargoIndex::load(&root).map_err(|e| WorkspaceError::Index(e.to_string()))?;
        let sources = Sources::load_dir(&root).map_err(|e| WorkspaceError::Io(e.to_string()))?;

        let mut builder = GraphBuilder::new(&repository_name, revision, cargo.clone());
        let mut parsed_any = false;
        for package in cargo.workspace_packages() {
            let package_root = package
                .manifest_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| root.clone());
            // Canonicalize so `strip_prefix(root)` matches Windows `\\?\` forms.
            let package_root = package_root.canonicalize().unwrap_or(package_root);
            for (relative, text) in collect_rust_files(&package_root, &root)
                .map_err(|e| WorkspaceError::Io(e.to_string()))?
            {
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
            return Err(WorkspaceError::Empty);
        }
        let graph = builder.build();
        Ok(Self {
            root,
            graph,
            sources,
            git,
            repository_name,
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

    /// In-memory file sources.
    pub fn sources(&self) -> &Sources {
        &self.sources
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

    /// Rebuilds the graph/sources when HEAD moved since load
    /// (conservative revision invalidation). Returns `true` when rebuilt.
    pub fn refresh_if_stale(&mut self) -> Result<bool, WorkspaceError> {
        let Some(current) = self.current_revision()? else {
            return Ok(false);
        };
        if current.sha == self.graph.revision.sha {
            return Ok(false);
        }
        let rebuilt = Self::load(&self.root)?;
        *self = rebuilt;
        Ok(true)
    }
}

/// Collects `.rs` files under `package_root`, keyed by path relative to
/// `repo_root` (`/`-separated). Skips `target/`, `.git/`, `.tpt-weave/`.
fn collect_rust_files(
    package_root: &Path,
    repo_root: &Path,
) -> std::io::Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    walk(package_root, repo_root, &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn walk(dir: &Path, repo_root: &Path, out: &mut Vec<(String, String)>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
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
            let text = std::fs::read_to_string(&path)?;
            out.push((relative, text));
        }
    }
    Ok(())
}
