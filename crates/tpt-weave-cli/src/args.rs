//! Argument parsing for the tpt-weave CLI (docs/decisions.md section 3).

use crate::error::CliError;
use std::collections::VecDeque;
use std::path::PathBuf;

/// Global flags plus the selected subcommand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cli {
    /// Working tree to operate on (`--path`, default `.`).
    pub path: PathBuf,
    /// Pretty JSON on stdout (`--json`).
    pub json: bool,
    /// Single-line JSON on stdout (implies `--json`).
    pub compact: bool,
    /// Disable ANSI colour (accepted for compatibility; output is plain).
    pub no_color: bool,
    /// Extra detail on successful runs.
    pub verbose: bool,
    /// Treat decision provider failures as hard errors (exit 5).
    pub strict_decisions: bool,
    pub command: Command,
}

/// Subcommands matching the planned UX surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Onboard an existing repo: init + index --full + doctor.
    Adopt,
    /// Write `.tpt-weave/manifest.toml`.
    Init { force: bool },
    /// Build/refresh the deterministic index.
    Index { full: bool },
    /// Validate manifest, index freshness, git and provider health.
    Doctor,
    /// Level-0 repository overview.
    Overview,
    /// Symbol lookup by name/query.
    Symbol { name: String },
    /// References from/to a symbol key.
    Refs { name: String },
    /// Expand a context id / subject to a representation level.
    Expand {
        id: String,
        level: Option<u8>,
        kind: Option<String>,
        package: Option<String>,
    },
    /// Build task context within the configured budget.
    Context {
        task: String,
        budget: Option<u32>,
        max_level: Option<u8>,
        no_deps: bool,
    },
    /// Reduced working-tree diff.
    Diff,
    /// Token accounting + cache statistics.
    Stats,
    /// Cache management (default status).
    Cache { action: CacheAction },
    /// Print usage and exit 0.
    Help,
    /// Print version and exit 0.
    Version,
}

/// `tpt-weave cache` actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CacheAction {
    /// Show cache statistics (default).
    #[default]
    Status,
    /// Remove every cache entry.
    Clear,
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            json: false,
            compact: false,
            no_color: false,
            verbose: false,
            strict_decisions: false,
            command: Command::Help,
        }
    }
}

impl Cli {
    /// Effective JSON mode (`--compact` implies `--json`).
    pub fn json_mode(&self) -> bool {
        self.json || self.compact
    }
}

struct Args {
    items: VecDeque<String>,
}

impl Args {
    fn new(args: Vec<String>) -> Self {
        Self { items: args.into() }
    }

    fn next(&mut self) -> Option<String> {
        self.items.pop_front()
    }

    fn next_value(&mut self, flag: &str) -> Result<String, CliError> {
        self.next()
            .ok_or_else(|| CliError::usage(format!("`{flag}` requires a value")))
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Parses a full argv tail (no program name).
pub fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    let mut raw = Args::new(args);
    let mut cli = Cli::default();
    let mut command: Option<Command> = None;

    while let Some(arg) = raw.next() {
        let is_flag = arg.starts_with('-') && arg != "-";
        if is_flag {
            apply_flag(&mut cli, &mut command, &arg, &mut raw)?;
            continue;
        }

        if command.is_some() {
            return Err(CliError::usage(format!("unexpected argument: {arg}")));
        }

        command = Some(parse_command(&arg, &mut raw)?);
    }

    cli.command = command.unwrap_or(Command::Help);
    Ok(cli)
}

fn apply_flag(
    cli: &mut Cli,
    command: &mut Option<Command>,
    arg: &str,
    raw: &mut Args,
) -> Result<(), CliError> {
    match arg {
        "--path" => {
            cli.path = PathBuf::from(raw.next_value("--path")?);
        }
        "--json" => cli.json = true,
        "--compact" => cli.compact = true,
        "--no-color" => cli.no_color = true,
        "--verbose" | "-v" => cli.verbose = true,
        "--strict-decisions" => cli.strict_decisions = true,
        "--help" | "-h" => {
            *command = Some(Command::Help);
        }
        "--version" | "-V" => {
            *command = Some(Command::Version);
        }
        "--force" => match command {
            Some(Command::Init { force }) => *force = true,
            _ => return Err(CliError::usage("`--force` is only valid with `init`")),
        },
        "--full" => match command {
            Some(Command::Index { full }) => *full = true,
            _ => return Err(CliError::usage("`--full` is only valid with `index`")),
        },
        "--no-deps" => match command {
            Some(Command::Context { no_deps, .. }) => *no_deps = true,
            _ => return Err(CliError::usage("`--no-deps` is only valid with `context`")),
        },
        "--budget" => {
            let value = raw.next_value("--budget")?;
            let budget: u32 = value
                .parse()
                .map_err(|_| CliError::usage(format!("invalid `--budget` value: {value}")))?;
            match command {
                Some(Command::Context { budget: slot, .. }) => *slot = Some(budget),
                _ => return Err(CliError::usage("`--budget` is only valid with `context`")),
            }
        }
        "--level" | "--max-level" => {
            let value = raw.next_value("--level")?;
            let level = parse_level(&value)?;
            match command {
                Some(Command::Expand { level: slot, .. })
                | Some(Command::Context {
                    max_level: slot, ..
                }) => *slot = Some(level),
                _ => {
                    return Err(CliError::usage(
                        "`--level` is only valid with `expand` or `context`",
                    ));
                }
            }
        }
        "--kind" => {
            let value = raw.next_value("--kind")?;
            match command {
                Some(Command::Expand { kind, .. }) => *kind = Some(value),
                _ => return Err(CliError::usage("`--kind` is only valid with `expand`")),
            }
        }
        "--package" => {
            let value = raw.next_value("--package")?;
            match command {
                Some(Command::Expand { package, .. }) => *package = Some(value),
                _ => {
                    return Err(CliError::usage("`--package` is only valid with `expand`"));
                }
            }
        }
        other => return Err(CliError::usage(format!("unknown flag: {other}"))),
    }
    Ok(())
}

fn parse_command(name: &str, raw: &mut Args) -> Result<Command, CliError> {
    Ok(match name {
        "adopt" => Command::Adopt,
        "init" => Command::Init { force: false },
        "index" => Command::Index { full: false },
        "doctor" => Command::Doctor,
        "overview" => Command::Overview,
        "symbol" => Command::Symbol {
            name: require_arg(raw, "symbol", "<name>")?,
        },
        "refs" => Command::Refs {
            name: require_arg(raw, "refs", "<name>")?,
        },
        "expand" => Command::Expand {
            id: require_arg(raw, "expand", "<context-id>")?,
            level: None,
            kind: None,
            package: None,
        },
        "context" => Command::Context {
            task: require_arg(raw, "context", "\"<task>\"")?,
            budget: None,
            max_level: None,
            no_deps: false,
        },
        "diff" => Command::Diff,
        "stats" => Command::Stats,
        "cache" => match raw.next().as_deref() {
            None | Some("status") => Command::Cache {
                action: CacheAction::Status,
            },
            Some("clear") => Command::Cache {
                action: CacheAction::Clear,
            },
            Some(other) => {
                return Err(CliError::usage(format!(
                    "unknown cache action `{other}` (status|clear)"
                )));
            }
        },
        "help" => Command::Help,
        "version" => Command::Version,
        other => {
            return Err(CliError::usage(format!(
                "unknown command `{other}` (see `tpt-weave help`)"
            )));
        }
    })
}

fn require_arg(raw: &mut Args, command: &str, placeholder: &str) -> Result<String, CliError> {
    if raw.is_empty() {
        return Err(CliError::usage(format!(
            "`{command}` requires {placeholder}"
        )));
    }
    raw.next_value(command)
}

/// Parses a level as `0..=5` or a snake_case name.
pub fn parse_level(value: &str) -> Result<u8, CliError> {
    if let Ok(number) = value.parse::<u8>() {
        if tpt_weave_core::ContextLevel::from_number(number).is_some() {
            return Ok(number);
        }
        return Err(CliError::usage(format!("level out of range: {number}")));
    }
    tpt_weave_core::ContextLevel::ALL
        .iter()
        .find(|level| level.name() == value)
        .map(|level| level.as_number())
        .ok_or_else(|| CliError::usage(format!("unknown level: {value}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(args: &[&str]) -> Result<Cli, CliError> {
        parse(args.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn defaults_to_help() {
        let cli = parse_str(&[]).expect("help");
        assert_eq!(cli.command, Command::Help);
    }

    #[test]
    fn parses_global_flags_before_command() {
        let cli = parse_str(&["--json", "--path", "/tmp/x", "overview"]).expect("ok");
        assert!(cli.json_mode());
        assert_eq!(cli.path, PathBuf::from("/tmp/x"));
        assert_eq!(cli.command, Command::Overview);
    }

    #[test]
    fn compact_implies_json() {
        let cli = parse_str(&["--compact", "stats"]).expect("ok");
        assert!(cli.compact);
        assert!(cli.json_mode());
    }

    #[test]
    fn rejects_unknown_flag() {
        let error = parse_str(&["--nope"]).expect_err("usage");
        assert_eq!(error.code(), crate::error::ExitCode::Usage);
    }

    #[test]
    fn rejects_unknown_command() {
        let error = parse_str(&["frobnicate"]).expect_err("usage");
        assert_eq!(error.code(), crate::error::ExitCode::Usage);
    }

    #[test]
    fn symbol_requires_name() {
        let error = parse_str(&["symbol"]).expect_err("usage");
        assert_eq!(error.code(), crate::error::ExitCode::Usage);
    }

    #[test]
    fn parses_context_task() {
        let cli = parse_str(&["context", "fix the bug"]).expect("ok");
        match cli.command {
            Command::Context { task, .. } => assert_eq!(task, "fix the bug"),
            other => panic!("wrong command: {other:?}"),
        }
    }

    #[test]
    fn parses_level_names_and_numbers() {
        assert_eq!(parse_level("skeleton").expect("name"), 3);
        assert_eq!(parse_level("5").expect("num"), 5);
        assert!(parse_level("9").is_err());
        assert!(parse_level("nope").is_err());
    }

    #[test]
    fn parses_cache_actions() {
        let status = parse_str(&["cache"]).expect("status");
        assert!(matches!(
            status.command,
            Command::Cache {
                action: CacheAction::Status
            }
        ));
        let clear = parse_str(&["cache", "clear"]).expect("clear");
        assert!(matches!(
            clear.command,
            Command::Cache {
                action: CacheAction::Clear
            }
        ));
    }

    #[test]
    fn force_only_with_init() {
        let cli = parse_str(&["init", "--force"]).expect("ok");
        assert!(matches!(cli.command, Command::Init { force: true }));
        let error = parse_str(&["overview", "--force"]).expect_err("usage");
        assert_eq!(error.code(), crate::error::ExitCode::Usage);
    }

    #[test]
    fn budget_and_level_on_context() {
        let cli = parse_str(&[
            "context",
            "task",
            "--budget",
            "8000",
            "--level",
            "signatures",
            "--no-deps",
        ])
        .expect("ok");
        match cli.command {
            Command::Context {
                budget,
                max_level,
                no_deps,
                ..
            } => {
                assert_eq!(budget, Some(8000));
                assert_eq!(max_level, Some(2));
                assert!(no_deps);
            }
            other => panic!("wrong command: {other:?}"),
        }
    }
}
