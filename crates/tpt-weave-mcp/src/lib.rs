//! MCP server exposing tpt-weave context tools (todo.md Phase 9;
//! spec.md section 19).
//!
//! The binary (`tpt-weave-mcp`) speaks MCP over stdio. Tool exposure is
//! filtered via [`ToolFilter`] / `TPT_WEAVE_TOOLS`; schemas stay compact
//! so clients pay less tool-list overhead.

#![forbid(unsafe_code)]

pub mod catalog;
pub mod server;
pub mod tools;
pub mod workspace;

pub use catalog::{ToolFilter, ToolGroup, ToolSpec, all_tools, schema_overhead};
pub use server::TptWeaveServer;
pub use tools::ToolError;
pub use workspace::{NO_REVISION, Workspace, WorkspaceError};
