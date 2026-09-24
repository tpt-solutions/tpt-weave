# Phase 12 — tpt-uir evaluation (context transport / IR)

Date: 2026-09-24

**Question (todo):** should tpt-uir become the context transport/IR for
tpt-weave, and does adopting it avoid unnecessary duplicate representations?

## What tpt-uir is

A `no_std`-friendly SSA intermediate representation for ML compilers
(region/block/operation graphs, typed attribute system) with three
serialisations: postcard (`tpt-uir-serde`), FlatBuffers
(`tpt-uir-flatbuffers`), and a round-trippable text format (`tpt-uir-text`),
plus CLI and C FFI.

## Evaluation

tpt-weave's context products are not compiler graphs. They are ordered lists
of hierarchical representations (levels 0–5) with token accounting,
decision provenance, and cache identity — see spec §9/§16. Mapping those onto
SSA ops would require:

- an op per context candidate with attributes for level, score, source span,
  cache key, decision provenance — i.e. tpt-uir's attribute system used as a
  JSON-in-a-trenchcoat;
- every consumer (CLI, MCP client, eval harness) to pull in the tpt-uir
  schema, breaking the "compact output" goal of the MCP tool-minimisation
  work in Phase 9;
- a second source of truth for context identity (tpt-uir op ids vs
  `ctx:<hash>` ids), duplicating representation rather than avoiding it.

Where tpt-uir genuinely fits: **tpt-infer-compile** already consumes/produces
UIR for model compilation. If tpt-weave is later asked to answer questions
about compiled artefacts ("which ops did this AOT build emit?"), indexing the
UIR text format as an additional *indexer input* is the right shape — tpt-uir
as an indexed artefact format, not as tpt-weave's transport.

## Verdict

**Do not adopt tpt-uir as the context transport.** JSON output (CLI) and MCP
tool results (agents) already are the stable transport boundaries; tpt-uir
would add a duplicate representation with no consumer. Re-evaluate only if a
tpt-weave consumer needs to interleave *compiler* context with source context.
This keeps responsibilities separate (spec §13: "evaluate … where it provides
a useful stable representation").

No source changes in either repository follow from this evaluation.
