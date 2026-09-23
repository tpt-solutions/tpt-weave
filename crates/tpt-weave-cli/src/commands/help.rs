//! Help and version output.

use super::Rendered;
use serde_json::json;

pub fn run() -> Rendered {
    let text = "\
tpt-weave — cross-repository AI-context optimisation

Usage:
  tpt-weave [global flags] <command> [args]

Commands:
  adopt                     onboard: init + index + register + baseline + doctor
  init                      write .tpt-weave/manifest.toml + .gitignore (--force)
  index                     build/refresh the deterministic index (--full to rebuild)
  doctor                    validate manifest, index freshness, git and provider
  overview                  level-0 repository overview
  symbol <name>             symbol lookup
  refs <name>               references to a symbol
  expand <context-id>       next representation level / full source
  context \"<task>\"          build task context within the configured budget
  diff                      reduced working-tree diff
  stats                     token accounting + cache statistics
  cache [status|clear]      cache management (default: status)
  help                      show this help
  version                   show version

Global flags:
  --path <dir>              operate on another working tree (default: .)
  --json                    pretty JSON output
  --compact                 single-line JSON (implies --json)
  --no-color                disable colour (output is already plain)
  --verbose                 extra detail on success
  --strict-decisions        exit 5 on decision provider failure

Exit codes:
  0 success
  1 internal/unexpected error
  2 usage error
  3 not found (symbol, context id, manifest missing)
  4 stale or invalid index (re-run `tpt-weave index`)
  5 decision provider failure under --strict-decisions
";
    Rendered::new(text.trim_end(), json!({ "help": text }))
}

pub fn version() -> Rendered {
    let version = env!("CARGO_PKG_VERSION");
    Rendered::new(
        format!("tpt-weave {version}"),
        json!({ "version": version }),
    )
}
