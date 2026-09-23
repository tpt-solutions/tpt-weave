//! Cache statistics (todo.md Phase 7 "Add cache statistics").

use crate::key::CacheKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Runtime counters plus on-disk inventory (todo.md Phase 7).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CacheStatistics {
    /// Successful `get` calls in this process.
    pub hits: u64,
    /// Missed `get` calls in this process.
    pub misses: u64,
    /// `put` calls in this process.
    pub stores: u64,
    /// Entry files currently on disk.
    pub entries: u64,
    /// Total bytes of entry files currently on disk.
    pub bytes: u64,
    /// Entries per namespace.
    pub entries_by_kind: BTreeMap<CacheKind, u64>,
    /// Bytes per namespace.
    pub bytes_by_kind: BTreeMap<CacheKind, u64>,
}

impl CacheStatistics {
    /// `hits / (hits + misses)` (0.0 when nothing has been requested).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}
