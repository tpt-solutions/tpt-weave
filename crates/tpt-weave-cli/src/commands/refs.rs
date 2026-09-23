//! `tpt-weave refs <name>` — references to/from a symbol.

use super::Rendered;
use super::symbol::record_json;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use tpt_weave_context::Retriever;

pub fn run(cli: &Cli, name: &str) -> Result<Rendered, CliError> {
    let workspace = Workspace::load_index(&cli.path)?;
    if workspace.graph().symbol(name).is_none()
        && workspace.graph().find_symbols(name).is_empty()
        && workspace.graph().find_methods_by_name(name).is_empty()
    {
        return Err(CliError::not_found(format!("unknown symbol: {name}")));
    }

    let retriever = Retriever::new(workspace.graph(), workspace.sources());
    let refs = retriever.find_references(name);

    if refs.is_empty() {
        return Ok(Rendered::new(
            format!("no references from `{name}`"),
            json!({ "key": name, "count": 0, "references": [] }),
        ));
    }

    let mut human = format!("references: {}\n", refs.len());
    for (record, kind) in &refs {
        human.push_str(&format!(
            "{kind:?}\t{}\n",
            super::symbol::symbol_line(record)
        ));
    }

    let json = json!({
        "key": name,
        "count": refs.len(),
        "references": refs.iter().map(|(record, kind)| {
            let mut value = record_json(record);
            if let Some(object) = value.as_object_mut() {
                object.insert("kind".into(), json!(format!("{kind:?}")));
            }
            value
        }).collect::<Vec<_>>(),
    });
    Ok(Rendered::new(human.trim_end(), json))
}
