# tpt-weave-index

Deterministic Cargo and Git repository indexing for
[tpt-weave](https://github.com/tpt-solutions/tpt-weave).

No language model is involved: everything here is projected from `cargo metadata --no-deps` and the `git`
command line into stable, serialisable models.

## What it provides

### `cargo` - workspace structure

- `CargoIndex` - the whole workspace: packages, workspace members, lookup by name
  (`package`, `workspace_packages`).
- `PackageIndex`, `TargetIndex` - packages, library/binary/test/bench targets, features.
- `DependencyIndex`, `DependencyKind` - normal/dev/build dependencies, optional dependencies.
- `CargoIndex::discover_tpt_dependencies` - finds dependencies on other TPT repositories, feeding
  cross-repository links.
- `IndexError` - failures to run or parse `cargo metadata`.

### `git` - repository state

- `GitRepository` - root discovery, current revision, working-tree status, current diff, changed-file queries
  and history lookup.
- `FileStatus`, `ChangeStatus`, `DiffEntry`, `CommitInfo` - typed results.
- `GitError` - missing git, non-repository, or command failures.

## Example

```rust,no_run
use tpt_weave_index::{CargoIndex, GitRepository};

let git = GitRepository::discover(".")?;
for file in git.status()? {
    println!("{file:?}");
}

let index = CargoIndex::load(".")?;
for package in index.workspace_packages() {
    println!("{}", package.name);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Requirements

`cargo` and `git` must be available on `PATH` at runtime.

## Design notes

- `#![forbid(unsafe_code)]`.
- Output ordering is sorted and stable so downstream cache keys and graphs are reproducible.

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
