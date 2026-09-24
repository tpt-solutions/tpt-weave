//! Phase 15 benchmark primitives for decisions and filesystem cache paths.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Instant;
use tpt_weave_cache::{CacheError, CacheKey, FilesystemCache};
use tpt_weave_decisions::{DecisionProvider, DecisionRequest};

/// Current benchmark report schema.
pub const BENCHMARK_REPORT_SCHEMA: u32 = 1;

/// One measured operation and its timing distribution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkSample {
    pub name: String,
    pub rounds: u32,
    pub total_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
    pub errors: u64,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl BenchmarkSample {
    fn from_durations(name: impl Into<String>, durations: &[f64], errors: u64) -> Self {
        let total_ms = durations.iter().sum::<f64>();
        let rounds = durations.len() as u32;
        let min_ms = durations.iter().copied().fold(f64::INFINITY, f64::min);
        let max_ms = durations.iter().copied().fold(0.0, f64::max);
        Self {
            name: name.into(),
            rounds,
            total_ms,
            min_ms: if durations.is_empty() { 0.0 } else { min_ms },
            max_ms,
            mean_ms: if rounds == 0 {
                0.0
            } else {
                total_ms / f64::from(rounds)
            },
            errors,
            metadata: BTreeMap::new(),
        }
    }

    /// Adds string-valued benchmark metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// A serialisable collection of benchmark samples.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub schema: u32,
    pub samples: Vec<BenchmarkSample>,
}

impl BenchmarkReport {
    /// Creates an empty report.
    pub fn new() -> Self {
        Self {
            schema: BENCHMARK_REPORT_SCHEMA,
            samples: Vec::new(),
        }
    }

    /// Appends a sample.
    pub fn push(&mut self, sample: BenchmarkSample) {
        self.samples.push(sample);
    }

    /// Pretty JSON representation.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

impl Default for BenchmarkReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Benchmarks a decision provider without changing its policy or fallback
/// behavior. Provider errors are counted rather than hidden.
pub fn benchmark_decision_provider(
    provider: &dyn DecisionProvider,
    request: &DecisionRequest,
    rounds: u32,
) -> BenchmarkReport {
    let mut durations = Vec::with_capacity(rounds as usize);
    let mut errors = 0u64;
    let mut input_tokens = 0u64;
    for _ in 0..rounds {
        let started = Instant::now();
        match provider.decide(request) {
            Ok(outcome) => input_tokens += u64::from(outcome.input_tokens),
            Err(_) => errors += 1,
        }
        durations.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    let mut report = BenchmarkReport::new();
    report.push(
        BenchmarkSample::from_durations("jev_decision", &durations, errors)
            .with_metadata("provider", provider.name().to_string())
            .with_metadata("input_tokens", input_tokens.to_string()),
    );
    report
}

/// Benchirms a cache miss/store followed by cache hits.
pub fn benchmark_cache(
    cache: &mut FilesystemCache,
    key: &CacheKey,
    value: &str,
    rounds: u32,
) -> Result<BenchmarkReport, CacheError> {
    let rounds = rounds.max(1);
    let mut durations = Vec::with_capacity(rounds as usize);
    let mut errors = 0u64;
    for round in 0..rounds {
        let started = Instant::now();
        let result = if round == 0 {
            let miss = cache.get::<String>(key)?;
            if miss.is_none() {
                cache.put(key, &value.to_string())?;
            }
            Ok::<(), CacheError>(())
        } else {
            cache.get::<String>(key).map(|_| ())
        };
        if result.is_err() {
            errors += 1;
        }
        durations.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    let (hits, misses, stores) = cache.counters();
    let mut report = BenchmarkReport::new();
    report.push(
        BenchmarkSample::from_durations("filesystem_cache", &durations, errors)
            .with_metadata("hits", hits.to_string())
            .with_metadata("misses", misses.to_string())
            .with_metadata("stores", stores.to_string()),
    );
    Ok(report)
}
