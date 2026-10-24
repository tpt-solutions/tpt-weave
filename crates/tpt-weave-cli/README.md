# tpt-weave-cli

The `tpt-weave` command-line interface for [tpt-weave](https://github.com/tpt-solutions/tpt-weave).

## Install

```sh
cargo install --path crates/tpt-weave-cli   # produces the `tpt-weave` binary
```

## Quick start

```sh
cd my-rust-repo
tpt-weave adopt                                   # init + full index + doctor + baseline benchmark
tpt-weave context "fix the image resampling bug"  # task context within the configured budget
```

## Commands

| Command | Purpose |
|---|---|
| `adopt` | One-step onboarding: `init` + `index --full` + `doctor` |
| `init [--force]` | Write `.tpt-weave/manifest.toml` and `.gitignore` entries |
| `index [--full]` | Build or refresh the deterministic index |
| `doctor` | Validate manifest, index freshness, git and provider health |
| `overview` | Level-0 repository overview |
| `symbol <name>` / `refs <key>` | Symbol lookup and resolved references |
| `skeleton <path> [--level N]` | One file's hierarchical representation |
| `expand <id> [--level N] [--kind K] [--package P]` | Expand a symbol, module, dependency, test or related set |
| `context <task> [--budget N] [--max-level N] [--no-deps]` | Build task context within a token budget |
| `diff` | Reduced working-tree diff |
| `stats` | Token accounting and cache statistics |
| `cache` | Cache management (status by default) |
| `integration [--write]` | Emit or persist agent / MCP integration metadata |
| `workload <capture>` | Aggregate a workload capture into a report |
| `experiment ...` | Run benchmark and experiment matrices |

Run `tpt-weave --help` for the authoritative option list. Output is human-readable by default; `--json` and
`--compact` select machine-readable forms, and `--verbose` adds detail.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Internal error |
| 2 | Usage error |
| 3 | Not found |
| 4 | Stale index |
| 5 | Decision provider failure (only under `--strict-decisions`) |

## Library API

The `tpt_weave_cli` library exposes `run(args) -> RunOutcome` (captured stdout, stderr, exit code), plus `Cli`,
`Command`, `Format`, `Output`, `CliError` and `ExitCode`, so the CLI can be exercised in-process by tests.

See [docs/using-tpt-weave.md](../../docs/using-tpt-weave.md) for the full workflow.

## Status

Part of the [tpt-weave](https://github.com/tpt-solutions/tpt-weave) workspace (pre-alpha, version 0.1.0,
MSRV Rust 1.85, edition 2024). The crate is not yet published to crates.io (`publish = false`; see
[docs/release-readiness.md](../../docs/release-readiness.md)). Public APIs are stable-by-intent unless noted;
experimental APIs are gated behind the `unstable` feature where present (see
[docs/decisions.md](../../docs/decisions.md) section 6). Changes are tracked in [CHANGELOG.md](CHANGELOG.md).

## Related documentation

- [Architecture](../../docs/architecture.md) - crate layering and data flow
- [Design specification](../../spec.md)
- [Using tpt-weave](../../docs/using-tpt-weave.md)

## License

Licensed under either of [Apache License, Version 2.0](../../LICENSE-APACHE) or [MIT license](../../LICENSE-MIT), at your option.
