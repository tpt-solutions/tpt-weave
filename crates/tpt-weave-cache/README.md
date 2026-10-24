# tpt-weave-cache

Filesystem cache with conservative invalidation for
[tpt-weave](https://github.com/tpt-solutions/tpt-weave). Stale source representations are never reused.

## How keys work

A `CacheKey` is derived from `CacheKeyData`: index schema version, repository revision, `CacheKind`, query,
representation level and context policy (spec section 15). A new revision, schema or policy simply hashes to
a different entry, so staleness is impossible by construction; `FilesystemCache::invalidate` sweeps the
leftovers. Uncommitted edits and manifest changes participate in the revision fingerprint.

`CacheKind` namespaces: `RepositoryIndex`, `SymbolLookup`, `Skeleton`, `ContextSelection`, `ToolReduction`.

## What it provides

- `FilesystemCache` - typed `get` / `put` (serde), `invalidate(&Revision)`, `clear`, `statistics`, plus hit and
  miss `counters`. Rooted under `.tpt-weave/cache` (`CACHE_DIR`, `cache_root`, `FilesystemCache::for_repository`).
- `CacheKey`, `CacheKeyData`, `CacheKind` - key construction (`CacheKey::at_revision`, `with_level`, ...).
- `CacheStatistics` - entries, bytes and counters, reported by `tpt-weave stats` and `tpt-weave cache`.
- `CacheError` - I/O and serialisation failures.

## Example

```rust,no_run
use tpt_weave_cache::{CacheKey, CacheKind, FilesystemCache};
use tpt_weave_core::Revision;
use std::path::Path;

let mut cache = FilesystemCache::for_repository(Path::new("."));
let revision = Revision::new("0123abcd");
let key = CacheKey::at_revision(CacheKind::Skeleton, &revision, "src/lib.rs");

if cache.get::<String>(&key)?.is_none() {
    cache.put(&key, &"rendered skeleton".to_string())?;
}
println!("{:?}", cache.statistics()?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Design notes

- `#![forbid(unsafe_code)]`.
- Correctness over hit rate: when in doubt the cache misses.

## Status

Part of the [tpt-weave](https://github.com/tpt-solutions/tpt-weave) workspace (pre-alpha, version 0.1.0,
MSRV Rust 1.85, edition 2024). The crate is not yet published to crates.io (`publish = false`; see
[docs/release-readiness.md](../../docs/release-readiness.md)). Public APIs are stable-by-intent unless noted;
experimental APIs are gated behind the `unstable` feature where present (see
[docs/decisions.md](../../docs/decisions.md) section 6). Changes are tracked in [CHANGELOG.md](CHANGELOG.md).

## Related documentation

- [Architecture](../../docs/architecture.md) - crate layering and data flow
- [Design specification](../../spec.md)
- [Using tpt-weave](../../docs/using-tpt-weave.md)

## License

Licensed under either of [Apache License, Version 2.0](../../LICENSE-APACHE) or [MIT license](../../LICENSE-MIT), at your option.
