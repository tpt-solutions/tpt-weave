//! `tpt-weave init` — write `.tpt-weave/manifest.toml` (idempotent).

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use tpt_weave_core::{Manifest, manifest_path};

pub fn run(cli: &Cli, force: bool) -> Result<Rendered, CliError> {
    let root = Workspace::canonical_root(&cli.path)?;
    let path = manifest_path(&root);

    if path.exists() && !force {
        let existing = Manifest::load(&path)?;
        return Ok(Rendered::new(
            format!(
                "manifest already present at {} (use --force to overwrite)",
                path.display()
            ),
            json!({
                "path": path.display().to_string(),
                "created": false,
                "repository": existing.repository,
            }),
        )
        .with_detail(format!("repository: {}", existing.repository)));
    }

    let repository = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "workspace".to_string());
    let manifest = Manifest::new(repository);
    manifest.save(&path)?;

    Ok(Rendered::new(
        format!("wrote {}", path.display()),
        json!({
            "path": path.display().to_string(),
            "created": true,
            "forced": force,
            "repository": manifest.repository,
            "schema": manifest.schema,
        }),
    ))
}
