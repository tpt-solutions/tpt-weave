# tpt-weave Performance Review

Review date: 2026-09-24  
Scope: local Windows process measurements for the current repository.

## Measured sample

Release CLI, `index --full`, current 119-file / 1394-symbol working tree:

- wall time: 9,517.3 ms;
- process CPU time: 2,562.5 ms;
- peak working set: 20,926,464 bytes (19.96 MiB);
- command exit code: 0.

This is one process-level sample, not a statistically powered benchmark or a
cross-platform claim. The machine, Rust toolchain, revision, profile, and
arguments must accompany future samples. `scripts/profile.ps1` is the repeatable
Windows helper.

## Implemented optimizations

- `LazySources` defers source reads and caches them per request path.
- Source discovery and indexing apply privacy policy before reading files.
- Rust parsing is parallelized with deterministic output ordering.
- Fingerprint-keyed parsed-file entries reuse unchanged source parse results.
- Incremental graph construction reuses unchanged symbol records while
  recomputing modules, dependencies, and all cross-file references.
- CLI JSON reports elapsed time and parse-cache hit/miss/store counters.
- Workload and experiment reports preserve raw/delivered/JEv/output token
  fields so structural and provider costs are not conflated.

## Remaining bottlenecks

Persisted-index reads and graph JSON deserialization still dominate the
medium-repository no-op path measured earlier. The current graph format is
verbose and intentionally human-inspectable. A compact, versioned graph/index
cache is the next performance task. A portable sampling profiler and
multi-machine benchmark matrix remain outstanding.

## Review disposition

The implementation-level performance review is complete for this phase. The
repository still lacks a cross-platform profiler, repeated warm/cold samples,
and a production 1B-token workload capture. Those items remain open in
`todo.md`.
