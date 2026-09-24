# Phase 12 — tpt-infer integration results

Date: 2026-09-24 · tpt-weave CLI (release build, deterministic fallback provider)

Integration level: **A (zero-code)** per spec §12 — `tpt-weave adopt` wrote
`.tpt-weave/` inside the tpt-infer working tree; no tpt-infer source changes
(the `.tpt-weave/` directory is excluded by the standard gitignore entries, so
their working tree stays clean).

## Repository

10-crate workspace (`tpt-infer-core`, `tpt-infer-ops`, `tpt-infer-graph`,
`tpt-infer-onnx`, `tpt-infer-quantize`, `tpt-infer-compile`,
`tpt-infer-runtime`, `tpt-infer-vision`, `tpt-infer`, `tpt-infer-cli`).

## Adoption

```
tpt-weave --path <tpt-infer> adopt
  indexed 999 symbols in 69 files
  registered 0 cross-repository link(s)
  baseline benchmark: 144,038 raw -> 11,983 delivered tokens (net 91.7%)
  doctor: ok (manifest, index freshness)
```

No TPT path-dependencies were found, so the cross-repository graph has no
links for this repo yet (correct behaviour — tpt-infer is a leaf).

## Test 1 — symbol retrieval

`tpt-weave symbol Tensor` returned 61 ordered matches with signatures and
locations (`tpt-infer-core::Tensor#struct`, 20 inherent methods,
`TensorView`/`TensorMut` traits, `TensorError` and its trait impls). Unknown
names exit 3 with a not-found error (`symbol InferenceEngine` → exit 3).

## Test 2 — dependency traversal

Cross-crate task context (deterministic scoring, dependency branches enabled):

```
task: "why does tpt-infer-runtime fail to load a quantised ONNX file from tpt-infer-quantize"
raw 147,897 tokens -> delivered 11,999 tokens (91.9% net reduction)
candidates from 5 crates: tpt-infer-onnx/load.rs (0.97), tpt-infer-graph/graph.rs (0.94),
tpt-infer-quantize/ptq.rs (0.94), tpt-infer-compile (0.93), tpt-infer-ops/webgpu.rs (0.93)
```

The selection correctly spans the onnx → graph → quantize → runtime path in a
single budget-bound response.

## Test 3 — model context + expansion

`context "reshape a Tensor and quantise an ONNX model for the runtime"`
selected 15 candidates totalling 11,999 of a 12,000-token budget
(selection ratio 8.3%); `expand ctx:…` on the top candidate delivered the full
`tensor.rs` source (2,848 tokens) from its skeleton entry.

## Notes / follow-ups

- `refs TensorError` returns "no references": explicit reference records are
  currently populated for call-site edges the indexer can prove; widening
  reference capture is tracked in the graph crate backlog, not Phase 12.
- `relationships` arrays on context candidates are empty for file-level
  candidates; symbol-level candidates populate them. Same follow-up.
- Baseline numbers here feed Phase 17 Experiment B (hierarchical levels) and
  the Phase 18 workload model.
