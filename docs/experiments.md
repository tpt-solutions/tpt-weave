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
- `cache_measurements` and `cross_repository_measurements` require all four
  measured variants explicitly; they do not synthesize JEv or cache hits.
- `VariantMeasurement::with_accuracy` records an externally measured result
  without implying a quality claim.
- `tpt_weave_eval::VariantMeasurement` and `ExperimentSuite` carry accuracy,
  JEv overhead, latency, and arbitrary measurement metadata for experiments A,
  D, and E. `ExperimentSuite::save`/`load` and `from_json` validate the report
  schema, unique variant names, and supplied accuracy ranges.

## Local accuracy corpus

The checked-in `docs/accuracy-corpus.example.json` is a small labeled fixture for
validating the experiment pipeline. Evaluate it with the deterministic fallback
provider:

```sh
tpt-weave --json experiment accuracy docs/accuracy-corpus.example.json
```

This command reports corpus size, correct/incorrect/error counts, and accuracy.
It is a reproducible local measurement, not a live OpenRouter/JEv quality
claim. A real labeled corpus should be reviewed for privacy and task
representativeness before using its result in a release decision.

## Experiment matrix

Experiment A — JEv relevance: compare deterministic-only with
 deterministic + JEv; record accuracy and JEv input tokens. The local
 labeled-corpus path is available as `tpt-weave experiment accuracy <corpus.json>`
 and deliberately evaluates the deterministic fallback provider; it is not a
 live JEv quality claim.

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

Use the debug binary for comparable local measurements. The CLI emits
`elapsed_ms` for `index`, `symbol`, and `context`; rebuilt indexes also expose
`parse_cache` hit/miss/store counters. A normal `index` reuses a graph only when
HEAD, the working tree, and the local manifest are unchanged; uncommitted edits
and privacy-configuration changes enter the incremental rebuild path. Run the
command several times and record the machine, revision, build profile, and sample
count with the report.

The current workspace does not contain a live JEv corpus, a production MCP
workload capture, or a platform-independent memory profiler. Those measurements
remain data-collection work; the report types make the missing inputs explicit.
`ExperimentSuite::from_json`, `load`, and `save` validate report schema, unique
variant names, and supplied accuracy ranges before evidence is reused.
See [workload.md](workload.md) for the Phase 18 capture format and
[release-readiness.md](release-readiness.md) for the local/external gate split.
