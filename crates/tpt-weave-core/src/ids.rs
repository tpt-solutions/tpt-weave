//! Stable identifiers for the repository model (spec.md section 26).

use crate::hash::fnv1a64_hex;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifier of a repository within the TPT ecosystem (e.g. `tpt-cv`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RepositoryId(String);

impl RepositoryId {
    /// Creates a repository id from a repository name.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Returns the repository name.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns `true` when the id is empty (i.e. unset).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for RepositoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for RepositoryId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RepositoryId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// A git revision: the commit sha plus the branch it was recorded on.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Revision {
    /// Full commit sha (hex).
    pub sha: String,
    /// Branch name at recording time, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

impl Revision {
    /// Creates a revision from a commit sha without branch information.
    pub fn new(sha: impl Into<String>) -> Self {
        Self {
            sha: sha.into(),
            branch: None,
        }
    }

    /// Attaches branch information.
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    /// Returns the first 12 characters of the sha (safe for non-hex input).
    pub fn short(&self) -> &str {
        let end = self
            .sha
            .char_indices()
            .nth(12)
            .map_or(self.sha.len(), |(idx, _)| idx);
        &self.sha[..end]
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.short())
    }
}

/// Kind of a source symbol, mirroring what the indexer extracts (spec.md §8.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Module,
    Struct,
    Enum,
    Trait,
    Impl,
    Function,
    Method,
    Constant,
    TypeAlias,
    Macro,
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            SymbolKind::Module => "module",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Trait => "trait",
            SymbolKind::Impl => "impl",
            SymbolKind::Function => "fn",
            SymbolKind::Method => "method",
            SymbolKind::Constant => "const",
            SymbolKind::TypeAlias => "type",
            SymbolKind::Macro => "macro",
        };
        f.write_str(name)
    }
}

/// Stable identifier of a symbol: repository + package + module path + name +
/// kind (spec.md section 26).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId {
    pub repository: RepositoryId,
    /// Cargo package (crate) name, e.g. `tpt-cv`.
    pub package: String,
    /// `::`-separated module path relative to the package root; empty for the
    /// crate root module.
    pub module: String,
    /// Symbol name; methods may be qualified as `Type::method`.
    pub name: String,
    pub kind: SymbolKind,
}

impl SymbolId {
    /// Canonical string form, e.g.
    /// `tpt-cv/tpt-cv::image::resample::resample#fn`.
    pub fn canonical_key(&self) -> String {
        let mut key = format!("{}/{}", self.repository, self.package);
        if !self.module.is_empty() {
            key.push_str("::");
            key.push_str(&self.module);
        }
        key.push_str("::");
        key.push_str(&self.name);
        key.push('#');
        key.push_str(&self.kind.to_string());
        key
    }
}

impl fmt::Display for SymbolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_key())
    }
}

/// Identifier of a context candidate or delivered context item.
///
/// Deterministic ids are derived with FNV-1a so the same
/// (source, level) tuple always maps to the same id across runs, which
/// cache keys (spec.md section 15) rely on.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContextId(String);

impl ContextId {
    /// Wraps an explicitly constructed id string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Derives a deterministic id from its parts, joined with the ASCII
    /// unit-separator character so parts cannot collide across boundaries.
    pub fn deterministic(parts: &[&str]) -> Self {
        let joined = parts.join("\u{1f}");
        Self(format!("ctx:{}", fnv1a64_hex(joined.as_bytes())))
    }

    /// Returns the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContextId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for ContextId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ContextId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_short_truncates_hex_and_is_boundary_safe() {
        let rev = Revision::new("0123456789abcdef0123456789abcdef01234567").with_branch("main");
        assert_eq!(rev.short(), "0123456789ab");
        assert_eq!(rev.branch.as_deref(), Some("main"));
        assert_eq!(rev.to_string(), "0123456789ab");

        // Garbage (multi-byte) input must not panic.
        let weird = Revision::new("αααααααααααααααα");
        assert!(weird.short().len() <= weird.sha.len());
    }

    #[test]
    fn symbol_key_is_stable() {
        let sym = SymbolId {
            repository: RepositoryId::new("tpt-cv"),
            package: "tpt-cv".into(),
            module: "image::resample".into(),
            name: "resample".into(),
            kind: SymbolKind::Function,
        };
        assert_eq!(
            sym.canonical_key(),
            "tpt-cv/tpt-cv::image::resample::resample#fn"
        );

        let root = SymbolId {
            repository: RepositoryId::new("tpt-cv"),
            package: "tpt-cv".into(),
            module: String::new(),
            name: "prelude".into(),
            kind: SymbolKind::Module,
        };
        assert_eq!(root.canonical_key(), "tpt-cv/tpt-cv::prelude#module");
    }

    #[test]
    fn context_ids_are_deterministic() {
        let a = ContextId::deterministic(&["tpt-cv", "file:src/lib.rs", "3"]);
        let b = ContextId::deterministic(&["tpt-cv", "file:src/lib.rs", "3"]);
        let c = ContextId::deterministic(&["tpt-cv", "file:src/lib.rs", "4"]);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.as_str().starts_with("ctx:"));
    }
}
