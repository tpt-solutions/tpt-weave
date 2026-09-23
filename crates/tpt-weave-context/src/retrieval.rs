//! Deterministic context retrieval (todo.md Phase 5, spec.md sections 10
//! and 16): lookups, relevance scoring, ordering, deduplication and
//! budgeting.

use crate::ContextError;
use crate::expand::{related_keys, test_keys};
use crate::levels::{estimate_tokens, file_records, represent_file};
use crate::sources::SourceProvider;
use std::collections::{BTreeMap, BTreeSet};
use tpt_weave_core::{
    ContextCandidate, ContextId, ContextLevel, ContextRequest, ContextResponse, ContextSource,
    RepositoryId, Revision, TokenAccounting,
};
use tpt_weave_graph::{DependencyEdge, ReferenceKind, RepositoryGraph};
use tpt_weave_rust::SymbolRecord;

/// Deterministic repository overview (todo.md Phase 5 "repository overview").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryOverview {
    pub repository: RepositoryId,
    pub revision: Revision,
    /// Workspace/member package names, sorted.
    pub crates: Vec<String>,
    /// Distinct files in the symbol table.
    pub files: usize,
    pub symbols: usize,
    pub public_symbols: usize,
    pub modules: usize,
    /// Distinct declared dependency packages.
    pub dependencies: usize,
    /// Registered cross-repository links, sorted.
    pub external_repositories: Vec<RepositoryId>,
}

impl RepositoryOverview {
    /// Summarises a graph.
    pub fn of(graph: &RepositoryGraph) -> Self {
        let mut crates: Vec<String> = graph.crates.iter().map(|c| c.name.clone()).collect();
        crates.sort();
        crates.dedup();
        let mut dependencies: Vec<String> = graph
            .dependencies
            .iter()
            .map(|e| e.to_package.clone())
            .collect();
        dependencies.sort();
        dependencies.dedup();
        let mut files: Vec<String> = graph.symbols.iter().map(|s| s.file.clone()).collect();
        files.sort();
        files.dedup();
        let public_symbols = graph
            .symbols
            .iter()
            .filter(|s| s.visibility.is_public())
            .count();
        Self {
            repository: graph.repository.clone(),
            revision: graph.revision.clone(),
            crates,
            files: files.len(),
            symbols: graph.symbols.len(),
            public_symbols,
            modules: graph.modules.len(),
            dependencies: dependencies.len(),
            external_repositories: graph.linked_repositories(),
        }
    }

    /// Level-0-style text rendering.
    pub fn text(&self) -> String {
        format!(
            "{}@{}\ncrates={}\nfiles={}\nsymbols={}\npublic={}\nmodules={}\ndependencies={}\nexternal={}\n",
            self.repository,
            self.revision.short(),
            self.crates.join(","),
            self.files,
            self.symbols,
            self.public_symbols,
            self.modules,
            self.dependencies,
            self.external_repositories
                .iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>()
                .join(","),
        )
    }

    /// Token estimate of [`Self::text`].
    pub fn token_estimate(&self) -> u32 {
        estimate_tokens(&self.text())
    }
}

/// Deterministic lookups plus budgeted retrieval over a graph.
pub struct Retriever<'a> {
    graph: &'a RepositoryGraph,
    sources: &'a dyn SourceProvider,
}

impl<'a> Retriever<'a> {
    /// Binds a graph and its file sources.
    pub fn new(graph: &'a RepositoryGraph, sources: &'a dyn SourceProvider) -> Self {
        Self { graph, sources }
    }

    /// Repository overview (todo.md Phase 5).
    pub fn repository_overview(&self) -> RepositoryOverview {
        RepositoryOverview::of(self.graph)
    }

    /// File lookup: repository-relative paths whose path contains `query`
    /// (case-insensitive; empty query matches everything), sorted.
    pub fn find_files(&self, query: &str) -> Vec<String> {
        let query = query.to_lowercase();
        let mut paths: Vec<String> = self.graph.symbols.iter().map(|s| s.file.clone()).collect();
        paths.sort();
        paths.dedup();
        paths.retain(|path| path.to_lowercase().contains(&query));
        paths
    }

    /// Symbol lookup by name (case-insensitive; matches `Type::method`
    /// segments and module paths), sorted by canonical key.
    pub fn find_symbols(&self, query: &str) -> Vec<&SymbolRecord> {
        let query = query.to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        let mut found: Vec<&SymbolRecord> = self
            .graph
            .symbols
            .iter()
            .filter(|record| {
                record
                    .id
                    .name
                    .to_lowercase()
                    .split("::")
                    .any(|segment| segment.contains(&query))
                    || record.id.module.to_lowercase().contains(&query)
            })
            .collect();
        found.sort_by_key(|record| record.id.canonical_key());
        found
    }

    /// Reference lookup: resolved targets of a symbol with their kinds.
    pub fn find_references(&self, key: &str) -> Vec<(&SymbolRecord, ReferenceKind)> {
        self.graph.references_of(key)
    }

    /// Dependency lookup: declared dependency edges of a package.
    pub fn find_dependencies(&self, package: &str) -> Vec<&DependencyEdge> {
        self.graph.dependencies_of(package)
    }

    /// Related-symbol lookup ([`related_keys`]), sorted by canonical key.
    pub fn find_related(&self, key: &str) -> Result<Vec<&SymbolRecord>, ContextError> {
        let keys = related_keys(self.graph, key)?;
        let mut found: Vec<&SymbolRecord> = self
            .graph
            .symbols
            .iter()
            .filter(|record| keys.contains(&record.id.canonical_key()))
            .collect();
        found.sort_by_key(|record| record.id.canonical_key());
        Ok(found)
    }

    /// Changed-file lookup: symbols defined in any of the changed paths
    /// (`/`-separated or as given; comparisons normalise separators),
    /// sorted by file then position.
    pub fn find_changed(&self, changed_files: &[String]) -> Vec<&SymbolRecord> {
        let wanted: BTreeSet<String> = changed_files
            .iter()
            .map(|path| path.replace('\\', "/"))
            .collect();
        let mut found: Vec<&SymbolRecord> = self
            .graph
            .symbols
            .iter()
            .filter(|record| wanted.contains(&record.file))
            .collect();
        found.sort_by(|a, b| {
            (a.file.as_str(), a.line, a.column).cmp(&(b.file.as_str(), b.line, b.column))
        });
        found
    }

    /// Test lookup ([`test_keys`]), sorted by file then position.
    pub fn find_tests(&self, key: &str) -> Result<Vec<&SymbolRecord>, ContextError> {
        let keys = test_keys(self.graph, key)?;
        let mut found: Vec<&SymbolRecord> = self
            .graph
            .symbols
            .iter()
            .filter(|record| keys.contains(&record.id.canonical_key()))
            .collect();
        found.sort_by(|a, b| {
            (a.file.as_str(), a.line, a.column).cmp(&(b.file.as_str(), b.line, b.column))
        });
        Ok(found)
    }

    /// Deterministic context retrieval: scores candidates against the task,
    /// orders them, deduplicates and selects within the budget
    /// (spec.md sections 10 and 16).
    ///
    /// `changed_files` (e.g. from git) receive a relevance boost; an empty
    /// task still yields the overview plus the changed files.
    pub fn retrieve(
        &self,
        request: &ContextRequest,
        changed_files: &[String],
    ) -> Result<ContextResponse, ContextError> {
        let words = tokenize(&request.task);
        let changed: BTreeSet<String> = changed_files
            .iter()
            .map(|path| path.replace('\\', "/"))
            .collect();

        // Seed relevance sets from the task words.
        let mut matched: BTreeSet<String> = BTreeSet::new();
        for word in &words {
            for record in self.find_symbols(word) {
                matched.insert(record.id.canonical_key());
            }
        }
        let mut related: BTreeSet<String> = matched.clone();
        for key in &matched {
            related.extend(related_keys(self.graph, key)?);
        }
        let mut tests: BTreeSet<String> = BTreeSet::new();
        for key in &matched {
            tests.extend(test_keys(self.graph, key)?);
        }

        let paths = self.find_files("");
        let mut file_seeds: Vec<FileSeed> = Vec::new();
        for path in &paths {
            let records = file_records(self.graph, path);
            let mut seed = FileSeed {
                path: path.clone(),
                raw: 0,
                selection: BTreeSet::new(),
            };
            let path_lower = path.to_lowercase();
            for word in &words {
                if path_lower.contains(word.as_str()) {
                    seed.raw += 2;
                }
            }
            for record in &records {
                let key = record.id.canonical_key();
                let name_lower = record.id.name.to_lowercase();
                let name_match = words.iter().any(|word| {
                    name_lower
                        .split("::")
                        .any(|segment| segment.contains(word.as_str()))
                });
                if name_match {
                    seed.raw += 3;
                    seed.selection.insert(key.clone());
                }
                let module_lower = record.id.module.to_lowercase();
                if !module_lower.is_empty()
                    && words
                        .iter()
                        .any(|word| module_lower.contains(word.as_str()))
                {
                    seed.raw += 1;
                }
                if related.contains(&key) {
                    seed.raw += 1;
                    seed.selection.insert(key.clone());
                }
                if tests.contains(&key) {
                    seed.raw += 2;
                    seed.selection.insert(key);
                }
            }
            if changed.contains(path.as_str()) {
                seed.raw += 4;
            }
            if seed.raw > 0 {
                file_seeds.push(seed);
            }
        }

        // Dependency candidates (opt-in via the request).
        let mut dep_seeds: Vec<(String, u32, u32)> = Vec::new();
        if request.include_dependencies {
            let mut packages: Vec<String> = self
                .graph
                .dependencies
                .iter()
                .map(|edge| edge.to_package.clone())
                .collect();
            packages.sort();
            packages.dedup();
            for package in packages {
                let package_lower = package.to_lowercase();
                let hits = words
                    .iter()
                    .filter(|word| package_lower.contains(word.as_str()))
                    .count();
                if hits == 0 {
                    continue;
                }
                dep_seeds.push((package, hits as u32 * 2, 0));
            }
        }

        // --- Build candidates ------------------------------------------------
        let mut candidates: Vec<ContextCandidate> = Vec::new();
        let overview = self.repository_overview();
        let overview_candidate = ContextCandidate::new(
            ContextSource::Repository,
            ContextLevel::Metadata,
            overview.token_estimate(),
        )
        .with_score(normalise(1));
        candidates.push(overview_candidate);

        // File candidates, one per path, carrying their relevant selection.
        let mut file_ids: BTreeMap<String, ContextId> = BTreeMap::new();
        let mut file_candidates: Vec<ContextCandidate> = Vec::new();
        for seed in &file_seeds {
            let has_source = self.sources.source(&seed.path).is_some();
            let level = effective_level(request.max_level, has_source);
            let representation = represent_file(
                self.graph,
                self.sources,
                &seed.path,
                level,
                &seed_selection(&seed.selection),
            )?;
            let candidate = ContextCandidate::new(
                ContextSource::File(seed.path.clone()),
                level,
                representation.token_estimate,
            )
            .with_score(normalise(seed.raw));
            file_ids.insert(seed.path.clone(), candidate.id.clone());
            file_candidates.push(candidate);
        }

        // Dependency candidates: public-API signature estimate.
        for (package, raw, _) in &dep_seeds {
            let api = self.graph.public_api(package);
            let tokens: u32 = if api.is_empty() {
                // Unknown external API: small placeholder estimate.
                16
            } else {
                api.iter()
                    .map(|record| estimate_tokens(&record.signature))
                    .fold(0u32, u32::saturating_add)
            };
            candidates.push(
                ContextCandidate::new(
                    ContextSource::Dependency(package.clone()),
                    ContextLevel::Signatures,
                    tokens,
                )
                .with_score(normalise(*raw)),
            );
        }

        // Symbol anchors at signature level (cheap expand handoff).
        for key in &matched {
            let Some(record) = self.graph.symbol(key) else {
                continue;
            };
            let mut candidate = ContextCandidate::new(
                ContextSource::Symbol(record.id.clone()),
                ContextLevel::Signatures,
                estimate_tokens(&record.signature),
            )
            .with_score(normalise(4));
            if let Some(file_id) = file_ids.get(&record.file) {
                candidate = candidate.with_relationship(file_id.clone());
            }
            candidates.push(candidate);
        }
        candidates.extend(file_candidates);

        // --- Score, order, deduplicate, budget -------------------------------
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.source.key().cmp(&b.source.key()))
        });

        let selected = select_within_budget(&candidates, request.budget_tokens);

        // Token accounting: raw = whole-repository source at level 5.
        let mut raw = u64::from(overview.token_estimate());
        for path in &paths {
            if let Some(source) = self.sources.source(path) {
                raw = raw.saturating_add(u64::from(estimate_tokens(source)));
            }
        }
        for (package, _, _) in &dep_seeds {
            let api = self.graph.public_api(package);
            if api.is_empty() {
                raw = raw.saturating_add(16);
            } else {
                for record in api {
                    raw = raw.saturating_add(u64::from(estimate_tokens(&record.signature)));
                }
            }
        }
        let delivered: Vec<ContextCandidate> = selected
            .iter()
            .map(|&index| candidates[index].clone())
            .collect();
        let selected_tokens: u64 = delivered.iter().map(|c| u64::from(c.token_estimate)).sum();
        raw = raw.max(selected_tokens);
        let tokens = TokenAccounting::new(raw, selected_tokens, 0, 0, false);

        Ok(ContextResponse::new(
            request.repository.clone(),
            delivered,
            tokens,
            None,
        ))
    }
}

struct FileSeed {
    path: String,
    raw: u32,
    selection: BTreeSet<String>,
}

fn seed_selection(keys: &BTreeSet<String>) -> crate::Selection {
    keys.iter()
        .fold(crate::Selection::new(), |acc, key| acc.with(key.clone()))
}

/// Level actually rendered for a file: the request's `max_level`, capped to
/// something source-free when the file's text is unavailable.
fn effective_level(max_level: ContextLevel, has_source: bool) -> ContextLevel {
    if has_source {
        return max_level;
    }
    match max_level {
        ContextLevel::Metadata => ContextLevel::Symbols,
        ContextLevel::Symbols => ContextLevel::Symbols,
        _ => ContextLevel::Signatures,
    }
}

/// Maps a raw integer score into `0.0..1.0` monotonically.
fn normalise(raw: u32) -> f32 {
    let raw = raw as f32;
    raw / (raw + 5.0)
}

/// Deterministic budget selection: greedy in score order (skipping items
/// that do not fit, so a small anchor can follow a large file), then
/// deduplicated — a symbol anchor is dropped when its file is already
/// selected at `Signatures` or deeper. Iterates until stable.
fn select_within_budget(candidates: &[ContextCandidate], budget: u32) -> Vec<usize> {
    let n = candidates.len();
    let mut taken = vec![false; n];
    let mut excluded = vec![false; n];
    let mut used = 0u32;

    // The repository overview is admitted first when it fits, so larger
    // files cannot crowd it out of the budget.
    for index in 0..n {
        if matches!(candidates[index].source, ContextSource::Repository)
            && candidates[index].token_estimate <= budget
        {
            taken[index] = true;
            used = used.saturating_add(candidates[index].token_estimate);
        }
    }

    loop {
        let mut changed = false;

        // Greedy pass in delivery order.
        for index in 0..n {
            if taken[index] || excluded[index] {
                continue;
            }
            let estimate = candidates[index].token_estimate;
            if used.saturating_add(estimate) <= budget {
                taken[index] = true;
                used = used.saturating_add(estimate);
                changed = true;
            }
        }

        // Deduplication: symbol subsumed by its file candidate.
        for index in 0..n {
            if !taken[index] {
                continue;
            }
            if !matches!(candidates[index].source, ContextSource::Symbol(_)) {
                continue;
            }
            let Some(symbol_file) = file_of_symbol(candidates, index) else {
                continue;
            };
            for other in 0..n {
                if other == index || !taken[other] {
                    continue;
                }
                if let ContextSource::File(path) = &candidates[other].source {
                    if *path == symbol_file && candidates[other].level >= ContextLevel::Signatures {
                        taken[index] = false;
                        excluded[index] = true;
                        used = used.saturating_sub(candidates[index].token_estimate);
                        changed = true;
                        break;
                    }
                }
            }
        }

        if !changed {
            break;
        }
    }

    (0..n).filter(|&index| taken[index]).collect()
}

/// File path a symbol anchor points at, recovered from its relationship id
/// is not possible; instead derive from the candidate's own source ordering.
fn file_of_symbol(candidates: &[ContextCandidate], index: usize) -> Option<String> {
    // Symbol candidates carry a relationship to their file candidate when
    // one exists; match that id back to a File source.
    let ContextSource::Symbol(_) = &candidates[index].source else {
        return None;
    };
    for related_id in &candidates[index].relationships {
        if let Some(other) = candidates
            .iter()
            .find(|candidate| &candidate.id == related_id)
        {
            if let ContextSource::File(path) = &other.source {
                return Some(path.clone());
            }
        }
    }
    None
}

/// Lowercased task words of length ≥ 2, minus light English glue, sorted
/// and deduplicated (deterministic bag of terms).
fn tokenize(task: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the", "a", "an", "in", "on", "of", "to", "for", "and", "or", "is", "it", "this", "that",
        "with", "please", "when", "then", "into", "from", "using", "use", "at", "by", "as", "be",
        "we", "our",
    ];
    let mut words: Vec<String> = task
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| word.len() >= 2 && !STOP.contains(word))
        .map(str::to_string)
        .collect();
    words.sort();
    words.dedup();
    words
}
