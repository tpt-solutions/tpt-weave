//! Compact MCP tool catalog and dynamic exposure
//! (todo.md Phase 9 "Tool minimisation": compact schemas, avoid sending
//! unnecessary tools, measure schema overhead; spec.md section 19).

use serde_json::{Value, json};
use std::sync::OnceLock;

/// Logical group a tool belongs to (for filtered exposure).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolGroup {
    /// Repository orientation: overview + stats.
    Orient,
    /// Symbol lookup and relationships.
    Symbol,
    /// Representations and expansion.
    Context,
    /// Git and test output.
    Workspace,
}

/// One MCP tool definition with a hand-written compact input schema.
#[derive(Clone, Debug)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub group: ToolGroup,
    /// Compact JSON Schema for `inputSchema`.
    pub schema: Value,
}

fn obj(props: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": props,
        "required": required,
        "additionalProperties": false,
    })
}

fn s(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

fn opt_s(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

fn build_tools() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "tpt_repo_overview",
            description: "Repo overview: crates, files, symbols, deps.",
            group: ToolGroup::Orient,
            schema: obj(json!({}), &[]),
        },
        ToolSpec {
            name: "tpt_find_symbol",
            description: "Find symbols by name substring.",
            group: ToolGroup::Symbol,
            schema: obj(json!({ "query": s("Name substring") }), &["query"]),
        },
        ToolSpec {
            name: "tpt_find_references",
            description: "Resolved references from a symbol key.",
            group: ToolGroup::Symbol,
            schema: obj(json!({ "key": s("Canonical symbol key") }), &["key"]),
        },
        ToolSpec {
            name: "tpt_get_signature",
            description: "Signature of one symbol.",
            group: ToolGroup::Symbol,
            schema: obj(json!({ "key": s("Canonical symbol key") }), &["key"]),
        },
        ToolSpec {
            name: "tpt_get_skeleton",
            description: "Skeleton of one file (no bodies).",
            group: ToolGroup::Context,
            schema: obj(json!({ "path": s("Repo-relative path") }), &["path"]),
        },
        ToolSpec {
            name: "tpt_expand",
            description: "Expand symbol/module/dep/test/related to a level.",
            group: ToolGroup::Context,
            schema: obj(
                json!({
                    "kind": json!({"type":"string","enum":["symbol","module","dependency","test","related"],"description":"What to expand"}),
                    "id": s("Symbol key / module path / package"),
                    "package": opt_s("Package (module/dependency)"),
                    "level": json!({"type":"integer","minimum":0,"maximum":5,"description":"0..5 (default 3)"}),
                }),
                &["kind", "id"],
            ),
        },
        ToolSpec {
            name: "tpt_dependencies",
            description: "Declared dependencies of a package.",
            group: ToolGroup::Symbol,
            schema: obj(json!({ "package": s("Package name") }), &["package"]),
        },
        ToolSpec {
            name: "tpt_related",
            description: "Symbols related to a symbol key.",
            group: ToolGroup::Symbol,
            schema: obj(json!({ "key": s("Canonical symbol key") }), &["key"]),
        },
        ToolSpec {
            name: "tpt_git_diff",
            description: "Compact working-tree diff summary.",
            group: ToolGroup::Workspace,
            schema: obj(json!({}), &[]),
        },
        ToolSpec {
            name: "tpt_test_result",
            description: "Reduce cargo test output to facts.",
            group: ToolGroup::Workspace,
            schema: obj(
                json!({
                    "output": s("Raw test stdout/stderr"),
                    "exit_code": json!({"type":"integer","description":"Process exit code (default 0)"}),
                }),
                &["output"],
            ),
        },
        ToolSpec {
            name: "tpt_context_stats",
            description: "Token/cache stats for this workspace.",
            group: ToolGroup::Orient,
            schema: obj(json!({}), &[]),
        },
    ]
}

fn tools_static() -> &'static [ToolSpec] {
    static TOOLS: OnceLock<Vec<ToolSpec>> = OnceLock::new();
    TOOLS.get_or_init(build_tools)
}

/// Every tool from spec.md section 19, with compact schemas.
pub fn all_tools() -> &'static [ToolSpec] {
    tools_static()
}

/// Which tools to expose (todo.md Phase 9: dynamic tool exposure).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolFilter {
    /// Exposed tool names (must be a subset of [`all_tools`] names).
    names: Vec<&'static str>,
}

impl ToolFilter {
    /// Every tool.
    pub fn all() -> Self {
        Self {
            names: all_tools().iter().map(|t| t.name).collect(),
        }
    }

    /// Only the given groups (deduped, catalog order preserved).
    pub fn groups(groups: &[ToolGroup]) -> Self {
        Self {
            names: all_tools()
                .iter()
                .filter(|t| groups.contains(&t.group))
                .map(|t| t.name)
                .collect(),
        }
    }

    /// Preset from a comma-separated list of group names or `all`:
    /// `orient`, `symbol`, `context`, `workspace`.
    pub fn parse(spec: &str) -> Self {
        let spec = spec.trim();
        if spec.is_empty() || spec.eq_ignore_ascii_case("all") {
            return Self::all();
        }
        let mut groups = Vec::new();
        for part in spec.split(',').map(str::trim) {
            match part.to_ascii_lowercase().as_str() {
                "orient" | "overview" => groups.push(ToolGroup::Orient),
                "symbol" | "symbols" => groups.push(ToolGroup::Symbol),
                "context" | "ctx" => groups.push(ToolGroup::Context),
                "workspace" | "git" => groups.push(ToolGroup::Workspace),
                "all" => return Self::all(),
                _ => {}
            }
        }
        if groups.is_empty() {
            Self::all()
        } else {
            Self::groups(&groups)
        }
    }

    /// Filter from `TPT_WEAVE_TOOLS` (defaults to all when unset).
    pub fn from_env() -> Self {
        match std::env::var("TPT_WEAVE_TOOLS") {
            Ok(value) => Self::parse(&value),
            Err(_) => Self::all(),
        }
    }

    /// Whether `name` is exposed.
    pub fn allows(&self, name: &str) -> bool {
        // `contains` needs `&&'static str`; lookup names are non-static `&str`.
        #[allow(clippy::manual_contains)]
        self.names.iter().any(|n| *n == name)
    }

    /// Exposed catalog entries, in catalog order.
    pub fn tools(&self) -> Vec<&'static ToolSpec> {
        all_tools().iter().filter(|t| self.allows(t.name)).collect()
    }

    /// Names exposed.
    pub fn names(&self) -> &[&'static str] {
        &self.names
    }
}

impl Default for ToolFilter {
    fn default() -> Self {
        Self::all()
    }
}

/// Serialized `tools/list` payload size in bytes for a filter
/// (todo.md Phase 9: "Measure MCP schema overhead").
pub fn schema_overhead(filter: &ToolFilter) -> usize {
    let tools: Vec<Value> = filter
        .tools()
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": t.schema,
            })
        })
        .collect();
    serde_json::to_vec(&tools)
        .map(|bytes| bytes.len())
        .unwrap_or(0)
}
