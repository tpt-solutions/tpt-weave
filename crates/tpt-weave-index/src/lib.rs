//! Deterministic repository indexing: Cargo metadata and Git state
//! (todo.md Phase 2; spec.md sections 8.1 and 8.3).
//!
//! - [`cargo`] — workspace members, packages, targets, dependencies,
//!   features and optional dependencies, projected from
//!   `cargo metadata --no-deps` into a stable serialisable model.
//! - [`git`] — repository root discovery, current revision, working-tree
//!   status, current diff, changed-file queries and history lookup.

#![forbid(unsafe_code)]

pub mod cargo;
pub mod git;

pub use cargo::{
    CargoIndex, DependencyIndex, DependencyKind, IndexError, PackageIndex, TargetIndex,
};
pub use git::{
    ChangeStatus, CommitInfo, DiffEntry, FileStatus, GitError, GitRepository,
};
