tpt-weave — TODO

Status (2026-09-23): Phase 0 complete (repository, MIT licence, workspace, README, spec.md, MSRV/CLI/schema/privacy/stability decisions in docs/decisions.md). Phase 1 complete (tpt-weave-core: core types + manifest configuration, 22 tests passing). Phases 2-20 pending.

Phase 0 — Repository and Architecture

Create tpt-weave repository.

Add MIT license.

Create workspace structure.

Add README.

Add spec.md.

Add this todo.md.

Define MSRV.

Define supported Rust editions.

Define initial CLI UX.

Define schema versioning strategy.

Define privacy/redaction policy.

Define stable vs experimental APIs.

Phase 1 — Core Repository Model

Core types

Implement RepositoryId.

Implement Revision.

Implement FileRecord.

Implement SymbolId.

Implement ContextId.

Implement ContextLevel.

Implement ContextCandidate.

Implement ContextRequest.

Implement ContextResponse.

Implement token accounting structures.

Configuration

Implement .tpt-weave/manifest.toml.

Implement repository configuration.

Implement privacy exclusions.

Implement context budgets.

Implement JEv configuration.

Implement provider configuration.

Phase 2 — Rust Indexer

Cargo

Integrate cargo metadata.

Index workspace members.

Index packages.

Index targets.

Index dependencies.

Index features.

Detect optional dependencies.

Source parsing

Integrate syn.

Extract modules.

Extract structs.

Extract enums.

Extract traits.

Extract impl blocks.

Extract functions.

Extract methods.

Extract constants.

Extract type aliases.

Extract macros.

Extract visibility.

Extract signatures.

Extract attributes.

Record source locations.

Git

Detect repository root.

Record current revision.

Detect modified files.

Index current diff.

Support changed-file queries.

Add optional history lookup.

Phase 3 — Symbol Graph

Build symbol table.

Build module graph.

Build crate graph.

Build dependency graph.

Build reference relationships.

Track callers/callees where possible.

Track trait implementations.

Track type relationships.

Track public API relationships.

Add cross-repository graph support.

Add graph persistence.

Add graph invalidation.

Phase 4 — Hierarchical Representations

Implement:

Level 0 metadata.

Level 1 symbol listing.

Level 2 signatures.

Level 3 source skeleton.

Level 4 targeted implementation.

Level 5 complete source.

Skeleton generation

Remove function bodies.

Preserve signatures.

Preserve type definitions.

Preserve impl relationships.

Preserve relevant attributes.

Preserve module structure.

Preserve line/source references.

Expansion

Expand symbol.

Expand module.

Expand dependency.

Expand test.

Expand related implementation.

Phase 5 — Deterministic Context Retrieval

Implement repository overview.

Implement file lookup.

Implement symbol lookup.

Implement reference lookup.

Implement dependency lookup.

Implement related-symbol lookup.

Implement changed-file lookup.

Implement test lookup.

Implement context budgeting.

Implement deterministic relevance scoring.

Implement context ordering.

Implement context deduplication.

Phase 6 — Tool Output Reduction

Support:

cargo check.

cargo test.

cargo clippy.

cargo build.

cargo fmt.

git status.

git diff.

file listings.

search results.

JSON outputs.

compiler diagnostics.

generic logs.

For each:

Preserve important facts.

Preserve failure details.

Store raw output locally.

Provide expansion mechanism.

Measure reduction.

Phase 7 — Cache

Design cache key.

Implement filesystem cache.

Cache repository index.

Cache symbol lookup.

Cache skeletons.

Cache context selections.

Cache tool-result reductions.

Implement revision invalidation.

Implement schema invalidation.

Implement manual cache clear.

Add cache statistics.

Phase 8 — JEv

Provider abstraction

Implement DecisionProvider trait.

Implement mock provider.

Implement deterministic fallback provider.

Define decision request schema.

Define decision response schema.

OpenRouter

Implement OpenRouter Decisions API client.

Support typesafe/jev-1.13.

Support configurable model ID.

Support API key configuration.

Support timeouts.

Support retries.

Support provider errors.

Record latency.

Record token usage.

Record decision confidence.

Decisions

Relevance.

Expansion.

Dependency traversal.

Context retention.

Context eviction.

Task classification.

Retrieval depth.

Policy

Implement configurable thresholds.

Implement low-confidence fallback.

Implement deterministic safety overrides.

Log decision outcomes for evaluation.

Phase 9 — MCP

Define MCP server.

Add tpt_repo_overview.

Add tpt_find_symbol.

Add tpt_find_references.

Add tpt_get_signature.

Add tpt_get_skeleton.

Add tpt_expand.

Add tpt_dependencies.

Add tpt_related.

Add tpt_git_diff.

Add tpt_test_result.

Add tpt_context_stats.

Tool minimisation

Investigate dynamic tool exposure.

Avoid sending unnecessary schemas.

Measure MCP schema overhead.

Add compact tool descriptions.

Phase 10 — CLI

Implement:

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

Human-readable output.

JSON output.

Machine-readable compact output.

Exit codes.

Error reporting.

Phase 11 — Evaluation Harness

Baseline

Capture raw agent context.

Capture raw token count.

Capture model output.

Capture task success.

Capture latency.

Capture cost.

tpt-weave

Capture selected context.

Capture JEv context.

Capture decision overhead.

Capture total token count.

Capture task success.

Capture latency.

Capture cost.

Metrics

Gross token reduction.

Net token reduction.

Cache savings.

JEv overhead.

Retrieval count.

Context misses.

Task success preservation.

Latency change.

Cost change.

Phase 12 — First TPT Repository Integrations

tpt-code-command-center

Add tpt-weave integration.

Replace duplicated repository exploration.

Route context requests through tpt-weave.

Add token dashboard.

Measure baseline.

Measure post-integration.

tpt-infer

Index repository.

Test symbol retrieval.

Test dependency traversal.

Test model context.

tpt-uir

Evaluate as context transport/IR.

Avoid unnecessary duplicate representations.

tpt-raglite

Evaluate as optional semantic retrieval backend.

Compare structural retrieval vs semantic retrieval.

Avoid duplicating RAG functionality unnecessarily.

Phase 13 — Complex TPT Repositories

Computer vision

tpt-cv

tpt-math

Agriculture

tpt-teleop-agri

Cross-repository traversal test.

Media

tpt-kinetix

tpt-cadence

tpt-visual

tpt-audio

tpt-voice

tpt-av-control

tpt-av-asset

tpt-av-ui

tpt-av-sync

tpt-av-test

Semantic capture

tpt-sensetel

Ensure tpt-sensetel and tpt-weave remain separate responsibilities.

Phase 14 — Standard TPT Integration

Define .tpt-weave standard.

Define registry metadata.

Add TPT repository registry integration.

Add repository auto-discovery.

Add cross-repository dependency discovery.

Add standard agent configuration.

Add standard MCP configuration.

Add CI indexing.

Add index validation in CI.

Phase 15 — Performance

Benchmark indexing.

Benchmark incremental indexing.

Benchmark symbol lookup.

Benchmark context generation.

Benchmark JEv decisions.

Benchmark MCP.

Benchmark cache.

Profile memory.

Profile CPU.

Reduce allocations.

Add parallel indexing.

Add incremental graph updates.

Phase 16 — Privacy and Security

Secret detection.

.env exclusion.

credential exclusion.

certificate exclusion.

private-path configuration.

remote-context audit log.

explicit remote-data policy.

local-only mode.

test redaction.

document threat model.

Phase 17 — Optimisation Experiments

Experiment A — JEv relevance

Compare deterministic-only.

Compare deterministic + JEv.

Measure accuracy.

Measure token savings.

Experiment B — Hierarchical source

Metadata only.

Symbols.

Signatures.

Skeleton.

Targeted implementation.

Full source.

Experiment C — Tool output

Raw output.

Deterministic reduction.

Reduction + JEv.

Experiment D — Cross-repository retrieval

No traversal.

Full traversal.

deterministic traversal.

JEv-selected traversal.

Experiment E — Cache

No cache.

file cache.

context cache.

tool-result cache.

Phase 18 — 1B Tokens/Hour Workload Test

This is a high-priority real-world benchmark.

Record a representative coding session.

Capture baseline token volume.

Identify repeated context.

Identify repeated tool output.

Identify repository exploration overhead.

Identify unnecessary dependency traversal.

Run deterministic tpt-weave.

Add JEv.

Measure total token reduction.

Measure model quality.

Calculate actual dollar savings.

Determine whether context reduction is sufficient to materially extend the user's budget.

Target:

1.0B raw tokens/hour
        |
        v
< 250M delivered tokens/hour

This is an engineering target, not a guaranteed result.

Stretch target:

1.0B raw
    ->
< 100M delivered

Only pursue the stretch target if task success remains stable.

Phase 19 — Documentation

README.

Architecture guide.

Integration guide.

MCP guide.

JEv guide.

OpenRouter guide.

Repository author guide.

Privacy guide.

Benchmark methodology.

Token accounting guide.

Troubleshooting guide.

Phase 20 — Release

Stable core API.

Stable index format.

Stable MCP API.

Stable CLI.

OpenRouter adapter.

JEv integration.

Documentation complete.

CI complete.

Security review.

Performance review.

Real TPT benchmark published.

crates.io publication.

GitHub release.

Add project to TPT open-source catalogue.

Initial Implementation Order

Do not attempt the whole system at once.

Build this minimum viable path first:

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

Only after this works should advanced semantic retrieval, cross-repository optimisation, and sophisticated caching be added.

First Concrete Milestone

The first useful release should make this possible:

cd tpt-cv

tpt-weave init
tpt-weave index

tpt-weave context "fix the image resampling bug"

Expected result:

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

The actual reduction must be measured rather than assumed.