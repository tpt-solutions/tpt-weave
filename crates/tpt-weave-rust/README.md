# tpt-weave-rust

Rust source extraction for [tpt-weave](https://github.com/tpt-solutions/tpt-weave), built on
[`syn`](https://crates.io/crates/syn).

Turns `.rs` files into structured symbols and compact skeletons, without compiling anything.

## What it provides

- **Symbol extraction** - `parse_file` / `parse_files` return a `ParsedFile` of `SymbolRecord`s covering
  modules, structs, enums, traits, impl blocks, functions, methods, constants, type aliases and macros, with
  `Visibility`, single-line signatures, attributes and source locations.
- **Mentions** - `Mention` / `MentionKind` identifier references used to build the symbol graph.
- **Skeletons** - `skeleton` and `skeleton_with` render a file with function bodies replaced by
  `BODY_PLACEHOLDER`, optionally keeping selected bodies (the basis of context levels 3 and 4).
- **Parse cache** - `ParseCache` is a fingerprint-keyed cache (`PARSE_CACHE_SCHEMA`) so unchanged files are not
  reparsed; `ParseCacheStats` reports hits and misses.
- **Parallel parsing** - `parse_files` fans out across worker threads while keeping output order deterministic.
- `ParseError` carries the message plus line and column.

## Example

```rust
use tpt_weave_core::RepositoryId;
use tpt_weave_rust::{FileInput, parse_file, skeleton};

let repository = RepositoryId::new("example");
let input = FileInput {
    repository: &repository,
    package: "example",
    path: "src/lib.rs",
    module_prefix: &[],
};
let source = "pub fn add(a: i32, b: i32) -> i32 { a + b }";

let parsed = parse_file(&input, source)?;
println!("{} symbol(s)", parsed.symbols.len());
println!("{}", skeleton(&input, source)?.text);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Known limitations

- An out-of-line `mod x;` is recorded but its file is not loaded here; module-tree resolution happens when the
  indexer walks a whole crate.
- A method id (`Type::method`) may coincide when two trait impls for one type declare the same method name;
  `SymbolRecord::parent` (the enclosing impl's canonical key) disambiguates.
- Only Rust is supported at this stage.

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
