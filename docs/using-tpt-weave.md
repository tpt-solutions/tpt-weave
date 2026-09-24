# Using tpt-weave on New and Existing Repositories

This guide covers practical workflows for a new Cargo repository and an
existing repository. The current indexer targets Rust Cargo workspaces;
non-Rust language adapters are not implemented yet.

## 1. Prerequisites

- Rust 1.85 or newer (edition 2024).
- `cargo` and a `Cargo.toml` at the repository root or in the workspace root.
- Git is recommended for revision-aware invalidation. Git-less trees work, but
  `index` conservatively rebuilds them.
- A shell or terminal with the `tpt-weave` binary available on `PATH`.

The repository is currently pre-alpha. Build the current binaries from a
checkout with:

```sh
cargo build --release -p tpt-weave-cli -p tpt-weave-mcp
```

Add `target/release` to `PATH`, or invoke the binaries by their full path. The
examples use POSIX paths; on Windows, use paths such as
`C:\work\my-repository`. For development, use:

```sh
cargo run -p tpt-weave-cli -- <command>
cargo run -p tpt-weave-mcp
```

Check the installation with:

```sh
tpt-weave version
tpt-weave help
```

## 2. New repository workflow

Use this sequence for a newly created Rust project or a project that has not
been indexed by tpt-weave yet.

### Step 1: enter the repository

```sh
cd /absolute/path/to/my-new-repository
tpt-weave overview
```

Before indexing, `overview` is expected to report that an index is missing.

### Step 2: create local configuration

```sh
tpt-weave init
```

`init` creates `.tpt-weave/manifest.toml` and adds the standard `/.tpt-weave/`
entry to `.gitignore`. It is idempotent. Use `tpt-weave init --force` only when
you intentionally want to replace an existing manifest.

The generated manifest uses the directory name as the repository name. Review
its `[repository]`, `[context]`, and `[privacy]` values before sharing any
configuration.

### Step 3: build the index

```sh
tpt-weave index
```

The first build reads Cargo metadata and Rust source, extracts symbols and
references, and writes `.tpt-weave/graph.json`. JSON output includes counts and
timing:

```sh
tpt-weave --json index
```

### Step 4: verify the installation

```sh
tpt-weave doctor
tpt-weave overview
```

A successful `doctor` checks the manifest, graph freshness, Git availability,
and provider configuration. With the default local-only privacy policy, no API
key is required.

### Step 5: retrieve task context

```sh
tpt-weave context "implement the parser error type"
tpt-weave symbol Parser
tpt-weave skeleton src/parser.rs --level skeleton
```

Use the context id returned by `context` when you need a more detailed
representation:

```sh
tpt-weave expand <context-id> --level implementation
```

## 3. Existing repository workflow

For an existing repository, the recommended one-command onboarding flow is:

```sh
cd /absolute/path/to/existing-repository
tpt-weave adopt
```

`adopt` performs the following steps in order:

1. creates or validates the manifest and standard `.gitignore` entry;
2. performs a full index build;
3. registers detected `tpt-*` cross-repository dependency links;
4. writes the model-free baseline report to
   `.tpt-weave/baseline-report.json`; and
5. runs `doctor`.

`adopt` does not overwrite an existing manifest unless you explicitly run
`tpt-weave init --force` first. Review the generated manifest before using
remote decisions.

After adoption, run the normal workflow:

```sh
tpt-weave doctor
tpt-weave overview
tpt-weave context "find the code that owns the public API"
```

The `.tpt-weave/` directory is local working state. It is ignored by default;
keep it out of commits unless your team has a deliberate policy for sharing a
manifest or generated reports.

## 4. Configuration

A representative manifest section set is:

```toml
schema = 1
repository = "my-repository"
language = "rust"
workspace = true

[features]
symbol_index = true
dependency_graph = true
source_skeletons = true
tool_output_reduction = true
jev = true

[context]
default_budget = 12000
maximum_budget = 32000

[privacy]
remote_decisions = false
exclude = [".env", ".env.*", "secrets/", "*.pem", "*.key"]
private_paths = ["/absolute/path/to/private/repository"]

[jev]
enabled = true
model = "typesafe/jev-1.13"
min_confidence = 0.70
timeout_ms = 5000
max_retries = 2

[provider]
name = "openrouter"
endpoint = "https://openrouter.ai/api/alpha/decisions"
api_key_env = "OPENROUTER_API_KEY"
```

### Privacy defaults

The default mode is local-only: `remote_decisions = false`. The default
exclusions cover environment files, secret directories, certificates, and
private-key extensions. `private_paths` adds absolute or repository-relative
path prefixes.

Do not put an API key in the manifest. If remote decisions are explicitly
enabled, set the environment variable named by `api_key_env` before running a
remote decision:

```sh
export OPENROUTER_API_KEY="your-key"
tpt-weave doctor
```

Remote requests redact detected credential values and can write a
content-free JSONL audit record. Review `docs/privacy.md` before enabling
remote data transfer.

## 5. Everyday commands

| Command | Purpose |
| --- | --- |
| `tpt-weave index` | Refresh after HEAD, source, or manifest changes |
| `tpt-weave index --full` | Force a complete index build |
| `tpt-weave overview` | Show repository size and graph summary |
| `tpt-weave symbol <name>` | Find symbols and signatures |
| `tpt-weave refs <name>` | Show resolved callers/callees and relationships |
| `tpt-weave context "<task>"` | Select budgeted task context |
| `tpt-weave skeleton <file>` | Render one file at a representation level |
| `tpt-weave expand <id>` | Expand a context or symbol to another level |
| `tpt-weave diff` | Show a reduced working-tree diff |
| `tpt-weave stats` | Show token and cache statistics |
| `tpt-weave cache status` | Inspect cache entries and hit rate |
| `tpt-weave cache clear` | Remove filesystem cache entries |
| `tpt-weave workload <capture.jsonl>` | Aggregate a Phase 18 JSONL capture; add `--duration` for rates |
| `tpt-weave experiment accuracy <corpus.json>` | Evaluate a labeled corpus with the deterministic provider |

Representation levels are `0` metadata, `1` symbols, `2` signatures, `3`
skeleton, `4` targeted implementation, and `5` full source.

Useful global flags are `--path`, `--json`, `--compact`, `--verbose`, and
`--strict-decisions`. Context requests also accept `--budget`,
`--max-level` (or `--level`), and `--no-deps`; `skeleton` and `expand` accept
`--level` as shown above.

## 6. JSON and automation

Global flags go before the command. `--json` emits pretty JSON and `--compact`
emits one JSON line:

```sh
tpt-weave --path /absolute/path/to/repository --json overview
tpt-weave --path /absolute/path/to/repository --compact context "fix the bug"
```

Useful automation fields include:

- `rebuilt` and `elapsed_ms` from `index`;
- `parse_cache.hits`, `parse_cache.misses`, and `parse_cache.stores`;
- `context_id`, `candidates`, `tokens`, and `changed_files` from `context`; and
- `elapsed_ms` from `symbol` and `context`.

A normal `index` reuses a graph only when Git HEAD, the working tree, and the
local manifest are unchanged. Uncommitted edits, non-Git trees, and newer
privacy configuration enter the rebuild path automatically.

## 7. Windows PowerShell equivalents

PowerShell uses environment variables rather than the POSIX `export` syntax:

```powershell
$env:PATH = "$PWD\target\release;$env:PATH"
$env:OPENROUTER_API_KEY = "your-key"
$env:TPT_WEAVE_PATH = "C:\work\my-repository"
$env:TPT_WEAVE_TOOLS = "orient,symbol"
.\target\release\tpt-weave.exe --path $env:TPT_WEAVE_PATH doctor
.\target\release\tpt-weave-mcp.exe
```

Use `Remove-Item Env:OPENROUTER_API_KEY` when finished if the key should not
remain in the current shell session.

## 8. Connect an MCP client

The MCP server speaks stdio. Start it from the repository root or set
`TPT_WEAVE_PATH`:

```sh
TPT_WEAVE_PATH=/absolute/path/to/repository tpt-weave-mcp
```

The server exposes repository overview, symbol, context, dependency, Git-diff,
test-result, and statistics tools. By default all tools are exposed. To reduce
schema overhead, set `TPT_WEAVE_TOOLS` to one or more groups:

```sh
TPT_WEAVE_PATH=/absolute/path/to/repository \
TPT_WEAVE_TOOLS=orient,symbol \
tpt-weave-mcp
```

Supported groups are `orient`, `symbol`, `context`, and `workspace` (aliases
such as `overview`, `symbols`, `ctx`, and `git` are also accepted). Configure
the MCP client to launch the binary with the environment variables it needs;
the server must receive the process on stdin/stdout, not a terminal prompt.

The MCP workspace refreshes when HEAD, the content-aware working-tree
fingerprint, or the local manifest changes. Restart the server after changing
its environment or tool filter.

## 9. Troubleshooting

### “No index found” or exit code 4

Run `tpt-weave index`, then `tpt-weave doctor`. A stale graph can also mean
that the manifest changed; indexing rebuilds it with the new privacy policy.

### “No parseable Rust sources”

Confirm that the repository contains a valid `Cargo.toml`, workspace members
are reachable, and the files are `*.rs`. Check that a privacy exclusion did
not remove the package you expected to index.

### Provider key errors

Keep `remote_decisions = false` for local-only operation. If it is `true`, set
the environment variable named by `[provider].api_key_env` and rerun `doctor`.
Never place the key in `manifest.toml`.

### MCP tools do not reflect edits

Confirm `TPT_WEAVE_PATH` points at the intended repository. For a Git tree,
the server fingerprints tracked diffs and untracked contents. For a Git-less
tree, restart the MCP server after source changes so it reloads the workspace.

### Cache problems

Inspect with `tpt-weave cache status`. Use `tpt-weave cache clear` to remove
cache entries; the next request rebuilds them. Clearing the cache does not
remove the graph or manifest.

## 10. Generated files and cleanup

Typical local files are:

- `.tpt-weave/manifest.toml` — configuration;
- `.tpt-weave/graph.json` — persisted graph;
- `.tpt-weave/parsed/` — parsed-file cache;
- `.tpt-weave/cache/` — filesystem cache entries; and
- `.tpt-weave/baseline-report.json` — local adopt benchmark report.

To uninstall from a repository, remove the `.tpt-weave/` directory and remove
the `/.tpt-weave/` line from `.gitignore` if you no longer want tpt-weave to
manage it. Back up the manifest first if the repository team uses it.


## 11. Standard agent, MCP, and registry integration

Generate the standard environment contract:

```sh
tpt-weave --json integration agent
tpt-weave --json integration agent --write
```

The write form creates `.tpt-weave/agent.json` with `TPT_WEAVE_PATH` and
`TPT_WEAVE_TOOLS`. Generate conventional MCP client configuration with:

```sh
tpt-weave --json integration mcp
tpt-weave --json integration mcp --write
```

The write form creates `.tpt-weave/mcp.json`; it points at the repository path
through `TPT_WEAVE_PATH` and runs the `tpt-weave-mcp` server. Review generated
files before copying them into an agent-specific configuration. The registry
view is read-only:

```sh
tpt-weave --json integration registry
```

It reports the optional `.tpt-weave/registry.json` and the `tpt-*` dependencies
discovered from Cargo metadata. Registry paths are maintained explicitly because
Cargo metadata does not always reveal the checkout location of an external
package.

## 12. Standard `.tpt-weave/` artifacts

| Path | Purpose |
| --- | --- |
| `.tpt-weave/manifest.toml` | Repository, context, privacy, JEv, and provider configuration |
| `.tpt-weave/graph.json` | Versioned deterministic repository graph |
| `.tpt-weave/parsed/` | Fingerprint-keyed parsed-file cache |
| `.tpt-weave/cache/` | Revision/schema-scoped filesystem cache |
| `.tpt-weave/agent.json` | Optional generated agent environment contract |
| `.tpt-weave/mcp.json` | Optional generated MCP client configuration |
| `.tpt-weave/registry.json` | Optional cross-repository registry metadata |
| `.tpt-weave/privacy-audit.jsonl` | Content-free remote decision audit records |
| `.tpt-weave/baseline-report.json` | Model-free adopt baseline report |

The directory is local working state and is ignored by default. Commit only a
manifest or another configuration artifact deliberately, never API keys or
private source content.

## 13. CI and repository integration

The repository CI workflow builds the workspace, runs tests and lint, checks the
MSRV, then initializes and validates `.tpt-weave/graph.json` with `init`,
`index --full`, `doctor`, and `overview`. A consumer repository can use the same
small sequence in its own workflow:

```sh
cargo run -p tpt-weave-cli -- init --force
cargo run -p tpt-weave-cli -- index --full --json
cargo run -p tpt-weave-cli -- doctor --json
```

