# tpt-weave — TODO

> **Status (2026-09-23):** Phases 0–11 complete (repository + decisions, core model + manifest
> configuration, cargo/git/syn indexer, symbol graph, hierarchical representations + expansion,
> deterministic context retrieval, tool output reduction, filesystem cache, JEv decision
> provider/OpenRouter client/policy engine, MCP server with 11 tools + tool minimisation, CLI
> with all planned subcommands + human/JSON/compact output + exit codes, evaluation harness with
> baseline/weave captures + net-token-reduction metrics + local adopt benchmark; 178 tests
> passing, clippy `-D warnings` clean). Phases 12–20 pending.

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

- [ ] Add tpt-weave integration.
- [ ] Replace duplicated repository exploration.
- [ ] Route context requests through tpt-weave.
- [ ] Add token dashboard.
- [ ] Measure baseline.
- [ ] Measure post-integration.

### tpt-infer

- [ ] Index repository.
- [ ] Test symbol retrieval.
- [ ] Test dependency traversal.
- [ ] Test model context.

### tpt-uir

- [ ] Evaluate as context transport/IR.
- [ ] Avoid unnecessary duplicate representations.

### tpt-raglite

- [ ] Evaluate as optional semantic retrieval backend.
- [ ] Compare structural retrieval vs semantic retrieval.
- [ ] Avoid duplicating RAG functionality unnecessarily.

## Phase 13 — Complex TPT Repositories

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

- [ ] Define .tpt-weave standard.
- [ ] Define registry metadata.
- [ ] Add TPT repository registry integration.
- [ ] Add repository auto-discovery.
- [ ] Add cross-repository dependency discovery.
- [ ] Add standard agent configuration.
- [ ] Add standard MCP configuration.
- [ ] Add CI indexing.
- [ ] Add index validation in CI.

## Phase 15 — Performance

- [ ] Benchmark indexing.
- [ ] Benchmark incremental indexing.
- [ ] Benchmark symbol lookup.
- [ ] Benchmark context generation.
- [ ] Benchmark JEv decisions.
- [ ] Benchmark MCP.
- [ ] Benchmark cache.
- [ ] Profile memory.
- [ ] Profile CPU.
- [ ] Reduce allocations.
- [ ] Add parallel indexing.
- [ ] Add incremental graph updates.

## Phase 16 — Privacy and Security

- [ ] Secret detection.
- [ ] .env exclusion.
- [ ] credential exclusion.
- [ ] certificate exclusion.
- [ ] private-path configuration.
- [ ] remote-context audit log.
- [ ] explicit remote-data policy.
- [ ] local-only mode.
- [ ] test redaction.
- [ ] document threat model.

## Phase 17 — Optimisation Experiments

### Experiment A — JEv relevance

- [ ] Compare deterministic-only.
- [ ] Compare deterministic + JEv.
- [ ] Measure accuracy.
- [ ] Measure token savings.

### Experiment B — Hierarchical source

- [ ] Metadata only.
- [ ] Symbols.
- [ ] Signatures.
- [ ] Skeleton.
- [ ] Targeted implementation.
- [ ] Full source.

### Experiment C — Tool output

- [ ] Raw output.
- [ ] Deterministic reduction.
- [ ] Reduction + JEv.

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
- [ ] Measure total token reduction.
- [ ] Measure model quality.
- [ ] Calculate actual dollar savings.
- [ ] Determine whether context reduction is sufficient to materially extend the user's budget.

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

- [ ] README.
- [ ] Architecture guide.
- [ ] Integration guide.
- [ ] MCP guide.
- [ ] JEv guide.
- [ ] OpenRouter guide.
- [ ] Repository author guide.
- [ ] Privacy guide.
- [ ] Benchmark methodology.
- [ ] Token accounting guide.
- [ ] Troubleshooting guide.

## Phase 20 — Release

- [ ] Stable core API.
- [ ] Stable index format.
- [ ] Stable MCP API.
- [ ] Stable CLI.
- [ ] OpenRouter adapter.
- [ ] JEv integration.
- [ ] Documentation complete.
- [ ] CI complete.
- [ ] Security review.
- [ ] Performance review.
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



