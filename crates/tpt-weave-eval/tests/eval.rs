//! Phase 11 evaluation harness tests: captures, metrics, reports, local
//! comparison (spec.md section 23).

use tpt_weave_context::Sources;
use tpt_weave_core::{ContextLevel, Revision, SCHEMA_VERSION};
use tpt_weave_eval::{
    AggregateMetrics, BaselineCapture, EvalReport, HarnessError, TaskMetrics, WeaveCapture,
    local_aggregate, local_comparison,
};
use tpt_weave_graph::{GraphBuilder, RepositoryGraph};
use tpt_weave_index::CargoIndex;
use tpt_weave_rust::{FileInput, parse_file};

fn pair(task_id: &str) -> (BaselineCapture, WeaveCapture) {
    let baseline = BaselineCapture::new(task_id, 10_000, true)
        .with_task("fix the bug")
        .with_latency_ms(2_000)
        .with_cost_usd(0.02);
    let weave = WeaveCapture::new(task_id, 2_500, true)
        .with_task("fix the bug")
        .with_jev_tokens(400)
        .with_latency_ms(1_500)
        .with_cost_usd(0.015)
        .with_counts(3, 1, 0)
        .with_cache(2, 800);
    (baseline, weave)
}

#[test]
fn task_metrics_primary_and_secondary() {
    let (baseline, weave) = pair("t1");
    let metrics = TaskMetrics::from_pair(&baseline, &weave).expect("pair");

    // gross = 1 - 2500/10000 = 0.75
    assert!((metrics.gross_token_reduction - 0.75).abs() < 1e-9);
    // net = 1 - (2500+400)/10000 = 0.71
    assert!((metrics.net_token_reduction - 0.71).abs() < 1e-9);
    assert_eq!(metrics.total_tokens, 2_500 + 400);
    assert_eq!(metrics.retrieval_count, 3);
    assert_eq!(metrics.context_misses, 1);
    assert_eq!(metrics.cache_savings_tokens, 800);
    assert_eq!(metrics.task_success_preservation(), 1.0);
    assert_eq!(metrics.latency_change_ms, -500);
    assert!((metrics.latency_change - 0.75).abs() < 1e-9);
    let cost = metrics.cost_change_usd.expect("both costs known");
    assert!((cost - (-0.005)).abs() < 1e-9, "cost delta {cost}");
}

#[test]
fn task_id_mismatch_is_error() {
    let baseline = BaselineCapture::new("a", 100, true);
    let weave = WeaveCapture::new("b", 50, true);
    let err = TaskMetrics::from_pair(&baseline, &weave).expect_err("mismatch");
    assert!(err.to_string().contains("mismatch"), "{err}");
}

#[test]
fn success_preservation_zero_when_weave_fails() {
    let baseline = BaselineCapture::new("t", 100, true);
    let weave = WeaveCapture::new("t", 50, false);
    let metrics = TaskMetrics::from_pair(&baseline, &weave).expect("pair");
    assert_eq!(metrics.task_success_preservation(), 0.0);
}

#[test]
fn success_preservation_ignores_failed_baseline() {
    let baseline = BaselineCapture::new("t", 100, false);
    let weave = WeaveCapture::new("t", 50, false);
    let metrics = TaskMetrics::from_pair(&baseline, &weave).expect("pair");
    assert_eq!(metrics.task_success_preservation(), 1.0);
}

#[test]
fn aggregate_folds_multiple_tasks() {
    let (b1, w1) = pair("t1");
    let (b2, w2) = pair("t2");
    // Make t2's baseline succeed but weave fail.
    let b2 = b2.with_success(true);
    let w2 = w2.with_success(false);

    let report =
        EvalReport::try_from_pairs(Some("suite".into()), [(b1, w1), (b2, w2)]).expect("report");
    assert_eq!(report.aggregate.tasks, 2);
    assert_eq!(report.aggregate.baseline_successes, 2);
    assert_eq!(report.aggregate.weave_successes, 1);
    assert!((report.aggregate.task_success_preservation - 0.5).abs() < 1e-9);
    assert_eq!(report.aggregate.raw_tokens, 20_000);
    assert_eq!(report.aggregate.delivered_tokens, 5_000);
    assert_eq!(report.aggregate.jev_tokens, 800);
    assert_eq!(report.aggregate.cache_savings_tokens, 1_600);
    // Means of 0.71 and 0.71 (same pair shape) — wait, w2 success only
    // changes preservation, not tokens.
    assert!((report.aggregate.net_token_reduction - 0.71).abs() < 1e-9);
    assert!(report.summary().contains("task success preservation"));
}

#[test]
fn empty_aggregate_is_safe() {
    let aggregate = AggregateMetrics::from_tasks(&[]);
    assert_eq!(aggregate.tasks, 0);
    assert_eq!(aggregate.task_success_preservation, 1.0);
    assert_eq!(aggregate.net_token_reduction, 0.0);
}

#[test]
fn report_roundtrips_json() {
    let (b, w) = pair("t1");
    let report = EvalReport::try_from_pairs(None, [(b, w)]).expect("report");
    let text = report.to_json();
    let parsed = EvalReport::from_json(&text).expect("parse");
    assert_eq!(parsed, report);

    let compact = report.to_json_compact();
    assert!(!compact.contains('\n'));
    let parsed = EvalReport::from_json(&compact).expect("parse compact");
    assert_eq!(parsed.aggregate.tasks, 1);
}

#[test]
fn report_save_and_load() {
    let dir = std::env::temp_dir().join(format!(
        "tpt-weave-eval-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let path = dir.join("nested").join("report.json");
    let (b, w) = pair("t1");
    let report = EvalReport::try_from_pairs(Some("repo".into()), [(b, w)]).expect("report");
    report.save(&path).expect("save");
    let loaded = EvalReport::load(&path).expect("load");
    assert_eq!(loaded, report);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn capture_text_fields_derive_tokens() {
    let text = "abcd".repeat(10); // 40 chars → 10 tokens
    let baseline = BaselineCapture::new("t", 0, true).with_raw_context(text.clone());
    assert_eq!(baseline.raw_tokens, 10);
    let weave = WeaveCapture::new("t", 0, true)
        .with_selected_context(&text)
        .with_jev_context(&text)
        .with_model_output(&text);
    assert_eq!(weave.selected_tokens, 10);
    assert_eq!(weave.jev_tokens, 10);
    assert_eq!(weave.output_tokens, 10);
}

/// Synthetic repository with several modules so budgeted retrieval can
/// select a strict subset of sources.
fn multi_module_repo(name: &str) -> (RepositoryGraph, Sources) {
    let cargo_toml =
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
    let dir = std::env::temp_dir().join(format!(
        "tpt-weave-eval-repo-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join("src")).expect("mkdir");
    std::fs::write(dir.join("Cargo.toml"), cargo_toml).expect("manifest");

    let padding = "// padding comment to give each module real token weight\n".repeat(40);
    let files = [
        (
            "src/lib.rs",
            format!("pub mod alpha;\npub mod beta;\npub mod gamma;\n{padding}"),
        ),
        (
            "src/alpha.rs",
            format!(
                "pub fn alpha_one(x: i32) -> i32 {{\n    x + 1\n}}\n\
                 pub fn alpha_two(x: i32) -> i32 {{\n    x + 2\n}}\n{padding}"
            ),
        ),
        (
            "src/beta.rs",
            format!(
                "pub fn beta_compute(a: i32, b: i32) -> i32 {{\n    a * b\n}}\n\
                 pub fn beta_sum(values: &[i32]) -> i32 {{\n    values.iter().sum()\n}}\n{padding}"
            ),
        ),
        (
            "src/gamma.rs",
            format!(
                "pub struct GammaState {{\n    pub flag: bool,\n    pub count: u32,\n}}\n\
                 impl GammaState {{\n    pub fn reset(&mut self) {{\n        self.count = 0;\n    }}\n}}\n{padding}"
            ),
        ),
    ];
    for (path, text) in &files {
        std::fs::write(dir.join(path), text).expect("write file");
    }

    let cargo = CargoIndex::load(&dir).expect("cargo metadata");
    let mut builder = GraphBuilder::new(name, Revision::new("0".repeat(40)), cargo);
    let mut sources = Sources::new();
    for (path, text) in files {
        let input = FileInput {
            repository: &tpt_weave_core::RepositoryId::new(name),
            package: name,
            path,
            module_prefix: &[],
        };
        let parsed = parse_file(&input, &text).expect("parse");
        builder = builder.add_file(name, parsed);
        sources = sources.with(path, text);
    }
    let graph = builder.build();
    let _ = std::fs::remove_dir_all(&dir);
    (graph, sources)
}

#[test]
fn local_comparison_reduces_tokens() {
    let (graph, sources) = multi_module_repo("fixture_multi");
    assert_eq!(graph.schema, SCHEMA_VERSION);
    // Tight budget: only the top-ranked module should fit.
    let comparison = local_comparison(&graph, &sources, "alpha_one", 250, ContextLevel::Signatures)
        .expect("comparison");
    assert!(comparison.raw_tokens > 0, "baseline raw tokens");
    assert!(
        comparison.delivered_tokens < comparison.raw_tokens,
        "delivered {} < raw {}",
        comparison.delivered_tokens,
        comparison.raw_tokens
    );
    assert!(comparison.structural_success, "has candidates");
    assert_eq!(comparison.budget_tokens, 250);

    let (baseline, weave) = comparison.into_pair();
    let metrics = TaskMetrics::from_pair(&baseline, &weave).expect("pair");
    assert!(
        metrics.net_token_reduction > 0.0,
        "positive net reduction: {}",
        metrics.net_token_reduction
    );
    assert!(metrics.gross_token_reduction > 0.0);
    assert!(metrics.baseline_success);
    assert!(metrics.weave_success);
}

#[test]
fn local_comparison_empty_repository_errors() {
    let empty = RepositoryGraph {
        schema: SCHEMA_VERSION,
        repository: tpt_weave_core::RepositoryId::new("empty"),
        revision: Revision::new("0".repeat(40)),
        crates: Vec::new(),
        modules: Vec::new(),
        symbols: Vec::new(),
        references: Vec::new(),
        dependencies: Vec::new(),
        external_links: Vec::new(),
        unresolved_mentions: 0,
    };
    let sources = Sources::new();
    let err = local_comparison(&empty, &sources, "task", 1_000, ContextLevel::Metadata)
        .expect_err("empty");
    assert!(matches!(err, HarnessError::Empty));
}

#[test]
fn local_aggregate_over_comparisons() {
    let (graph, sources) = multi_module_repo("fixture_agg");
    let c1 =
        local_comparison(&graph, &sources, "alpha_one", 400, ContextLevel::Signatures).expect("c1");
    let c2 = local_comparison(
        &graph,
        &sources,
        "beta_compute",
        400,
        ContextLevel::Signatures,
    )
    .expect("c2");
    let aggregate = local_aggregate(&[c1, c2]);
    assert_eq!(aggregate.tasks, 2);
    assert_eq!(aggregate.baseline_successes, 2);
    assert_eq!(aggregate.weave_successes, 2);
    assert_eq!(aggregate.task_success_preservation, 1.0);
    assert!(aggregate.raw_tokens > 0);
    assert!(aggregate.delivered_tokens < aggregate.raw_tokens);
}
