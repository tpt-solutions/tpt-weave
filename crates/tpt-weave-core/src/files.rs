//! File records for the deterministic repository index (spec.md sections 6
//! and 8).

use serde::{Deserialize, Serialize};

/// Language of an indexed file, derived from the file extension.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLanguage {
    Rust,
    Toml,
    Markdown,
    Json,
    Yaml,
    Shell,
    /// Extensionless files are treated as text.
    #[default]
    Text,
    Other,
}

impl SourceLanguage {
    /// Detects the language from a path's extension. Extensionless files are
    /// treated as [`SourceLanguage::Text`].
    pub fn from_path(path: &str) -> Self {
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
        let extension = name.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase());
        match extension.as_deref() {
            Some("rs") => SourceLanguage::Rust,
            Some("toml") => SourceLanguage::Toml,
            Some("md") | Some("markdown") => SourceLanguage::Markdown,
            Some("json") => SourceLanguage::Json,
            Some("yml") | Some("yaml") => SourceLanguage::Yaml,
            Some("sh") | Some("bash") | Some("ps1") | Some("bat") | Some("cmd") => {
                SourceLanguage::Shell
            }
            Some("txt") => SourceLanguage::Text,
            Some(_) => SourceLanguage::Other,
            None => SourceLanguage::Text,
        }
    }
}

/// Record of one indexed file (the `files.json` artefact, spec.md section 6).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileRecord {
    /// Repository-relative path, always `/`-separated.
    pub path: String,
    /// File size in bytes.
    pub bytes: u64,
    /// Line count.
    pub lines: u64,
    pub language: SourceLanguage,
    /// Stable content hash (see [`crate::hash::fnv1a64_hex`]); empty until
    /// the indexer fills it.
    pub content_hash: String,
}

impl FileRecord {
    /// Creates a record, normalising path separators to `/` and deriving the
    /// language from the path.
    pub fn new(
        path: impl Into<String>,
        bytes: u64,
        lines: u64,
        content_hash: impl Into<String>,
    ) -> Self {
        let path = path.into().replace('\\', "/");
        let language = SourceLanguage::from_path(&path);
        Self {
            path,
            bytes,
            lines,
            language,
            content_hash: content_hash.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_languages() {
        assert_eq!(SourceLanguage::from_path("src/lib.rs"), SourceLanguage::Rust);
        assert_eq!(
            SourceLanguage::from_path("Cargo.toml"),
            SourceLanguage::Toml
        );
        assert_eq!(
            SourceLanguage::from_path("docs/readme.MD"),
            SourceLanguage::Markdown
        );
        assert_eq!(SourceLanguage::from_path("Makefile"), SourceLanguage::Text);
        assert_eq!(
            SourceLanguage::from_path("build.gradle"),
            SourceLanguage::Other
        );
    }

    #[test]
    fn normalises_windows_paths() {
        let record = FileRecord::new("src\\image\\resample.rs", 1024, 42, "abc");
        assert_eq!(record.path, "src/image/resample.rs");
        assert_eq!(record.language, SourceLanguage::Rust);
    }
}
