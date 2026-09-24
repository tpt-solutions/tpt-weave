//! Fingerprint-keyed parsed-file cache for incremental indexing.

use crate::parse::{ParseFileInput, ParsedFile, ParsedSource, parse_files};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tpt_weave_core::{MANIFEST_DIR, hash::fnv1a64_hex};

/// Current parsed-file cache schema.
pub const PARSE_CACHE_SCHEMA: u32 = 1;

/// Cache hit/miss/store counters for one indexing run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub stores: u64,
}

/// Fingerprint-keyed parsed files stored below `.tpt-weave/parsed/`.
#[derive(Debug)]
pub struct ParseCache {
    root: PathBuf,
    stats: ParseCacheStats,
    changed_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    schema: u32,
    fingerprint: String,
    package: String,
    file: ParsedFile,
}

impl ParseCache {
    /// Creates a cache rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            stats: ParseCacheStats::default(),
            changed_paths: Vec::new(),
        }
    }

    /// Creates the standard cache for a repository working tree.
    pub fn for_repository(repository_root: &Path) -> Self {
        Self::new(repository_root.join(MANIFEST_DIR).join("parsed"))
    }

    /// Cache directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Counters for the current run.
    pub fn stats(&self) -> ParseCacheStats {
        self.stats
    }

    /// Repository-relative paths whose contents were reparsed in this run.
    /// The list is sorted and deduplicated.
    pub fn changed_paths(&self) -> &[String] {
        &self.changed_paths
    }

    /// Reuses cached parse results and reparses only changed fingerprints.
    pub fn parse_files(&mut self, mut inputs: Vec<ParseFileInput>) -> Vec<ParsedSource> {
        inputs.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.package.cmp(&right.package))
        });
        let mut cached = Vec::with_capacity(inputs.len());
        let mut misses = Vec::new();
        let mut fingerprints = BTreeMap::new();
        self.stats = ParseCacheStats::default();
        self.changed_paths.clear();
        for input in inputs {
            let fingerprint = fingerprint(&input);
            if let Some(file) = self.read(&input, &fingerprint) {
                self.stats.hits += 1;
                cached.push(ParsedSource {
                    package: input.package,
                    file,
                });
            } else {
                self.stats.misses += 1;
                self.changed_paths.push(input.path.clone());
                fingerprints.insert((input.package.clone(), input.path.clone()), fingerprint);
                misses.push(input);
            }
        }
        let parsed = parse_files(misses);
        for source in &parsed {
            let fingerprint = fingerprints
                .get(&(source.package.clone(), source.file.path.clone()))
                .cloned()
                .unwrap_or_default();
            if self.write(source, fingerprint) {
                self.stats.stores += 1;
            }
        }
        cached.extend(parsed);
        self.changed_paths.sort();
        self.changed_paths.dedup();
        cached.sort_by(|left, right| {
            left.file
                .path
                .cmp(&right.file.path)
                .then_with(|| left.package.cmp(&right.package))
        });
        cached
    }

    fn read(&self, input: &ParseFileInput, fingerprint: &str) -> Option<ParsedFile> {
        let path = self.path_for(&input.package, &input.path, fingerprint);
        let bytes = fs::read(path).ok()?;
        let entry: CacheEntry = serde_json::from_slice(&bytes).ok()?;
        (entry.schema == PARSE_CACHE_SCHEMA
            && entry.fingerprint == fingerprint
            && entry.package == input.package
            && entry.file.path == input.path)
            .then_some(entry.file)
    }

    fn write(&self, source: &ParsedSource, fingerprint: String) -> bool {
        let path = self.path_for(&source.package, &source.file.path, &fingerprint);
        let Some(parent) = path.parent() else {
            return false;
        };
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
        let entry = CacheEntry {
            schema: PARSE_CACHE_SCHEMA,
            fingerprint,
            package: source.package.clone(),
            file: source.file.clone(),
        };
        serde_json::to_vec(&entry)
            .ok()
            .and_then(|bytes| fs::write(path, bytes).ok())
            .is_some()
    }

    fn path_for(&self, package: &str, path: &str, fingerprint: &str) -> PathBuf {
        let digest = fnv1a64_hex(format!("{package}\u{1f}{path}\u{1f}{fingerprint}").as_bytes());
        self.root.join(format!("{digest}.json"))
    }
}

fn fingerprint(input: &ParseFileInput) -> String {
    let material = format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}",
        input.repository.as_str(),
        input.module_prefix.join("\u{1f}"),
        input.path,
        input.source
    );
    fnv1a64_hex(material.as_bytes())
}
