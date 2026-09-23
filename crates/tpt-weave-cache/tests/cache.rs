//! Filesystem cache with conservative invalidation (todo.md Phase 7).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tpt_weave_cache::{CacheKey, CacheKind, FilesystemCache, cache_root};
use tpt_weave_core::{ContextLevel, MANIFEST_DIR, RepositoryId, Revision, SCHEMA_VERSION};

/// Sample cached value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Sample {
    symbols: Vec<String>,
    revision: String,
}

fn sample(rev: &str) -> Sample {
    Sample {
        symbols: vec!["make".into(), "Pixel".into()],
        revision: rev.into(),
    }
}

fn rev(sha: &str) -> Revision {
    Revision::new(sha)
}

/// A unique temp directory per test (process id + label).
fn temp_root(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tpt-weave-cache-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[test]
fn round_trips_a_repository_index_entry() {
    let root = temp_root("roundtrip");
    let mut cache = FilesystemCache::new(&root);
    let revision = rev("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let key = CacheKey::repository_index(&RepositoryId::new("demo"), &revision);

    // Miss before anything is stored.
    assert_eq!(cache.get::<Sample>(&key).expect("get"), None);

    cache.put(&key, &sample(&revision.sha)).expect("put");
    let hit = cache.get::<Sample>(&key).expect("get").expect("entry");
    assert_eq!(hit, sample(&revision.sha));

    let (hits, misses, stores) = cache.counters();
    assert_eq!((hits, misses, stores), (1, 1, 1));

    // Deterministic path: same key → same file.
    let again = CacheKey::repository_index(&RepositoryId::new("demo"), &revision);
    assert_eq!(key.digest(), again.digest());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn cache_keys_cover_schema_revision_query_policy_and_level() {
    let revision = rev("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let other = rev("cccccccccccccccccccccccccccccccccccccccc");

    // Spec.md section 15: revision, schema, query, policy, level all
    // participate in the key.
    let base =
        CacheKey::context_selection(&revision, "fix bug", "budget=12000", ContextLevel::Skeleton);
    let different_revision =
        CacheKey::context_selection(&other, "fix bug", "budget=12000", ContextLevel::Skeleton);
    let different_query = CacheKey::context_selection(
        &revision,
        "other task",
        "budget=12000",
        ContextLevel::Skeleton,
    );
    let different_policy =
        CacheKey::context_selection(&revision, "fix bug", "budget=4000", ContextLevel::Skeleton);
    let different_level =
        CacheKey::context_selection(&revision, "fix bug", "budget=12000", ContextLevel::Full);

    let mut digests: Vec<String> = vec![
        base.digest(),
        different_revision.digest(),
        different_query.digest(),
        different_policy.digest(),
        different_level.digest(),
    ];
    // Schema is baked into every canonical form.
    assert!(
        base.canonical()
            .contains(&format!("\"schema\":{SCHEMA_VERSION}"))
    );
    digests.sort();
    digests.dedup();
    assert_eq!(digests.len(), 5, "every dimension must change the digest");

    // Kind and query distinguish the other namespaces too.
    let index = CacheKey::repository_index(&RepositoryId::new("demo"), &revision);
    let lookup = CacheKey::symbol_lookup(&revision, "fix bug");
    let skeleton = CacheKey::skeleton(&revision, "src/lib.rs", "make", ContextLevel::Skeleton);
    assert_ne!(index.digest(), lookup.digest());
    assert_ne!(lookup.digest(), skeleton.digest());
    assert_ne!(
        CacheKey::skeleton(&revision, "src/lib.rs", "make", ContextLevel::Skeleton).digest(),
        CacheKey::skeleton(
            &revision,
            "src/lib.rs",
            "make",
            ContextLevel::Implementation
        )
        .digest()
    );
    assert_ne!(
        CacheKey::tool_reduction("cargo test", "output a").digest(),
        CacheKey::tool_reduction("cargo test", "output b").digest()
    );
    assert_ne!(
        CacheKey::tool_reduction("cargo test", "same").digest(),
        CacheKey::tool_reduction("cargo check", "same").digest()
    );
    let _ = base;
}

#[test]
fn revision_change_is_a_miss_without_touching_the_old_entry() {
    let root = temp_root("revision");
    let mut cache = FilesystemCache::new(&root);
    let old = CacheKey::symbol_lookup(&rev("dddddddddddddddddddddddddddddddddddddddd"), "make");
    cache.put(&old, &sample("old")).expect("put");

    let new = CacheKey::symbol_lookup(&rev("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"), "make");
    assert_eq!(cache.get::<Sample>(&new).expect("get"), None);
    // The old entry still exists (until invalidated) but is unreachable
    // under the new key.
    let (hits, misses, _) = cache.counters();
    assert_eq!((hits, misses), (0, 1));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn schema_drift_invalidates_on_read_and_sweep() {
    let root = temp_root("schema");
    let mut cache = FilesystemCache::new(&root);
    let revision = rev("ffffffffffffffffffffffffffffffffffffffff");
    let key = CacheKey::symbol_lookup(&revision, "make");
    cache.put(&key, &sample("ok")).expect("put");

    // Doctour the envelope's schema field, simulating an entry written by
    // an older build: the digest path still resolves, but the envelope
    // check must reject and delete it.
    let path = root
        .join(CacheKind::SymbolLookup.as_str())
        .join(format!("{}.json", key.digest()));
    let raw = fs::read_to_string(&path).expect("read");
    let doctored = raw.replace(
        &format!("\"schema\":{SCHEMA_VERSION}"),
        &format!("\"schema\":{}", SCHEMA_VERSION + 99),
    );
    assert_ne!(raw, doctored, "fixture must change the schema");
    fs::write(&path, doctored).expect("write");

    assert_eq!(cache.get::<Sample>(&key).expect("get"), None);
    assert!(!path.exists(), "stale schema entry is dropped");
    let (hits, misses, _) = cache.counters();
    assert_eq!((hits, misses), (0, 1));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn invalidate_sweeps_other_revisions_but_keeps_current_and_tool_entries() {
    let root = temp_root("invalidate");
    let mut cache = FilesystemCache::new(&root);
    let current = rev("1111111111111111111111111111111111111111");
    let stale = rev("2222222222222222222222222222222222222222");

    let keep = CacheKey::symbol_lookup(&current, "make");
    let drop = CacheKey::skeleton(&stale, "src/lib.rs", "make", ContextLevel::Skeleton);
    let tool = CacheKey::tool_reduction("cargo test", "raw output");
    cache.put(&keep, &sample("current")).expect("put");
    cache.put(&drop, &sample("stale")).expect("put");
    cache.put(&tool, &sample("tool")).expect("put");

    let removed = cache.invalidate(&current).expect("invalidate");
    assert_eq!(removed, 1);

    // Current-revision entry survives.
    assert_eq!(
        cache.get::<Sample>(&keep).expect("get").expect("kept"),
        sample("current")
    );
    // Stale-revision entry is gone.
    assert_eq!(cache.get::<Sample>(&drop).expect("get"), None);
    // Revision-independent tool reduction survives.
    assert_eq!(
        cache.get::<Sample>(&tool).expect("get").expect("kept"),
        sample("tool")
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn manual_clear_removes_every_entry() {
    let root = temp_root("clear");
    let mut cache = FilesystemCache::new(&root);
    let revision = rev("3333333333333333333333333333333333333333");
    cache
        .put(
            &CacheKey::repository_index(&RepositoryId::new("demo"), &revision),
            &sample("a"),
        )
        .expect("put");
    cache
        .put(&CacheKey::symbol_lookup(&revision, "make"), &sample("b"))
        .expect("put");
    cache
        .put(&CacheKey::tool_reduction("cargo test", "x"), &sample("c"))
        .expect("put");

    let cleared = cache.clear().expect("clear");
    assert_eq!(cleared, 3);

    let key = CacheKey::symbol_lookup(&revision, "make");
    assert_eq!(cache.get::<Sample>(&key).expect("get"), None);
    let stats = cache.statistics().expect("stats");
    assert_eq!(stats.entries, 0);

    // Clear is repeatable (idempotent on an empty cache).
    assert_eq!(cache.clear().expect("clear"), 0);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn reports_statistics_over_counters_and_disk() {
    let root = temp_root("stats");
    let mut cache = FilesystemCache::new(&root);
    let revision = rev("4444444444444444444444444444444444444444");

    let index = CacheKey::repository_index(&RepositoryId::new("demo"), &revision);
    let lookup = CacheKey::symbol_lookup(&revision, "make");
    cache.put(&index, &sample("index")).expect("put");
    cache.put(&lookup, &sample("lookup")).expect("put");

    assert_eq!(
        cache.get::<Sample>(&lookup).expect("get"),
        Some(sample("lookup"))
    );
    assert_eq!(
        cache
            .get::<Sample>(&CacheKey::symbol_lookup(&revision, "nope"))
            .expect("get"),
        None
    );

    let stats = cache.statistics().expect("stats");
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.stores, 2);
    assert_eq!(stats.hit_rate(), 0.5);
    assert_eq!(stats.entries, 2);
    assert!(stats.bytes > 0);
    assert_eq!(stats.entries_by_kind[&CacheKind::RepositoryIndex], 1);
    assert_eq!(stats.entries_by_kind[&CacheKind::SymbolLookup], 1);
    assert_eq!(
        stats.bytes_by_kind[&CacheKind::RepositoryIndex]
            + stats.bytes_by_kind[&CacheKind::SymbolLookup],
        stats.bytes
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn standard_location_is_inside_the_tpt_weave_directory() {
    let root = temp_root("location");
    let cache = FilesystemCache::for_repository(&root);
    assert_eq!(cache.root(), cache_root(&root));
    assert_eq!(
        cache.root(),
        root.join(MANIFEST_DIR).join("cache"),
        "spec.md layout: .tpt-weave/cache"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn corrupt_entries_count_as_misses_and_are_dropped() {
    let root = temp_root("corrupt");
    let mut cache = FilesystemCache::new(&root);
    let revision = rev("5555555555555555555555555555555555555555");
    let key = CacheKey::skeleton(&revision, "src/lib.rs", "make", ContextLevel::Skeleton);
    cache.put(&key, &sample("ok")).expect("put");

    let path = root
        .join(CacheKind::Skeleton.as_str())
        .join(format!("{}.json", key.digest()));
    fs::write(&path, "{ not json").expect("corrupt");
    assert_eq!(cache.get::<Sample>(&key).expect("get"), None);
    assert!(!path.exists(), "corrupt entry dropped");

    let _ = fs::remove_dir_all(&root);
}
