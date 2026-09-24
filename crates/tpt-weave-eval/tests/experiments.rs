//! Phase 17 experiment-matrix tests.

use tpt_weave_context::Sources;
use tpt_weave_core::{ContextLevel, RepositoryId, Revision, SCHEMA_VERSION};
use tpt_weave_eval::{
    ExperimentSuite, VariantMeasurement, comparison_suite, hierarchy_measurements,
    jev_relevance_measurements, tool_output_measurements,
};
use tpt_weave_graph::RepositoryGraph;
use tpt_weave_tools::{ToolKind, ToolOutput};

fn graph() -> RepositoryGraph {
    RepositoryGraph {
        schema: SCHEMA_VERSION,
        repository: RepositoryId::new("experiments"),
        revision: Revision::new("0".repeat(40)),
        crates: Vec::new(),
        modules: Vec::new(),
        symbols: Vec::new(),
        references: Vec::new(),
        dependencies: Vec::new(),
        external_links: Vec::new(),
        unresolved_mentions: 0,
    }
}

#[test]
fn hierarchy_records_all_six_levels() {
    let sources = Sources::new().with("src/lib.rs", "pub fn demo() {}\n");
    let cases = hierarchy_measurements(&graph(), &sources, "src/lib.rs").unwrap();
    assert_eq!(cases.len(), ContextLevel::ALL.len());
    assert_eq!(cases[0].variant, "metadata");
    assert_eq!(cases.last().unwrap().variant, "full");
    assert!(cases.iter().all(|case| case.raw_tokens > 0));
}

#[test]
fn tool_and_jev_suites_include_reductions_and_overhead() {
    let output = ToolOutput::new(
        "cargo test",
        "running 3 tests\ntest a ... ok\ntest b ... ok\ntest c ... ok\n",
    );
    let tool = tool_output_measurements(ToolKind::CargoTest, &output, Some(7));
    assert_eq!(tool.cases.len(), 3);
    assert!(tool.cases[1].delivered_tokens < tool.cases[0].delivered_tokens);
    assert_eq!(tool.cases[2].jev_tokens, 7);

    let jev = jev_relevance_measurements(100, 40, Some(0.8), 35, Some(0.9), 10);
    assert_eq!(jev.cases.len(), 2);
    assert!(jev.net_token_reduction() > 0.0);
}

#[test]
fn externally_measured_variants_round_trip_as_json() {
    let suite: ExperimentSuite = comparison_suite(
        "cache",
        [
            VariantMeasurement::new("no_cache", 100, 100, 4),
            VariantMeasurement::new("context_cache", 100, 30, 2),
        ],
    );
    let json = suite.to_json();
    let back: ExperimentSuite = serde_json::from_str(&json).unwrap();
    assert_eq!(suite, back);
    assert!(suite.summary().contains("2 variants"));
}
