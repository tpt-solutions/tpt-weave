# tpt-weave-graph

The repository symbol graph for [tpt-weave](https://github.com/tpt-solutions/tpt-weave): symbols, modules,
crates, dependencies and resolved references, with JSON persistence.

## What it provides

- `GraphBuilder` - builds a `RepositoryGraph` deterministically from a `tpt_weave_index::CargoIndex` plus files
  parsed by `tpt_weave_rust`.
- `RepositoryGraph` - symbol table, module graph, crate graph and dependency graph, plus queries for
  resolved references, callers/callees, type and trait-impl relationships, public API surface and
  cross-repository links.
- Node and edge types: `CrateNode`, `ModuleNode`, `ReferenceEdge`, `ReferenceKind`, `DependencyEdge`,
  `ExternalLink`.
- Persistence: `graph_path`, `GRAPH_FILE`, `RepositoryGraph::save` / `load`, and `GraphError`.

## Example

```rust,no_run
use tpt_weave_core::Revision;
use tpt_weave_graph::{GraphBuilder, RepositoryGraph};
use tpt_weave_index::CargoIndex;

let cargo = CargoIndex::load(".")?;
let graph = GraphBuilder::new("example", Revision::new("0123abcd"), cargo)
    // .add_file("package-name", parsed_file) for each parsed source file
    .build();
graph.save("graph.json")?;
let reloaded = RepositoryGraph::load("graph.json")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Invalidation

A persisted graph is reused only when both its schema version and the repository revision match; anything else
is treated as a miss. This is the conservative invalidation rule from spec section 21 - a stale graph is never
served.

## Design notes

- `#![forbid(unsafe_code)]`.
- All collections are ordered, so identical inputs produce byte-identical graph files.

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

MIT - see [LICENSE](../../LICENSE).
