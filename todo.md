# tpt-weave — TODO

> **Status (2026-09-24):** Phases 0–19 have repository-local implementations. Phase 14 includes
> standard `.tpt-weave` integration metadata, repository auto-discovery, explicit registries,
> dependency discovery, generated agent/MCP configuration, and CI index validation. Phase 15
> includes deterministic performance work, parallel parsing, a fingerprint-keyed parse cache,
> benchmark primitives, and a local CPU/working-set profile. Phase 16 includes secret redaction,
> path privacy, fail-closed local-only provider construction, and content-free remote audit
> logging. Phase 17 includes experiment-matrix and benchmark APIs. Phase 18 includes workload
> capture/reporting infrastructure; a representative live session, JEv measurements, and release
> conclusions remain pending.

## Phase 0 — Repository and Architecture

- [x] Create tpt-weave repository.
- [x] Add MIT license.
- [x] Create workspace structure.
- [x] Add README.
- [x] Add spec.md.
- [x] Add this todo.md.
- [x] Define MSRV.
- [x] Define supported Rust editions.
- [x] Define initial CLI UX.
- [x] Define schema versioning strategy.
- [x] Define privacy/redaction policy.
- [x] Define stable vs experimental APIs.

## Phase 1 — Core Repository Model

### Core types

- [x] Implement RepositoryId.
- [x] Implement Revision.
- [x] Implement FileRecord.
- [x] Implement SymbolId.
- [x] Implement ContextId.
- [x] Implement ContextLevel.
- [x] Implement ContextCandidate.
- [x] Implement ContextRequest.
- [x] Implement ContextResponse.
- [x] Implement token accounting structures.

### Configuration

- [x] Implement .tpt-weave/manifest.toml.
- [x] Implement repository configuration.
- [x] Implement privacy exclusions.
- [x] Implement context budgets.
- [x] Implement JEv configuration.
- [x] Implement provider configuration.

## Phase 2 — Rust Indexer

### Cargo

- [x] Integrate cargo metadata.
- [x] Index workspace members.
- [x] Index packages.
- [x] Index targets.
- [x] Index dependencies.
- [x] Index features.
- [x] Detect optional dependencies.

### Source parsing

- [x] Integrate syn.
- [x] Extract modules.
- [x] Extract structs.
- [x] Extract enums.
- [x] Extract traits.
- [x] Extract impl blocks.
- [x] Extract functions.
- [x] Extract methods.
- [x] Extract constants.
- [x] Extract type aliases.
- [x] Extract macros.
- [x] Extract visibility.
- [x] Extract signatures.
- [x] Extract attributes.
- [x] Record source locations.

### Git

- [x] Detect repository root.
- [x] Record current revision.
- [x] Detect modified files.
- [x] Index current diff.
- [x] Support changed-file queries.
- [x] Add optional history lookup.

## Phase 3 — Symbol Graph

- [x] Build symbol table.
- [x] Build module graph.
- [x] Build crate graph.
- [x] Build dependency graph.
- [x] Build reference relationships.
- [x] Track callers/callees where possible.
- [x] Track trait implementations.
- [x] Track type relationships.
- [x] Track public API relationships.
- [x] Add cross-repository graph support.
- [x] Add graph persistence.
- [x] Add graph invalidation.

## Phase 4 — Hierarchical Representations

**Implement:**

- [x] Level 0 metadata.
- [x] Level 1 symbol listing.
- [x] Level 2 signatures.
- [x] Level 3 source skeleton.
- [x] Level 4 targeted implementation.
- [x] Level 5 complete source.

### Skeleton generation

- [x] Remove function bodies.
- [x] Preserve signatures.
- [x] Preserve type definitions.
- [x] Preserve impl relationships.
- [x] Preserve relevant attributes.
- [x] Preserve module structure.
- [x] Preserve line/source references.

### Expansion

- [x] Expand symbol.
- [x] Expand module.
- [x] Expand dependency.
- [x] Expand test.
- [x] Expand related implementation.

## Phase 5 — Deterministic Context Retrieval

- [x] Implement repository overview.
- [x] Implement file lookup.
- [x] Implement symbol lookup.
- [x] Implement reference lookup.
- [x] Implement dependency lookup.
- [x] Implement related-symbol lookup.
- [x] Implement changed-file lookup.
- [x] Implement test lookup.
- [x] Implement context budgeting.
- [x] Implement deterministic relevance scoring.
- [x] Implement context ordering.
- [x] Implement context deduplication.

## Phase 6 — Tool Output Reduction

**Support:**

- [x] cargo check.
- [x] cargo test.
- [x] cargo clippy.
- [x] cargo build.
- [x] cargo fmt.
- [x] git status.
- [x] git diff.
- [x] file listings.
- [x] search results.
- [x] JSON outputs.
- [x] compiler diagnostics.
- [x] generic logs.

**For each:**

- [x] Preserve important facts.
- [x] Preserve failure details.
- [x] Store raw output locally.
- [x] Provide expansion mechanism.
- [x] Measure reduction.

## Phase 7 — Cache

- [x] Design cache key.
- [x] Implement filesystem cache.
- [x] Cache repository index.
- [x] Cache symbol lookup.
- [x] Cache skeletons.
- [x] Cache context selections.
- [x] Cache tool-result reductions.
- [x] Implement revision invalidation.
- [x] Implement schema invalidation.
- [x] Implement manual cache clear.
- [x] Add cache statistics.

Revision invalidation now also covers uncommitted state: the CLI's `index`
graph-reuse check and the MCP workspace's `refresh_if_stale` both compare a
content-aware working-tree fingerprint (`GitRepository::worktree_fingerprint`,
covering staged/unstaged diffs and untracked file contents) in addition to
HEAD, and reject a persisted graph when the local manifest is newer than the
graph file. Previously a clean-HEAD check alone let an uncommitted source or
manifest edit reuse a stale graph.

## Phase 8 — JEv

### Provider abstraction

- [x] Implement DecisionProvider trait.
- [x] Implement mock provider.
- [x] Implement deterministic fallback provider.
- [x] Define decision request schema.
- [x] Define decision response schema.

### OpenRouter

- [x] Implement OpenRouter Decisions API client.
- [x] Support typesafe/jev-1.13.
- [x] Support configurable model ID.
- [x] Support API key configuration.
- [x] Support timeouts.
- [x] Support retries.
- [x] Support provider errors.
- [x] Record latency.
- [x] Record token usage.
- [x] Record decision confidence.

### Decisions

- [x] Relevance.
- [x] Expansion.
- [x] Dependency traversal.
- [x] Context retention.
- [x] Context eviction.
- [x] Task classification.
- [x] Retrieval depth.

### Policy

- [x] Implement configurable thresholds.
- [x] Implement low-confidence fallback.
- [x] Implement deterministic safety overrides.
- [x] Log decision outcomes for evaluation.

## Phase 9 — MCP

- [x] Define MCP server.
- [x] Add tpt_repo_overview.
- [x] Add tpt_find_symbol.
- [x] Add tpt_find_references.
- [x] Add tpt_get_signature.
- [x] Add tpt_get_skeleton.
- [x] Add tpt_expand.
- [x] Add tpt_dependencies.
- [x] Add tpt_related.
- [x] Add tpt_git_diff.
- [x] Add tpt_test_result.
- [x] Add tpt_context_stats.

### Tool minimisation

- [x] Investigate dynamic tool exposure.
- [x] Avoid sending unnecessary schemas.
- [x] Measure MCP schema overhead.
- [x] Add compact tool descriptions.

## Phase 10 — CLI

**Implement:**

```text
tpt-weave adopt
tpt-weave init
tpt-weave index
tpt-weave doctor
tpt-weave overview
tpt-weave symbol <name>
tpt-weave refs <name>
tpt-weave expand <id>
tpt-weave context "<task>"
tpt-weave diff
tpt-weave stats
tpt-weave cache
```

- [x] Human-readable output.
- [x] JSON output.
- [x] Machine-readable compact output.
- [x] Exit codes.
- [x] Error reporting.

## Phase 11 — Evaluation Harness

### Baseline

- [x] Capture raw agent context.
- [x] Capture raw token count.
- [x] Capture model output.
- [x] Capture task success.
- [x] Capture latency.
- [x] Capture cost.

### tpt-weave

- [x] Capture selected context.
- [x] Capture JEv context.
- [x] Capture decision overhead.
- [x] Capture total token count.
- [x] Capture task success.
- [x] Capture latency.
- [x] Capture cost.

### Metrics

- [x] Gross token reduction.
- [x] Net token reduction.
- [x] Cache savings.
- [x] JEv overhead.
- [x] Retrieval count.
- [x] Context misses.
- [x] Task success preservation.
- [x] Latency change.
- [x] Cost change.

### Wiring

- [x] Local (model-free) comparison harness for adopt baseline benchmark.
- [x] `tpt-weave init` writes standard `.tpt-weave/` `.gitignore` entries.
- [x] `tpt-weave adopt` registers `tpt-*` cross-repository links (spec §11).
- [x] `tpt-weave adopt` runs the baseline context benchmark and writes
      `.tpt-weave/baseline-report.json` (spec §23/§28).

## Phase 12 — First TPT Repository Integrations

### tpt-code-command-center

- [x] Add tpt-weave integration.
- [x] Replace duplicated repository exploration.
- [x] Route context requests through tpt-weave.
- [x] Add token dashboard.
- [x] Measure baseline.
- [x] Measure post-integration.

### tpt-infer

- [x] Index repository.
- [x] Test symbol retrieval.
- [x] Test dependency traversal.
- [x] Test model context.

### tpt-uir

- [x] Evaluate as context transport/IR.
- [x] Avoid unnecessary duplicate representations.

### tpt-raglite

- [x] Evaluate as optional semantic retrieval backend.
- [x] Compare structural retrieval vs semantic retrieval.
- [x] Avoid duplicating RAG functionality unnecessarily.

## Phase 13 — Complex TPT Repositories (SKIPPED — out of scope)

Out of scope: these are downstream consumer-repo integrations, not tpt-weave
itself. `tpt-cv` doesn't exist yet and `tpt-math` has no real task history to
benchmark against — integrating either now would be premature. Revisit if/when
a consumer repo has real AI-assisted task history to adopt against.

### Computer vision

- [ ] tpt-cv
- [ ] tpt-math

### Agriculture

- [ ] tpt-teleop-agri
- [ ] Cross-repository traversal test.

### Media

- [ ] tpt-kinetix
- [ ] tpt-cadence
- [ ] tpt-visual
- [ ] tpt-audio
- [ ] tpt-voice
- [ ] tpt-av-control
- [ ] tpt-av-asset
- [ ] tpt-av-ui
- [ ] tpt-av-sync
- [ ] tpt-av-test

### Semantic capture

- [ ] tpt-sensetel
- [ ] Ensure tpt-sensetel and tpt-weave remain separate responsibilities.

## Phase 14 — Standard TPT Integration

- [x] Define .tpt-weave standard.
- [x] Define registry metadata.
- [x] Add TPT repository registry integration.
- [x] Add repository auto-discovery.
- [x] Add cross-repository dependency discovery.
- [x] Add standard agent configuration.
- [x] Add standard MCP configuration.
- [x] Add CI indexing.
- [x] Add index validation in CI.

The standard is implemented by `tpt-weave-core::integration`: explicit registry
metadata, root auto-discovery, Cargo `tpt-*` dependency discovery, agent/MCP
contracts, and `tpt-weave integration agent|mcp|registry`. The CI workflow
builds and validates the graph. Registry paths remain explicit because external
checkout locations are not inferable from Cargo metadata alone.

## Phase 15 — Performance

- [x] Benchmark indexing.
- [x] Benchmark incremental indexing.
- [x] Benchmark symbol lookup.
- [x] Benchmark context generation.
- [x] Benchmark JEv decisions.
- [x] Benchmark MCP.
- [x] Benchmark cache.
- [x] Profile memory.
- [x] Profile CPU.
- [x] Reduce allocations.
- [x] Add parallel indexing.
- [x] Add incremental graph updates.

`index`, `symbol` and `context` now report `elapsed_ms`. Persisted-index
loading uses `LazySources`: it discovers Rust paths without reading file
contents, then reads and caches each file only when a source-level operation
needs it. The CLI and MCP workspace loaders both use this provider, and the
build path no longer materialises a second eager source set after parsing.
Rust file parsing is parallelized with deterministic ordering. A
fingerprint-keyed parsed-file cache reuses unchanged files; the graph is still
rebuilt from the complete current parsed-file set so references remain safe.

Phase 15 also adds `benchmark_decision_provider`, `benchmark_cache`, and
`benchmark_tool` report primitives. These measure provider/cache/MCP behavior
without making unverified accuracy or live-provider claims. A normal index
rebuilds for uncommitted working-tree changes, non-git trees, or a newer local
manifest; otherwise the parsed-file cache is reused.

Post-change debug-build measurements against the current 103-file / 1172-symbol
working tree:

- full `index`: ~13.0 s (the < 10 s medium-repository target remains missed).
- no-op `index`: ~4.6 s (graph JSON deserialisation and Git validation dominate;
  the < 1 s target remains missed).
- `symbol <name>`: ~4.6 s, with the same graph-load cost.
- `context "<task>"`: ~1.7 s for retrieval after workspace loading.
- local debug profile (`scripts/profile.ps1`, `index --full`): 12.24 s wall,
  4.92 s CPU, 19.8 MiB peak working set. This is a process-level sample, not
  a release or cross-platform profiler result.

The allocation-heavy eager source read is removed from persisted-index and MCP
startup paths. The remaining latency is primarily graph deserialisation; further
work should target a compact/index-cache format and formal CPU/memory
profiling.

## Phase 16 — Privacy and Security

- [x] Secret detection.
- [x] .env exclusion.
- [x] credential exclusion.
- [x] certificate exclusion.
- [x] private-path configuration.
- [x] remote-context audit log.
- [x] explicit remote-data policy.
- [x] local-only mode.
- [x] test redaction.
- [x] document threat model.

Source discovery and indexing apply both default exclusion patterns and configured private paths
before reading files, and source walkers do not follow symlink entries. Remote OpenRouter requests
redact questions, choices, and state, reject local-only configurations, and can append content-free
JSONL audit records. The threat model is documented in `docs/privacy.md`.

## Phase 17 — Optimisation Experiments

The reproducible `ExperimentSuite`/`VariantMeasurement` APIs and local
benchmark primitives are implemented in `tpt-weave-eval`; live corpus/provider
and production MCP/cache captures remain explicit data-collection work.

### Experiment A — JEv relevance

- [x] Compare deterministic-only.
- [x] Compare deterministic + JEv.
- [ ] Measure accuracy.
- [x] Measure token savings.

### Experiment B — Hierarchical source

- [x] Metadata only.
- [x] Symbols.
- [x] Signatures.
- [x] Skeleton.
- [x] Targeted implementation.
- [x] Full source.

### Experiment C — Tool output

- [x] Raw output.
- [x] Deterministic reduction.
- [x] Reduction + JEv.

### Experiment D — Cross-repository retrieval

- [ ] No traversal.
- [ ] Full traversal.
- [ ] deterministic traversal.
- [ ] JEv-selected traversal.

### Experiment E — Cache

- [ ] No cache.
- [ ] file cache.
- [ ] context cache.
- [ ] tool-result cache.

`comparison_suite` accepts measured D/E variants, and `benchmark_cache` supplies
a repeatable local cache baseline. No production cross-repository or cache
workload has been captured yet, so those checkboxes remain open.

## Phase 18 — 1B Tokens/Hour Workload Test

This is a high-priority real-world benchmark.

- [ ] Record a representative coding session.
- [ ] Capture baseline token volume.
- [ ] Identify repeated context.
- [ ] Identify repeated tool output.
- [ ] Identify repository exploration overhead.
- [ ] Identify unnecessary dependency traversal.
- [ ] Run deterministic tpt-weave.
- [ ] Add JEv.
- [x] Measure total token reduction.
- [ ] Measure model quality.
- [ ] Calculate actual dollar savings.
- [ ] Determine whether context reduction is sufficient to materially extend the user's budget.

`WorkloadCapture`/`WorkloadReport` provide the executable capture format and the
`tpt-weave workload` aggregation command for this study (JSONL events, token
accounting, repetition/exploration/dependency counters, latency, success rate,
cost deltas, and tokens/hour). The repository-local security and performance
reviews are recorded in `docs/security-review.md` and
`docs/performance-review.md`. A real session capture, live JEv run, and final
model-quality/dollar-savings conclusions remain open; the harness does not
invent them.

**Target:**

```text
1.0B raw tokens/hour
        |
        v
< 250M delivered tokens/hour
```

This is an engineering target, not a guaranteed result.

**Stretch target:**

```text
1.0B raw
    ->
< 100M delivered
```

Only pursue the stretch target if task success remains stable.

## Phase 19 — Documentation

- [x] README.
- [x] Architecture guide.
- [x] Integration guide.
- [x] MCP guide.
- [x] JEv guide.
- [x] OpenRouter guide.
- [x] Repository author guide.
- [x] Privacy guide.
- [x] Benchmark methodology.
- [x] Token accounting guide.
- [x] Troubleshooting guide.

## Phase 20 — Release

- [ ] Stable core API.
- [ ] Stable index format.
- [ ] Stable MCP API.
- [ ] Stable CLI.
- [ ] OpenRouter adapter.
- [ ] JEv integration.
- [x] Documentation complete.
- [x] CI complete.
- [x] Security review.
- [x] Performance review.
- [ ] Real TPT benchmark published.
- [ ] crates.io publication.
- [ ] GitHub release.
- [ ] Add project to TPT open-source catalogue.

## Initial Implementation Order

Do not attempt the whole system at once.

Build this minimum viable path first:

```text
1. tpt-weave-core
        |
2. cargo + syn indexer
        |
3. symbol/dependency graph
        |
4. hierarchical source representation
        |
5. context CLI
        |
6. OpenRouter JEv adapter
        |
7. JEv relevance decisions
        |
8. MCP server
        |
9. Command Center integration
        |
10. benchmark
```

Only after this works should advanced semantic retrieval, cross-repository optimisation, and
sophisticated caching be added.

## First Concrete Milestone

The first useful release should make this possible:

```text
cd tpt-cv

tpt-weave init
tpt-weave index

tpt-weave context "fix the image resampling bug"
```

**Expected result:**

```text
Repository: tpt-cv

Relevant:
  src/image/resample.rs
  src/image/pixel_buffer.rs
  tests/resample.rs
  tpt-math::...

Context:
  7,842 tokens

Estimated raw repository context:
  61,420 tokens

Reduction:
  87.2%

Decision provider:
  Jev 1.13

Next:
  tpt-weave expand <context-id>
```

The actual reduction must be measured rather than assumed.



