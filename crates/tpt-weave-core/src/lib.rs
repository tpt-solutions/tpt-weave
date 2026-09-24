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
pub mod integration;
pub mod privacy;
pub mod tokens;

pub use config::{
    ConfigError, ContextConfig, DEFAULT_CONTEXT_BUDGET, FeaturesConfig, JevConfig, MANIFEST_DIR,
    MANIFEST_FILE, MAXIMUM_CONTEXT_BUDGET, Manifest, PrivacyConfig, ProviderConfig,
    discover_repository_root, manifest_path,
};
pub use context::{ContextCandidate, ContextLevel, ContextRequest, ContextResponse, ContextSource};
pub use files::{FileRecord, SourceLanguage};
pub use ids::{ContextId, RepositoryId, Revision, SymbolId, SymbolKind};
pub use integration::{
    AGENT_CONFIG_FILE, AgentEnvironment, GRAPH_FILE, IntegrationError, MCP_CONFIG_FILE,
    McpServerConfig, REGISTRY_FILE, RegistryEntry, RepositoryRegistry, standard_path,
};
pub use privacy::{REDACTION_MARKER, Redaction, SecretFinding, SecretKind, redact_secrets};
pub use tokens::TokenAccounting;

/// Schema version written into `.tpt-weave/manifest.toml` and every generated
/// index artifact. See `docs/decisions.md` section 4 for the bump policy.
pub const SCHEMA_VERSION: u32 = 1;
