# tpt-weave Release Readiness

This repository is pre-alpha. The following checklist separates locally
verifiable release gates from decisions that require external access or release
authority. A checked local gate is not a declaration that the project is
published or that APIs are stable.

## Local gates

Run from the repository root:

```sh
cargo fmt --all -- --check
git diff --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo package -p tpt-weave-core --allow-dirty --locked
```

The last command verifies the package layout for the foundational crate. A
full workspace package requires the internal `tpt-weave-*` crates to be
available on crates.io; it is not expected to pass before publication order is
established.

## Publication order

1. Publish the foundational libraries in dependency order:
   `tpt-weave-core`, `tpt-weave-index`, `tpt-weave-rust`, `tpt-weave-graph`,
   `tpt-weave-context`, `tpt-weave-tools`, `tpt-weave-cache`,
   `tpt-weave-decisions`, `tpt-weave-openrouter`, `tpt-weave-eval`, then the
   `tpt-weave-mcp` and CLI packages.
2. Run `cargo publish --dry-run <package>` for each package immediately before
   publishing it.
3. Verify the published package with `cargo install` in a clean temporary
   project and run the documented smoke commands.
4. Tag the release only after the index schema, CLI behavior, MCP tool catalog,
   and provider adapter have been reviewed together.

The packages currently declare `publish = false`; removing that guard is a
release decision, not an automated consequence of passing tests.

## External gates still required

- Representative Phase 18 coding-session capture with raw baseline, repeated
  context/tool output, exploration, and dependency-traversal measurements.
- A live, reviewed JEv run with quality, latency, token, and cost data.
- Four-way Phase 17 cache and cross-repository measurements on real downstream
  repositories.
- Independent dependency/security review and approval of provider/privacy
  policy.
- Stable API decision for the core, index format, CLI, MCP surface, OpenRouter
  adapter, and JEv integration.
- GitHub release, crates.io publication, and TPT open-source catalogue entry.

## Evidence locations

- Local performance and experiment methodology: [benchmark-methodology.md](benchmark-methodology.md)
- Workload event format: [workload.md](workload.md)
- Security review: [security-review.md](security-review.md)
- Performance review: [performance-review.md](performance-review.md)
- Privacy policy: [privacy.md](privacy.md)
