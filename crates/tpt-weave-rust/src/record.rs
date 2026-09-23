//! Symbol extraction records (todo.md Phase 2 "Source parsing").

use serde::{Deserialize, Serialize};
use std::fmt;
use tpt_weave_core::SymbolId;

/// Visibility of an extracted symbol.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// `pub`
    Public,
    /// `pub(crate)`
    Crate,
    /// `pub(in path)` with the rendered path.
    Restricted(String),
    /// No visibility modifier (private).
    Inherited,
}

impl Visibility {
    /// Returns `true` only for plain `pub`.
    pub fn is_public(&self) -> bool {
        matches!(self, Visibility::Public)
    }
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Public => f.write_str("pub"),
            Visibility::Crate => f.write_str("pub(crate)"),
            Visibility::Restricted(path) => write!(f, "pub(in {path})"),
            Visibility::Inherited => Ok(()),
        }
    }
}

/// One extracted symbol with its source location and rendered signature.
///
/// Serialises to the `symbols.json` index artefact (spec.md section 6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolRecord {
    pub id: SymbolId,
    /// Repository-relative path of the containing file (`/`-separated).
    pub file: String,
    /// 1-based line of the symbol's name (or `impl` keyword).
    pub line: u32,
    /// 1-based column of the symbol's name.
    pub column: u32,
    pub visibility: Visibility,
    /// Single-line rendered signature without any function body.
    pub signature: String,
    /// Raw attribute strings, e.g. `#[derive(Clone)]` and `#[doc = "..."]`.
    pub attributes: Vec<String>,
    /// Canonical key of the enclosing symbol (module, impl or trait), when
    /// there is one; `None` at file top level.
    pub parent: Option<String>,
}

impl SymbolRecord {
    /// The symbol's simple name (methods are `Type::method`).
    pub fn name(&self) -> &str {
        &self.id.name
    }

    /// The symbol's module path within its package.
    pub fn module(&self) -> &str {
        &self.id.module
    }
}
