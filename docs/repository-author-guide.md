# tpt-weave Repository Author Guide

1. Keep Cargo metadata and Rust source in the repository root/workspace.
2. Run `tpt-weave init` for a new repository or `tpt-weave adopt` for an existing
   one.
3. Review `.tpt-weave/manifest.toml`, especially context budgets, privacy
   exclusions, and remote-decision settings.
4. Run `tpt-weave index` after source, Cargo, or manifest changes.
5. Run `tpt-weave doctor` and inspect `--json` output in CI.
6. Use `context`, `symbol`, `skeleton`, and `expand` rather than duplicating
   repository exploration in an agent.
7. Add related repositories to `.tpt-weave/registry.json` with explicit paths;
   never commit secrets or generated private source data.

The current indexer targets Rust Cargo workspaces. Other languages require a
separate adapter and should not be silently represented as indexed.
