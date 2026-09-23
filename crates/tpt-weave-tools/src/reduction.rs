//! The compact reduced record and its measurements (todo.md Phase 6
//! "For each: preserve / store / expand / measure").

use crate::kind::ToolKind;
use crate::reduce::ToolOutput;
use crate::store::RawStore;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;

/// Outcome of the tool invocation as judged by its reducer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReductionStatus {
    /// The tool reported success.
    Success,
    /// The tool reported a failure; `reason` preserves the key detail.
    Failure { reason: String },
}

/// A compact deterministic record replacing verbose tool output
/// (spec.md section 14).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reduction {
    /// Which reducer produced this record.
    pub kind: ToolKind,
    /// The command that produced the raw output.
    pub command: String,
    /// Process exit code of the invocation.
    pub exit_code: i32,
    /// Success/failure with the preserved failure reason.
    pub status: ReductionStatus,
    /// Lines of the compact record (facts, failure details, detail flag).
    pub lines: Vec<String>,
    /// Token estimate of the raw output.
    pub raw_tokens: u64,
    /// Token estimate of the rendered record.
    pub reduced_tokens: u64,
    /// Local path of the stored raw output, once [`Reduction::store_raw`]
    /// has been called.
    pub detail: Option<PathBuf>,
}

impl Reduction {
    /// The compact record as a single string.
    pub fn render(&self) -> String {
        self.lines.join("\n")
    }

    /// Fraction of raw tokens removed: `1 - reduced / raw`
    /// (0.0 when there is no raw output).
    pub fn reduction(&self) -> f64 {
        if self.raw_tokens == 0 {
            return 0.0;
        }
        1.0 - (self.reduced_tokens as f64 / self.raw_tokens as f64)
    }

    /// Whether the tool reported failure.
    pub fn failed(&self) -> bool {
        matches!(self.status, ReductionStatus::Failure { .. })
    }

    /// Stores the raw output locally and records its expansion path
    /// (todo.md Phase 6: "Store raw output locally", "Provide expansion
    /// mechanism").
    pub fn store_raw(
        &mut self,
        output: &ToolOutput,
        store: &RawStore,
    ) -> io::Result<PathBuf> {
        let path = store.store(output)?;
        self.detail = Some(path.clone());
        Ok(path)
    }

    /// Reads the stored raw output back (the expansion mechanism).
    pub fn expand(&self, store: &RawStore) -> io::Result<String> {
        let Some(path) = &self.detail else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "raw output has not been stored",
            ));
        };
        store.expand(path)
    }
}
