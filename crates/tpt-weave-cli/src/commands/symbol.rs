//! `tpt-weave symbol <name>` — symbol lookup.

use super::Rendered;
use crate::args::Cli;
use crate::error::CliError;
use crate::workspace::Workspace;
use serde_json::json;
use std::time::Instant;
use tpt_weave_context::Retriever;
use tpt_weave_rust::SymbolRecord;

pub fn run(cli: &Cli, name: &str) -> Result<Rendered, CliError> {
    let started = Instant::now();
    let workspace = Workspace::load_index(&cli.path)?;
    let retriever = Retriever::new(workspace.graph(), workspace.sources());
    let found = retriever.find_symbols(name);
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    if found.is_empty() {
        return Err(CliError::not_found(format!("no symbols matching `{name}`")));
    }

    let mut human = format!("matches: {} ({:.1} ms)\n", found.len(), elapsed_ms);
    for record in &found {
        human.push_str(&symbol_line(record));
        human.push('\n');
    }

    let json = json!({
        "query": name,
        "matches": found.len(),
        "symbols": found.iter().map(|record| record_json(record)).collect::<Vec<_>>(),
        "elapsed_ms": elapsed_ms,
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
