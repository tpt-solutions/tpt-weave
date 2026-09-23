//! Filesystem cache store (todo.md Phase 7 "Implement filesystem cache",
//! spec.md section 21: conservative invalidation).

use crate::key::{CacheKey, CacheKeyData, CacheKind};
use crate::stats::CacheStatistics;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tpt_weave_core::{MANIFEST_DIR, Revision, SCHEMA_VERSION};

/// Subdirectory of `.tpt-weave/` holding cache entries.
pub const CACHE_DIR: &str = "cache";

/// The on-disk envelope: the full key plus the stored value, so entries
/// can be verified, swept and inventoried without knowing `T`.
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry<T> {
    key: CacheKeyData,
    value: T,
}

/// Errors from cache operations.
#[derive(Debug)]
pub enum CacheError {
    /// Filesystem failure.
    Io(io::Error),
    /// An entry failed to (de)serialise.
    Json(serde_json::Error),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::Io(err) => write!(f, "cache i/o error: {err}"),
            CacheError::Json(err) => write!(f, "cache entry error: {err}"),
        }
    }
}

impl std::error::Error for CacheError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CacheError::Io(err) => Some(err),
            CacheError::Json(err) => Some(err),
        }
    }
}

impl From<io::Error> for CacheError {
    fn from(err: io::Error) -> Self {
        CacheError::Io(err)
    }
}

impl From<serde_json::Error> for CacheError {
    fn from(err: serde_json::Error) -> Self {
        CacheError::Json(err)
    }
}

/// Path of the cache directory for a repository working tree.
pub fn cache_root(repo_root: &Path) -> PathBuf {
    repo_root.join(MANIFEST_DIR).join(CACHE_DIR)
}

/// A filesystem cache with hit/miss statistics and conservative
/// invalidation (revision sweep, schema mismatch, manual clear).
pub struct FilesystemCache {
    root: PathBuf,
    hits: u64,
    misses: u64,
    stores: u64,
}

impl FilesystemCache {
    /// Creates a cache rooted at `root` (use [`cache_root`] for the
    /// standard `.tpt-weave/cache` location).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            hits: 0,
            misses: 0,
            stores: 0,
        }
    }

    /// Creates the standard cache for a repository working tree.
    pub fn for_repository(repo_root: &Path) -> Self {
        Self::new(cache_root(repo_root))
    }

    /// The directory entries live in.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `(hits, misses, stores)` for this process.
    pub fn counters(&self) -> (u64, u64, u64) {
        (self.hits, self.misses, self.stores)
    }

    /// Looks up `key`. A missing file, an unreadable envelope, a schema
    /// mismatch or a key mismatch all count as a miss; corrupt or stale
    /// files are removed so the next `put` starts clean (spec.md section
    /// 21: never reuse stale data).
    pub fn get<T: DeserializeOwned>(&mut self, key: &CacheKey) -> Result<Option<T>, CacheError> {
        let path = self.path_for(key);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                self.misses += 1;
                return Ok(None);
            }
            Err(err) => return Err(err.into()),
        };
        match serde_json::from_slice::<CacheEntry<T>>(&bytes) {
            Ok(entry) if entry.key == *key.data() => {
                self.hits += 1;
                Ok(Some(entry.value))
            }
            _ => {
                // Schema drift, digest collision or tampering: drop it.
                let _ = fs::remove_file(&path);
                self.misses += 1;
                Ok(None)
            }
        }
    }

    /// Stores `value` under `key`, creating parent directories.
    pub fn put<T: Serialize>(&mut self, key: &CacheKey, value: &T) -> Result<(), CacheError> {
        let entry = CacheEntry {
            key: key.data().clone(),
            value,
        };
        let bytes = serde_json::to_vec(&entry)?;
        let path = self.path_for(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)?;
        self.stores += 1;
        Ok(())
    }

    /// Removes every revision-scoped entry that does not belong to
    /// `revision`, plus anything with a foreign schema or a corrupt
    /// envelope. Revision-independent entries (tool reductions) and
    /// entries already at `revision` are kept. Returns the number of
    /// files removed (todo.md Phase 7 "revision invalidation").
    pub fn invalidate(&mut self, revision: &Revision) -> Result<u64, CacheError> {
        let mut removed = 0u64;
        for path in self.entry_files()? {
            match fs::read(&path) {
                Ok(bytes) => {
                    let keep = serde_json::from_slice::<CacheEntry<serde_json::Value>>(&bytes)
                        .map(|entry| {
                            entry.key.schema == SCHEMA_VERSION
                                && (entry.key.revision.is_none()
                                    || entry.key.revision.as_deref() == Some(revision.sha.as_str()))
                        })
                        .unwrap_or(false);
                    if !keep && fs::remove_file(&path).is_ok() {
                        removed += 1;
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                Err(_) => {
                    if fs::remove_file(&path).is_ok() {
                        removed += 1;
                    }
                }
            }
        }
        Ok(removed)
    }

    /// Removes every entry (todo.md Phase 7 "manual cache clear").
    /// Returns the number of files removed.
    pub fn clear(&mut self) -> Result<u64, CacheError> {
        let mut removed = 0u64;
        for path in self.entry_files()? {
            if fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// Runtime counters plus an on-disk inventory
    /// (todo.md Phase 7 "cache statistics").
    pub fn statistics(&self) -> Result<CacheStatistics, CacheError> {
        let mut stats = CacheStatistics {
            hits: self.hits,
            misses: self.misses,
            stores: self.stores,
            ..CacheStatistics::default()
        };
        for (kind, path) in self.typed_files()? {
            let bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            stats.entries += 1;
            stats.bytes += bytes;
            *stats.entries_by_kind.entry(kind).or_insert(0) += 1;
            *stats.bytes_by_kind.entry(kind).or_insert(0) += bytes;
        }
        Ok(stats)
    }

    fn path_for(&self, key: &CacheKey) -> PathBuf {
        self.root
            .join(key.data().kind.as_str())
            .join(format!("{}.json", key.digest()))
    }

    /// Every entry file under any kind directory.
    fn entry_files(&self) -> io::Result<Vec<PathBuf>> {
        Ok(self
            .typed_files()?
            .into_iter()
            .map(|(_, path)| path)
            .collect())
    }

    /// Every entry file paired with its namespace kind.
    fn typed_files(&self) -> io::Result<Vec<(CacheKind, PathBuf)>> {
        const KINDS: &[CacheKind] = &[
            CacheKind::RepositoryIndex,
            CacheKind::SymbolLookup,
            CacheKind::Skeleton,
            CacheKind::ContextSelection,
            CacheKind::ToolReduction,
        ];
        let mut files = Vec::new();
        for kind in KINDS {
            let dir = self.root.join(kind.as_str());
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    files.push((*kind, path));
                }
            }
        }
        files.sort_by(|a, b| a.1.cmp(&b.1));
        Ok(files)
    }
}
