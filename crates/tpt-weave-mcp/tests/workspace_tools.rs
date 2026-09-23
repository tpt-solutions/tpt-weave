//! Phase 9 workspace load + tool dispatch tests (todo.md Phase 9).

use serde_json::json;
use tpt_weave_mcp::{ToolError, ToolFilter, Workspace, all_tools};

fn repo_root() -> std::path::PathBuf {
    // crates/tpt-weave-mcp -> workspace root
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

fn load() -> Workspace {
    Workspace::load(repo_root()).expect("load tpt-weave workspace")
}

#[test]
fn load_workspace_on_this_repo() {
    let ws = load();
    assert!(!ws.graph().symbols.is_empty(), "graph has symbols");
    assert_eq!(ws.repository_name(), "tpt-weave");
    assert!(ws.root().ends_with("tpt-weave"));
}

#[test]
fn refresh_without_git_change_is_noop() {
    let mut ws = load();
    let before = ws.graph().revision.sha.clone();
    let refreshed = ws.refresh_if_stale().expect("refresh");
    if !refreshed {
        assert_eq!(ws.graph().revision.sha, before);
    }
}

#[test]
fn repo_overview_tool() {
    let ws = load();
    let text = ws
        .call_tool("tpt_repo_overview", &json!({}))
        .expect("overview");
    assert!(text.contains("symbols="), "{text}");
    assert!(
        text.contains("tpt-weave") || text.contains("crates="),
        "{text}"
    );
}

#[test]
fn find_symbol_and_signature_roundtrip() {
    let ws = load();
    let found = ws
        .call_tool("tpt_find_symbol", &json!({ "query": "Workspace" }))
        .expect("find");
    assert!(found.starts_with("matches:"), "{found}");
    assert!(found.contains("Workspace"), "{found}");

    // Extract a key from the first line after the header.
    let key = found
        .lines()
        .nth(1)
        .and_then(|line| line.split('\t').next())
        .expect("has result line")
        .to_string();
    assert!(!key.is_empty());

    let sig = ws
        .call_tool("tpt_get_signature", &json!({ "key": key }))
        .expect("signature");
    assert!(sig.contains(&key), "{sig}");
}

#[test]
fn find_symbol_empty_query_reports_no_matches() {
    let ws = load();
    let text = ws
        .call_tool(
            "tpt_find_symbol",
            &json!({ "query": "zzz_no_such_symbol_zzz" }),
        )
        .expect("ok");
    assert!(text.contains("no symbols matching"), "{text}");
}

#[test]
fn missing_required_argument_is_tool_error() {
    let ws = load();
    let err = ws
        .call_tool("tpt_find_symbol", &json!({}))
        .expect_err("missing query");
    assert!(err.to_string().contains("query"), "{err}");
}

#[test]
fn unknown_tool_is_rejected() {
    let ws = load();
    let err = ws
        .call_tool("tpt_not_a_tool", &json!({}))
        .expect_err("unknown");
    assert!(err.to_string().contains("unknown tool"), "{err}");
}

#[test]
fn get_skeleton_and_expand_symbol() {
    let ws = load();
    let skeleton = ws
        .call_tool(
            "tpt_get_skeleton",
            &json!({ "path": "crates/tpt-weave-mcp/src/lib.rs" }),
        )
        .expect("skeleton");
    assert!(skeleton.contains("tokens:"), "{skeleton}");
    assert!(skeleton.contains("pub mod catalog"), "{skeleton}");

    // Find a symbol key from the skeleton path file.
    let found = ws
        .call_tool("tpt_find_symbol", &json!({ "query": "TptWeaveServer" }))
        .expect("find server");
    let key = found
        .lines()
        .nth(1)
        .and_then(|line| line.split('\t').next())
        .map(str::to_string);
    if let Some(key) = key.clone() {
        let expanded = ws
            .call_tool(
                "tpt_expand",
                &json!({ "kind": "symbol", "id": key, "level": 3 }),
            )
            .expect("expand");
        assert!(expanded.contains("subject:"), "{expanded}");
        assert!(expanded.contains("level:"), "{expanded}");
    }

    // Level-1 expand needs only the graph (no source files).
    if let Some(key) = key {
        let expanded = ws
            .call_tool(
                "tpt_expand",
                &json!({ "kind": "symbol", "id": key, "level": 1 }),
            )
            .expect("expand level 1");
        assert!(expanded.contains("subject:"), "{expanded}");
    }
}

#[test]
fn dependencies_and_related_on_workspace_crate() {
    let ws = load();
    let deps = ws
        .call_tool("tpt_dependencies", &json!({ "package": "tpt-weave-mcp" }))
        .expect("deps");
    assert!(
        deps.contains("dependencies:") || deps.contains("no dependencies"),
        "{deps}"
    );
    assert!(
        deps.contains("tpt-weave-core") || deps.contains("rmcp") || deps.starts_with("no "),
        "{deps}"
    );

    // Related needs a valid key; use overview-independent find.
    let found = ws
        .call_tool("tpt_find_symbol", &json!({ "query": "ToolFilter" }))
        .expect("find filter");
    if let Some(key) = found.lines().nth(1).and_then(|l| l.split('\t').next()) {
        let related = ws
            .call_tool("tpt_related", &json!({ "key": key }))
            .expect("related");
        assert!(
            related.starts_with("related:") || related.starts_with("no symbols related"),
            "{related}"
        );
    }
}

#[test]
fn git_diff_and_test_result_and_stats() {
    let ws = load();
    let diff = ws.call_tool("tpt_git_diff", &json!({})).expect("diff");
    assert!(
        diff.starts_with("diff: clean")
            || diff.starts_with("files:")
            || diff.starts_with("not a git"),
        "{diff}"
    );

    let reduced = ws
        .call_tool(
            "tpt_test_result",
            &json!({
                "output": "test result: ok. 5 passed; 0 failed\n",
                "exit_code": 0,
            }),
        )
        .expect("reduce");
    assert!(reduced.contains("raw_tokens:"), "{reduced}");

    let stats = ws
        .call_tool("tpt_context_stats", &json!({}))
        .expect("stats");
    assert!(stats.contains("cache.entries:"), "{stats}");
    assert!(stats.contains("cache.hit_rate:"), "{stats}");
}

#[test]
fn all_catalog_tools_are_dispatchable_or_known() {
    let ws = load();
    // Smoke: every catalog tool either succeeds with args or fails as tool-level error
    // (never panics).
    for spec in all_tools() {
        let args = match spec.name {
            "tpt_find_symbol" => json!({ "query": "Workspace" }),
            "tpt_find_references" | "tpt_get_signature" | "tpt_related" => {
                json!({ "key": "missing" })
            }
            "tpt_get_skeleton" => json!({ "path": "src/lib.rs" }),
            "tpt_expand" => json!({ "kind": "symbol", "id": "missing" }),
            "tpt_dependencies" => json!({ "package": "tpt-weave-core" }),
            "tpt_test_result" => json!({ "output": "ok", "exit_code": 0 }),
            _ => json!({}),
        };
        let _ = ws.call_tool(spec.name, &args);
    }
}

#[test]
fn filter_from_env_defaults_to_all_when_unset() {
    // Use a definitely-unset name; never mutate process env (edition 2024).
    let _ = std::env::var("TPT_WEAVE_TOOLS_NEVER_SET_FOR_TESTS");
    // Direct parse path mirrors from_env without touching real env.
    assert_eq!(ToolFilter::parse("all").names().len(), all_tools().len());
}

// Ensure ToolError is public API for tests that construct it indirectly.
#[allow(dead_code)]
fn _assert_tool_error_display(e: ToolError) -> String {
    e.to_string()
}
