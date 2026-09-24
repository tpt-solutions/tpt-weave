# tpt-weave

Cross-repository AI-context optimisation for the TPT Solutions ecosystem.

tpt-weave is the reusable AI-context infrastructure layer for TPT Solutions repositories. It indexes Rust
workspaces deterministically, builds hierarchical source representations, and delivers only the context a
coding task needs — so expensive models such as GLM-5.3-Flash receive far fewer tokens without losing the
information required to complete the task correctly.

> **Status:** pre-alpha. The [design specification](spec.md) is *Proposed*. Phases 0–17 are implemented,
> with Phase 15 performance work, Phase 16 privacy/security controls, and Phase 17 experiment/benchmark
> APIs. Phase 18 workload capture/reporting infrastructure is implemented; live JEv/production workload
> measurements and Phases 18–20 conclusions remain pending.

## Core principle

> Do not compress information merely because it is large. Determine what information is necessary, expose
> the smallest useful representation first, and retrieve more detail only when required.

Deterministic algorithms run first; JEv (via OpenRouter) only makes small structured decisions over the
already-reduced candidate set. See [spec.md](spec.md) §4.

## Workspace layout

Start as one repository with a workspace (spec §25):

| Crate | Status | Purpose |
|---|---|---|
| `crates/tpt-weave-core` | Phase 1 done | Core types, token accounting, `.tpt-weave/manifest.toml` configuration |
| `crates/tpt-weave-index` | Phase 2 done | Cargo metadata + git indexing (status, diff, history) |
| `crates/tpt-weave-rust` | Phase 2 done | Rust source extraction via syn (symbols, signatures, locations, skeletons) |
| `crates/tpt-weave-graph` | Phase 3 done | Symbol/module/crate/dependency graph, references, persistence |
| `crates/tpt-weave-context` | Phases 4–5 done | Hierarchical representations (levels 0–5), expansion, deterministic retrieval |
| `crates/tpt-weave-tools` | Phase 6 done | Deterministic tool-output reduction (cargo/git/tests/diffs/logs/JSON) with raw storage |
| `crates/tpt-weave-cache` | Phase 7 done | Filesystem cache: revision/schema keys, invalidation, clear, statistics |
| `crates/tpt-weave-decisions` | Phase 8 done | DecisionProvider trait, mock/fallback providers, JEv categories, policy engine, decision log |
| `crates/tpt-weave-openrouter` | Phase 8 done | OpenRouter Decisions API (JEv) blocking client: retries, timeouts, usage/confidence |
| `crates/tpt-weave-mcp` | Phase 9 done | MCP server (stdio): 11 tools, dynamic tool filter, compact schemas |
| `crates/tpt-weave-cli` | Phase 10 done | `tpt-weave` binary: adopt/init/index/doctor/overview/symbol/refs/expand/context/diff/stats/cache |
| `crates/tpt-weave-eval` | Phases 11/15/17/18 infra | Eval harness, benchmark primitives, experiment matrices, workload capture/reporting (spec §23) |

## Quick start

Development (current):

```sh
cargo test --workspace
```

Planned CLI (UX defined in [docs/decisions.md](docs/decisions.md), implemented in Phase 10):

```sh
cd tpt-cv
tpt-weave init
tpt-weave index
tpt-weave context "fix the image resampling bug"
```

Or, for an existing repository, `adopt` in one step:

```sh
cd tpt-cv
tpt-weave adopt
```

## Documentation

- [spec.md](spec.md) — design specification
- [todo.md](todo.md) — phased implementation plan
- [docs/decisions.md](docs/decisions.md) — architecture decisions: MSRV, Rust editions, initial CLI UX,
  schema versioning, privacy/redaction policy, stable vs experimental APIs
- [docs/using-tpt-weave.md](docs/using-tpt-weave.md) — practical setup and daily-use guide for new and existing repositories
- [docs/architecture.md](docs/architecture.md) — crate/data-flow architecture
- [docs/integration.md](docs/integration.md) — standard repository and adapter integration
- [docs/mcp.md](docs/mcp.md) — MCP server and client configuration
- [docs/jev.md](docs/jev.md) — JEv decision policy and failure semantics
- [docs/openrouter.md](docs/openrouter.md) — OpenRouter adapter configuration
- [docs/repository-author-guide.md](docs/repository-author-guide.md) — repository author workflow
- [docs/token-accounting.md](docs/token-accounting.md) — token accounting methodology
- [docs/troubleshooting.md](docs/troubleshooting.md) — common failures and remedies
- [docs/privacy.md](docs/privacy.md) — privacy policy, redaction, and threat model
- [docs/benchmark-methodology.md](docs/benchmark-methodology.md) — benchmark and workload measurement methodology
- [docs/security-review.md](docs/security-review.md) — repository-local security/privacy review
- [docs/performance-review.md](docs/performance-review.md) — measured performance review and bottlenecks
- [docs/release-readiness.md](docs/release-readiness.md) — local release gates, publication order, and external blockers
- [docs/experiments.md](docs/experiments.md) — performance benchmarks and Phase 17 experiment matrices
- [docs/workload.md](docs/workload.md) — Phase 18 workload capture and report format

## Toolchain

- **MSRV:** Rust 1.85 (edition 2024), enforced via `rust-version` in the workspace manifest
- **Edition:** 2024
- **License:** MIT (see [LICENSE](LICENSE))
