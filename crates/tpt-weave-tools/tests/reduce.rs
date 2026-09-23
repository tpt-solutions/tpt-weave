//! Tool-output reduction (todo.md Phase 6, spec.md section 14).

use std::fs;
use tpt_weave_tools::{RawStore, Reduction, ReductionStatus, ToolKind, ToolOutput, reduce};

/// Big passing `cargo test` run: many harness lines, zero failures.
fn cargo_test_success() -> String {
    let mut out = String::new();
    for harness in 0..6 {
        out.push_str("running 30 tests\n");
        for i in 0..30 {
            out.push_str(&format!("test suite{harness}::test_{i} ... ok\n"));
        }
        out.push_str(
            "test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; \
             0 filtered out; finished in 0.01s\n\n",
        );
    }
    out
}

/// Failing run with two named failures and detail blocks (spec shape).
fn cargo_test_failure() -> String {
    let mut out = String::from("running 4 tests\n");
    for name in ["a", "b", "c"] {
        out.push_str(&format!("test crate::{name} ... ok\n"));
    }
    out.push_str("test crate::foo::test_a ... FAILED\n");
    out.push_str("test crate::bar::test_b ... FAILED\n\n");
    out.push_str("failures:\n\n");
    out.push_str("---- crate::foo::test_a stdout ----\n");
    out.push_str("thread 'crate::foo::test_a' panicked at src/lib.rs:10:5:\n");
    out.push_str("assertion failed: left == right\n");
    out.push_str("\n---- crate::bar::test_b stdout ----\n");
    out.push_str("thread 'crate::bar::test_b' panicked at src/lib.rs:20:5:\n");
    out.push_str("boom\n\nfailures:\n");
    out.push_str("    crate::foo::test_a\n");
    out.push_str("    crate::bar::test_b\n\n");
    out.push_str(
        "test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; \
         0 filtered out; finished in 0.02s\n",
    );
    out
}

fn cargo_check_errors() -> String {
    "error[E0308]: mismatched types\n  --> src/lib.rs:10:5\n   |\n10 |     x\n   |     ^ expected `i32`, found `&str`\n   |\n   = note: ...\n\nwarning: unused variable: `y`\n  --> src/lib.rs:3:9\n\nerror: could not compile `demo` (lib) due to 1 previous error\n"
        .to_string()
}

fn git_diff_output() -> String {
    let mut out = String::new();
    for (file, add, del) in [
        ("src/a.rs", 12, 3),
        ("src/b.rs", 5, 0),
        ("tests/c.rs", 0, 7),
        ("README.md", 60, 9),
    ] {
        out.push_str(&format!("diff --git a/{file} b/{file}\n"));
        out.push_str("--- a/{file}\n+++ b/{file}\n@@ -1,3 +1,4 @@\n");
        for i in 0..add {
            out.push_str(&format!("+added line {i}\n"));
        }
        for i in 0..del {
            out.push_str(&format!("-removed line {i}\n"));
        }
        out.push_str(" context\n");
    }
    out
}

#[test]
fn reduces_a_passing_test_run_to_counts() {
    let raw = cargo_test_success();
    let output = ToolOutput::new("cargo test --workspace", raw);
    let reduction = reduce(ToolKind::CargoTest, &output);

    let rendered = reduction.render();
    assert!(rendered.starts_with("TEST OK"), "{rendered}");
    assert!(rendered.contains("tests: 180"));
    assert!(rendered.contains("passed: 180"));
    assert!(rendered.contains("failed: 0"));
    assert!(rendered.contains("harnesses: 6"));
    assert!(rendered.contains("DETAIL AVAILABLE: yes"));
    assert_eq!(reduction.status, ReductionStatus::Success);
    assert!(!reduction.failed());

    // Measured reduction (todo.md Phase 6 "Measure reduction").
    assert!(reduction.reduced_tokens < reduction.raw_tokens);
    assert!(reduction.reduction() > 0.5, "{}", reduction.reduction());
}

#[test]
fn preserves_test_failure_details() {
    let output = ToolOutput::new("cargo test", cargo_test_failure()).with_exit_code(101);
    let reduction = reduce(ToolKind::CargoTest, &output);

    let rendered = reduction.render();
    assert!(rendered.starts_with("TEST FAILURE"), "{rendered}");
    assert!(rendered.contains("tests: 4"));
    assert!(rendered.contains("passed: 2"));
    assert!(rendered.contains("failed: 2"));
    assert!(rendered.contains("FAILURES:"));
    assert!(rendered.contains("crate::foo::test_a"));
    assert!(rendered.contains("crate::bar::test_b"));

    assert!(reduction.failed());
    assert_eq!(
        reduction.status,
        ReductionStatus::Failure {
            reason: "2 tests failed".to_string()
        }
    );
}

#[test]
fn caps_long_failure_lists() {
    let mut out = String::from("running 25 tests\n");
    for i in 0..25 {
        out.push_str(&format!("test failure_case_{i} ... FAILED\n"));
    }
    out.push_str("\nfailures:\n");
    for i in 0..25 {
        out.push_str(&format!("    failure_case_{i}\n"));
    }
    out.push_str("\ntest result: FAILED. 0 passed; 25 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n");

    let reduction = reduce(ToolKind::CargoTest, &ToolOutput::new("cargo test", out));
    let rendered = reduction.render();
    assert!(rendered.contains("... +5 more"), "{rendered}");
    let listed = rendered
        .lines()
        .filter(|l| l.starts_with("failure_case_"))
        .count();
    assert_eq!(listed, 20);
}

#[test]
fn reduces_compiler_diagnostics_with_locations() {
    let output = ToolOutput::new("cargo check", cargo_check_errors()).with_exit_code(101);
    let reduction = reduce(ToolKind::CargoCheck, &output);

    let rendered = reduction.render();
    assert!(rendered.starts_with("CHECK FAILURE"), "{rendered}");
    assert!(rendered.contains("errors: 2"));
    assert!(rendered.contains("warnings: 1"));
    assert!(rendered.contains("DIAGNOSTICS:"));
    assert!(rendered.contains("error[E0308]: mismatched types (src/lib.rs:10)"));
    assert!(rendered.contains("warning: unused variable: `y` (src/lib.rs:3)"));
    assert!(reduction.failed());
    assert_eq!(
        reduction.status,
        ReductionStatus::Failure {
            reason: "2 errors".to_string()
        }
    );
}

#[test]
fn reduces_clean_check_and_clippy() {
    let check = reduce(
        ToolKind::CargoCheck,
        &ToolOutput::new("cargo check", "    Checking demo v0.1.0\n    Finished\n"),
    );
    assert!(check.render().starts_with("CHECK OK"));
    assert!(check.render().contains("errors: 0"));
    assert!(!check.failed());

    let clippy = reduce(
        ToolKind::CargoClippy,
        &ToolOutput::new(
            "cargo clippy",
            "warning: 3 warnings emitted\n    Finished\n",
        ),
    );
    assert!(clippy.render().starts_with("CLIPPY OK"));
    // Bare `warning:` lines without `-->` still parse as diagnostics.
    assert!(clippy.render().contains("warnings: 1"));
}

#[test]
fn reduces_fmt_diffs() {
    let raw = "Diff in /repo/src/lib.rs at line 12:\n@@ ... @@\nDiff in /repo/src/math.rs at line 3:\n@@ ... @@\n";
    let reduction = reduce(
        ToolKind::CargoFmt,
        &ToolOutput::new("cargo fmt --check", raw).with_exit_code(1),
    );
    let rendered = reduction.render();
    assert!(rendered.starts_with("FMT FAILURE"), "{rendered}");
    assert!(rendered.contains("files needing format: 2"));
    assert!(rendered.contains("/repo/src/lib.rs"));
    assert!(rendered.contains("/repo/src/math.rs"));
    assert_eq!(
        reduction.status,
        ReductionStatus::Failure {
            reason: "2 files need formatting".to_string()
        }
    );

    let clean = reduce(
        ToolKind::CargoFmt,
        &ToolOutput::new("cargo fmt --check", ""),
    );
    assert!(clean.render().starts_with("FMT OK"));
    assert!(!clean.failed());
}

#[test]
fn reduces_git_status_porcelain_and_long_form() {
    let porcelain = "## main...origin/main\n M src/lib.rs\nA  src/new.rs\n?? tests/x.rs\n D old.rs\nR  old.rs -> new.rs\n";
    let reduction = reduce(
        ToolKind::GitStatus,
        &ToolOutput::new("git status", porcelain),
    );
    let rendered = reduction.render();
    assert!(rendered.contains("files: 5"), "{rendered}");
    assert!(rendered.contains("modified: 1"));
    assert!(rendered.contains("added: 1"));
    assert!(rendered.contains("deleted: 1"));
    assert!(rendered.contains("renamed: 1"));
    assert!(rendered.contains("untracked: 1"));
    assert!(rendered.contains("FILES:"));
    assert!(!reduction.failed());

    let long_form = "On branch main\nChanges to be committed:\n  (use \"git restore --staged ...\"\n\tnew file:   staged.rs\nChanges not staged for commit:\n  (use \"git add ...\")\n\tmodified:   dirty.rs\nUntracked files:\n  (use \"git add ...\")\n\tfresh.rs\n";
    let reduction = reduce(
        ToolKind::GitStatus,
        &ToolOutput::new("git status", long_form),
    );
    let rendered = reduction.render();
    assert!(rendered.contains("added: 1"), "{rendered}");
    assert!(rendered.contains("modified: 1"));
    assert!(rendered.contains("untracked: 1"));
    assert!(rendered.contains("?? fresh.rs"));
}

#[test]
fn reduces_git_diff_to_per_file_counts() {
    let reduction = reduce(
        ToolKind::GitDiff,
        &ToolOutput::new("git diff", git_diff_output()),
    );
    let rendered = reduction.render();
    assert!(rendered.contains("files: 4"), "{rendered}");
    assert!(rendered.contains("additions: 77"));
    assert!(rendered.contains("deletions: 19"));
    assert!(rendered.contains("src/a.rs +12 -3"));
    assert!(rendered.contains("tests/c.rs +0 -7"));
    assert!(rendered.contains("DETAIL AVAILABLE: yes"));
    assert!(!reduction.failed());
    assert!(reduction.reduction() > 0.5, "{}", reduction.reduction());

    // `git diff --exit-code` exit 1 means "has differences", not failure.
    let has_changes = reduce(
        ToolKind::GitDiff,
        &ToolOutput::new("git diff --exit-code", git_diff_output()).with_exit_code(1),
    );
    assert!(!has_changes.failed());

    let empty = reduce(ToolKind::GitDiff, &ToolOutput::new("git diff", ""));
    assert!(empty.render().contains("files: 0"));
}

#[test]
fn reduces_file_listings_to_histograms() {
    let mut raw = String::new();
    for i in 0..150 {
        raw.push_str(&format!("crates/demo/src/file{i}.rs\n"));
    }
    for i in 0..30 {
        raw.push_str(&format!("crates/demo/tests/test{i}.rs\n"));
    }
    raw.push_str("Cargo.toml\nREADME.md\ntarget/\n");

    let reduction = reduce(
        ToolKind::FileListing,
        &ToolOutput::new("find . -type f", raw),
    );
    let rendered = reduction.render();
    assert!(rendered.starts_with("FILE LISTING"), "{rendered}");
    assert!(rendered.contains("files: 182"));
    assert!(rendered.contains("directories: 1"));
    assert!(rendered.contains(".rs 180"));
    assert!(rendered.contains("TOP LEVEL:"));
    assert!(rendered.contains("crates: 180"));
    assert!(reduction.reduction() > 0.3, "{}", reduction.reduction());
}

#[test]
fn reduces_search_results_to_per_file_counts() {
    let mut raw = String::new();
    for i in 0..12 {
        raw.push_str(&format!("src/lib.rs:{i}:let make = Pixel;\n"));
    }
    raw.push_str("src/math.rs:3:pub fn square(x: u32) -> u32 { x * x }\n");
    raw.push_str("src/math.rs:8:pub fn cube(x: u32) -> u32 { x * square(x) }\n");
    raw.push_str("C:\\repo\\src\\win.rs:7:let x = 1;\n");
    raw.push_str("unparseable noise line\n");

    let reduction = reduce(ToolKind::Search, &ToolOutput::new("rg make", raw));
    let rendered = reduction.render();
    assert!(rendered.starts_with("SEARCH"), "{rendered}");
    assert!(rendered.contains("matches: 15"));
    assert!(rendered.contains("files: 3"));
    assert!(rendered.contains("other lines: 1"));
    assert!(rendered.contains("src/lib.rs: 12"));
    assert!(rendered.contains("src/math.rs: 2"));
    assert!(rendered.contains("C:\\repo\\src\\win.rs: 1"));
    assert!(!reduction.failed());
}

#[test]
fn reduces_json_documents_to_their_shape() {
    let object = r#"{"name":"demo","count":3,"items":[1,2,3],"nested":{"deep":true},"note":"a fairly long note value that should be truncated when it exceeds the scalar preview cap"}"#;
    let reduction = reduce(ToolKind::Json, &ToolOutput::new("jq .", object));
    let rendered = reduction.render();
    assert!(rendered.starts_with("JSON object"), "{rendered}");
    assert!(rendered.contains("keys: 5"));
    assert!(rendered.contains("name: \"demo\""));
    assert!(rendered.contains("count: 3"));
    assert!(rendered.contains("items: […]"));
    assert!(rendered.contains("nested: {…}"));
    assert!(rendered.contains("…\""));

    let array = reduce(ToolKind::Json, &ToolOutput::new("jq .", "[1,2,3,4]"));
    assert!(array.render().contains("JSON array"));
    assert!(array.render().contains("length: 4"));

    let invalid = reduce(
        ToolKind::Json,
        &ToolOutput::new("jq .", "{not json").with_exit_code(1),
    );
    assert!(invalid.render().starts_with("JSON FAILURE"));
    assert!(invalid.failed());
}

#[test]
fn reduces_logs_keeping_error_lines() {
    let mut raw = String::new();
    for i in 0..400 {
        raw.push_str(&format!("2026-09-23T10:00:{i:02} INFO worker started\n"));
    }
    raw.push_str("2026-09-23T10:05:00 ERROR database connection refused\n");
    raw.push_str("2026-09-23T10:05:01 WARN retrying in 5s\n");
    raw.push_str("2026-09-23T10:05:02 ERROR fatal: shutting down\n");

    let reduction = reduce(ToolKind::Log, &ToolOutput::new("app.log", raw));
    let rendered = reduction.render();
    assert!(rendered.starts_with("LOG"), "{rendered}");
    assert!(rendered.contains("lines: 403"));
    assert!(rendered.contains("errors: 2"));
    assert!(rendered.contains("warnings: 1"));
    assert!(rendered.contains("ERRORS:"));
    assert!(rendered.contains("database connection refused"));
    assert!(rendered.contains("fatal: shutting down"));
    assert!(!reduction.failed());
    assert!(reduction.reduction() > 0.7, "{}", reduction.reduction());
}

#[test]
fn falls_back_to_diagnostics_when_tests_fail_to_compile() {
    let raw = "error[E0425]: cannot find value `missing` in this scope\n  --> src/lib.rs:5:5\n\nerror: could not compile `demo` (lib) due to 1 previous error\n";
    let reduction = reduce(
        ToolKind::CargoTest,
        &ToolOutput::new("cargo test", raw).with_exit_code(101),
    );
    let rendered = reduction.render();
    assert!(rendered.starts_with("TEST FAILURE"), "{rendered}");
    assert!(rendered.contains("error[E0425]: cannot find value `missing`"));
    assert!(reduction.failed());
}

#[test]
fn stores_raw_output_and_expands_it_back() {
    let temp = std::env::temp_dir().join(format!("tpt-weave-tools-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let store = RawStore::new(&temp);

    let raw = cargo_test_failure();
    let output = ToolOutput::new("cargo test", raw.clone()).with_exit_code(101);
    let mut reduction = reduce(ToolKind::CargoTest, &output);

    // Expansion before storage fails cleanly.
    assert!(reduction.expand(&store).is_err());

    let path = reduction.store_raw(&output, &store).expect("store");
    assert!(path.exists());
    assert_eq!(reduction.detail.as_ref(), Some(&path));

    // Expansion returns the raw output verbatim (spec: "Full raw output
    // remains locally available. The model can request it.").
    let expanded = reduction.expand(&store).expect("expand");
    assert_eq!(expanded, raw);

    // Deterministic: identical content stores at the same path.
    let again = store.store(&output).expect("re-store");
    assert_eq!(path, again);

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn empty_output_reports_no_detail() {
    let reduction: Reduction = reduce(ToolKind::Log, &ToolOutput::new("noop", ""));
    assert!(reduction.render().contains("DETAIL AVAILABLE: no"));
    assert_eq!(reduction.raw_tokens, 0);
    assert_eq!(reduction.reduction(), 0.0);
}

#[test]
fn reduction_is_deterministic() {
    let output = ToolOutput::new("cargo test", cargo_test_failure()).with_exit_code(101);
    let a = reduce(ToolKind::CargoTest, &output);
    let b = reduce(ToolKind::CargoTest, &output);
    assert_eq!(a, b);
    assert_eq!(a.render(), b.render());
    assert_eq!(a.raw_tokens, b.raw_tokens);
}

#[test]
fn exit_code_without_output_becomes_a_failure() {
    let output = ToolOutput::new("cargo build", "").with_exit_code(101);
    let reduction = reduce(ToolKind::CargoBuild, &output);
    assert!(reduction.failed());
    assert_eq!(
        reduction.status,
        ReductionStatus::Failure {
            reason: "exit code 101".to_string()
        }
    );
}

#[test]
fn infers_tool_kinds_from_commands() {
    assert_eq!(
        ToolKind::infer("cargo test --workspace"),
        ToolKind::CargoTest
    );
    assert_eq!(ToolKind::infer("cargo check"), ToolKind::CargoCheck);
    assert_eq!(
        ToolKind::infer("cargo clippy -- -D warnings"),
        ToolKind::CargoClippy
    );
    assert_eq!(
        ToolKind::infer("cargo build --release"),
        ToolKind::CargoBuild
    );
    assert_eq!(ToolKind::infer("cargo fmt --check"), ToolKind::CargoFmt);
    assert_eq!(
        ToolKind::infer("git status --porcelain"),
        ToolKind::GitStatus
    );
    assert_eq!(ToolKind::infer("git diff --stat"), ToolKind::GitDiff);
    assert_eq!(ToolKind::infer("rg make"), ToolKind::Search);
    assert_eq!(ToolKind::infer("grep -rn make src"), ToolKind::Search);
    assert_eq!(ToolKind::infer("ls -la"), ToolKind::FileListing);
    assert_eq!(ToolKind::infer("find . -type f"), ToolKind::FileListing);
    assert_eq!(ToolKind::infer("jq ."), ToolKind::Json);
    assert_eq!(ToolKind::infer("journalctl -u app"), ToolKind::Log);
}
