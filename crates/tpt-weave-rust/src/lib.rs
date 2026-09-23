//! Rust source extraction via `syn` (spec.md section 8.2, todo.md Phase 2).
//!
//! Extracts modules, structs, enums, traits, impl blocks, functions,
//! methods, constants, type aliases and macros together with visibility,
//! single-line signatures, attributes and source locations.
//!
//! Known limitation: an out-of-line `mod x;` is recorded but its file is not
//! loaded from here — module-tree file resolution happens when the indexer
//! walks a whole crate (and the module graph arrives in Phase 3).
//!
//! Known limitation: a method id (`Type::method`) may coincide when two
//! trait impls for the same type declare the same method name;
//! [`record::SymbolRecord::parent`] (the enclosing impl's canonical key)
//! then distinguishes the records.

#![forbid(unsafe_code)]

pub mod parse;
pub mod record;

pub use parse::{parse_file, FileInput, ParseError, ParsedFile};
pub use record::{SymbolRecord, Visibility};
