# tpt-weave-context

Hierarchical context representations, expansion and deterministic retrieval for
[tpt-weave](https://github.com/tpt-solutions/tpt-weave). This is the crate that decides *how much* of a
repository a model gets to see.

## The six levels

| Level | Representation | Source |
|---|---|---|
| 0 | Repository metadata and overview | symbol table |
| 1 | Symbol listing | symbol table |
| 2 | Signatures | symbol table |
| 3 | Skeleton (bodies elided) | `tpt_weave_rust::skeleton_with` |
| 4 | Targeted implementation (selected bodies kept) | `tpt_weave_rust::skeleton_with` |
| 5 | Complete source | file contents |

Start low and expand only when a task needs more detail.

## What it provides

- `represent_file`, `FileRepresentation`, `Selection`, `estimate_tokens` - render a file at a chosen level.
- Expansion - `expand_symbol`, `expand_module`, `expand_dependency`, `expand_test`, `expand_related`
  (and `related_keys`, `test_keys`) walk the graph to choose which bodies to keep, returning an `Expansion`.
- `Retriever` - deterministic, budget-aware retrieval: `find_symbols`, `find_references`, `find_dependencies`,
  `find_related`, `find_tests`, `find_changed`, `retrieve`, `repository_overview`.
- `RepositoryOverview` - the level-0 summary.
- `SourceProvider`, `Sources`, `LazySources` - where file contents come from (in-memory or lazy from disk,
  with optional privacy filtering).
- `ContextError` - unknown symbol/package/module, missing source, parse failure.

## Example

```rust,no_run
use tpt_weave_context::{Retriever, Sources};
use tpt_weave_graph::RepositoryGraph;

let graph = RepositoryGraph::load(".tpt-weave/graph.json")?;
let sources = Sources::load_dir(".")?;
let retriever = Retriever::new(&graph, &sources);

println!("{}", retriever.repository_overview().text());
for symbol in retriever.find_symbols("resample") {
    println!("{}", symbol.signature);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Design notes

- `#![forbid(unsafe_code)]`.
- Retrieval is fully deterministic: the same graph and request always yield the same selection, which is what
  makes caching and benchmarking meaningful.

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
