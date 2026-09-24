//! Standard TPT integration artifacts and repository registry metadata.

use crate::{MANIFEST_DIR, SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

/// Standard file names below `.tpt-weave/`.
pub const GRAPH_FILE: &str = "graph.json";
pub const BASELINE_REPORT_FILE: &str = "baseline-report.json";
pub const PARSED_DIR: &str = "parsed";
pub const CACHE_DIR: &str = "cache";
pub const AUDIT_FILE: &str = "privacy-audit.jsonl";
pub const REGISTRY_FILE: &str = "registry.json";
/// Generated standard agent configuration filename.
pub const AGENT_CONFIG_FILE: &str = "agent.json";
/// Generated standard MCP client configuration filename.
pub const MCP_CONFIG_FILE: &str = "mcp.json";
/// Standard `.tpt-weave/` path for a generated artifact.
pub fn standard_path(repo_root: &Path, relative: &str) -> PathBuf {
    repo_root.join(MANIFEST_DIR).join(relative)
}

/// Registry entry for one TPT repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub path: PathBuf,
    pub language: String,
    pub workspace: bool,
}

impl RegistryEntry {
    /// Creates a Rust repository entry.
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            language: "rust".to_string(),
            workspace: true,
        }
    }

    /// Validates the entry before it is used for discovery.
    pub fn validate(&self) -> Result<(), IntegrationError> {
        if self.name.trim().is_empty() {
            return Err(IntegrationError::Invalid(
                "registry entry name must not be empty".to_string(),
            ));
        }
        if self.path.as_os_str().is_empty() {
            return Err(IntegrationError::Invalid(format!(
                "registry entry `{}` has an empty path",
                self.name
            )));
        }
        if self.language.trim().is_empty() {
            return Err(IntegrationError::Invalid(format!(
                "registry entry `{}` has an empty language",
                self.name
            )));
        }
        Ok(())
    }

    /// Resolves a relative entry path against the registry location.
    pub fn resolved_path(&self, registry_path: &Path) -> PathBuf {
        if self.path.is_absolute() {
            self.path.clone()
        } else {
            registry_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(&self.path)
        }
    }
}

/// A standard registry document shared by TPT agent tooling.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryRegistry {
    pub schema: u32,
    #[serde(default)]
    pub repositories: Vec<RegistryEntry>,
}

impl Default for RepositoryRegistry {
    fn default() -> Self {
        Self {
            schema: SCHEMA_VERSION,
            repositories: Vec::new(),
        }
    }
}

impl RepositoryRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates schema and all entries.
    pub fn validate(&self) -> Result<(), IntegrationError> {
        if self.schema != SCHEMA_VERSION {
            return Err(IntegrationError::Schema {
                found: self.schema,
                expected: SCHEMA_VERSION,
            });
        }
        let mut names = BTreeSet::new();
        for entry in &self.repositories {
            entry.validate()?;
            if !names.insert(entry.name.clone()) {
                return Err(IntegrationError::Invalid(format!(
                    "duplicate registry entry `{}`",
                    entry.name
                )));
            }
        }
        Ok(())
    }

    /// Finds an entry by repository name.
    pub fn get(&self, name: &str) -> Option<&RegistryEntry> {
        self.repositories.iter().find(|entry| entry.name == name)
    }

    /// Adds or replaces an entry, preserving sorted deterministic order.
    pub fn upsert(&mut self, entry: RegistryEntry) -> Result<(), IntegrationError> {
        entry.validate()?;
        self.repositories
            .retain(|existing| existing.name != entry.name);
        self.repositories.push(entry);
        self.repositories
            .sort_by(|left, right| left.name.cmp(&right.name));
        self.validate()
    }

    /// Loads the optional standard registry at `<root>/.tpt-weave/registry.json`.
    /// Returns `None` when no registry is configured.
    pub fn load_standard(root: &Path) -> Result<Option<Self>, IntegrationError> {
        let path = standard_path(root, REGISTRY_FILE);
        if path.is_file() {
            Self::load(path).map(Some)
        } else {
            Ok(None)
        }
    }

    /// Resolves a repository name to an absolute or registry-relative path.
    pub fn resolve_path(&self, registry_path: &Path, name: &str) -> Option<PathBuf> {
        self.get(name)
            .map(|entry| entry.resolved_path(registry_path))
    }

    /// Parses a JSON registry document.
    pub fn from_json(text: &str) -> Result<Self, IntegrationError> {
        let registry: Self = serde_json::from_str(text).map_err(IntegrationError::Json)?;
        registry.validate()?;
        Ok(registry)
    }

    /// Renders the registry as pretty JSON.
    pub fn to_json(&self) -> Result<String, IntegrationError> {
        serde_json::to_string_pretty(self).map_err(IntegrationError::Json)
    }

    /// Loads a registry file and validates it.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, IntegrationError> {
        let path = path.as_ref();
        Self::from_json(&std::fs::read_to_string(path).map_err(IntegrationError::Io)?)
    }

    /// Writes a validated registry file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), IntegrationError> {
        self.validate()?;
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(IntegrationError::Io)?;
        }
        std::fs::write(path, self.to_json()?).map_err(IntegrationError::Io)
    }
}

/// Standard environment variables used by the CLI/MCP integration.
pub const ENV_REPOSITORY_PATH: &str = "TPT_WEAVE_PATH";
pub const ENV_MCP_TOOLS: &str = "TPT_WEAVE_TOOLS";
pub const ENV_PROVIDER_KEY: &str = "OPENROUTER_API_KEY";

/// The standard agent environment contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentEnvironment {
    pub repository: String,
    pub mcp_server: String,
    #[serde(default)]
    pub tools: Vec<String>,
}

impl AgentEnvironment {
    /// Builds the standard environment for a repository and MCP binary.
    pub fn new(repository: impl Into<String>, mcp_server: impl Into<String>) -> Self {
        Self {
            repository: repository.into(),
            mcp_server: mcp_server.into(),
            tools: vec![
                "orient".to_string(),
                "symbol".to_string(),
                "context".to_string(),
            ],
        }
    }

    /// Converts the contract to environment variables for a process launch.
    pub fn to_env(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            (ENV_REPOSITORY_PATH.to_string(), self.repository.clone()),
            (ENV_MCP_TOOLS.to_string(), self.tools.join(",")),
        ])
    }
}

/// A standard MCP client configuration entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl McpServerConfig {
    /// Creates a standard `tpt-weave-mcp` server configuration.
    pub fn new(mcp_server: impl Into<String>, repository: impl Into<String>) -> Self {
        let mut env = BTreeMap::new();
        env.insert(ENV_REPOSITORY_PATH.to_string(), repository.into());
        Self {
            name: "tpt-weave".to_string(),
            command: mcp_server.into(),
            args: Vec::new(),
            env,
        }
    }

    /// Renders the conventional JSON MCP client object.
    pub fn to_json(&self) -> Result<String, IntegrationError> {
        serde_json::to_string_pretty(self).map_err(IntegrationError::Json)
    }
}

/// Errors from standard integration artefacts.
#[derive(Debug)]
pub enum IntegrationError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
    Schema { found: u32, expected: u32 },
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "integration i/o error: {error}"),
            Self::Json(error) => write!(f, "integration JSON error: {error}"),
            Self::Invalid(detail) => write!(f, "invalid integration metadata: {detail}"),
            Self::Schema { found, expected } => {
                write!(f, "integration schema {found} does not match {expected}")
            }
        }
    }
}

impl std::error::Error for IntegrationError {}

impl From<std::io::Error> for IntegrationError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for IntegrationError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
