//! Filesystem cache with conservative invalidation (todo.md Phase 7,
//! spec.md sections 15 and 21).
//!
//! Cache keys cover the repository revision, index schema, query, context
//! policy and representation level (spec.md section 15), so stale source
//! representations are never reused after a relevant change: a new
//! revision or schema simply hashes to a different entry, and
//! [`FilesystemCache::invalidate`] sweeps the leftovers.

#![forbid(unsafe_code)]

mod key;
mod stats;
mod store;

pub use key::{CacheKey, CacheKeyData, CacheKind};
pub use stats::CacheStatistics;
pub use store::{cache_root, CacheError, FilesystemCache, CACHE_DIR};
