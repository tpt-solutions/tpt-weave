//! `tpt-weave integration` — standard agent, MCP, and registry metadata.

use super::Rendered;
use crate::args::{Cli, IntegrationAction};
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use tpt_weave_core::{
    AGENT_CONFIG_FILE, AgentEnvironment, MCP_CONFIG_FILE, McpServerConfig, RepositoryRegistry,
    standard_path,
};
use tpt_weave_index::CargoIndex;

pub fn run(cli: &Cli, action: IntegrationAction, write: bool) -> Result<Rendered, CliError> {
    let root = Workspace::canonical_root(&cli.path)?;
    let root_display = display_path(&root);
    let repository = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "workspace".to_string());

    match action {
        IntegrationAction::Agent => {
            let contract = AgentEnvironment::new(root_display, "tpt-weave-mcp");
            let env = contract.to_env();
            let value = json!({
                "schema": 1,
                "repository": contract.repository,
                "mcp_server": contract.mcp_server,
                "tools": contract.tools,
                "env": env,
            });
            let path = standard_path(&root, AGENT_CONFIG_FILE);
            write_optional(&path, &value, write)?;
            Ok(
                Rendered::new(format!("standard agent contract for {repository}"), value)
                    .with_detail(format!(
                        "file: {}\nenv: TPT_WEAVE_PATH, TPT_WEAVE_TOOLS",
                        path.display()
                    )),
            )
        }
        IntegrationAction::Mcp => {
            let config = McpServerConfig::new("tpt-weave-mcp", &root_display);
            let value: serde_json::Value = serde_json::from_str(&config.to_json()?)
                .map_err(|error| CliError::internal(error.to_string()))?;
            let path = standard_path(&root, MCP_CONFIG_FILE);
            write_optional(&path, &value, write)?;
            Ok(Rendered::new(
                format!("standard MCP server configuration for {repository}"),
                value,
            )
            .with_detail(format!("file: {}", path.display())))
        }
        IntegrationAction::Registry => {
            if write {
                return Err(CliError::usage(
                    "`integration registry --write` is not supported; registry paths are maintained explicitly",
                ));
            }
            let registry = RepositoryRegistry::load_standard(&root)
                .map_err(|error| CliError::internal(error.to_string()))?;
            let cargo = CargoIndex::load(&root)?;
            let discovered = cargo.discover_tpt_dependencies();
            let value = json!({
                "schema": 1,
                "repository": repository,
                "registry": registry,
                "discovered_tpt_dependencies": discovered
                    .iter()
                    .map(|(package, repository)| json!({
                        "package": package,
                        "repository": repository,
                    }))
                    .collect::<Vec<_>>(),
                "registry_path": standard_path(&root, tpt_weave_core::REGISTRY_FILE)
                    .display()
                    .to_string(),
            });
            Ok(Rendered::new(
                format!(
                    "{} discovered dependency link(s); registry {}",
                    discovered.len(),
                    if registry.is_some() {
                        "loaded"
                    } else {
                        "not configured"
                    }
                ),
                value,
            ))
        }
    }
}

fn display_path(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
}

fn write_optional(
    path: &std::path::Path,
    value: &serde_json::Value,
    write: bool,
) -> Result<(), CliError> {
    if !write {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| CliError::internal(format!("{}: {error}", parent.display())))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| CliError::internal(error.to_string()))?;
    std::fs::write(path, text)
        .map_err(|error| CliError::internal(format!("{}: {error}", path.display())))
}
