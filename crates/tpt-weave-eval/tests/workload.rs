//! Phase 18 workload capture/report tests.

use tpt_weave_eval::{WorkloadCapture, WorkloadEvent, WorkloadEventKind, WorkloadReport};

#[test]
fn aggregates_repetition_exploration_quality_and_cost() {
    let events = vec![
        WorkloadEvent::new(WorkloadEventKind::Context, "overview", 100, 20),
        WorkloadEvent::new(WorkloadEventKind::Context, "overview", 100, 20),
        WorkloadEvent::new(WorkloadEventKind::ToolOutput, "cargo-test", 200, 40),
        WorkloadEvent::new(WorkloadEventKind::ToolOutput, "cargo-test", 200, 40).with_cache_hit(),
        WorkloadEvent::new(WorkloadEventKind::RepositoryExploration, "ls", 50, 50),
        WorkloadEvent::new(WorkloadEventKind::DependencyTraversal, "tpt-cv", 30, 30),
        WorkloadEvent::new(WorkloadEventKind::ModelCall, "model-1", 0, 10)
            .with_output_tokens(5)
            .with_latency_ms(125)
            .with_success(true)
            .with_costs(0.02, 0.01),
    ];
    let report = WorkloadReport::from_events("session", &events, Some(3600.0));
    assert_eq!(report.raw_tokens, 680);
    assert_eq!(report.delivered_tokens, 210);
    assert_eq!(report.jev_tokens, 0);
    assert_eq!(report.output_tokens, 5);
    assert_eq!(report.total_latency_ms, 125);
    assert_eq!(report.repeated_context_tokens, 20);
    assert_eq!(report.repeated_tool_output_tokens, 40);
    assert_eq!(report.repository_exploration_tokens, 50);
    assert_eq!(report.dependency_traversal_tokens, 30);
    assert_eq!(report.cache_hits, 1);
    assert_eq!(report.model_attempts, 1);
    assert_eq!(report.model_successes, 1);
    assert_eq!(report.task_success_rate, Some(1.0));
    assert_eq!(report.cost_savings_usd, Some(0.01));
    assert_eq!(report.raw_tokens_per_hour, Some(680.0));
    assert_eq!(report.total_tokens, 215);

    let path = std::env::temp_dir().join(format!(
        "tpt-weave-workload-report-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    report.save(&path).expect("save report");
    assert_eq!(WorkloadReport::load(&path).expect("load report"), report);
    std::fs::remove_file(path).ok();
}

#[test]
fn jsonl_capture_round_trips_and_rejects_empty_or_invalid_input() {
    let capture = WorkloadCapture::new(
        "session",
        vec![WorkloadEvent::new(
            WorkloadEventKind::Context,
            "overview",
            10,
            2,
        )],
    );
    let jsonl = capture.to_jsonl().expect("serialise");
    let back = WorkloadCapture::from_jsonl(&jsonl).expect("parse");
    assert_eq!(back.events, capture.events);
    let named =
        WorkloadCapture::from_jsonl_with_session_id("named-session", &jsonl).expect("named parse");
    assert_eq!(named.session_id, "named-session");
    assert_eq!(named.events, capture.events);
    assert!(matches!(
        WorkloadCapture::from_jsonl("\n"),
        Err(tpt_weave_eval::WorkloadError::Empty)
    ));
    assert!(matches!(
        WorkloadCapture::from_jsonl("not-json"),
        Err(tpt_weave_eval::WorkloadError::JsonLine { line: 1, .. })
    ));

    let path = std::env::temp_dir().join(format!(
        "tpt-weave-workload-capture-{}-{}.jsonl",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    capture.save_jsonl(&path).expect("save capture");
    assert_eq!(
        WorkloadCapture::load_jsonl(&path)
            .expect("load capture")
            .events,
        capture.events
    );
    std::fs::remove_file(path).ok();
}
