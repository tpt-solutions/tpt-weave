//! tpt-weave command-line interface (todo.md Phase 10; UX in
//! `docs/decisions.md` section 3).
//!
//! Human-readable output by default; `--json` / `--compact` select
//! machine-readable forms. Exit codes follow the documented table
//! (0 success, 1 internal, 2 usage, 3 not found, 4 stale index,
//! 5 decision provider failure under `--strict-decisions`).

#![forbid(unsafe_code)]

pub mod args;
pub mod commands;
pub mod error;
pub mod output;
pub mod workspace;

pub use args::{Cli, Command};
pub use error::{CliError, ExitCode};
pub use output::{Format, Output};

/// Parsed invocation outcome for [`run`].
pub struct RunOutcome {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Parses `args` (without the program name), dispatches the command and
/// captures stdout/stderr. The binary prints these and exits with `code`.
pub fn run(args: Vec<String>) -> RunOutcome {
    let cli = match args::parse(args) {
        Ok(cli) => cli,
        Err(error) => {
            return RunOutcome {
                code: error.code().as_i32(),
                stdout: String::new(),
                stderr: format!("error: {error}\n"),
            };
        }
    };

    let output = Output::from(&cli);
    match commands::dispatch(&cli) {
        Ok(rendered) => {
            let body = output.render(&rendered);
            let mut stdout = body;
            if !stdout.ends_with('\n') {
                stdout.push('\n');
            }
            if cli.verbose && !rendered.detail.is_empty() {
                stdout.push_str(&rendered.detail);
                if !stdout.ends_with('\n') {
                    stdout.push('\n');
                }
            }
            RunOutcome {
                code: 0,
                stdout,
                stderr: String::new(),
            }
        }
        Err(error) => {
            let code = error.code().as_i32();
            let stderr = if output.json() {
                let mut text = serde_json::to_string(&serde_json::json!({
                    "error": error.to_string(),
                    "exit_code": code,
                }))
                .unwrap_or_else(|_| format!("{{\"error\":\"{}\"}}", error));
                text.push('\n');
                text
            } else {
                format!("error: {error}\n")
            };
            RunOutcome {
                code,
                stdout: String::new(),
                stderr,
            }
        }
    }
}
