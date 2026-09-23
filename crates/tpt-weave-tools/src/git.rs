//! git status / git diff reducers (spec.md section 14).

use crate::reduce::{exit_status, push_capped};
use crate::reduction::ReductionStatus;
use std::collections::BTreeMap;

const FILE_CAP: usize = 50;

/// Reduces `git status` output (porcelain v1 or long form).
pub(crate) fn status_output(text: &str, exit_code: i32) -> (Vec<String>, ReductionStatus) {
    let mut counts: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut other = 0u64;
    let mut files: Vec<String> = Vec::new();
    let mut in_untracked = false;

    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(entry) = parse_porcelain(line) {
            let category = classify(entry.x, entry.y);
            bump(&mut counts, &mut other, category);
            files.push(line.to_string());
            in_untracked = false;
            continue;
        }
        let trimmed = line.trim();
        if trimmed == "Untracked files:" {
            in_untracked = true;
            continue;
        }
        if in_untracked {
            if trimmed.is_empty() {
                in_untracked = false;
                continue;
            }
            // Hint lines (`  (use "git add ...")`) are not paths.
            if trimmed.starts_with('(') {
                continue;
            }
            if line.starts_with(' ') || line.starts_with('\t') {
                bump(&mut counts, &mut other, "untracked");
                files.push(format!("?? {trimmed}"));
                continue;
            }
            in_untracked = false;
        }
        if let Some(category) = long_form_category(trimmed) {
            bump(&mut counts, &mut other, category);
            files.push(trimmed.to_string());
        }
    }

    let total: u64 = counts.values().sum::<u64>() + other;
    let mut lines = vec![
        "GIT STATUS".to_string(),
        format!("files: {total}"),
        format!("modified: {}", get(&counts, "modified")),
        format!("added: {}", get(&counts, "added")),
        format!("deleted: {}", get(&counts, "deleted")),
        format!("renamed: {}", get(&counts, "renamed")),
        format!("untracked: {}", get(&counts, "untracked")),
    ];
    if other > 0 {
        lines.push(format!("other: {other}"));
    }
    if !files.is_empty() {
        lines.push("FILES:".to_string());
        push_capped(&mut lines, files, FILE_CAP);
    }
    (lines, exit_status(exit_code, text))
}

struct Porcelain<'a> {
    x: char,
    y: char,
    #[allow(dead_code)]
    path: &'a str,
}

/// Recognises `XY path` (porcelain v1) lines.
fn parse_porcelain(line: &str) -> Option<Porcelain<'_>> {
    let bytes = line.as_bytes();
    if bytes.len() < 4 || bytes[2] != b' ' {
        return None;
    }
    let x = line.chars().next()?;
    let y = line.chars().nth(1)?;
    if !" MADRCUT?!".contains(x) || !" MADRCUT?!".contains(y) {
        return None;
    }
    Some(Porcelain {
        x,
        y,
        path: line[3..].trim(),
    })
}

/// Priority classification: unmerged > added > deleted > renamed > copied
/// > type > modified > untracked.
fn classify(x: char, y: char) -> &'static str {
    if x == '?' && y == '?' {
        return "untracked";
    }
    if x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D') {
        return "unmerged";
    }
    for (letter, name) in [
        ('A', "added"),
        ('D', "deleted"),
        ('R', "renamed"),
        ('C', "renamed"),
        ('T', "other"),
        ('M', "modified"),
    ] {
        if x == letter || y == letter {
            return name;
        }
    }
    "other"
}

/// Long-form (`modified: path`) category keywords.
fn long_form_category(line: &str) -> Option<&'static str> {
    let lower = line.to_ascii_lowercase();
    let mappings = [
        ("modified:", "modified"),
        ("new file:", "added"),
        ("added:", "added"),
        ("deleted:", "deleted"),
        ("renamed:", "renamed"),
        ("copied:", "renamed"),
        ("typechange:", "other"),
        ("unmerged:", "unmerged"),
        ("both modified:", "modified"),
        ("both added:", "added"),
        ("deleted by them:", "deleted"),
        ("deleted by us:", "deleted"),
    ];
    mappings
        .iter()
        .find(|(prefix, _)| lower.starts_with(prefix))
        .map(|(_, name)| *name)
}

fn bump(counts: &mut BTreeMap<&'static str, u64>, other: &mut u64, category: &'static str) {
    if let Some(slot) = counts.get_mut(category) {
        *slot += 1;
    } else if category == "other" || category == "unmerged" {
        *other += 1;
    } else {
        counts.insert(category, 1);
    }
}

fn get(counts: &BTreeMap<&'static str, u64>, key: &str) -> u64 {
    counts.get(key).copied().unwrap_or(0)
}

/// Reduces unified `git diff` output to per-file add/delete counts.
pub(crate) fn diff_output(text: &str, exit_code: i32) -> (Vec<String>, ReductionStatus) {
    let mut additions: BTreeMap<String, u64> = BTreeMap::new();
    let mut deletions: BTreeMap<String, u64> = BTreeMap::new();
    let mut current: Option<String> = None;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            current = parse_diff_git_path(rest).inspect(|path| {
                additions.entry(path.clone()).or_insert(0);
            });
            continue;
        }
        let Some(path) = current.as_ref() else {
            continue;
        };
        if line.starts_with("+++ ") || line.starts_with("--- ") {
            continue;
        }
        if line.starts_with('+') {
            *additions.get_mut(path).expect("path registered") += 1;
        } else if line.starts_with('-') {
            deletions.entry(path.clone()).or_insert(0);
            *deletions.get_mut(path).expect("path registered") += 1;
        }
    }

    // Collect every file (deletions-only files never saw a `+` line).
    let mut paths: Vec<String> = additions.keys().cloned().collect();
    for path in deletions.keys() {
        if !additions.contains_key(path) {
            paths.push(path.clone());
        }
    }
    paths.sort();

    let total_add: u64 = additions.values().sum();
    let total_del: u64 = deletions.values().sum();

    let mut lines = vec![
        "GIT DIFF".to_string(),
        format!("files: {}", paths.len()),
        format!("additions: {total_add}"),
        format!("deletions: {total_del}"),
    ];
    if !paths.is_empty() {
        lines.push("FILES:".to_string());
        let rows = paths.iter().map(|path| {
            let add = additions.get(path).copied().unwrap_or(0);
            let del = deletions.get(path).copied().unwrap_or(0);
            format!("{path} +{add} -{del}")
        });
        push_capped(&mut lines, rows, FILE_CAP);
    }

    // Exit code 1 from `git diff --exit-code` only means "has changes".
    let status = if exit_code <= 1 {
        ReductionStatus::Success
    } else {
        exit_status(exit_code, text)
    };
    (lines, status)
}

/// Extracts the `b/` side path from `a/src/lib.rs b/src/lib.rs`.
fn parse_diff_git_path(rest: &str) -> Option<String> {
    if let Some((_, b_side)) = rest.rsplit_once(" b/") {
        return Some(b_side.trim().to_string());
    }
    rest.split_whitespace().nth(1).map(|s| s.to_string())
}
