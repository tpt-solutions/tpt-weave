# Phase 18 Workload Capture

tpt-weave provides a content-independent measurement format for the 1B
Tokens/Hour workload study. It records token counts and outcome metadata; it
does not record source text, prompts, secrets, or model output.

## Event format

Write one JSON object per line to a capture file:

```json
{"kind":"context","id":"overview","raw_tokens":1000,"delivered_tokens":120}
{"kind":"tool_output","id":"cargo-test","raw_tokens":4000,"delivered_tokens":300}
{"kind":"model_call","id":"attempt-1","raw_tokens":0,"delivered_tokens":120,"output_tokens":80,"latency_ms":1250,"success":true,"cost_usd":0.01,"baseline_cost_usd":0.02}
```

`kind` is one of `context`, `tool_output`, `repository_exploration`,
`dependency_traversal`, or `model_call`. Use stable IDs so repeated context and
tool output can be identified. Optional JEv, cache, success, latency, and cost
fields are preserved when supplied.

The Rust API is `tpt_weave_eval::WorkloadCapture`; it supports JSONL parsing,
serialization, file load/save, and `WorkloadReport` aggregation. The report
includes raw/delivered/JEv/output tokens, repeated-context and repeated-tool
tokens, repository exploration and dependency traversal overhead, total event
latency, cache hits, model success rate, cost savings, and tokens/hour when
duration is known. The CLI can aggregate the same file:

```sh
tpt-weave workload session.jsonl --duration 3600
```

`--duration` is optional and must be a finite positive number. Relative capture
paths are resolved against the selected repository path. The command does not
read source text or prompts; it only consumes metadata already present in the
JSONL capture.

## Real-session procedure

1. Choose a representative coding session and record its wall-clock duration.
2. Export each model/tool/context interaction as an event without storing text.
3. Run the deterministic tpt-weave path and record the same event categories.
4. Repeat with JEv enabled only after privacy, provider, and budget review.
5. Compare quality and cost, then calculate whether the context reduction
   materially extends the available budget.

The target is 1.0B raw tokens/hour with under 250M delivered tokens/hour; the
stretch target is under 100M delivered tokens/hour. These are engineering
targets, not claims about this repository.

## Profiling

On Windows, build a release CLI and run:

```powershell
.\scripts\profile.ps1 -Path . -Arguments @("index", "--full")
```

The script reports wall time, process CPU time, and peak working set. It is a
repeatable local measurement helper, not a cross-platform profiler. For release
performance claims, run several samples on an otherwise idle machine and record
the Rust version, build profile, Git revision, and hardware.

