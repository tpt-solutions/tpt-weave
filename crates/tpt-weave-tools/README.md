# tpt-weave-tools

Deterministic tool-output reduction for [tpt-weave](https://github.com/tpt-solutions/tpt-weave). Verbose
command output goes in; a compact record that keeps the facts and failure details comes out.

Raw output is never lost: callers keep it (optionally via `RawStore`) and can recover it with
`Reduction::expand`. Every `Reduction` measures its own token reduction.

## Supported tool kinds

`ToolKind` covers `CargoCheck`, `CargoTest`, `CargoClippy`, `CargoBuild`, `CargoFmt`, `GitStatus`, `GitDiff`,
`FileListing`, `Search`, `Json`, `Diagnostics` and `Log`, each with a dedicated reducer.

## What it provides

- `reduce(kind, &ToolOutput) -> Reduction` - the single entry point.
- `ToolOutput` - command, stdout, stderr and exit code (`with_stderr`, `with_exit_code`).
- `Reduction`, `ReductionStatus` - reduced `lines`, `status`, `raw_tokens` / `reduced_tokens`, `reduction()`
  ratio, `failed()`, `render()`, and `expand()` back to the raw output.
- `RawStore` - keeps raw output on disk so a reduction can point back to it (`store_raw`).

## Example

```rust
use tpt_weave_tools::{ToolKind, ToolOutput, reduce};

let output = ToolOutput::new(
    "cargo test",
    "running 120 tests\n...\ntest result: ok. 120 passed; 0 failed",
);
let reduction = reduce(ToolKind::CargoTest, &output);
println!("{}", reduction.render());
println!("{} -> {} tokens", reduction.raw_tokens, reduction.reduced_tokens);
```

## Design notes

- `#![forbid(unsafe_code)]`.
- No model calls: reduction is a pure function of the input.
- Failures are never dropped.

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
