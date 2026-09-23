# tpt-weave

Cross-repository AI-context optimisation for the TPT Solutions ecosystem.

tpt-weave is the reusable AI-context infrastructure layer for TPT Solutions repositories. It indexes Rust
workspaces deterministically, builds hierarchical source representations, and delivers only the context a
coding task needs — so expensive models such as GLM-5.3-Flash receive far fewer tokens without losing the
information required to complete the task correctly.

> **Status:** pre-alpha. The [design specification](spec.md) is *Proposed*. Phases 0–9 of
> [todo.md](todo.md) are complete (repository scaffold, architecture decisions, core types and
> configuration, indexer, symbol graph, hierarchical representations, deterministic context
> retrieval, tool output reduction, filesystem cache, JEv decision provider + OpenRouter client +
> policy engine, MCP server with 11 tools and tool minimisation). All other phases are pending.

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
| `crates/tpt-weave-cli` … `crates/tpt-weave-eval` | planned | CLI, eval (spec §25) |

## Quick start

Development (current):

```sh
cargo test --workspace
```

Planned CLI (UX defined in [docs/decisions.md](docs/decisions.md), implementation is Phase 10):

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

## Toolchain

- **MSRV:** Rust 1.85 (edition 2024), enforced via `rust-version` in the workspace manifest
- **Edition:** 2024
- **License:** MIT (see [LICENSE](LICENSE))
