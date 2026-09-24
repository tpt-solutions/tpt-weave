# tpt-weave Repository Integration

## Adoption

Run `tpt-weave adopt` in an existing Cargo repository for the complete local
onboarding path. It writes or validates configuration, indexes the workspace,
records `tpt-*` cross-repository links, writes a model-free baseline report, and
runs `doctor`. For a new repository use `tpt-weave init` followed by
`tpt-weave index`.

## Standard metadata

The `.tpt-weave/` directory is the standard local integration boundary:

- `manifest.toml` is the reviewed repository configuration;
- `graph.json` is the deterministic repository index;
- `registry.json` optionally maps related repository names to paths;
- `agent.json` and `mcp.json` are generated adapter contracts.

The registry is deliberately explicit. Cargo metadata can identify a `tpt-*`
dependency, but not reliably identify where an external checkout lives. Do not
invent paths; maintain them in registry metadata.

## Generated adapter configuration

Use `tpt-weave integration agent --write` and
`tpt-weave integration mcp --write` to generate local adapter files. The files
are templates for an agent/client, not a claim that every MCP client accepts
the same wrapper format. Review command paths and environment variables before
deployment.

## Cross-repository behavior

`integration registry` reports discovered dependency names. During indexing,
those names become graph external links. A future linked repository must have
its own valid index and compatible schema before cross-repository retrieval is
considered safe.
