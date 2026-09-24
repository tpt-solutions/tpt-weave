//! `.tpt-weave/manifest.toml` configuration (spec.md sections 6, 7 and 27).

use crate::SCHEMA_VERSION;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// Sub-directory holding all tpt-weave working state.
pub const MANIFEST_DIR: &str = ".tpt-weave";
/// Manifest file name inside [`MANIFEST_DIR`].
pub const MANIFEST_FILE: &str = "manifest.toml";
/// Default context budget in tokens (also used by
/// [`crate::ContextRequest::DEFAULT_BUDGET`]).
pub const DEFAULT_CONTEXT_BUDGET: u32 = 12_000;
/// Hard ceiling for context budgets.
pub const MAXIMUM_CONTEXT_BUDGET: u32 = 32_000;

/// Path of the manifest for a repository working tree.
pub fn manifest_path(repo_root: &Path) -> PathBuf {
    repo_root.join(MANIFEST_DIR).join(MANIFEST_FILE)
}

/// Finds the nearest working-tree root for a repository-aware command.
///
/// Search order is: an existing tpt-weave manifest, a Git root, then the
/// nearest Cargo workspace. A new repository with no marker is returned
/// unchanged, so `init` can still create its first manifest in place.
pub fn discover_repository_root(start: impl AsRef<Path>) -> Result<PathBuf, ConfigError> {
    let start = start.as_ref();
    let current = std::fs::canonicalize(start)?;
    let mut candidate = current.as_path();
    let mut git_candidate = None;
    let mut cargo_candidate = None;
    loop {
        if manifest_path(candidate).is_file() {
            return Ok(candidate.to_path_buf());
        }
        if git_candidate.is_none() && candidate.join(".git").exists() {
            git_candidate = Some(candidate.to_path_buf());
        }
        if cargo_candidate.is_none() && candidate.join("Cargo.toml").is_file() {
            cargo_candidate = Some(candidate.to_path_buf());
        }
        let Some(parent) = candidate.parent() else {
            break;
        };
        candidate = parent;
    }
    Ok(git_candidate.or(cargo_candidate).unwrap_or(current))
}

/// Errors produced while loading, validating or saving a [`Manifest`].
#[derive(Debug)]
pub enum ConfigError {
    /// Filesystem failure.
    Io(std::io::Error),
    /// The manifest is not valid TOML.
    Parse(toml::de::Error),
    /// The manifest could not be serialised.
    Serialize(toml::ser::Error),
    /// The manifest parsed but violates a constraint.
    Invalid(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(err) => write!(f, "i/o error: {err}"),
            ConfigError::Parse(err) => write!(f, "invalid manifest toml: {err}"),
            ConfigError::Serialize(err) => write!(f, "manifest serialisation failed: {err}"),
            ConfigError::Invalid(msg) => write!(f, "invalid manifest: {msg}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Io(err) => Some(err),
            ConfigError::Parse(err) => Some(err),
            ConfigError::Serialize(err) => Some(err),
            ConfigError::Invalid(_) => None,
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::Io(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        ConfigError::Parse(err)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(err: toml::ser::Error) -> Self {
        ConfigError::Serialize(err)
    }
}

/// Top-level manifest document written to `.tpt-weave/manifest.toml`.
/// This is configuration, not generated prose (spec.md section 7).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Manifest {
    /// Must equal [`SCHEMA_VERSION`].
    pub schema: u32,
    pub repository: String,
    pub language: String,
    pub workspace: bool,
    pub features: FeaturesConfig,
    pub context: ContextConfig,
    pub privacy: PrivacyConfig,
    pub jev: JevConfig,
    pub provider: ProviderConfig,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            schema: SCHEMA_VERSION,
            repository: String::new(),
            language: "rust".to_string(),
            workspace: true,
            features: FeaturesConfig::default(),
            context: ContextConfig::default(),
            privacy: PrivacyConfig::default(),
            jev: JevConfig::default(),
            provider: ProviderConfig::default(),
        }
    }
}

impl Manifest {
    /// Creates a manifest for `repository` with all other fields defaulted.
    pub fn new(repository: impl Into<String>) -> Self {
        Self {
            repository: repository.into(),
            ..Self::default()
        }
    }

    /// Loads and validates a manifest from disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path.as_ref())?;
        let manifest: Self = toml::from_str(&text)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validates the manifest and writes it to disk, creating parent
    /// directories as needed.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        self.validate()?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path.as_ref(), self.render()?)?;
        Ok(())
    }

    /// Renders the manifest as TOML text.
    pub fn render(&self) -> Result<String, ConfigError> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Validates every section (spec.md sections 7 and 27).
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schema != SCHEMA_VERSION {
            return Err(ConfigError::Invalid(format!(
                "unsupported schema {} (this build supports {SCHEMA_VERSION}); \
                 re-run `tpt-weave init`",
                self.schema
            )));
        }
        if self.repository.trim().is_empty() {
            return Err(ConfigError::Invalid(
                "`repository` must not be empty".to_string(),
            ));
        }
        if self.language.trim().is_empty() {
            return Err(ConfigError::Invalid(
                "`language` must not be empty".to_string(),
            ));
        }
        self.context.validate()?;
        self.privacy.validate()?;
        self.jev.validate()?;
        self.provider.validate()?;
        Ok(())
    }
}

/// Optional feature switches (spec.md section 7 `[features]`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeaturesConfig {
    pub symbol_index: bool,
    pub dependency_graph: bool,
    pub source_skeletons: bool,
    pub tool_output_reduction: bool,
    pub jev: bool,
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            symbol_index: true,
            dependency_graph: true,
            source_skeletons: true,
            tool_output_reduction: true,
            jev: true,
        }
    }
}

/// Context budgets in tokens (spec.md section 7 `[context]`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContextConfig {
    pub default_budget: u32,
    pub maximum_budget: u32,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            default_budget: DEFAULT_CONTEXT_BUDGET,
            maximum_budget: MAXIMUM_CONTEXT_BUDGET,
        }
    }
}

impl ContextConfig {
    /// Validates budget invariants (spec.md section 7).
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.default_budget == 0 {
            return Err(ConfigError::Invalid(
                "`context.default_budget` must be > 0".to_string(),
            ));
        }
        if self.maximum_budget < self.default_budget {
            return Err(ConfigError::Invalid(
                "`context.maximum_budget` must be >= `context.default_budget`".to_string(),
            ));
        }
        Ok(())
    }

    /// Clamps a requested budget into `[1, maximum_budget]`; 0 falls back to
    /// `default_budget`.
    pub fn clamp(&self, requested: u32) -> u32 {
        if requested == 0 {
            self.default_budget
        } else {
            requested.min(self.maximum_budget)
        }
    }
}

/// Privacy / redaction policy (spec.md section 27 `[privacy]`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacyConfig {
    /// When `false` (the default) no context or source content may leave the
    /// machine — fully local operation.
    pub remote_decisions: bool,
    /// Exclusion patterns applied to repository-relative paths before any
    /// content could be sent remotely. Supported forms: exact path segment
    /// (`.env`), directory (`secrets/`), extension (`*.pem`), prefix
    /// (`id_rsa*`). See `docs/decisions.md` section 5.
    pub exclude: Vec<String>,
    /// Additional private path patterns. These may be absolute paths or
    /// repository-relative prefixes and are excluded in addition to `exclude`.
    pub private_paths: Vec<String>,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            remote_decisions: false,
            exclude: [
                ".env",
                ".env.*",
                "secrets/",
                "credentials/",
                "*.pem",
                "*.key",
                "*.crt",
                "*.p12",
                "id_rsa*",
            ]
            .iter()
            .map(ToString::to_string)
            .collect(),
            private_paths: Vec::new(),
        }
    }
}

impl PrivacyConfig {
    /// Validates exclusion and private-path patterns.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.exclude.iter().any(String::is_empty) {
            return Err(ConfigError::Invalid(
                "`privacy.exclude` must not contain empty patterns".to_string(),
            ));
        }
        if self.private_paths.iter().any(String::is_empty) {
            return Err(ConfigError::Invalid(
                "`privacy.private_paths` must not contain empty patterns".to_string(),
            ));
        }
        Ok(())
    }

    /// Returns `true` when `rel_path` matches a configured exclusion pattern.
    pub fn is_excluded(&self, rel_path: &str) -> bool {
        let path = rel_path.replace('\\', "/");
        self.exclude
            .iter()
            .any(|pattern| matches_pattern(pattern, &path))
    }

    /// Returns `true` when an absolute or relative path matches a configured
    /// private path. This is intentionally separate from [`Self::is_excluded`]
    /// so callers can report which policy caused a rejection.
    pub fn is_private_path(&self, path: &str) -> bool {
        let path = path.replace('\\', "/");
        self.private_paths
            .iter()
            .any(|pattern| matches_private_path(pattern, &path))
    }
}

/// Glob-lite matcher for the four supported exclusion forms.
fn matches_pattern(pattern: &str, path: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    if let Some(directory) = pattern.strip_suffix('/') {
        return path.split('/').any(|segment| segment == directory);
    }
    if let Some(extension) = pattern.strip_prefix("*.") {
        return path.ends_with(&format!(".{extension}"));
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return path.split('/').any(|segment| segment.starts_with(prefix));
    }
    path.split('/').any(|segment| segment == pattern)
}

fn matches_private_path(pattern: &str, path: &str) -> bool {
    let pattern = pattern.replace('\\', "/").trim_end_matches('/').to_string();
    if pattern.is_empty() {
        return false;
    }
    let path = path.replace('\\', "/");
    if pattern.contains('/') {
        path == pattern || path.starts_with(&format!("{pattern}/"))
    } else {
        path.split('/').any(|segment| segment == pattern)
    }
}

/// JEv decision settings (spec.md sections 4 and 4.3, todo.md Phase 8).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct JevConfig {
    pub enabled: bool,
    /// OpenRouter model id.
    pub model: String,
    /// Minimum confidence required to act on a decision without expanding
    /// (spec.md section 4.3: thresholds must be configurable).
    pub min_confidence: f32,
    pub timeout_ms: u64,
    pub max_retries: u32,
}

impl Default for JevConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model: "typesafe/jev-1.13".to_string(),
            min_confidence: 0.70,
            timeout_ms: 5_000,
            max_retries: 2,
        }
    }
}

impl JevConfig {
    /// Validates thresholds and provider-independent constraints.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !(0.0..=1.0).contains(&self.min_confidence) {
            return Err(ConfigError::Invalid(
                "`jev.min_confidence` must be within 0.0..=1.0".to_string(),
            ));
        }
        if self.enabled && self.model.trim().is_empty() {
            return Err(ConfigError::Invalid(
                "`jev.model` must not be empty when JEv is enabled".to_string(),
            ));
        }
        if self.timeout_ms == 0 {
            return Err(ConfigError::Invalid(
                "`jev.timeout_ms` must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Remote decision provider settings (todo.md Phase 8). API keys are never
/// stored here — only the environment variable *name*
/// (`docs/decisions.md` section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderConfig {
    /// Provider name, e.g. `openrouter`.
    pub name: String,
    /// Decisions endpoint (spec.md section 18:
    /// `https://openrouter.ai/api/alpha/decisions`).
    pub endpoint: String,
    /// Environment variable *name* holding the API key.
    pub api_key_env: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            name: "openrouter".to_string(),
            endpoint: "https://openrouter.ai/api/alpha/decisions".to_string(),
            api_key_env: "OPENROUTER_API_KEY".to_string(),
        }
    }
}

impl ProviderConfig {
    /// Validates endpoint scheme and environment variable name syntax.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.name.trim().is_empty() {
            return Err(ConfigError::Invalid(
                "`provider.name` must not be empty".to_string(),
            ));
        }
        if !self.endpoint.starts_with("http://") && !self.endpoint.starts_with("https://") {
            return Err(ConfigError::Invalid(
                "`provider.endpoint` must be an http(s) url".to_string(),
            ));
        }
        if self.api_key_env.is_empty()
            || !self
                .api_key_env
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(ConfigError::Invalid(
                "`provider.api_key_env` must be a valid environment variable name".to_string(),
            ));
        }
        Ok(())
    }
}
