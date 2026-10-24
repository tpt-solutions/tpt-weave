# tpt-weave-mcp

[Model Context Protocol](https://modelcontextprotocol.io) server for
[tpt-weave](https://github.com/tpt-solutions/tpt-weave). It exposes repository context to MCP clients (Claude
Code, editors, agents) over stdio with deliberately compact tool schemas.

## Install and run

```sh
cargo install --path crates/tpt-weave-mcp   # produces the `tpt-weave-mcp` binary
tpt-weave-mcp                                # serves the repository in the current directory
```

Generate ready-made client configuration with `tpt-weave integration --write`; see
[docs/mcp.md](../../docs/mcp.md).

## Tools

| Group | Tools |
|---|---|
| Orient | `tpt_repo_overview`, `tpt_context_stats` |
| Symbol | `tpt_find_symbol`, `tpt_find_references`, `tpt_get_signature`, `tpt_dependencies`, `tpt_related` |
| Context | `tpt_get_skeleton`, `tpt_expand` (symbol / module / dependency / test / related, levels 0-5) |
| Workspace | `tpt_git_diff`, `tpt_test_result` |

Set `TPT_WEAVE_TOOLS` to expose a subset; fewer tools means fewer tokens spent on the tool list itself.

## Library API

`TptWeaveServer`, `Workspace`, `ToolFilter`, `ToolGroup`, `ToolSpec`, `all_tools`, `schema_overhead`,
`benchmark_tool` / `McpBenchmark` (per-tool latency and response size), `ToolError`, `WorkspaceError`.

## Design notes

- `#![forbid(unsafe_code)]`; built on [`rmcp`](https://crates.io/crates/rmcp) and `tokio`.
- Every response is built from the same deterministic engines as the CLI and honours the manifest's privacy
  and budget settings.

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
