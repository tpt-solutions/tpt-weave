//! Fingerprint-keyed parse cache tests.

use tpt_weave_core::RepositoryId;
use tpt_weave_rust::{ParseCache, ParseFileInput};

#[test]
fn reuses_unchanged_files_and_reparses_changed_source() {
    let root = std::env::temp_dir().join(format!(
        "tpt-weave-parse-cache-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let source = "pub fn original() {}\n";
    let input = ParseFileInput {
        repository: RepositoryId::new("cache-test"),
        package: "demo".to_string(),
        path: "src/lib.rs".to_string(),
        module_prefix: Vec::new(),
        source: source.to_string(),
    };

    let mut first = ParseCache::for_repository(&root);
    let initial = first.parse_files(vec![input.clone()]);
    assert_eq!(first.stats().misses, 1);
    assert_eq!(first.stats().stores, 1);
    assert_eq!(initial.len(), 1);

    let mut second = ParseCache::for_repository(&root);
    let reused = second.parse_files(vec![input.clone()]);
    assert_eq!(second.stats().hits, 1);
    assert_eq!(second.stats().misses, 0);
    assert_eq!(reused, initial);

    let mut changed = input;
    changed.source = "pub fn changed() {}\n".to_string();
    let mut third = ParseCache::for_repository(&root);
    let reparsed = third.parse_files(vec![changed]);
    assert_eq!(third.stats().hits, 0);
    assert_eq!(third.stats().misses, 1);
    assert_eq!(reparsed.len(), 1);

    let _ = std::fs::remove_dir_all(root);
}
