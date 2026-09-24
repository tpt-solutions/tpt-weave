//! File source access for representation rendering (todo.md Phase 4).

use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use tpt_weave_core::PrivacyConfig;

/// Provides the raw source text of repository files.
pub trait SourceProvider {
    /// Returns the file's source for a repository-relative, `/`-separated
    /// path, if it is available.
    fn source(&self, path: &str) -> Option<&str>;
}

/// An in-memory set of file sources keyed by repository-relative path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sources {
    files: BTreeMap<String, String>,
}

impl Sources {
    /// An empty source set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts one file's source (chainable).
    pub fn with(mut self, path: impl Into<String>, source: impl Into<String>) -> Self {
        self.files.insert(path.into(), source.into());
        self
    }

    /// Inserts one file's source in place.
    pub fn insert(&mut self, path: impl Into<String>, source: impl Into<String>) {
        self.files.insert(path.into(), source.into());
    }

    /// Loads every `*.rs` file under `root` as a repository-relative path,
    /// skipping `target/`, `.git/`, `.tpt-weave/`, and privacy-excluded paths.
    pub fn load_dir(root: impl AsRef<Path>) -> io::Result<Self> {
        Self::load_dir_with_privacy(root, &PrivacyConfig::default())
    }

    /// Loads every `*.rs` file under `root`, applying `privacy` before any
    /// file is read.
    pub fn load_dir_with_privacy(
        root: impl AsRef<Path>,
        privacy: &PrivacyConfig,
    ) -> io::Result<Self> {
        let root = root.as_ref();
        let mut sources = Self::new();
        collect(root, root, privacy, &mut sources.files)?;
        Ok(sources)
    }
}

fn collect(
    dir: &Path,
    root: &Path,
    privacy: &PrivacyConfig,
    out: &mut BTreeMap<String, String>,
) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "target" || name == ".git" || name == ".tpt-weave" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect(&path, root, privacy, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if privacy.is_excluded(&relative) || privacy.is_private_path(&path.to_string_lossy()) {
                continue;
            }
            out.insert(relative, std::fs::read_to_string(&path)?);
        }
    }
    Ok(())
}

impl SourceProvider for Sources {
    fn source(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(String::as_str)
    }
}

impl SourceProvider for BTreeMap<String, String> {
    fn source(&self, path: &str) -> Option<&str> {
        self.get(path).map(String::as_str)
    }
}

/// File source access that discovers `*.rs` paths under a root cheaply (a
/// directory walk with no reads) and defers reading each file's contents
/// until it is actually requested, caching the result thereafter.
///
/// `Sources::load_dir` reads every file up front, which is wasted work for
/// commands (e.g. `symbol`) that only need the graph and never call
/// [`SourceProvider::source`]. `LazySources` keeps the same directory-walk
/// exclusions (`target/`, `.git/`, `.tpt-weave/`) but pays the read cost only
/// for files a caller actually touches.
#[derive(Debug, Default)]
pub struct LazySources {
    root: PathBuf,
    files: BTreeMap<String, OnceCell<Option<String>>>,
}

impl LazySources {
    /// Discovers every `*.rs` file under `root` (paths only, no reads),
    /// skipping `target/`, `.git/`, `.tpt-weave/`, and privacy-excluded paths.
    pub fn open(root: impl AsRef<Path>) -> io::Result<Self> {
        Self::open_with_privacy(root, &PrivacyConfig::default())
    }

    /// Discovers Rust paths without reading them, applying `privacy` to each
    /// repository-relative path.
    pub fn open_with_privacy(root: impl AsRef<Path>, privacy: &PrivacyConfig) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        let mut files = BTreeMap::new();
        collect_paths(&root, &root, privacy, &mut files)?;
        Ok(Self { root, files })
    }
}

fn collect_paths(
    dir: &Path,
    root: &Path,
    privacy: &PrivacyConfig,
    out: &mut BTreeMap<String, OnceCell<Option<String>>>,
) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "target" || name == ".git" || name == ".tpt-weave" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_paths(&path, root, privacy, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if privacy.is_excluded(&relative) || privacy.is_private_path(&path.to_string_lossy()) {
                continue;
            }
            out.insert(relative, OnceCell::new());
        }
    }
    Ok(())
}

impl SourceProvider for LazySources {
    fn source(&self, path: &str) -> Option<&str> {
        let cell = self.files.get(path)?;
        let content = cell.get_or_init(|| std::fs::read_to_string(self.root.join(path)).ok());
        content.as_deref()
    }
}
