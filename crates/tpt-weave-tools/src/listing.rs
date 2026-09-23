//! File listing reduction (spec.md section 14: "directory listings").

use crate::reduce::{exit_status, push_capped};
use crate::reduction::ReductionStatus;
use std::collections::BTreeMap;

const HISTOGRAM_CAP: usize = 15;

/// Reduces a one-path-per-line file listing to counts and histograms.
/// Paths ending in `/` are directories; everything else counts as a file.
pub(crate) fn listing_output(text: &str, exit_code: i32) -> (Vec<String>, ReductionStatus) {
    let mut files = 0u64;
    let mut directories = 0u64;
    let mut extensions: BTreeMap<String, u64> = BTreeMap::new();
    let mut top_level: BTreeMap<String, u64> = BTreeMap::new();

    for line in text.lines() {
        let path = line.trim().trim_end_matches('\r');
        if path.is_empty() || path.starts_with("total ") {
            continue;
        }
        let top = path.split('/').next().unwrap_or(path);
        let top = if top.is_empty() { "/" } else { top };
        *top_level.entry(top.to_string()).or_insert(0) += 1;

        if path.ends_with('/') {
            directories += 1;
            continue;
        }
        files += 1;
        if let Some(name) = path.rsplit('/').next() {
            if let Some((_, ext)) = name.rsplit_once('.') {
                if !ext.is_empty() && ext.len() <= 8 {
                    *extensions.entry(format!(".{ext}")).or_insert(0) += 1;
                }
            }
        }
    }

    let mut lines = vec![
        "FILE LISTING".to_string(),
        format!("files: {files}"),
        format!("directories: {directories}"),
    ];
    if !extensions.is_empty() {
        lines.push("EXTENSIONS:".to_string());
        // Largest counts first, then name, for a stable order.
        let mut ranked: Vec<(String, u64)> = extensions.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        push_capped(
            &mut lines,
            ranked.into_iter().map(|(ext, n)| format!("{ext} {n}")),
            HISTOGRAM_CAP,
        );
    }
    if !top_level.is_empty() {
        lines.push("TOP LEVEL:".to_string());
        let mut ranked: Vec<(String, u64)> = top_level.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        push_capped(
            &mut lines,
            ranked.into_iter().map(|(dir, n)| format!("{dir}: {n}")),
            HISTOGRAM_CAP,
        );
    }

    (lines, exit_status(exit_code, text))
}
