//! MCP tool dispatch over a loaded [`Workspace`](crate::workspace::Workspace)
//! (todo.md Phase 9; spec.md section 19).

use crate::workspace::Workspace;
use serde_json::Value;
use tpt_weave_cache::{FilesystemCache, cache_root};
use tpt_weave_context::{
    Retriever, Selection, expand_dependency, expand_module, expand_related, expand_symbol,
    expand_test, represent_file,
};
use tpt_weave_core::ContextLevel;
use tpt_weave_tools::{ToolKind, ToolOutput, reduce};

/// A tool call failed (surfaced as an MCP tool-level error).
#[derive(Debug)]
pub struct ToolError {
    pub message: String,
}

impl ToolError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Wraps a workspace load/refresh failure for tool-level error reporting.
    pub fn from_workspace(error: crate::workspace::WorkspaceError) -> Self {
        Self::new(error.to_string())
    }
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ToolError {}

fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::new(format!("missing string argument `{key}`")))
}

fn opt_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

fn level_from(args: &Value, default: ContextLevel) -> Result<ContextLevel, ToolError> {
    match args.get("level") {
        None => Ok(default),
        Some(Value::Number(n)) => {
            let number = n.as_u64().unwrap_or(0) as u8;
            ContextLevel::from_number(number)
                .ok_or_else(|| ToolError::new(format!("level out of range: {number}")))
        }
        Some(Value::String(name)) => ContextLevel::ALL
            .into_iter()
            .find(|l| l.name() == name)
            .ok_or_else(|| ToolError::new(format!("unknown level: {name}"))),
        Some(_) => Err(ToolError::new("level must be a number or name")),
    }
}

fn symbol_line(record: &tpt_weave_rust::SymbolRecord) -> String {
    format!(
        "{}\t{}:{}\t{}\t{}",
        record.id.canonical_key(),
        record.file,
        record.line,
        record.visibility,
        record.signature
    )
}

impl Workspace {
    /// Dispatches one MCP tool by name (spec.md section 19).
    pub fn call_tool(&self, name: &str, args: &Value) -> Result<String, ToolError> {
        match name {
            "tpt_repo_overview" => self.repo_overview(),
            "tpt_find_symbol" => self.find_symbol(args),
            "tpt_find_references" => self.find_references(args),
            "tpt_get_signature" => self.get_signature(args),
            "tpt_get_skeleton" => self.get_skeleton(args),
            "tpt_expand" => self.expand(args),
            "tpt_dependencies" => self.dependencies(args),
            "tpt_related" => self.related(args),
            "tpt_git_diff" => self.git_diff(),
            "tpt_test_result" => self.test_result(args),
            "tpt_context_stats" => self.context_stats(),
            other => Err(ToolError::new(format!("unknown tool: {other}"))),
        }
    }

    fn retriever(&self) -> Retriever<'_> {
        Retriever::new(self.graph(), self.sources())
    }

    fn repo_overview(&self) -> Result<String, ToolError> {
        Ok(self.retriever().repository_overview().text())
    }

    fn find_symbol(&self, args: &Value) -> Result<String, ToolError> {
        let query = arg_str(args, "query")?;
        let retriever = self.retriever();
        let found = retriever.find_symbols(query);
        if found.is_empty() {
            return Ok(format!("no symbols matching `{query}`"));
        }
        let mut lines = vec![format!("matches: {}", found.len())];
        lines.extend(found.iter().map(|r| symbol_line(r)));
        Ok(lines.join("\n"))
    }

    fn find_references(&self, args: &Value) -> Result<String, ToolError> {
        let key = arg_str(args, "key")?;
        let retriever = self.retriever();
        let refs = retriever.find_references(key);
        if refs.is_empty() {
            return Ok(format!("no references from `{key}`"));
        }
        let mut lines = vec![format!("references: {}", refs.len())];
        for (record, kind) in refs {
            lines.push(format!("{kind:?}\t{}", symbol_line(record)));
        }
        Ok(lines.join("\n"))
    }

    fn get_signature(&self, args: &Value) -> Result<String, ToolError> {
        let key = arg_str(args, "key")?;
        let record = self
            .graph()
            .symbol(key)
            .ok_or_else(|| ToolError::new(format!("unknown symbol: {key}")))?;
        Ok(format!(
            "{}\t{}:{}\n{}",
            record.id.canonical_key(),
            record.file,
            record.line,
            record.signature
        ))
    }

    fn get_skeleton(&self, args: &Value) -> Result<String, ToolError> {
        let path = arg_str(args, "path")?;
        let representation = represent_file(
            self.graph(),
            self.sources(),
            path,
            ContextLevel::Skeleton,
            &Selection::new(),
        )
        .map_err(|e| ToolError::new(e.to_string()))?;
        Ok(format!(
            "tokens: {}\n{}",
            representation.token_estimate, representation.text
        ))
    }

    fn expand(&self, args: &Value) -> Result<String, ToolError> {
        let kind = arg_str(args, "kind")?;
        let id = arg_str(args, "id")?;
        let package = opt_str(args, "package");
        let level = level_from(args, ContextLevel::Skeleton)?;
        let sources = self.sources();
        let graph = self.graph();

        let expansion = match kind {
            "symbol" => expand_symbol(graph, sources, id, level),
            "module" => {
                let package =
                    package.ok_or_else(|| ToolError::new("module expand needs `package`"))?;
                expand_module(graph, sources, package, id, level)
            }
            "dependency" => expand_dependency(graph, sources, id, level),
            "test" => expand_test(graph, sources, id, level),
            "related" => expand_related(graph, sources, id, level),
            other => {
                return Err(ToolError::new(format!(
                    "unknown expand kind `{other}` (symbol|module|dependency|test|related)"
                )));
            }
        }
        .map_err(|e| ToolError::new(e.to_string()))?;

        let mut lines = vec![
            format!("subject: {}", expansion.subject),
            format!("level: {}", expansion.level.name()),
            format!("symbols: {}", expansion.symbols.len()),
            format!("tokens: {}", expansion.total_tokens()),
        ];
        for file in &expansion.files {
            lines.push(format!(
                "--- {} (tokens: {}) ---",
                file.path, file.token_estimate
            ));
            lines.push(file.text.clone());
        }
        Ok(lines.join("\n"))
    }

    fn dependencies(&self, args: &Value) -> Result<String, ToolError> {
        let package = arg_str(args, "package")?;
        let retriever = self.retriever();
        let edges = retriever.find_dependencies(package);
        if edges.is_empty() {
            return Ok(format!("no dependencies for `{package}`"));
        }
        let mut lines = vec![format!("dependencies: {}", edges.len())];
        for edge in edges {
            let optional = if edge.optional { " optional" } else { "" };
            let internal = if edge.internal { " internal" } else { "" };
            lines.push(format!(
                "{}\t{:?}{}{}",
                edge.to_package, edge.kind, optional, internal
            ));
        }
        Ok(lines.join("\n"))
    }

    fn related(&self, args: &Value) -> Result<String, ToolError> {
        let key = arg_str(args, "key")?;
        let retriever = self.retriever();
        let found = retriever
            .find_related(key)
            .map_err(|e| ToolError::new(e.to_string()))?;
        if found.is_empty() {
            return Ok(format!("no symbols related to `{key}`"));
        }
        let mut lines = vec![format!("related: {}", found.len())];
        lines.extend(found.iter().map(|r| symbol_line(r)));
        Ok(lines.join("\n"))
    }

    fn git_diff(&self) -> Result<String, ToolError> {
        let Some(git) = self.git() else {
            return Ok("not a git repository".to_string());
        };
        let entries = git.diff().map_err(|e| ToolError::new(e.to_string()))?;
        if entries.is_empty() {
            return Ok("diff: clean".to_string());
        }
        let mut lines = vec![format!("files: {}", entries.len())];
        for entry in entries {
            lines.push(format!("{:?}\t{}", entry.status, entry.path));
        }
        Ok(lines.join("\n"))
    }

    fn test_result(&self, args: &Value) -> Result<String, ToolError> {
        let output = arg_str(args, "output")?;
        let exit_code = args.get("exit_code").and_then(Value::as_i64).unwrap_or(0) as i32;
        let tool_output = ToolOutput::new("cargo test", output).with_exit_code(exit_code);
        let reduction = reduce(ToolKind::CargoTest, &tool_output);
        let mut text = reduction.render();
        text.push_str(&format!(
            "\nraw_tokens: {}\nreduced_tokens: {}\nreduction: {:.1}%",
            reduction.raw_tokens,
            reduction.reduced_tokens,
            reduction.reduction() * 100.0
        ));
        Ok(text)
    }

    fn context_stats(&self) -> Result<String, ToolError> {
        let overview = self.retriever().repository_overview();
        let cache = FilesystemCache::new(cache_root(self.root()));
        let stats = cache
            .statistics()
            .map_err(|e| ToolError::new(e.to_string()))?;
        Ok(format!(
            "{}\ncache.entries: {}\ncache.bytes: {}\ncache.hit_rate: {:.3}\n",
            overview.text(),
            stats.entries,
            stats.bytes,
            stats.hit_rate(),
        ))
    }
}
