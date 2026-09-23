//! Output rendering: human text, pretty JSON, compact JSON.

use crate::args::Cli;
use serde_json::Value;

/// Selected stdout encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Human-readable multi-line text.
    Human,
    /// Pretty-printed JSON.
    Json,
    /// Single-line JSON.
    Compact,
}

/// Resolved output settings from the global flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Output {
    pub format: Format,
    pub verbose: bool,
}

impl From<&Cli> for Output {
    fn from(cli: &Cli) -> Self {
        let format = if cli.compact {
            Format::Compact
        } else if cli.json {
            Format::Json
        } else {
            Format::Human
        };
        Self {
            format,
            verbose: cli.verbose,
        }
    }
}

impl Output {
    /// Renders one command result for stdout.
    pub fn render(&self, rendered: &crate::commands::Rendered) -> String {
        match self.format {
            Format::Human => rendered.human.clone(),
            Format::Json => serde_json::to_string_pretty(&rendered.json)
                .unwrap_or_else(|_| rendered.human.clone()),
            Format::Compact => {
                serde_json::to_string(&rendered.json).unwrap_or_else(|_| rendered.human.clone())
            }
        }
    }

    /// Whether JSON was requested (used for error shapes).
    pub fn json(&self) -> bool {
        !matches!(self.format, Format::Human)
    }
}

/// Serialises a value as pretty JSON for intermediate construction.
pub fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}
