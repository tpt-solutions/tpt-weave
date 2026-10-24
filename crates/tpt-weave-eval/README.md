# tpt-weave-eval

Evaluation, benchmarking and workload-measurement infrastructure for
[tpt-weave](https://github.com/tpt-solutions/tpt-weave) (spec section 23). It answers one question with
numbers: did the model get fewer tokens *without* getting a worse result?

## Metrics

- **Net Token Reduction** (primary) - tokens saved versus a full-source baseline.
- **Task Success Preservation** (secondary) - whether tasks that succeeded at baseline still succeed with weave.

## What it provides

| Area | Items |
|---|---|
| Captures | `BaselineCapture`, `WeaveCapture` - raw vs selected context, model output, success, latency, cost |
| Metrics and reports | `TaskMetrics`, `AggregateMetrics`, `TaskEvaluation`, `EvalReport` (`EVAL_REPORT_SCHEMA`) |
| Local harness | `local_comparison`, `local_report`, `local_aggregate`, `full_source_tokens` - model-free baseline vs weave over an indexed repo (used by `tpt-weave adopt`) |
| Benchmarks | `BenchmarkReport`, `BenchmarkSample`, `benchmark_cache`, `benchmark_decision_provider` |
| Experiments | `ExperimentSuite`, `VariantMeasurement`, `comparison_suite`, `hierarchy_measurements`, `tool_output_measurements`, `cache_measurements`, `cross_repository_measurements`, `jev_relevance_measurements` |
| Accuracy | `AccuracyCorpus`, `LabeledDecision`, `evaluate_accuracy` returning an `AccuracyReport` |
| Workload | `WorkloadCapture`, `WorkloadEvent`, `WorkloadEventKind`, `WorkloadReport` - JSONL capture of real sessions and aggregate reporting |

All reports carry a schema constant and serialise to JSON for archiving and comparison.

## Example

```rust,no_run
use tpt_weave_context::Sources;
use tpt_weave_core::ContextLevel;
use tpt_weave_eval::local_comparison;
use tpt_weave_graph::RepositoryGraph;

let graph = RepositoryGraph::load(".tpt-weave/graph.json")?;
let sources = Sources::load_dir(".")?;
let comparison = local_comparison(&graph, &sources, "fix the resampling bug", 8_000, ContextLevel::Implementation)?;
println!("{comparison:?}");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Related documentation

[benchmark-methodology](../../docs/benchmark-methodology.md) -
[experiments](../../docs/experiments.md) -
[workload](../../docs/workload.md) -
[token-accounting](../../docs/token-accounting.md)

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
