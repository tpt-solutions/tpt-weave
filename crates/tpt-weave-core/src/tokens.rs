//! Token accounting for every context operation (spec.md section 16).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Token accounting record. The primary metric of the whole system is
/// `net_reduction` (spec.md section 23):
/// `1 - (selected + jev) / raw`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenAccounting {
    /// Raw tokens the unoptimised context would have occupied.
    pub raw_tokens: u64,
    /// Tokens actually delivered.
    pub selected_tokens: u64,
    /// `raw_tokens - selected_tokens` (computed, saturating at 0).
    pub saved_tokens: u64,
    /// `selected_tokens / raw_tokens` (0.0 when `raw_tokens` is 0).
    pub selection_ratio: f64,
    /// Tokens spent on decision calls (JEv overhead).
    pub jev_tokens: u64,
    /// Tokens produced by the downstream model (output), when measured.
    #[serde(default)]
    pub model_tokens: u64,
    /// Whether the selection was served from cache.
    pub cache_hit: bool,
}

impl TokenAccounting {
    /// Creates an accounting record, deriving `saved_tokens` and
    /// `selection_ratio` from the raw/selected counts.
    pub fn new(
        raw_tokens: u64,
        selected_tokens: u64,
        jev_tokens: u64,
        model_tokens: u64,
        cache_hit: bool,
    ) -> Self {
        let saved_tokens = raw_tokens.saturating_sub(selected_tokens);
        let selection_ratio = if raw_tokens == 0 {
            0.0
        } else {
            selected_tokens as f64 / raw_tokens as f64
        };
        Self {
            raw_tokens,
            selected_tokens,
            saved_tokens,
            selection_ratio,
            jev_tokens,
            model_tokens,
            cache_hit,
        }
    }

    /// `1 - selected / raw`: reduction before decision overhead.
    pub fn gross_reduction(&self) -> f64 {
        if self.raw_tokens == 0 {
            0.0
        } else {
            1.0 - self.selected_tokens as f64 / self.raw_tokens as f64
        }
    }

    /// `1 - (selected + jev) / raw`: reduction after decision overhead
    /// (may be negative when overhead exceeds the saving).
    pub fn net_reduction(&self) -> f64 {
        if self.raw_tokens == 0 {
            0.0
        } else {
            1.0 - (self.selected_tokens + self.jev_tokens) as f64 / self.raw_tokens as f64
        }
    }

    /// Human-readable report matching the spec.md section 16 example.
    pub fn summary(&self) -> String {
        let width = group(self.raw_tokens).len().max(group(self.selected_tokens).len());
        format!(
            "{:<16}{:>w$}\n{:<16}{:>w$}\n{:<16}{:>w$.1}%\n{:<16}{:>w$}\n{:<16}{:>w$.1}%",
            "Raw context:",
            group(self.raw_tokens),
            "Delivered:",
            group(self.selected_tokens),
            "Reduction:",
            self.gross_reduction() * 100.0,
            "JEv overhead:",
            group(self.jev_tokens),
            "Net reduction:",
            self.net_reduction() * 100.0,
            w = width,
        )
    }
}

impl Default for TokenAccounting {
    fn default() -> Self {
        Self::new(0, 0, 0, 0, false)
    }
}

impl fmt::Display for TokenAccounting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.summary())
    }
}

/// Formats an integer with thousands separators (`48,231`).
fn group(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_derived_fields() {
        let accounting = TokenAccounting::new(48_231, 7_312, 612, 0, true);
        assert_eq!(accounting.saved_tokens, 40_919);
        assert!((accounting.selection_ratio - 0.151_60).abs() < 1e-4);
        assert!((accounting.gross_reduction() - 0.848_40).abs() < 1e-4);
        assert!(accounting.net_reduction() < accounting.gross_reduction());
        assert!(accounting.cache_hit);
    }

    #[test]
    fn summary_matches_spec_example_shape() {
        let summary = TokenAccounting::new(61_420, 7_842, 210, 0, false).summary();
        assert!(summary.contains("61,420"), "{summary}");
        assert!(summary.contains("7,842"), "{summary}");
        assert!(summary.contains("87.2%"), "{summary}");

        let zero = TokenAccounting::default();
        assert_eq!(zero.gross_reduction(), 0.0);
        assert_eq!(zero.net_reduction(), 0.0);
    }
}
