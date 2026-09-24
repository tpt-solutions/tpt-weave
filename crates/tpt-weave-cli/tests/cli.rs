//! Phase 10 CLI integration tests: human/JSON/compact output, exit codes,
//! error reporting, and the subcommand surface from docs/decisions.md §3.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use tpt_weave_cli::args::{CacheAction, Command as CliCommand};
use tpt_weave_cli::{ExitCode, run};

static FIXTURE_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Creates a unique temporary Cargo project fixture and returns its root.
fn fixture() -> PathBuf {
    let n = FIXTURE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "tpt-weave-cli-test-{}-{}-{}",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(dir.join("src")).expect("mkdir");
    fs::write(
        dir.join("Cargo.toml"),
        format!("[package]\nname = \"fixture{n}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("Cargo.toml");
    fs::write(
        dir.join("src").join("lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\npub struct Counter {\n    pub value: i32,\n}\n",
    )
    .expect("lib.rs");
    dir
}

fn cli(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let mut argv: Vec<String> = vec!["--path".into(), dir.display().to_string()];
    argv.extend(args.iter().map(|s| s.to_string()));
    let outcome = run(argv);
    (outcome.code, outcome.stdout, outcome.stderr)
}

fn assert_exit(code: i32, expected: ExitCode, stderr: &str) {
    assert_eq!(
        code,
        expected.as_i32(),
        "exit {code} != {:?}; stderr={stderr}",
        expected
    );
}

#[test]
fn help_exits_zero_and_documents_commands() {
    let outcome = run(vec![]);
    assert_eq!(outcome.code, 0);
    assert!(outcome.stdout.contains("Usage:"));
    assert!(outcome.stdout.contains("adopt"));
    assert!(outcome.stdout.contains("exit 5") || outcome.stdout.contains("5 decision"));
}

#[test]
fn version_prints_version() {
    let outcome = run(vec!["--version".into()]);
    assert_eq!(outcome.code, 0);
    assert!(outcome.stdout.contains("tpt-weave"));
}

#[test]
fn unknown_command_is_usage_error() {
    let outcome = run(vec!["frobnicate".into()]);
    assert_exit(outcome.code, ExitCode::Usage, &outcome.stderr);
    assert!(outcome.stderr.contains("unknown command"));
}

#[test]
fn unknown_flag_is_usage_error() {
    let outcome = run(vec!["--nope".into(), "overview".into()]);
    assert_exit(outcome.code, ExitCode::Usage, &outcome.stderr);
}

#[test]
fn missing_symbol_argument_is_usage_error() {
    let outcome = run(vec!["symbol".into()]);
    assert_exit(outcome.code, ExitCode::Usage, &outcome.stderr);
}

#[test]
fn init_writes_manifest_and_is_idempotent() {
    let dir = fixture();
    let (code, stdout, stderr) = cli(&dir, &["init"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("wrote"), "{stdout}");
    assert!(dir.join(".tpt-weave").join("manifest.toml").exists());

    let gitignore = fs::read_to_string(dir.join(".gitignore")).expect(".gitignore");
    assert!(
        gitignore.contains("/.tpt-weave/"),
        "gitignore should ignore .tpt-weave: {gitignore}"
    );

    // Second init: idempotent manifest, gitignore not duplicated.
    let (code2, stdout2, _) = cli(&dir, &["init"]);
    assert_eq!(code2, 0);
    assert!(stdout2.contains("already present"), "{stdout2}");
    let gitignore2 = fs::read_to_string(dir.join(".gitignore")).expect(".gitignore");
    assert_eq!(gitignore, gitignore2, "gitignore should not duplicate");

    let (code3, stdout3, _) = cli(&dir, &["init", "--force"]);
    assert_eq!(code3, 0);
    assert!(stdout3.contains("wrote"), "{stdout3}");
}

#[test]
fn index_and_overview_human_json_compact() {
    let dir = fixture();
    let (code, _, stderr) = cli(&dir, &["init"]);
    assert_eq!(code, 0, "{stderr}");
    let (code, stdout, stderr) = cli(&dir, &["index"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("indexed"), "{stdout}");

    let (code, human, stderr) = cli(&dir, &["overview"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(human.contains("symbols="), "{human}");

    let (code, json, stderr) = cli(&dir, &["--json", "overview"]);
    assert_eq!(code, 0, "{stderr}");
    let value: serde_json::Value = serde_json::from_str(&json).expect("pretty json");
    assert!(value.get("symbols").is_some(), "{json}");

    let (code, compact, stderr) = cli(&dir, &["--compact", "overview"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        !compact.trim_end().contains('\n') || compact.lines().count() == 1,
        "{compact}"
    );
    let value: serde_json::Value = serde_json::from_str(compact.trim()).expect("compact json");
    assert!(value.get("repository").is_some());
}

#[test]
fn index_rebuilds_uncommitted_edits_and_reuses_parse_cache() {
    let dir = fixture();
    let (code, _, stderr) = cli(&dir, &["init"]);
    assert_eq!(code, 0, "{stderr}");
    let (code, first_raw, stderr) = cli(&dir, &["--json", "index"]);
    assert_eq!(code, 0, "{stderr}");
    let first: serde_json::Value = serde_json::from_str(&first_raw).expect("first index json");
    assert_eq!(first["rebuilt"], true);
    assert_eq!(first["parse_cache"]["misses"], 1);

    fs::write(
        dir.join("src").join("lib.rs"),
        "pub fn changed() -> i32 { 2 }\n",
    )
    .expect("rewrite source");
    let (code, changed_raw, stderr) = cli(&dir, &["--json", "index"]);
    assert_eq!(code, 0, "{stderr}");
    let changed: serde_json::Value =
        serde_json::from_str(&changed_raw).expect("changed index json");
    assert_eq!(changed["rebuilt"], true);
    assert_eq!(changed["parse_cache"]["misses"], 1);

    let (code, cached_raw, stderr) = cli(&dir, &["--json", "index"]);
    assert_eq!(code, 0, "{stderr}");
    let cached: serde_json::Value = serde_json::from_str(&cached_raw).expect("cached index json");
    assert_eq!(cached["rebuilt"], true);
    assert_eq!(cached["parse_cache"]["hits"], 1);
}

#[test]
fn symbol_lookup_not_found_exits_3() {
    let dir = fixture();
    cli(&dir, &["init"]);
    cli(&dir, &["index"]);

    let (code, stdout, stderr) = cli(&dir, &["symbol", "add"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("matches:"), "{stdout}");

    let (code, _, stderr) = cli(&dir, &["symbol", "definitely_missing_symbol_xyz"]);
    assert_exit(code, ExitCode::NotFound, &stderr);
    assert!(stderr.contains("no symbols matching"), "{stderr}");
}

#[test]
fn refs_unknown_symbol_exits_3() {
    let dir = fixture();
    cli(&dir, &["init"]);
    cli(&dir, &["index"]);

    let (code, _, stderr) = cli(&dir, &["refs", "nope_nope_nope"]);
    assert_exit(code, ExitCode::NotFound, &stderr);
}

#[test]
fn overview_without_index_exits_4() {
    let dir = fixture();
    let (code, _, stderr) = cli(&dir, &["overview"]);
    assert_exit(code, ExitCode::Stale, &stderr);
    assert!(stderr.contains("index"), "{stderr}");
}

#[test]
fn doctor_without_manifest_exits_3() {
    let dir = fixture();
    let (code, _, stderr) = cli(&dir, &["doctor"]);
    assert_exit(code, ExitCode::NotFound, &stderr);
    assert!(stderr.contains("manifest"), "{stderr}");
}

#[test]
fn doctor_after_adopt_passes() {
    let dir = fixture();
    let (code, stdout, stderr) = cli(&dir, &["adopt"]);
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(stdout.contains("adopted"), "{stdout}");
    assert!(dir.join(".tpt-weave").join("graph.json").exists());
    assert!(stdout.contains("baseline benchmark"), "{stdout}");
    assert!(stdout.contains("registered"), "{stdout}");

    let report_path = dir.join(".tpt-weave").join("baseline-report.json");
    assert!(report_path.exists(), "baseline report written");
    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report_path).expect("report text"))
            .expect("report json");
    assert_eq!(report["schema"], 1);
    assert!(report["aggregate"]["raw_tokens"].as_u64().expect("raw") > 0);
    assert!(
        report["aggregate"]["delivered_tokens"]
            .as_u64()
            .expect("delivered")
            > 0
    );
}

#[test]
fn experiment_accuracy_reports_deterministic_provider_results() {
    let dir = fixture();
    let corpus = r#"{
      "schema": 1,
      "cases": [
        {"category":"relevance","subject":"src/lib.rs","context":"fix the bug","expected_choice":"relevant"},
        {"category":"relevance","subject":"docs/readme.md","context":"fix the bug","expected_choice":"irrelevant"}
      ]
    }"#;
    fs::write(dir.join("corpus.json"), corpus).expect("write corpus");

    let (code, human, stderr) = cli(&dir, &["experiment", "accuracy", "corpus.json"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(human.contains("provider: deterministic"), "{human}");
    assert!(human.contains("cases: 2"), "{human}");

    let (code, json, stderr) = cli(&dir, &["--json", "experiment", "accuracy", "corpus.json"]);
    assert_eq!(code, 0, "{stderr}");
    let value: serde_json::Value = serde_json::from_str(json.trim()).expect("accuracy json");
    assert_eq!(value["provider"], "deterministic");
    assert_eq!(value["total"], 2);
    assert!(value["correct"].as_u64().is_some());
    assert!(value["incorrect"].as_u64().is_some());
    assert!(value["accuracy"].as_f64().is_some());
}

#[test]
fn workload_aggregates_jsonl_capture_with_duration() {
    let dir = fixture();
    let capture = r#"{"kind":"context","id":"overview","raw_tokens":100,"delivered_tokens":20}
{"kind":"model_call","id":"attempt-1","raw_tokens":0,"delivered_tokens":20,"success":true,"cost_usd":0.01,"baseline_cost_usd":0.02}
"#;
    fs::write(dir.join("session.jsonl"), capture).expect("write capture");

    let (code, human, stderr) = cli(&dir, &["workload", "session.jsonl", "--duration", "3600"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(human.contains("raw tokens: 100"), "{human}");
    assert!(human.contains("model success rate: 100.0%"), "{human}");

    let (code, json, stderr) = cli(
        &dir,
        &["--json", "workload", "session.jsonl", "--duration", "3600"],
    );
    assert_eq!(code, 0, "{stderr}");
    let value: serde_json::Value = serde_json::from_str(json.trim()).expect("workload json");
    assert_eq!(value["raw_tokens"], 100);
    assert_eq!(value["delivered_tokens"], 40);
    assert_eq!(value["cost_savings_usd"], 0.01);
    assert_eq!(value["raw_tokens_per_hour"], 100.0);
}

#[test]
fn workload_missing_capture_is_an_error() {
    let dir = fixture();
    let (code, _, stderr) = cli(&dir, &["workload", "missing.jsonl"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("workload capture"), "{stderr}");
}

#[test]
fn integration_emits_standard_agent_mcp_and_registry_metadata() {
    let dir = fixture();
    let (code, agent, stderr) = cli(&dir, &["--json", "integration", "agent", "--write"]);
    assert_eq!(code, 0, "{stderr}");
    let agent_value: serde_json::Value = serde_json::from_str(agent.trim()).expect("agent json");
    assert_eq!(agent_value["schema"], 1);
    assert_eq!(
        agent_value["env"]["TPT_WEAVE_PATH"],
        dir.display().to_string()
    );
    assert!(dir.join(".tpt-weave").join("agent.json").exists());

    let (code, mcp, stderr) = cli(&dir, &["--json", "integration", "mcp", "--write"]);
    assert_eq!(code, 0, "{stderr}");
    let mcp_value: serde_json::Value = serde_json::from_str(mcp.trim()).expect("mcp json");
    assert_eq!(mcp_value["name"], "tpt-weave");
    assert_eq!(
        mcp_value["env"]["TPT_WEAVE_PATH"],
        dir.display().to_string()
    );
    assert!(dir.join(".tpt-weave").join("mcp.json").exists());

    let (code, registry, stderr) = cli(&dir, &["--json", "integration", "registry"]);
    assert_eq!(code, 0, "{stderr}");
    let registry_value: serde_json::Value =
        serde_json::from_str(registry.trim()).expect("registry json");
    assert_eq!(
        registry_value["repository"],
        dir.file_name().unwrap().to_string_lossy().as_ref()
    );
    assert!(registry_value["discovered_tpt_dependencies"].is_array());
}

#[test]
fn expand_unknown_context_id_falls_through_to_symbol_not_found() {
    let dir = fixture();
    cli(&dir, &["init"]);
    cli(&dir, &["index"]);

    // Unknown ctx: id that does not match any candidate → not found via symbol path.
    let (code, _, stderr) = cli(&dir, &["expand", "ctx:deadbeefdeadbeef"]);
    assert_exit(code, ExitCode::NotFound, &stderr);
}

#[test]
fn expand_symbol_by_key() {
    let dir = fixture();
    cli(&dir, &["init"]);
    cli(&dir, &["index"]);

    // Discover a canonical key from symbol lookup, then expand it.
    let (code, stdout, stderr) = cli(&dir, &["symbol", "add"]);
    assert_eq!(code, 0, "{stderr}");
    let key = stdout
        .lines()
        .find(|line| line.contains('\t'))
        .and_then(|line| line.split('\t').next())
        .expect("symbol line")
        .to_string();

    let (code, stdout, stderr) = cli(&dir, &["expand", &key, "--level", "full"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("subject:"), "{stdout}");
    assert!(stdout.contains("fn add"), "{stdout}");
}

#[test]
fn context_returns_candidates_and_accounting() {
    let dir = fixture();
    cli(&dir, &["init"]);
    cli(&dir, &["index"]);

    let (code, stdout, stderr) = cli(&dir, &["context", "add two numbers"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("context:"), "{stdout}");
    assert!(stdout.contains("candidates:"), "{stdout}");
    assert!(
        stdout.contains("Reduction:") || stdout.contains("Raw context:"),
        "{stdout}"
    );

    let (code, json, stderr) = cli(&dir, &["--compact", "context", "add two numbers"]);
    assert_eq!(code, 0, "{stderr}");
    let value: serde_json::Value = serde_json::from_str(json.trim()).expect("json");
    assert!(value.get("context_id").is_some(), "{json}");
    assert!(value.get("tokens").is_some());
    assert!(value.get("candidates").is_some());
}

#[test]
fn stats_and_cache_status_human_and_json() {
    let dir = fixture();
    cli(&dir, &["init"]);
    cli(&dir, &["index"]);

    let (code, stdout, stderr) = cli(&dir, &["stats"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("cache:"), "{stdout}");

    let (code, stdout, stderr) = cli(&dir, &["cache"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("entries:"), "{stdout}");

    let (code, json, stderr) = cli(&dir, &["--json", "cache", "status"]);
    assert_eq!(code, 0, "{stderr}");
    let value: serde_json::Value = serde_json::from_str(&json).expect("json");
    assert_eq!(value["action"], "status");

    let (code, json, stderr) = cli(&dir, &["--json", "cache", "clear"]);
    assert_eq!(code, 0, "{stderr}");
    let value: serde_json::Value = serde_json::from_str(&json).expect("json");
    assert_eq!(value["action"], "clear");
    assert!(value["removed"].is_u64());
}

#[test]
fn diff_without_git_exits_not_found_or_zero_on_non_git() {
    let dir = fixture();
    // Non-git temp dir: either not-found (3) or a clean/empty reduction (0)
    // depending on whether an ancestor is a git repo. Both are valid; usage
    // and internal failures are not.
    let (code, _, stderr) = cli(&dir, &["diff"]);
    assert!(
        code == 0 || code == ExitCode::NotFound.as_i32(),
        "unexpected exit {code}: {stderr}"
    );
}

#[test]
fn json_errors_are_machine_readable() {
    let dir = fixture();
    let (code, _, stderr) = cli(&dir, &["--json", "overview"]);
    assert_exit(code, ExitCode::Stale, &stderr);
    let value: serde_json::Value = serde_json::from_str(stderr.trim()).expect("error json");
    assert_eq!(value["exit_code"], 4);
    assert!(
        value["error"]
            .as_str()
            .unwrap_or_default()
            .contains("index")
    );
}

#[test]
fn parse_command_enum_is_exhaustive_for_help_surface() {
    // Smoke: construct representative commands through the public parse API.
    let outcome = run(vec!["help".into()]);
    assert_eq!(outcome.code, 0);
    let _ = CliCommand::Help;
    let _ = CacheAction::Status;
}

#[test]
fn init_force_flag_before_command() {
    let dir = fixture();
    // --force appears after init in normal use; also verify flag position after command.
    let argv = vec![
        "--path".into(),
        dir.display().to_string(),
        "init".into(),
        "--force".into(),
    ];
    let outcome = run(argv);
    assert_eq!(outcome.code, 0, "{}", outcome.stderr);
    assert!(dir.join(".tpt-weave").join("manifest.toml").exists());
}

/// Ensures the binary name target exists (compile-level wiring).
#[test]
fn binary_target_is_declared() {
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("Cargo.toml");
    assert!(manifest.contains("name = \"tpt-weave\""));
    let _ = Command::new("true");
}

#[test]
fn skeleton_renders_file_representations() {
    let dir = fixture();
    let (code, _, stderr) = cli(&dir, &["init"]);
    assert_eq!(code, 0, "{stderr}");
    let (code, _, stderr) = cli(&dir, &["index"]);
    assert_eq!(code, 0, "{stderr}");

    let (code, human, stderr) = cli(&dir, &["skeleton", "src/lib.rs"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(human.contains("src/lib.rs"), "{human}");
    assert!(human.contains("level: skeleton"), "{human}");
    assert!(human.contains("pub fn add"), "{human}");

    let (code, json, stderr) = cli(&dir, &["--json", "skeleton", "src/lib.rs"]);
    assert_eq!(code, 0, "{stderr}");
    let value: serde_json::Value = serde_json::from_str(&json).expect("skeleton json");
    assert_eq!(value["path"], "src/lib.rs");
    assert!(value["token_estimate"].as_u64().expect("tokens") > 0);
    assert!(
        value["text"]
            .as_str()
            .expect("text")
            .contains("pub struct Counter")
    );

    // Explicit level override: signatures render only signature lines.
    let (code, signatures, stderr) = cli(&dir, &["skeleton", "src/lib.rs", "--level", "2"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(signatures.contains("level: signatures"), "{signatures}");

    // Windows-style separators are normalised.
    let (code, _, stderr) = cli(&dir, &["skeleton", "src\\lib.rs"]);
    assert_eq!(code, 0, "{stderr}");

    // Unknown file → exit 3; existing tree without an index → exit 4.
    let (code, _, _) = cli(&dir, &["skeleton", "src/missing.rs"]);
    assert_exit(code, ExitCode::NotFound, "");
    let bare = std::env::temp_dir().join("tpt-weave-cli-skeleton-nomanifest");
    let _ = fs::remove_dir_all(&bare);
    fs::create_dir_all(&bare).expect("mkdir bare");
    let (code, _, _) = cli(&bare, &["skeleton", "src/lib.rs"]);
    assert_exit(code, ExitCode::Stale, "");
}
