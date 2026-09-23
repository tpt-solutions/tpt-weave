//! rmcp `ServerHandler` wiring for tpt-weave tools
//! (todo.md Phase 9; hand-rolled — no `#[tool]` macros, dynamic filtering).

use std::sync::{Arc, Mutex};

use rmcp::ErrorData as McpError;
use rmcp::RoleServer;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::RequestContext;

use crate::catalog::{ToolFilter, ToolSpec};
use crate::workspace::Workspace;

/// MCP server over a loaded [`Workspace`] with a [`ToolFilter`].
pub struct TptWeaveServer {
    workspace: Mutex<Workspace>,
    filter: ToolFilter,
}

impl TptWeaveServer {
    /// Builds a server that owns `workspace` and only exposes `filter` tools.
    pub fn new(workspace: Workspace, filter: ToolFilter) -> Self {
        Self {
            workspace: Mutex::new(workspace),
            filter,
        }
    }

    /// The active tool filter.
    pub fn filter(&self) -> &ToolFilter {
        &self.filter
    }

    fn with_workspace<T>(&self, f: impl FnOnce(&mut Workspace) -> T) -> Result<T, McpError> {
        let mut guard = self
            .workspace
            .lock()
            .map_err(|_| McpError::internal_error("workspace lock poisoned", None))?;
        Ok(f(&mut guard))
    }

    fn to_tool(spec: &ToolSpec) -> Tool {
        let schema: JsonObject = match spec.schema.clone() {
            serde_json::Value::Object(map) => map,
            _ => JsonObject::new(),
        };
        Tool::new(spec.name, spec.description, Arc::new(schema))
    }
}

fn tool_error_result(message: impl Into<String>) -> CallToolResponse {
    CallToolResult::error(vec![ContentBlock::text(message.into())]).into()
}

fn tool_ok_result(text: impl Into<String>) -> CallToolResponse {
    CallToolResult::success(vec![ContentBlock::text(text.into())]).into()
}

impl ServerHandler for TptWeaveServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "tpt-weave: repository overview, symbol lookup, hierarchical context, \
                 and workspace tools. Use tpt_repo_overview first.",
            )
            .with_server_info(Implementation::new(
                "tpt-weave-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        let tools: Vec<Tool> = self
            .filter
            .tools()
            .iter()
            .map(|spec| Self::to_tool(spec))
            .collect();
        std::future::ready(Ok(ListToolsResult::with_all_items(tools)))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.filter
            .tools()
            .into_iter()
            .find(|spec| spec.name == name)
            .map(Self::to_tool)
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResponse, McpError>> + Send + '_ {
        let name = request.name.to_string();
        let args = match request.arguments {
            Some(map) => serde_json::Value::Object(map),
            None => serde_json::Value::Object(Default::default()),
        };

        async move {
            if !self.filter.allows(&name) {
                return Err(McpError::method_not_found::<
                    rmcp::model::CallToolRequestMethod,
                >());
            }

            let outcome = self.with_workspace(|ws| {
                // Conservative invalidation: rebuild when HEAD moved (spec §21).
                let refresh = ws.refresh_if_stale().map(|_| ());
                match refresh {
                    Ok(()) => ws.call_tool(&name, &args),
                    Err(err) => Err(crate::tools::ToolError::from_workspace(err)),
                }
            });

            match outcome {
                Ok(Ok(text)) => Ok(tool_ok_result(text)),
                Ok(Err(err)) => Ok(tool_error_result(err.to_string())),
                Err(err) => Ok(tool_error_result(err.to_string())),
            }
        }
    }
}
