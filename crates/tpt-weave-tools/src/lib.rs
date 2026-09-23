//! Deterministic tool-output reduction (todo.md Phase 6, spec.md section
//! 14).
//!
//! Verbose tool output (`cargo test`, `cargo check`, `git diff`, listings,
//! search results, JSON, logs, compiler diagnostics) is transformed into a
//! compact deterministic record that preserves important facts and failure
//! details. The raw output is always kept by the caller, storable via
//! [`RawStore`] and expandable again through [`Reduction::expand`], and
//! every [`Reduction`] measures its own token reduction.

#![forbid(unsafe_code)]

mod cargo;
mod git;
mod json;
mod kind;
mod listing;
mod log;
mod reduce;
mod reduction;
mod search;
mod store;

pub use kind::ToolKind;
pub use reduce::{reduce, ToolOutput};
pub use reduction::{Reduction, ReductionStatus};
pub use store::RawStore;
