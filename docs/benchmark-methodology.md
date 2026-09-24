# tpt-weave Benchmark Methodology

## Measurement layers

Use three layers and do not combine their claims:

1. **Structural token reduction** — compare full source with the selected
   context at a fixed budget and representation level.
2. **Runtime cost** — measure index, symbol, context, provider, cache, and MCP
   operations with repeated samples and explicit build profile.
3. **Workload outcomes** — compare task success, quality, provider cost, and
   actual model tokens on a representative session capture.

The first two can be run locally without a live provider. The third requires
real task and model data.

## Token accounting

The deterministic estimator is one token per four Unicode scalar values,
rounded up. Report it as an estimate, not a provider tokenizer. Primary net
reduction is:

```text
1 - (delivered context tokens + JEv input tokens) / raw tokens
```

Keep output tokens, cache savings, and provider cost in separate fields.

## Runtime protocol

Record:

- repository revision and working-tree state;
- Rust toolchain and build profile;
- machine, OS, and available CPU/memory;
- number of warm-up and measured samples;
- command, input, output, exit code, elapsed time, and peak working set where
  available.

Use `tpt-weave --json index|symbol|context` for the built-in elapsed fields.
On Windows, `scripts/profile.ps1` provides a process-level CPU and peak working
set sample. It is a local helper, not a portable profiler.

## Experiment matrices

Use `ExperimentSuite` and `VariantMeasurement` for Phase 17. Supply measured
values for every variant; do not synthesize a JEv result, cache hit, quality
score, or dollar saving. The explicit constructors cover hierarchy, tool
reduction, four-way cache, and four-way cross-repository matrices.

## Workload protocol

Record only event metadata in `WorkloadCapture` JSONL. Do not store prompts,
source text, model output, credentials, or private paths. Include stable event
IDs so repeated context and tool output can be counted. Aggregate with
`WorkloadReport`, then inspect success preservation and cost before drawing a
conclusion about budget extension.
