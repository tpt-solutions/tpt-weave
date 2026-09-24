# tpt-weave Troubleshooting

| Symptom | Action |
| --- | --- |
| Exit 2 / unknown command | Run `tpt-weave help`; put global flags before the command. |
| Exit 3 / missing manifest or symbol | Run `init` or check the requested name and `--path`. |
| Exit 4 / stale index | Run `tpt-weave index`; a changed manifest, HEAD, or working tree invalidates the graph. |
| No parseable sources | Confirm Cargo metadata, workspace membership, Rust extensions, and privacy exclusions. |
| Remote decision error | Keep local-only mode, then review the provider key environment, endpoint, timeout, and budget. |
| MCP tools are stale | Check `TPT_WEAVE_PATH`; restart after environment or tool-filter changes. |
| Cache appears corrupt | Run `tpt-weave cache clear`; the next request rebuilds it. |
| Registry path is missing | Add the related repository explicitly; Cargo metadata cannot infer every checkout location. |

For reproducible performance data, use the profiling script and record the
build profile and machine details. Do not treat a debug-build sample as a release
claim.
