//! JSON tool-result reduction (spec.md section 14: "JSON tool results").

use crate::reduce::push_capped;
use crate::reduction::ReductionStatus;
use serde_json::Value;

const ENTRY_CAP: usize = 20;
const SCALAR_CAP: usize = 60;

/// Reduces a JSON document to its shape: type, size and a capped preview
/// of entries. Nested containers render as `{…}` / `[…]` markers.
pub(crate) fn json_output(text: &str, exit_code: i32) -> (Vec<String>, ReductionStatus) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return (
            vec!["JSON EMPTY".to_string()],
            ReductionStatus::Success,
        );
    }

    let parsed: Result<Value, _> = serde_json::from_str(trimmed);
    let value = match parsed {
        Ok(value) => value,
        Err(error) => {
            return (
                vec![
                    "JSON FAILURE".to_string(),
                    format!("parse error: {error}"),
                ],
                ReductionStatus::Failure {
                    reason: "invalid JSON".to_string(),
                },
            );
        }
    };

    let mut lines = Vec::new();
    match &value {
        Value::Object(map) => {
            lines.push("JSON object".to_string());
            lines.push(format!("keys: {}", map.len()));
            lines.push("ENTRIES:".to_string());
            push_capped(
                &mut lines,
                map.iter()
                    .map(|(key, value)| format!("{key}: {}", preview(value))),
                ENTRY_CAP,
            );
        }
        Value::Array(items) => {
            lines.push("JSON array".to_string());
            lines.push(format!("length: {}", items.len()));
            lines.push("ELEMENTS:".to_string());
            push_capped(&mut lines, items.iter().map(preview), ENTRY_CAP);
        }
        scalar => {
            lines.push("JSON scalar".to_string());
            lines.push(format!("value: {}", preview(scalar)));
        }
    }

    let status = if exit_code == 0 {
        ReductionStatus::Success
    } else {
        ReductionStatus::Failure {
            reason: format!("exit code {exit_code}"),
        }
    };
    (lines, status)
}

/// A one-line rendering of a value: scalars inline, containers collapsed.
fn preview(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => {
            let flattened: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
            if flattened.chars().count() > SCALAR_CAP {
                let truncated: String = flattened.chars().take(SCALAR_CAP).collect();
                format!("\"{truncated}…\"")
            } else {
                format!("\"{flattened}\"")
            }
        }
        Value::Array(_) => "[…]".to_string(),
        Value::Object(_) => "{…}".to_string(),
    }
}
