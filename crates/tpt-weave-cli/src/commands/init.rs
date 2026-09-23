//! `tpt-weave init` — write `.tpt-weave/manifest.toml` (idempotent) and
//! ensure the standard `.gitignore` entries (spec.md section 28).

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use tpt_weave_core::{Manifest, manifest_path};

/// Standard `.gitignore` entries for a tpt-weave working tree (spec §28).
pub const GITIGNORE_ENTRIES: &[&str] = &["# tpt-weave working state", "/.tpt-weave/"];

pub fn run(cli: &Cli, force: bool) -> Result<Rendered, CliError> {
    let root = Workspace::canonical_root(&cli.path)?;
    let path = manifest_path(&root);

    let (created, existing, repository, schema) = if path.exists() && !force {
        let existing = Manifest::load(&path)?;
        (
            false,
            Some(existing.clone()),
            existing.repository,
            existing.schema,
        )
    } else {
        let repository = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "workspace".to_string());
        let manifest = Manifest::new(&repository);
        manifest.save(&path)?;
        (true, None, repository, manifest.schema)
    };

    let gitignore_updated = ensure_gitignore(&root)?;

    let (human, json) = if created {
        (
            format!("wrote {}", path.display()),
            json!({
                "path": path.display().to_string(),
                "created": true,
                "forced": force,
                "repository": repository,
                "schema": schema,
                "gitignore_updated": gitignore_updated,
            }),
        )
    } else {
        let _ = existing;
        (
            format!(
                "manifest already present at {} (use --force to overwrite)",
                path.display()
            ),
            json!({
                "path": path.display().to_string(),
                "created": false,
                "repository": repository,
                "gitignore_updated": gitignore_updated,
            }),
        )
    };

    Ok(Rendered::new(human, json).with_detail(format!(
        "repository: {repository}{}",
        if gitignore_updated {
            "\n.gitignore: updated"
        } else {
            ""
        }
    )))
}

/// Appends the standard entries to `<root>/.gitignore` when missing.
/// Returns `true` when the file was modified.
pub fn ensure_gitignore(root: &std::path::Path) -> Result<bool, CliError> {
    let path = root.join(".gitignore");
    let existing = if path.exists() {
        std::fs::read_to_string(&path)
            .map_err(|e| CliError::internal(format!("failed to read {}: {e}", path.display())))?
    } else {
        String::new()
    };

    let has_entry = |entry: &str| -> bool {
        let bare = entry.trim_start_matches('/');
        existing.lines().any(|line| {
            let line = line.trim();
            line == entry
                || line == bare
                || line == format!("{bare}/")
                || line == format!("/{bare}/")
        })
    };

    let mut missing: Vec<&str> = Vec::new();
    let needs_newline = !existing.is_empty() && !existing.ends_with('\n');
    for entry in GITIGNORE_ENTRIES {
        if entry.is_empty() {
            if needs_newline {
                // Newline is added below when appending non-empty entries.
            }
            continue;
        }
        if !has_entry(entry) {
            missing.push(entry);
        }
    }

    if missing.is_empty() {
        return Ok(false);
    }

    let mut text = existing;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    for entry in missing {
        text.push_str(entry);
        text.push('\n');
    }
    std::fs::write(&path, text)
        .map_err(|e| CliError::internal(format!("failed to write {}: {e}", path.display())))?;
    Ok(true)
}
