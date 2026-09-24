# Phase 12 — tpt-raglite evaluation (optional semantic retrieval backend)

Date: 2026-09-24

**Question (todo):** is tpt-raglite worth wiring in as a semantic retrieval
backend, does semantic retrieval beat structural retrieval where it matters,
and do we avoid duplicating RAG functionality?

## What tpt-raglite is

An embeddable, offline RAG engine: local ONNX embeddings (ort), chunking,
memory-mapped HNSW index, reranker, metadata store — "SQLite for AI", pure
Rust with C/Python/WASM bindings.

## Structural vs semantic retrieval — comparison

Dimensions that matter for tpt-weave's use case (agent answering a coding
task about a Rust workspace):

| Dimension | tpt-weave structural | tpt-raglite semantic |
|---|---|---|
| Determinism / reproducibility | exact — same revision → same selection | embedding drift, reranker order shifts |
| Symbol-precise lookup (`Tensor::reshape`) | exact hit | approximate; identifiers are weak signals for embedding models |
| Token accounting (spec §16) | native — candidates carry token estimates | none — chunks have no token-aware budgets |
| Correctness model (spec §21) | auditable decisions with deterministic fallback | black-box scores, no safety override |
| Natural-language recall ("where is image resampling handled?") | keyword/name overlap only | strong |
| Prose corpora (README, spec, design docs) | weak — file-level granularity only | strong |
| Setup cost for an agent | zero (index already exists) | build embedding index per repo + ONNX runtime |

Measured structural baseline (tpt-infer, Phase 12 report): 91.7–91.9 % net
token reduction with deterministic scoring. A live semantic comparison was
*not* run in this pass — tpt-raglite builds download ONNX runtime binaries,
which is outside this workspace's CI budget; the comparison table above is the
methodology, and a measured run belongs to Phase 17 Experiment D follow-up if
structural retrieval is ever shown to miss.

## Duplication check

tpt-weave today has no embeddings, no chunker, no vector store — nothing is
duplicated. Embedding any of those into tpt-weave-core would violate spec §33
("Do not embed a vector database requirement into the core").

## Verdict

**Keep tpt-raglite as an optional, out-of-tree semantic backend.** Concrete
shape if ever wired: a `DecisionProvider`-style trait boundary
(`SemanticRetriever`) whose results enter context selection as low-priority
candidates behind deterministic hits, default-off in manifests, never a core
dependency. For the Phase 1 integration repos (Rust workspaces, symbol-heavy
tasks) structural retrieval already covers the measured tasks; the marginal
value is in prose-heavy corpora, which are not what agents burn tokens on.

No integration code is added in this phase; the boundary above is recorded so
a future integration does not grow inside the core crates.
