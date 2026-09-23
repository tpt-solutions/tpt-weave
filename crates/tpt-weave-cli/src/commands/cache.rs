//! `tpt-weave cache [status|clear]` — cache management.

use super::Rendered;
use crate::args::{CacheAction, Cli};
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use tpt_weave_cache::FilesystemCache;

pub fn run(cli: &Cli, action: CacheAction) -> Result<Rendered, CliError> {
    let root = Workspace::canonical_root(&cli.path)?;
    let mut cache = FilesystemCache::for_repository(&root);

    match action {
        CacheAction::Status => {
            let stats = cache.statistics()?;
            let human = format!(
                "cache: {}\n  entries: {}\n  bytes: {}\n  hit_rate: {:.3}\n  hits: {}\n  misses: {}\n  stores: {}\n",
                cache.root().display(),
                stats.entries,
                stats.bytes,
                stats.hit_rate(),
                stats.hits,
                stats.misses,
                stats.stores,
            );
            let mut kinds = serde_json::Map::new();
            for (kind, count) in &stats.entries_by_kind {
                kinds.insert(kind.as_str().to_string(), json!(count));
            }
            let json = json!({
                "action": "status",
                "root": cache.root().display().to_string(),
                "entries": stats.entries,
                "bytes": stats.bytes,
                "hit_rate": stats.hit_rate(),
                "hits": stats.hits,
                "misses": stats.misses,
                "stores": stats.stores,
                "entries_by_kind": kinds,
                "bytes_by_kind": stats
                    .bytes_by_kind
                    .iter()
                    .map(|(k, v)| (k.as_str().to_string(), json!(v)))
                    .collect::<serde_json::Map<_, _>>(),
            });
            Ok(Rendered::new(human.trim_end(), json))
        }
        CacheAction::Clear => {
            let removed = cache.clear()?;
            let human = format!("cleared {removed} cache entr{}", if removed == 1 { "y" } else { "ies" });
            Ok(Rendered::new(
                human.clone(),
                json!({
                    "action": "clear",
                    "removed": removed,
                    "root": cache.root().display().to_string(),
                }),
            ))
        }
    }
}
