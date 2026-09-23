//! `tpt-weave-mcp` binary: MCP server on stdio (todo.md Phase 9).

#![forbid(unsafe_code)]

use rmcp::ServiceExt;
use tpt_weave_mcp::{ToolFilter, TptWeaveServer, Workspace, WorkspaceError};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::var("TPT_WEAVE_PATH").unwrap_or_else(|_| ".".to_string());
    let workspace =
        Workspace::load(&root).map_err(|e: WorkspaceError| std::io::Error::other(e.to_string()))?;
    let server = TptWeaveServer::new(workspace, ToolFilter::from_env());
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
