# Phase 15 Performance and Phase 17 Experiment Reports

tpt-weave records measurements as data. It does not turn an unmeasured design
variant into a claimed result.

## Benchmark APIs

- `tpt_weave_eval::benchmark_decision_provider` measures provider latency,
  errors, and reported input tokens without changing policy/fallback behavior.
- `tpt_weave_eval::benchmark_cache` measures a cache miss/store followed by hits.
- `tpt_weave_mcp::benchmark_tool` measures repeated MCP tool dispatch.
- `tpt_weave_eval::hierarchy_measurements` records metadata, symbols,
  signatures, skeleton, targeted implementation, and full-source token variants.
- `hierarchy_measurements_with_selection` permits an explicitly measured level-4 selection.
- `tpt_weave_eval::tool_output_measurements` records raw, deterministic, and
  optional JEv-augmented tool-output variants.
- `VariantMeasurement` and `ExperimentSuite` carry accuracy, JEv overhead,
  latency, and arbitrary measurement metadata for experiments A, D, and E.

## Experiment matrix

Experiment A — JEv relevance: compare deterministic-only with
 deterministic + JEv; record accuracy and JEv input tokens.

Experiment B — hierarchical source: use `hierarchy_measurements` for all six
levels.

Experiment C — tool output: use `tool_output_measurements`; the JEv variant is
only included when a measured JEv input-token count is supplied.

Experiment D — cross-repository retrieval: provide measured no-traversal,
full-traversal, deterministic-traversal, and JEv-selected-traversal cases to
`comparison_suite`.

Experiment E — cache: provide measured no-cache, file-cache, context-cache, and
tool-result-cache cases to `comparison_suite`; `benchmark_cache` supplies a
repeatable local cache baseline.

## Local commands

Use the debug binary for comparable local measurements. The CLI already emits
`elapsed_ms` for `index`, `symbol`, and `context`; rebuilt indexes also expose
`parse_cache` hit/miss/store counters. Run the command several times and record
the machine, revision, build profile, and sample count with the report.

The current workspace does not contain a live JEv corpus, a production MCP
workload capture, or a platform-independent memory profiler. Those measurements
remain data-collection work; the report types make the missing inputs explicit.
