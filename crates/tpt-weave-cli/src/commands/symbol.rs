//! `tpt-weave symbol <name>` — symbol lookup.

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use tpt_weave_context::Retriever;
use tpt_weave_rust::SymbolRecord;

pub fn run(cli: &Cli, name: &str) -> Result<Rendered, CliError> {
    let workspace = Workspace::load_index(&cli.path)?;
    let retriever = Retriever::new(workspace.graph(), workspace.sources());
    let found = retriever.find_symbols(name);

    if found.is_empty() {
        return Err(CliError::not_found(format!("no symbols matching `{name}`")));
    }

    let mut human = format!("matches: {}\n", found.len());
    for record in &found {
        human.push_str(&symbol_line(record));
        human.push('\n');
    }

    let json = json!({
        "query": name,
        "matches": found.len(),
        "symbols": found.iter().map(|record| record_json(record)).collect::<Vec<_>>(),
    });
    Ok(Rendered::new(human.trim_end(), json))
}

pub(crate) fn symbol_line(record: &SymbolRecord) -> String {
    format!(
        "{}\t{}:{}\t{}\t{}",
        record.id.canonical_key(),
        record.file,
        record.line,
        record.visibility,
        record.signature
    )
}

pub(crate) fn record_json(record: &SymbolRecord) -> serde_json::Value {
    json!({
        "key": record.id.canonical_key(),
        "name": record.id.name,
        "package": record.id.package,
        "module": record.id.module,
        "kind": record.id.kind,
        "file": record.file,
        "line": record.line,
        "column": record.column,
        "visibility": record.visibility,
        "signature": record.signature,
        "public": record.visibility.is_public(),
    })
}
