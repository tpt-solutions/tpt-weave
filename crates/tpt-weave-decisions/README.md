# tpt-weave-decisions

Decision-provider abstraction, JEv categories, policy engine and evaluation log for
[tpt-weave](https://github.com/tpt-solutions/tpt-weave).

Deterministic algorithms run first; a decision provider (JEv, via OpenRouter) only makes small *structured*
choices over the already-reduced candidate set. This crate keeps the application decoupled from any backend
and guarantees that a provider failure can never fail a task.

## What it provides

- `DecisionProvider` - the trait every backend implements. Built in: `MockProvider` (tests) and
  `DeterministicFallbackProvider` (always available, confidence `FALLBACK_CONFIDENCE`).
- `DecisionCategory` - `Relevance`, `Expansion`, `DependencyTraversal`, `ContextRetention`, `ContextEviction`,
  `ToolResultRetention`, `TaskClassification`, `RetrievalDepth`.
- `DecisionRequest`, `Decision`, `DecisionOutcome` - provider-neutral request and response types.
- `DecisionPolicy`, `Judgement`, `SafetyOverride`, `DecisionSource` - confidence threshold, safety overrides
  and low-confidence fallback.
- `DecisionEngine`, `DecisionLog`, `DecisionRecord` - runs every decision through the policy and records the
  outcome (`DecisionLog::write_jsonl` for evaluation). **The engine never fails**: provider errors
  (`ProviderError`) degrade to the deterministic fallback.

## Example

```rust,no_run
use tpt_weave_decisions::{
    DecisionCategory, DecisionEngine, DecisionPolicy, DeterministicFallbackProvider,
};

# let provider = DeterministicFallbackProvider::new();
let engine = DecisionEngine::new(provider, DecisionPolicy::new(0.7));
let judgement = engine.decide(DecisionCategory::Relevance, "fn resample", "task: fix resampling");
println!("{judgement:?}");
println!("{} decision(s) logged", engine.log().entries().len());
```

## Related

- [`tpt-weave-openrouter`](../tpt-weave-openrouter) - the real JEv backend.
- [docs/jev.md](../../docs/jev.md) - decision policy and failure semantics.

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
