# tpt-weave MCP Guide

## Launch

`tpt-weave-mcp` is a stdio MCP server. Set `TPT_WEAVE_PATH` to the repository
root and launch the binary with stdin/stdout connected to the MCP client:

```sh
TPT_WEAVE_PATH=/absolute/path/to/repository tpt-weave-mcp
```

On Windows, set the environment variable in the client process configuration or
PowerShell before launching the executable.

## Tool exposure

By default the catalog is available. Set `TPT_WEAVE_TOOLS` to a comma-separated
subset of `orient`, `symbol`, `context`, and `workspace` to reduce schema
overhead. `integration agent` and `integration mcp` generate these standard
values for an adapter.

The server refreshes after Git HEAD, working-tree content, or manifest changes.
Restart the process after changing its environment or tool filter.

## Failure handling

Treat tool errors as structured errors, retry only when the server identifies a
transient condition, and never place API keys in tool arguments. Use the CLI
`doctor` command for manifest, graph, and provider checks.
