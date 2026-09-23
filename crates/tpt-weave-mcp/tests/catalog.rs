//! Phase 9 MCP catalog/filter/schema-overhead tests (todo.md Phase 9).

use tpt_weave_mcp::{ToolFilter, ToolGroup, all_tools, schema_overhead};

#[test]
fn catalog_has_eleven_spec_tools() {
    let all = all_tools();
    assert_eq!(all.len(), 11);
    let names: Vec<&str> = all.iter().map(|t| t.name).collect();
    for expected in [
        "tpt_repo_overview",
        "tpt_find_symbol",
        "tpt_find_references",
        "tpt_get_signature",
        "tpt_get_skeleton",
        "tpt_expand",
        "tpt_dependencies",
        "tpt_related",
        "tpt_git_diff",
        "tpt_test_result",
        "tpt_context_stats",
    ] {
        assert!(names.contains(&expected), "missing tool {expected}");
    }
}

#[test]
fn all_tools_have_object_schemas_with_additional_properties_false() {
    for spec in all_tools() {
        let schema = &spec.schema;
        assert_eq!(schema["type"], "object", "{}", spec.name);
        assert_eq!(schema["additionalProperties"], false, "{}", spec.name);
        assert!(schema["properties"].is_object(), "{}", spec.name);
        assert!(schema["required"].is_array(), "{}", spec.name);
        assert!(!spec.description.is_empty(), "{}", spec.name);
        assert!(
            spec.description.len() <= 80,
            "description too long: {}",
            spec.name
        );
    }
}

#[test]
fn filter_all_exposes_every_tool() {
    let filter = ToolFilter::all();
    assert_eq!(filter.names().len(), 11);
    for spec in all_tools() {
        assert!(filter.allows(spec.name));
    }
}

#[test]
fn filter_groups_subset_catalog_order() {
    let filter = ToolFilter::groups(&[ToolGroup::Orient]);
    let names = filter.names();
    assert_eq!(names, &["tpt_repo_overview", "tpt_context_stats"]);
    assert!(!filter.allows("tpt_find_symbol"));

    let symbol = ToolFilter::groups(&[ToolGroup::Symbol, ToolGroup::Context]);
    assert!(symbol.allows("tpt_find_symbol"));
    assert!(symbol.allows("tpt_expand"));
    assert!(!symbol.allows("tpt_git_diff"));
    // Catalog order preserved across groups.
    let symbol_names: Vec<&str> = symbol.names().to_vec();
    let mut sorted = symbol_names.clone();
    sorted.sort_by_key(|n| {
        all_tools()
            .iter()
            .position(|t| t.name == *n)
            .unwrap_or(usize::MAX)
    });
    assert_eq!(symbol_names, sorted);
}

#[test]
fn filter_parse_group_aliases() {
    assert_eq!(
        ToolFilter::parse("orient").names(),
        ToolFilter::groups(&[ToolGroup::Orient]).names()
    );
    assert_eq!(
        ToolFilter::parse("symbol,context").names(),
        ToolFilter::groups(&[ToolGroup::Symbol, ToolGroup::Context]).names()
    );
    assert_eq!(ToolFilter::parse("all").names().len(), 11);
    assert_eq!(ToolFilter::parse("").names().len(), 11);
    assert_eq!(ToolFilter::parse("bogus").names().len(), 11);
}

#[test]
fn schema_overhead_compact_and_monotone() {
    let full = ToolFilter::all();
    let full_bytes = schema_overhead(&full);
    assert!(full_bytes > 0);
    assert!(full_bytes < 4096, "full catalog too large: {full_bytes}");

    let orient = ToolFilter::groups(&[ToolGroup::Orient]);
    let orient_bytes = schema_overhead(&orient);
    assert!(orient_bytes > 0);
    assert!(orient_bytes < full_bytes);

    // Empty filter: no tools, no payload.
    let none = ToolFilter::parse("");
    assert_eq!(schema_overhead(&none), full_bytes); // empty parse falls back to all
}

#[test]
fn tool_groups_partition_all_tools() {
    let total: usize = [
        ToolGroup::Orient,
        ToolGroup::Symbol,
        ToolGroup::Context,
        ToolGroup::Workspace,
    ]
    .iter()
    .map(|g| ToolFilter::groups(std::slice::from_ref(g)).names().len())
    .sum();
    assert_eq!(total, all_tools().len());
    for spec in all_tools() {
        assert!(
            matches!(
                spec.group,
                ToolGroup::Orient | ToolGroup::Symbol | ToolGroup::Context | ToolGroup::Workspace
            ),
            "{}",
            spec.name
        );
    }
}
