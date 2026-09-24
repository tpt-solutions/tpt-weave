//! Integration tests against throwaway git repositories.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tpt_weave_index::{ChangeStatus, GitError, GitRepository};

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Creates a temp repo with one commit on `main` and returns its directory.
fn temp_repo(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "tpt-weave-git-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-b", "main"]);
    fs::write(dir.join("file.txt"), "one\n").unwrap();
    git(&dir, &["add", "."]);
    git(
        &dir,
        &[
            "-c",
            "user.name=tpt-test",
            "-c",
            "user.email=tpt-test@example.com",
            "commit",
            "-m",
            "initial",
        ],
    );
    dir
}

#[test]
fn discovers_repo_and_records_revision() {
    let dir = temp_repo("discover");
    let repo = GitRepository::discover(&dir).expect("discover");
    assert_eq!(
        repo.root().canonicalize().expect("canonical root"),
        dir.canonicalize().expect("canonical dir")
    );

    let revision = repo.current_revision().expect("revision");
    assert_eq!(revision.sha.len(), 40);
    assert!(revision.sha.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(revision.branch.as_deref(), Some("main"));
    assert_eq!(revision.short().len(), 12);

    assert!(repo.status().expect("status").is_empty(), "repo is clean");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn reports_modifications_diff_and_history() {
    let dir = temp_repo("diff");
    let repo = GitRepository::discover(&dir).unwrap();

    // Modify the tracked file and add an untracked file.
    fs::write(dir.join("file.txt"), "two\n").unwrap();
    fs::write(dir.join("new.txt"), "brand new\n").unwrap();

    let status = repo.status().expect("status");
    assert_eq!(status.len(), 2);
    let modified = status
        .iter()
        .find(|s| s.path == "file.txt")
        .expect("modified entry");
    assert_eq!(modified.status, ChangeStatus::Modified);
    assert!(modified.unstaged);
    assert!(!modified.staged);
    let untracked = status
        .iter()
        .find(|s| s.path == "new.txt")
        .expect("untracked entry");
    assert_eq!(untracked.status, ChangeStatus::Untracked);
    let first_fingerprint = repo.worktree_fingerprint().expect("fingerprint");
    fs::write(dir.join("file.txt"), "three\n").unwrap();
    let second_fingerprint = repo.worktree_fingerprint().expect("fingerprint after edit");
    assert_ne!(first_fingerprint, second_fingerprint);

    // Changed-file query: tracked content changes vs HEAD only.
    assert_eq!(repo.changed_files().expect("changed_files"), ["file.txt"]);
    let diff = repo.diff().expect("diff");
    assert_eq!(diff.len(), 1);
    assert_eq!(diff[0].path, "file.txt");
    assert_eq!(diff[0].status, ChangeStatus::Modified);

    // Staging flips the status to staged.
    git(&dir, &["add", "file.txt"]);
    let status = repo.status().unwrap();
    let modified = status
        .iter()
        .find(|s| s.path == "file.txt")
        .expect("staged entry");
    assert!(modified.staged);
    assert!(!modified.unstaged);

    // History lookup.
    let history = repo.history(10, None).expect("history");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].subject, "initial");
    assert!(!history[0].date.is_empty());
    assert!(!history[0].author.is_empty());
    let head = repo.current_revision().unwrap();
    assert_eq!(history[0].sha, head.sha);
    assert!(head.sha.starts_with(&history[0].short_sha));
    assert!(repo.history(10, Some("missing.txt")).unwrap().is_empty());
    assert!(repo.history(0, None).unwrap().is_empty());

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn discover_reports_missing_repository() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("tpt-weave-norepo-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let err = GitRepository::discover(&dir).expect_err("no repo here");
    assert!(matches!(err, GitError::NotARepository(_)), "{err:?}");
    fs::remove_dir_all(&dir).ok();
}
