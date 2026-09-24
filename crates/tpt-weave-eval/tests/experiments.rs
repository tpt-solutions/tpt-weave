//! Phase 17 experiment-matrix tests.

use tpt_weave_context::Sources;
use tpt_weave_core::{ContextLevel, RepositoryId, Revision, SCHEMA_VERSION};
use tpt_weave_decisions::{DecisionCategory, DeterministicFallbackProvider, MockProvider};
use tpt_weave_eval::{
    AccuracyCorpus, ExperimentReportError, ExperimentSuite, LabeledDecision, VariantMeasurement,
    cache_measurements, comparison_suite, cross_repository_measurements, evaluate_accuracy,
    hierarchy_measurements, jev_relevance_measurements, tool_output_measurements,
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
fn four_way_measurement_matrices_are_explicit() {
    let case = |name: &str, delivered: u64| {
        VariantMeasurement::new(name, 100, delivered, 1).with_accuracy(0.75)
    };
    let cache = cache_measurements(
        case("no_cache", 100),
        case("file_cache", 80),
        case("context_cache", 60),
        case("tool_result_cache", 50),
    );
    assert_eq!(cache.cases.len(), 4);
    assert_eq!(cache.cases[3].variant, "tool_result_cache");

    let traversal = cross_repository_measurements(
        case("no_traversal", 10),
        case("full_traversal", 100),
        case("deterministic_traversal", 40),
        case("jev_selected_traversal", 20),
    );
    assert_eq!(traversal.experiment, "cross_repository_retrieval");
    assert!(
        traversal
            .cases
            .iter()
            .all(|case| case.accuracy == Some(0.75))
    );
}

#[test]
fn labeled_corpus_evaluates_provider_accuracy_and_errors() {
    let provider = DeterministicFallbackProvider::new();
    let cases = vec![
        LabeledDecision::new(DecisionCategory::Relevance, "a", "same", "relevant"),
        LabeledDecision::new(DecisionCategory::Relevance, "b", "same", "irrelevant"),
    ];
    let report = evaluate_accuracy(&provider, &cases).expect("valid corpus");
    assert_eq!(report.total, 2);
    assert_eq!(report.correct + report.incorrect + report.errors, 2);
    assert_eq!(report.provider, "deterministic");
    assert!((0.0..=1.0).contains(&report.accuracy));

    let mock = MockProvider::answering("relevant", 0.9);
    let all_relevant = vec![LabeledDecision::new(
        DecisionCategory::Relevance,
        "a",
        "context",
        "relevant",
    )];
    let exact = evaluate_accuracy(&mock, &all_relevant).expect("mock corpus");
    assert_eq!(exact.accuracy, 1.0);
    assert!(evaluate_accuracy(&mock, &[]).is_err());
}

#[test]
fn corpus_file_loads_and_validates() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/accuracy-corpus.example.json"
    );
    let corpus = AccuracyCorpus::load(path).expect("example corpus");
    assert_eq!(corpus.schema, 1);
    assert_eq!(corpus.cases.len(), 4);
    let invalid = AccuracyCorpus::new(Vec::new());
    assert!(matches!(
        invalid.validate(),
        Err(ExperimentReportError::Invalid(_))
    ));
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
    let back = ExperimentSuite::from_json(&json).unwrap();
    assert_eq!(suite, back);
    assert!(suite.summary().contains("2 variants"));
}

#[test]
fn experiment_reports_reject_bad_schema_duplicates_and_accuracy() {
    let mut suite = comparison_suite(
        "jev_relevance",
        [VariantMeasurement::new("deterministic_only", 100, 40, 1)],
    );
    let path = std::env::temp_dir().join(format!(
        "tpt-weave-experiment-report-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    suite.save(&path).expect("save");
    assert_eq!(ExperimentSuite::load(&path).expect("load"), suite);
    std::fs::remove_file(path).ok();

    suite.schema = 999;
    assert!(matches!(
        suite.validate(),
        Err(tpt_weave_eval::ExperimentReportError::Schema { found: 999, .. })
    ));
    suite.schema = 1;
    suite.cases.push(suite.cases[0].clone());
    assert!(matches!(
        suite.validate(),
        Err(tpt_weave_eval::ExperimentReportError::Invalid(detail)) if detail.contains("duplicate")
    ));
    suite.cases.pop();
    suite.cases[0].accuracy = Some(1.5);
    assert!(matches!(
        suite.validate(),
        Err(tpt_weave_eval::ExperimentReportError::Invalid(detail)) if detail.contains("accuracy")
    ));
}
