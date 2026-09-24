# Phase 12 — tpt-code-command-center integration

Date: 2026-09-24

Command Center is the user-facing coding environment/orchestrator (spec §32);
tpt-weave supplies its repository-context intelligence. Integrated at
spec §12 **Level B** (agent/CLI routing) — no dependency on tpt-weave code,
only the CLI binary.

## What changed (tpt-code-command-center)

- `src/modules/weave.ts` — new module: detects `.tpt-weave/manifest.toml` in
  workspace roots, shells out to `tpt-weave --path <root> skeleton <file>
  --json [--level N]`, LRU-caches skeletons per file, accumulates session
  savings, exposes `tpt-weave stats --json` for the dashboard, and a
  `weaveDashboardSnapshot` for the UI. Every failure path returns undefined
  so the proxy falls back — the pipeline never breaks on CLI absence.
- `src/modules/smartContext.ts` — `runSmartContext` now takes an optional
  weave resolver; `.rs` tool results inside an indexed workspace get their
  representation from the index (counted as `weaveReplaced`), everything
  else keeps the tree-sitter/regex outline.
- `src/proxy/pipeline.ts` — wires the query from config; emits
  `weave:skeleton=N` in module actions (visible in TPT Inspect).
- `src/ui/dashboard.ts` — Weave module row + "Repository Context
  (tpt-weave)" panel: repository/files/symbols, raw vs delivered tokens,
  reduction %, index cache hit rate, session replacements.
- Settings: `tpt.weave.enabled` (default **false** — opt-in, zero change for
  existing users), `tpt.weave.cliPath`, `tpt.weave.level` (default 2).
- Tests: `src/test/weave.test.js` (10 cases: root detection, relativisation,
  caching, fallbacks, level forwarding, smart-context routing); suite is
  58/58 green (`npm test`). `scripts/measure-weave.js` measures baseline vs
  post-integration over a real indexed repository.

This removes the duplicated repository exploration the proxy was doing for
Rust (its own tree-sitter outlines) in favour of the shared index, per
spec §32 "Command Center should not duplicate repository indexing logic".

## Measured baseline vs post-integration (tpt-infer, 12 largest .rs files, 304 KB)

| Leg | Delivered | Reduction vs raw | Notes |
|---|---|---|---|
| raw (no optimisation) | 77,728 tokens | — | full file contents |
| baseline (tree-sitter outline) | 5,293 tokens | 93.2 % | declaration lines truncated at 120 chars |
| post-integration (weave, level 2 signatures) | 6,782 tokens | 91.3 % | exact, untruncated signatures from the indexed parse; 12/12 hits, 0 fallbacks |
| post-integration (weave, level 3 skeleton) | 28,286 tokens | 62.9 % | full signatures + types + attributes; `tpt.weave.level: 3` |

Reading: at the default signatures level the index costs ≈ the same tokens
as the built-in outline (+1.5 k tokens over 12 files) and buys exact
signatures, cross-crate graph context (`context`, `expand`), and a cached
repeat path (second lookup of a file is free). Level 3 is the
high-fidelity option when structure matters more than tokens.

Whole-repository context numbers (from `tpt-weave adopt` on tpt-infer,
1022 symbols / 74 files): 144,038 raw → 11,983 delivered tokens, 91.7 % net
reduction — that is the ceiling Smart Context cannot reach with per-file
outlines alone because it never selects across files.

Cold-start latency: ~0.8 s per skeleton (CLI loads index + sources per
invocation; 12 parallel ≈ 10 s worst case). Mitigations already in place:
skeleton LRU cache per session, 60 s TTL on stats; follow-up in tpt-weave
backlog: a long-lived serve mode to amortise index loading (Phase 15).

## Baseline measurement tooling

`npm run compile && node scripts/measure-weave.js <indexed-repo> <cli-path>`
prints both legs as JSON rows plus a summary; run against any `tpt-weave
adopt`-ed repository. The dashboard's session counters update live from the
same data source.
