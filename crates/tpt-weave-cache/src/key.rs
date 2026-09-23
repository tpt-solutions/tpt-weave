//! Cache keys (todo.md Phase 7 "Design cache key", spec.md section 15:
//! keys include repository revision, index schema, query, context policy
//! and representation level).

use serde::{Deserialize, Serialize};
use std::fmt;
use tpt_weave_core::{hash::fnv1a64_hex, ContextLevel, RepositoryId, Revision, SCHEMA_VERSION};

/// Which cache namespace an entry belongs to (todo.md Phase 7: index,
/// symbol lookup, skeletons, context selections, tool-result reductions).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheKind {
    /// The repository index (stable context, spec.md section 15).
    RepositoryIndex,
    /// Cached symbol lookup results.
    SymbolLookup,
    /// Rendered source skeletons.
    Skeleton,
    /// Deterministic context selections (with the context policy).
    ContextSelection,
    /// Reduced tool outputs (keyed by command + output content).
    ToolReduction,
}

impl CacheKind {
    /// Directory name for this namespace.
    pub fn as_str(self) -> &'static str {
        match self {
            CacheKind::RepositoryIndex => "repository_index",
            CacheKind::SymbolLookup => "symbol_lookup",
            CacheKind::Skeleton => "skeleton",
            CacheKind::ContextSelection => "context_selection",
            CacheKind::ToolReduction => "tool_reduction",
        }
    }
}

impl fmt::Display for CacheKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The persisted parts of a cache key; its JSON form is the canonical
/// string the digest is taken over (deterministic field order).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheKeyData {
    /// Index schema the entry was written under (spec.md section 21:
    /// schema invalidation).
    pub schema: u32,
    /// Repository revision the entry belongs to; `None` marks
    /// revision-independent entries (tool reductions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// Cache namespace.
    pub kind: CacheKind,
    /// Canonical query (symbol key, file path, task, command + output
    /// digest, ...).
    pub query: String,
    /// Representation level, where the entry is level-dependent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<ContextLevel>,
    /// Context policy fingerprint (budget, max level, dependency
    /// inclusion, ...); empty when not applicable.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub policy: String,
}

/// A deterministic cache key covering schema, revision, query, level and
/// policy (spec.md section 15).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheKey {
    data: CacheKeyData,
}

impl CacheKey {
    /// Builds a key directly from its parts.
    pub fn new(
        kind: CacheKind,
        revision: Option<&Revision>,
        query: impl Into<String>,
    ) -> Self {
        Self {
            data: CacheKeyData {
                schema: SCHEMA_VERSION,
                revision: revision.map(|r| r.sha.clone()),
                kind,
                query: query.into(),
                level: None,
                policy: String::new(),
            },
        }
    }

    /// Builds a revision-scoped key (the common case).
    pub fn at_revision(kind: CacheKind, revision: &Revision, query: impl Into<String>) -> Self {
        Self::new(kind, Some(revision), query)
    }

    /// Attaches the representation level.
    pub fn with_level(mut self, level: ContextLevel) -> Self {
        self.data.level = Some(level);
        self
    }

    /// Attaches the context policy fingerprint.
    pub fn with_policy(mut self, policy: impl Into<String>) -> Self {
        self.data.policy = policy.into();
        self
    }

    /// Repository index key (todo.md Phase 7 "Cache repository index").
    pub fn repository_index(repository: &RepositoryId, revision: &Revision) -> Self {
        Self::at_revision(CacheKind::RepositoryIndex, revision, format!("repo:{repository}"))
    }

    /// Symbol lookup key (todo.md Phase 7 "Cache symbol lookup").
    pub fn symbol_lookup(revision: &Revision, query: &str) -> Self {
        Self::at_revision(CacheKind::SymbolLookup, revision, query)
    }

    /// Skeleton key for a file at a level, with a fingerprint of the
    /// kept-selection (todo.md Phase 7 "Cache skeletons").
    pub fn skeleton(revision: &Revision, path: &str, selection: &str, level: ContextLevel) -> Self {
        Self::at_revision(CacheKind::Skeleton, revision, format!("file:{path}\u{1f}{selection}"))
            .with_level(level)
    }

    /// Context selection key: task + policy + level
    /// (todo.md Phase 7 "Cache context selections").
    pub fn context_selection(
        revision: &Revision,
        task: &str,
        policy: &str,
        level: ContextLevel,
    ) -> Self {
        Self::at_revision(CacheKind::ContextSelection, revision, task)
            .with_policy(policy)
            .with_level(level)
    }

    /// Tool reduction key: command + content digest of the raw output.
    /// Revision-independent — identical output reduces identically
    /// (todo.md Phase 7 "Cache tool-result reductions").
    pub fn tool_reduction(command: &str, raw_output: &str) -> Self {
        let digest = fnv1a64_hex(raw_output.as_bytes());
        Self::new(CacheKind::ToolReduction, None, format!("{command}\u{1f}{digest}"))
    }

    /// The canonical JSON form the digest is taken over.
    pub fn canonical(&self) -> String {
        // Struct field order is fixed by definition, so this is stable.
        serde_json::to_string(&self.data).expect("cache key serialises")
    }

    /// The 16-hex-character digest of [`CacheKey::canonical`], used as the
    /// cache file name.
    pub fn digest(&self) -> String {
        fnv1a64_hex(self.canonical().as_bytes())
    }

    /// The key's parts (for envelope verification).
    pub fn data(&self) -> &CacheKeyData {
        &self.data
    }
}
