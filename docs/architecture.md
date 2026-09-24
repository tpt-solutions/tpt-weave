# tpt-weave Architecture

## Layers

- **Core** (`tpt-weave-core`) defines repository, graph, context, privacy,
  manifest, and integration metadata types.
- **Indexing** (`tpt-weave-index`, `tpt-weave-rust`) projects Cargo metadata,
  Git state, and Rust source into deterministic records.
- **Graph** (`tpt-weave-graph`) builds and persists symbols, modules, crates,
  dependencies, references, and cross-repository links.
- **Context/tools/cache** provide hierarchical source representations,
  deterministic retrieval, output reduction, and revision-scoped caching.
- **Decisions** provide a provider-neutral JEv interface, local fallback,
  policy evaluation, and audit-friendly logging.
- **Adapters** (`tpt-weave-cli`, `tpt-weave-mcp`, `tpt-weave-openrouter`) expose
  the model to shell, agent, and remote decision clients.
- **Evaluation** (`tpt-weave-eval`) records baselines, experiments, benchmarks,
  and workload captures without fabricating model results.

## Data flow

1. `init` writes a validated manifest and the standard `.tpt-weave/` ignore
   entry.
2. `index` loads Cargo metadata, discovers the repository root, reads
   policy-aware Rust sources, parses files, and constructs a versioned graph.
3. `context` or an MCP tool selects the smallest useful representation and
   expands only when requested.
4. The filesystem cache is keyed by repository revision, schema, query, and
   policy; corrupt or foreign entries are discarded.
5. Remote decisions, when explicitly enabled, receive redacted structured
   requests and produce content-free audit records.

## Determinism and invalidation

Graph output is sorted before serialization. Parsing uses stable ordering, and
cached parse results are selected by source fingerprint. References are
recomputed when a graph changes, so an incremental update cannot retain an edge
whose target was removed. Git HEAD, working-tree content, and the local manifest
are part of workspace freshness.

## Privacy boundary

Source indexing is local by default. `.env`, credential, certificate, private
key, and configured private paths are excluded before reading. Remote provider
construction fails closed unless the privacy policy explicitly enables remote
decisions; request values are redacted and audit records contain metadata only.
See [privacy.md](privacy.md).
