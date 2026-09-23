//! Rust source extraction via `syn` (spec.md section 8.2, todo.md Phase 2).
//!
//! Extracts modules, structs, enums, traits, impl blocks, functions,
//! methods, constants, type aliases and macros together with visibility,
//! single-line signatures, attributes and source locations, plus
//! [`mentions`] — identifier references used for the symbol graph
//! (todo.md Phase 3) — and [`skeleton`] rendering for hierarchical
//! context levels (todo.md Phase 4).
//!
//! Known limitation: an out-of-line `mod x;` is recorded but its file is not
//! loaded from here — module-tree file resolution happens when the indexer
//! walks a whole crate.
//!
//! Known limitation: a method id (`Type::method`) may coincide when two
//! trait impls for the same type declare the same method name;
//! [`record::SymbolRecord::parent`] (the enclosing impl's canonical key)
//! then distinguishes the records.

#![forbid(unsafe_code)]

pub mod mentions;
pub mod parse;
pub mod record;
pub mod skeleton;

pub use mentions::{Mention, MentionKind};
pub use parse::{parse_file, FileInput, ParseError, ParsedFile};
pub use record::{SymbolRecord, Visibility};
pub use skeleton::{skeleton, skeleton_with, Skeleton, BODY_PLACEHOLDER};

