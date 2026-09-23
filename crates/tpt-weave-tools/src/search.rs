//! Search result reduction (spec.md section 14: "grep/search").

use crate::reduce::{exit_status, push_capped};
use crate::reduction::ReductionStatus;
use std::collections::BTreeMap;

const FILE_CAP: usize = 20;

/// Reduces `path:line:content` search results to per-file match counts.
/// Lines that do not parse are counted separately so nothing is silently
/// dropped.
pub(crate) fn search_output(text: &str, exit_code: i32) -> (Vec<String>, ReductionStatus) {
    let mut by_file: BTreeMap<String, u64> = BTreeMap::new();
    let mut matches = 0u64;
    let mut other = 0u64;

    for line in text.lines() {
        match parse_match(line) {
            Some(path) => {
                *by_file.entry(path).or_insert(0) += 1;
                matches += 1;
            }
            None => {
                if !line.trim().is_empty() {
                    other += 1;
                }
            }
        }
    }

    let mut lines = vec![
        "SEARCH".to_string(),
        format!("matches: {matches}"),
        format!("files: {}", by_file.len()),
    ];
    if other > 0 {
        lines.push(format!("other lines: {other}"));
    }
    if !by_file.is_empty() {
        lines.push("FILES:".to_string());
        push_capped(
            &mut lines,
            by_file.into_iter().map(|(path, n)| format!("{path}: {n}")),
            FILE_CAP,
        );
    }
    (lines, exit_status(exit_code, text))
}

/// Extracts the path from `path:line:content`, tolerating `:` inside
/// Windows paths (`C:\src\lib.rs:10:content`).
fn parse_match(line: &str) -> Option<String> {
    let mut search_from = 0;
    while let Some(offset) = line[search_from..].find(':') {
        let colon = search_from + offset;
        let candidate = &line[colon + 1..];
        let field_end = candidate
            .find(':')
            .map(|i| colon + 1 + i)
            .unwrap_or(line.len());
        let number = &line[colon + 1..field_end];
        if !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) {
            if field_end < line.len() {
                return Some(line[..colon].to_string());
            }
            return None;
        }
        search_from = colon + 1;
    }
    None
}
