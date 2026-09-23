//! Graph persistence and invalidation (todo.md Phase 3).

use crate::model::RepositoryGraph;
use std::path::{Path, PathBuf};
use tpt_weave_core::{MANIFEST_DIR, SCHEMA_VERSION};

/// Graph artefact file name inside `.tpt-weave/`.
pub const GRAPH_FILE: &str = "graph.json";

/// Path of the graph artefact for a repository working tree.
pub fn graph_path(repo_root: &Path) -> PathBuf {
    repo_root.join(MANIFEST_DIR).join(GRAPH_FILE)
}

/// Errors from graph (de)serialisation.
#[derive(Debug)]
pub enum GraphError {
    /// Filesystem failure.
    Io(std::io::Error),
    /// The JSON could not be parsed.
    Json(serde_json::Error),
    /// The artefact was written by a different schema (stale artefact —
    /// re-run the indexer, spec.md section 21).
    SchemaMismatch { found: u32, expected: u32 },
    /// A reference edge points at a symbol that is not in the table.
    DanglingEdge(String),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::Io(err) => write!(f, "graph i/o error: {err}"),
            GraphError::Json(err) => write!(f, "invalid graph json: {err}"),
            GraphError::SchemaMismatch { found, expected } => write!(
                f,
                "graph schema {found} does not match expected {expected}; re-index"
            ),
            GraphError::DanglingEdge(detail) => write!(f, "dangling graph edge: {detail}"),
        }
    }
}

impl std::error::Error for GraphError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GraphError::Io(err) => Some(err),
            GraphError::Json(err) => Some(err),
            GraphError::SchemaMismatch { .. } | GraphError::DanglingEdge(_) => None,
        }
    }
}

impl From<std::io::Error> for GraphError {
    fn from(err: std::io::Error) -> Self {
        GraphError::Io(err)
    }
}

impl From<serde_json::Error> for GraphError {
    fn from(err: serde_json::Error) -> Self {
        GraphError::Json(err)
    }
}

impl RepositoryGraph {
    /// Serialises the graph as pretty JSON (deterministic field order).
    pub fn to_json(&self) -> Result<String, GraphError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Parses graph JSON, failing closed on schema mismatch.
    pub fn from_json(json: &str) -> Result<Self, GraphError> {
        let graph: Self = serde_json::from_str(json)?;
        if graph.schema != SCHEMA_VERSION {
            return Err(GraphError::SchemaMismatch {
                found: graph.schema,
                expected: SCHEMA_VERSION,
            });
        }
        Ok(graph)
    }

    /// Writes the graph to `path`, creating parent directories.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), GraphError> {
        self.validate()?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path.as_ref(), self.to_json()?)?;
        Ok(())
    }

    /// Loads a graph from `path` (schema validated).
    pub fn load(path: impl AsRef<Path>) -> Result<Self, GraphError> {
        Self::from_json(&std::fs::read_to_string(path.as_ref())?)
    }

    /// Fails on internal inconsistencies (dangling edge endpoints).
    pub fn validate(&self) -> Result<(), GraphError> {
        for edge in &self.references {
            if self.symbol(&edge.from).is_none() || self.symbol(&edge.to).is_none() {
                return Err(GraphError::DanglingEdge(format!(
                    "{} -[{:?}]-> {}",
                    edge.from, edge.kind, edge.to
                )));
            }
        }
        Ok(())
    }
}
