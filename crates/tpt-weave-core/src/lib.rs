//! Core repository model types, token accounting and manifest configuration
//! for tpt-weave (Phase 1 of `todo.md`; design in `spec.md` sections 6, 7, 9,
//! 16, 26 and 27).
//!
//! Stability: this crate's public modules are stable-by-intent; experimental
//! APIs are gated behind the `unstable` cargo feature. See
//! `docs/decisions.md` section 6.

#![forbid(unsafe_code)]

pub mod config;
pub mod context;
pub mod files;
pub mod hash;
pub mod ids;
pub mod tokens;

pub use config::{
    manifest_path, ConfigError, ContextConfig, FeaturesConfig, JevConfig, Manifest, PrivacyConfig,
    ProviderConfig, DEFAULT_CONTEXT_BUDGET, MANIFEST_DIR, MANIFEST_FILE,
    MAXIMUM_CONTEXT_BUDGET,
};
pub use context::{
    ContextCandidate, ContextLevel, ContextRequest, ContextResponse, ContextSource,
};
pub use files::{FileRecord, SourceLanguage};
pub use ids::{ContextId, RepositoryId, Revision, SymbolId, SymbolKind};
pub use tokens::TokenAccounting;

/// Schema version written into `.tpt-weave/manifest.toml` and every generated
/// index artifact. See `docs/decisions.md` section 4 for the bump policy.
pub const SCHEMA_VERSION: u32 = 1;
