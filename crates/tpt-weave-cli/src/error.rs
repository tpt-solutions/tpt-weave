//! CLI failure modes and process exit codes (docs/decisions.md section 3).

use std::fmt;

/// Documented process exit codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitCode {
    /// Success.
    Success,
    /// Internal/unexpected error.
    Internal,
    /// Usage error (bad flags or arguments).
    Usage,
    /// Not found (symbol, context id, manifest missing).
    NotFound,
    /// Stale or invalid index (re-run `tpt-weave index`).
    Stale,
    /// Decision provider failure under `--strict-decisions`.
    Decision,
}

impl ExitCode {
    /// Numeric exit status.
    pub fn as_i32(self) -> i32 {
        match self {
            ExitCode::Success => 0,
            ExitCode::Internal => 1,
            ExitCode::Usage => 2,
            ExitCode::NotFound => 3,
            ExitCode::Stale => 4,
            ExitCode::Decision => 5,
        }
    }
}

/// A command failure mapped to an exit code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliError {
    message: String,
    code: ExitCode,
}

impl CliError {
    /// Usage error (exit 2).
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: ExitCode::Usage,
        }
    }

    /// Not found (exit 3).
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: ExitCode::NotFound,
        }
    }

    /// Stale or invalid index (exit 4).
    pub fn stale(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: ExitCode::Stale,
        }
    }

    /// Decision provider failure under `--strict-decisions` (exit 5).
    pub fn decision(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: ExitCode::Decision,
        }
    }

    /// Internal/unexpected error (exit 1).
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: ExitCode::Internal,
        }
    }

    /// Mapped exit code.
    pub fn code(&self) -> ExitCode {
        self.code
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(error: std::io::Error) -> Self {
        Self::internal(format!("i/o error: {error}"))
    }
}

impl From<tpt_weave_core::ConfigError> for CliError {
    fn from(error: tpt_weave_core::ConfigError) -> Self {
        match error {
            tpt_weave_core::ConfigError::Io(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Self::not_found(format!("manifest missing: {err}"))
            }
            other => Self::internal(other.to_string()),
        }
    }
}

impl From<tpt_weave_graph::GraphError> for CliError {
    fn from(error: tpt_weave_graph::GraphError) -> Self {
        match error {
            tpt_weave_graph::GraphError::SchemaMismatch { .. } => {
                Self::stale(format!("{error}; re-run `tpt-weave index`"))
            }
            tpt_weave_graph::GraphError::Io(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Self::stale("no index found; run `tpt-weave index`")
            }
            other => Self::internal(other.to_string()),
        }
    }
}

impl From<tpt_weave_cache::CacheError> for CliError {
    fn from(error: tpt_weave_cache::CacheError) -> Self {
        Self::internal(error.to_string())
    }
}

impl From<tpt_weave_context::ContextError> for CliError {
    fn from(error: tpt_weave_context::ContextError) -> Self {
        match error {
            tpt_weave_context::ContextError::UnknownSymbol(key) => {
                Self::not_found(format!("unknown symbol: {key}"))
            }
            tpt_weave_context::ContextError::UnknownPackage(package) => {
                Self::not_found(format!("unknown package: {package}"))
            }
            tpt_weave_context::ContextError::UnknownModule(module) => {
                Self::not_found(format!("unknown module: {module}"))
            }
            other => Self::internal(other.to_string()),
        }
    }
}

impl From<tpt_weave_index::GitError> for CliError {
    fn from(error: tpt_weave_index::GitError) -> Self {
        Self::internal(error.to_string())
    }
}

impl From<tpt_weave_index::IndexError> for CliError {
    fn from(error: tpt_weave_index::IndexError) -> Self {
        Self::internal(error.to_string())
    }
}
