# tpt-weave-core

Core repository model, token accounting, privacy primitives and manifest configuration for
[tpt-weave](https://github.com/tpt-solutions/tpt-weave), the cross-repository AI-context optimisation layer.

Every other tpt-weave crate depends on this one. It has no knowledge of Rust parsing, git or networking - only
the shared vocabulary and the on-disk contract.

## What it provides

| Module | Contents |
|---|---|
| `config` | `Manifest` for `.tpt-weave/manifest.toml`: `ContextConfig` (budgets), `FeaturesConfig`, `PrivacyConfig`, `JevConfig`, `ProviderConfig`; `discover_repository_root`, `manifest_path` |
| `context` | `ContextLevel` (levels 0-5), `ContextRequest`, `ContextResponse`, `ContextCandidate`, `ContextSource` |
| `ids` | Typed identifiers: `RepositoryId`, `Revision`, `SymbolId`, `SymbolKind`, `ContextId` |
| `files` | `FileRecord`, `SourceLanguage` |
| `tokens` | `TokenAccounting` - raw vs selected tokens, saved tokens, selection ratio, JEv/model token use, cache hit |
| `hash` | Stable FNV-1a 64 hashing (`fnv1a64`, `fnv1a64_hex`) used for cache keys and fingerprints |
| `privacy` | `redact_secrets`, `Redaction`, `SecretFinding`, `SecretKind`, `REDACTION_MARKER` |
| `integration` | Standard `.tpt-weave` layout: `standard_path`, `RepositoryRegistry`, `RegistryEntry`, `McpServerConfig`, `AgentEnvironment` |

The crate-level constant `SCHEMA_VERSION` is written into the manifest and every generated index artifact;
the bump policy lives in [docs/decisions.md](../../docs/decisions.md) section 4.

## Example

```rust
use tpt_weave_core::{Manifest, redact_secrets};

// Defaults apply for any field the manifest omits.
let manifest = Manifest::default();
println!("default budget: {} tokens", manifest.context.default_budget);

// Redact secrets before any text leaves the process.
let redaction = redact_secrets("api_key = \"sk-live-1234567890abcdef\"");
println!("{} finding(s): {}", redaction.count(), redaction.text);
```

## Features

| Feature | Default | Purpose |
|---|---|---|
| `unstable` | off | Gates experimental APIs that may change without a semver bump |

## Design notes

- `#![forbid(unsafe_code)]`.
- Deterministic by construction: hashing and serialisation never depend on iteration order or wall-clock time.
- Privacy-first: remote providers are opt-in and the redaction engine is available to every crate.

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
