//! Git integration (spec.md section 8.3, todo.md Phase 2 "Git").
//!
//! Shells out to the `git` CLI (deterministic, no native build
//! dependencies) for repository discovery, revision recording, working-tree
//! status, the current diff, changed-file queries and history lookup.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tpt_weave_core::Revision;

/// Errors from git operations.
#[derive(Debug)]
pub enum GitError {
    /// Failed to spawn `git`.
    Io(std::io::Error),
    /// `git` reported no repository at or above the given path.
    NotARepository(PathBuf),
    /// `git` exited non-zero for any other reason.
    CommandFailed { status: Option<i32>, stderr: String },
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::Io(err) => write!(f, "failed to run git: {err}"),
            GitError::NotARepository(path) => {
                write!(f, "not a git repository: {}", path.display())
            }
            GitError::CommandFailed { status, stderr } => {
                write!(f, "git command failed (status {status:?}): {stderr}")
            }
        }
    }
}

impl std::error::Error for GitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GitError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for GitError {
    fn from(err: std::io::Error) -> Self {
        GitError::Io(err)
    }
}

/// How a file changed (normalised across `status` and `diff` output).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Untracked,
}

/// One entry from `git status --porcelain=v1`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStatus {
    /// Repository-relative path (`/`-separated).
    pub path: String,
    /// Original path for renames/copies.
    pub old_path: Option<String>,
    pub status: ChangeStatus,
    /// Change is staged (in the index).
    pub staged: bool,
    /// Change is unstaged (in the working tree), or the file is untracked.
    pub unstaged: bool,
}

/// One entry from `git diff --name-status`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffEntry {
    pub path: String,
    /// Old path for renames/copies.
    pub old_path: Option<String>,
    pub status: ChangeStatus,
}

/// One commit from `git log`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    /// Author date, ISO-8601 (`%aI`).
    pub date: String,
    pub subject: String,
}

/// A git working tree discovered from some starting directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitRepository {
    root: PathBuf,
}

impl GitRepository {
    /// Walks up from `start` until a repository work tree is found.
    pub fn discover(start: impl AsRef<Path>) -> Result<Self, GitError> {
        let start = start.as_ref();
        let stdout = git_stdout(start, &["rev-parse", "--show-toplevel"])?;
        let root = PathBuf::from(stdout.trim());
        if root.as_os_str().is_empty() {
            return Err(GitError::NotARepository(start.to_path_buf()));
        }
        Ok(Self { root })
    }

    /// Wraps a known repository root without verifying it.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Repository work-tree root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Records the current revision: HEAD sha plus branch name when on a
    /// branch (`None` when detached).
    pub fn current_revision(&self) -> Result<Revision, GitError> {
        let sha = git_stdout(&self.root, &["rev-parse", "HEAD"])?
            .trim()
            .to_string();
        let branch = git_stdout(&self.root, &["branch", "--show-current"])?
            .trim()
            .to_string();
        let mut revision = Revision::new(sha);
        if !branch.is_empty() {
            revision = revision.with_branch(branch);
        }
        Ok(revision)
    }

    /// Working-tree status: staged, unstaged and untracked files
    /// ("Detect modified files").
    pub fn status(&self) -> Result<Vec<FileStatus>, GitError> {
        let stdout = git_stdout(&self.root, &["status", "--porcelain=v1", "-z", "-uall"])?;
        let mut entries = Vec::new();
        let mut records = stdout.split('\0');
        while let Some(record) = records.next() {
            if record.is_empty() {
                break;
            }
            if record.len() < 4 {
                continue;
            }
            let xy = &record[..2];
            let path = record[3..].to_string();
            let mut old_path = None;
            if xy.starts_with('R') || xy.starts_with('C') {
                old_path = records.next().map(str::to_string);
            }
            let (status, staged, unstaged) = classify_porcelain(xy);
            entries.push(FileStatus {
                path,
                old_path,
                status,
                staged,
                unstaged,
            });
        }
        Ok(entries)
    }

    /// Tracked files whose content differs from HEAD (staged or unstaged).
    /// Untracked files are not included — use [`Self::status`] for those.
    pub fn changed_files(&self) -> Result<Vec<String>, GitError> {
        let stdout = git_stdout(
            &self.root,
            &["-c", "core.quotepath=false", "diff", "--name-only", "HEAD"],
        )?;
        Ok(stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect())
    }

    /// Structured index of the current diff against HEAD (todo.md Phase 2:
    /// "Index current diff").
    pub fn diff(&self) -> Result<Vec<DiffEntry>, GitError> {
        let stdout = git_stdout(
            &self.root,
            &[
                "-c",
                "core.quotepath=false",
                "diff",
                "--name-status",
                "-M",
                "HEAD",
            ],
        )?;
        Ok(stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(parse_name_status)
            .collect())
    }

    /// History lookup (todo.md Phase 2: "Add optional history lookup"):
    /// newest-first commits, optionally restricted to one path.
    pub fn history(&self, limit: usize, path: Option<&str>) -> Result<Vec<CommitInfo>, GitError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let count = format!("-n{limit}");
        let mut args: Vec<&str> = vec![
            "-c",
            "core.quotepath=false",
            "log",
            count.as_str(),
            "--format=%H%x09%h%x09%an%x09%aI%x09%s",
        ];
        if let Some(path) = path {
            args.push("--");
            args.push(path);
        }
        let stdout = git_stdout(&self.root, &args)?;
        Ok(stdout
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(parse_log_line)
            .collect())
    }
}

/// Runs `git` in `dir` and returns stdout; maps git's repository error to
/// [`GitError::NotARepository`].
fn git_stdout(dir: &Path, args: &[&str]) -> Result<String, GitError> {
    let output = Command::new("git").args(args).current_dir(dir).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            return Err(GitError::NotARepository(dir.to_path_buf()));
        }
        return Err(GitError::CommandFailed {
            status: output.status.code(),
            stderr: stderr.into_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Classifies one `XY` pair from `git status --porcelain`.
fn classify_porcelain(xy: &str) -> (ChangeStatus, bool, bool) {
    let bytes = xy.as_bytes();
    if bytes.len() < 2 {
        return (ChangeStatus::Modified, false, false);
    }
    let (x, y) = (bytes[0], bytes[1]);
    if x == b'?' && y == b'?' {
        return (ChangeStatus::Untracked, false, true);
    }
    let staged = x != b' ';
    let unstaged = y != b' ';
    let unmerged = x == b'U' || y == b'U' || (x == b'A' && y == b'A') || (x == b'D' && y == b'D');
    if unmerged {
        return (ChangeStatus::Unmerged, staged, unstaged);
    }
    let code = if x != b' ' { x } else { y };
    let status = match code {
        b'A' => ChangeStatus::Added,
        b'D' => ChangeStatus::Deleted,
        b'R' => ChangeStatus::Renamed,
        b'C' => ChangeStatus::Copied,
        b'T' => ChangeStatus::TypeChanged,
        _ => ChangeStatus::Modified,
    };
    (status, staged, unstaged)
}

/// Parses one `--name-status` line: `STATUS<tab>path` or
/// `R###<tab>old<tab>new`.
fn parse_name_status(line: &str) -> DiffEntry {
    let mut parts = line.split('\t');
    let code = parts.next().unwrap_or("");
    let status_char = code.chars().next().unwrap_or('M');
    match status_char {
        'R' | 'C' => {
            let old_path = parts.next().unwrap_or("").to_string();
            let path = parts.next().unwrap_or("").to_string();
            let status = if status_char == 'R' {
                ChangeStatus::Renamed
            } else {
                ChangeStatus::Copied
            };
            DiffEntry {
                path,
                old_path: Some(old_path),
                status,
            }
        }
        _ => {
            let path = parts.next().unwrap_or("").to_string();
            let status = match status_char {
                'A' => ChangeStatus::Added,
                'D' => ChangeStatus::Deleted,
                'T' => ChangeStatus::TypeChanged,
                'U' => ChangeStatus::Unmerged,
                _ => ChangeStatus::Modified,
            };
            DiffEntry {
                path,
                old_path: None,
                status,
            }
        }
    }
}

/// Parses one tab-separated `--format` log line.
fn parse_log_line(line: &str) -> Option<CommitInfo> {
    let mut parts = line.splitn(5, '\t');
    Some(CommitInfo {
        sha: parts.next()?.to_string(),
        short_sha: parts.next()?.to_string(),
        author: parts.next()?.to_string(),
        date: parts.next()?.to_string(),
        subject: parts.next().unwrap_or("").to_string(),
    })
}
