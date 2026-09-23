//! File source access for representation rendering (todo.md Phase 4).

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

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
    /// skipping `target/`, `.git/` and `.tpt-weave/` directories.
    pub fn load_dir(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref();
        let mut sources = Self::new();
        collect(root, root, &mut sources.files)?;
        Ok(sources)
    }
}

fn collect(dir: &Path, root: &Path, out: &mut BTreeMap<String, String>) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "target" || name == ".git" || name == ".tpt-weave" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect(&path, root, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
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
