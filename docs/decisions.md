# tpt-weave — Architecture Decisions

Phase 0 decisions required by [todo.md](../todo.md). Each entry records the decision and its consequences.
Spec references point to [spec.md](../spec.md).

## 1. MSRV (Minimum Supported Rust Version)

**Decision:** MSRV is **Rust 1.85.0**.

- Enforced with `rust-version = "1.85"` in `[workspace.package]`; `cargo` fails fast on older toolchains.
- The environment toolchain (1.97.x) is used for development; CI (Phase 20) will run an MSRV job.
- MSRV bumps are explicit release notes events: minor bumps pre-1.0, major bumps post-1.0.

**Rationale:** 1.85 is the first stable release of the 2024 edition line, giving a modern baseline
(async traits, `OnceLock`, current trait upcasting) without chasing the latest compiler.

## 2. Supported Rust editions

**Decision:** all workspace crates build with **edition 2024 only**.

- No support burden for 2015/2018/2021 workspace code; the MSRV (1.85) already implies edition 2024.
- Nightly-only features are forbidden (`#![forbid(unsafe_code)]` where practical).
- Downstream crates on older editions may still depend on published tpt-weave libraries; Rust editions
  are per-crate and do not constrain consumers.

## 3. Initial CLI UX

**Decision:** subcommand CLI, human-readable output by default, machine-readable via flags.

```text
tpt-weave adopt                     # onboard an existing repo: init + index --full + register + doctor
tpt-weave init                      # write .tpt-weave/manifest.toml (idempotent, --force to overwrite)
tpt-weave index [--full]            # build/refresh the deterministic index
tpt-weave doctor                    # validate manifest, index freshness, git and provider health
tpt-weave overview                  # level-0 repository overview
tpt-weave symbol <name>             # symbol lookup
tpt-weave refs <name>               # references to a symbol
tpt-weave expand <context-id>       # next representation level / full source for a context id
tpt-weave context "<task>"          # build task context within the configured budget
tpt-weave diff                      # reduced working-tree diff
tpt-weave stats                     # token accounting + cache statistics
tpt-weave cache [status|clear]      # cache management (default: status)
```

- `adopt` is sugar for `init` + `index --full` + graph registration + `doctor`, run in
  sequence — it must not duplicate the Cargo/workspace detection logic those commands
  already implement. Its graph-registration and benchmark steps are no-ops until Phase 3
  (graph) and Phase 11 (eval harness) exist; implement it last among the subcommands, once
  the commands it composes are working (spec §28).

**Global flags:** `--path <dir>` (operate on another working tree), `--json` (pretty JSON),
`--compact` (single-line JSON, implies `--json`), `--no-color`, `--verbose`.

**Exit codes:**

| Code | Meaning |
|---|---|
| 0 | success |
| 1 | internal/unexpected error |
| 2 | usage error (bad flags or arguments) |
| 3 | not found (symbol, context id, manifest missing) |
| 4 | stale or invalid index (re-run `tpt-weave index`) |
| 5 | decision provider failure under `--strict-decisions` (without it, JEv failure falls back to deterministic ranking and still exits 0 — spec §22) |

## 4. Schema versioning strategy

**Decision:** integer `schema` field, single constant in `tpt-weave-core`, mismatch fails closed.

- `tpt_weave_core::SCHEMA_VERSION: u32` (currently `1`) is written into `.tpt-weave/manifest.toml` and every
  generated index artifact.
- **Adding** optional fields with serde defaults does **not** bump the schema; **removing/renaming fields or
  changing their meaning** does.
- On mismatch, readers reject with a message telling the user to re-run `tpt-weave init` / `tpt-weave index`.
  No migrations exist before 1.0 (spec §21: conservative invalidation).
- Cache keys always include the schema (spec §15).
- Crate semver is independent of the schema: a schema bump is at least a minor crate bump pre-1.0, a major
  bump post-1.0. CLI, MCP surface and index schema are versioned separately from the library API (Phase 20).

## 5. Privacy / redaction policy

**Decision:** local-only by default; secrets are excluded before anything may leave the machine.

- Default `[privacy] remote_decisions = false` — fully local operation unless a repository opts in
  (spec §27 `local-only mode`).
- Conservative default exclusions: `.env`, `.env.*`, `secrets/`, `credentials/`, `*.pem`, `*.key`,
  `*.crt`, `*.p12`, `id_rsa*`. Pattern forms: exact path segment, `dir/` (any directory of that name),
  `*.ext`, `prefix*`.
- API keys are **never** stored in the manifest — only the environment variable *name*
  (`[provider] api_key_env`, default `OPENROUTER_API_KEY`).
- Before any remote decision call: apply exclusions, redact environment/credential-looking values, and
  append a content-free audit entry (repository, item count, token count) when remote decisions are enabled.
- `.tpt-weave/` is gitignored by default; the manifest itself is configuration only (no prose, no secrets)
  and may be committed if a repository wants shared defaults (spec §6).

## 6. Stable vs experimental APIs

**Decision:** intent-based stability now, hard gates before release (Phase 20).

- **Stable-by-intent** (regular semver discipline, no breaking changes without a version bump):
  `tpt_weave_core::{ids, files, context, tokens, config}` — the Phase 1 core.
- **Experimental:** anything new and not yet exercised by two downstream consumers (MCP schema, decisions
  API surface, cache internals, CLI flags). Experimental Rust APIs live behind the `unstable` cargo
  feature (declared, default off) and are documented as experimental.
- Pre-1.0: breaking changes to stable-by-intent APIs bump the minor version and are listed in release
  notes; post-1.0 they require a major bump.
- Non-Rust surfaces follow §4: index/MCP/CLI each carry their own version, independent of crate semver.

## 7. Decision provider is synchronous (blocking)

**Decision:** `DecisionProvider::decide` is a blocking call (`reqwest::blocking`); no async runtime
in the decision path (Phase 8).

- `tpt-weave-openrouter` depends on `reqwest` with `default-features = false` +
  `blocking`/`json`/`rustls-tls`; no tokio/hyper runtime is pulled into the library surface.
- Spec §18's provider sketch is async; the deviation is deliberate — decisions are single short
  HTTP calls (default timeout 5 s, `jev.timeout_ms`), made from sync code paths (CLI, retrieval,
  MCP handlers). Async can be layered on later without changing the trait consumers see if we
  introduce a separate `AsyncDecisionProvider`.
- Retries: exponential backoff from a configurable base (`.with_backoff`, tests use zero) over
  transport failures and retryable statuses (429, 500, 502, 503, 524, 529); 400/401/402/403/404
  fail immediately. API keys are read from the env var named by `[provider] api_key_env` only
  (§5: never stored in the manifest).
