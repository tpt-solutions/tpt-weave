//! Local raw-output storage and expansion (todo.md Phase 6: "Store raw
//! output locally", "Provide expansion mechanism").

use crate::reduce::ToolOutput;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Local store for raw tool output, keyed by a content hash so identical
/// output is written (and read) at the same deterministic path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawStore {
    root: PathBuf,
}

impl RawStore {
    /// Creates a store rooted at `root` (typically
    /// `.tpt-weave/tool-output`).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The directory raw output is written to.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Writes the invocation's raw output and returns its path. Re-storing
    /// identical content returns the same path without rewriting.
    pub fn store(&self, output: &ToolOutput) -> io::Result<PathBuf> {
        let content = output.stored_content();
        let path = self
            .root
            .join(format!("{:016x}.txt", fnv1a64(content.as_bytes())));
        if !path.exists() {
            fs::create_dir_all(&self.root)?;
            fs::write(&path, &content)?;
        }
        Ok(path)
    }

    /// Reads previously stored raw output back (expansion).
    pub fn expand(&self, path: &Path) -> io::Result<String> {
        let full = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        fs::read_to_string(full)
    }
}

/// FNV-1a 64-bit: stable across platforms and toolchain versions, which
/// `DefaultHasher` does not promise.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
