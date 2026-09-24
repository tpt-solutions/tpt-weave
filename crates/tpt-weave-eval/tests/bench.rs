//! Phase 15 benchmark primitive tests.

use tpt_weave_cache::{CacheKey, FilesystemCache};
use tpt_weave_decisions::{DecisionCategory, MockProvider};
use tpt_weave_eval::{benchmark_cache, benchmark_decision_provider};

#[test]
fn decision_benchmark_records_provider_and_rounds() {
    let provider = MockProvider::answering("relevant", 0.9);
    let request = DecisionCategory::Relevance.ask("src/lib.rs", "fix bug");
    let report = benchmark_decision_provider(&provider, &request, 3);
    assert_eq!(report.samples.len(), 1);
    assert_eq!(report.samples[0].rounds, 3);
    assert_eq!(report.samples[0].errors, 0);
    assert_eq!(report.samples[0].metadata["provider"], "mock");
}

#[test]
fn cache_benchmark_records_a_store_then_hits() {
    let path = std::env::temp_dir().join(format!(
        "tpt-weave-cache-benchmark-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut cache = FilesystemCache::new(&path);
    let key = CacheKey::tool_reduction("cargo test", "raw output");
    let report = benchmark_cache(&mut cache, &key, "reduced output", 3).unwrap();
    assert_eq!(report.samples[0].rounds, 3);
    assert_eq!(report.samples[0].errors, 0);
    assert_eq!(report.samples[0].metadata["stores"], "1");
    assert_eq!(report.samples[0].metadata["hits"], "2");
    let _ = std::fs::remove_dir_all(path);
}
