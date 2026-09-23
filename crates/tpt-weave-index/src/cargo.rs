//! Cargo metadata integration (spec.md section 8.1, todo.md Phase 2 "Cargo").
//!
//! Runs `cargo metadata --no-deps` and projects format-version 1 into a
//! stable, serialisable model. The raw field names were captured from a
//! real invocation (verified against this workspace).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Errors from loading or interpreting `cargo metadata`.
#[derive(Debug)]
pub enum IndexError {
    /// Failed to spawn or run `cargo`.
    Io(std::io::Error),
    /// `cargo metadata` exited non-zero (e.g. no `Cargo.toml`).
    CommandFailed {
        status: Option<i32>,
        stderr: String,
    },
    /// The JSON could not be parsed.
    Parse(serde_json::Error),
    /// The metadata document is a format this build does not understand.
    UnsupportedFormat(u32),
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::Io(err) => write!(f, "failed to run cargo metadata: {err}"),
            IndexError::CommandFailed { status, stderr } => {
                write!(f, "cargo metadata failed (status {status:?}): {stderr}")
            }
            IndexError::Parse(err) => write!(f, "invalid cargo metadata json: {err}"),
            IndexError::UnsupportedFormat(version) => write!(
                f,
                "unsupported cargo metadata format version {version} (expected 1)"
            ),
        }
    }
}

impl std::error::Error for IndexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IndexError::Io(err) => Some(err),
            IndexError::Parse(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for IndexError {
    fn from(err: std::io::Error) -> Self {
        IndexError::Io(err)
    }
}

impl From<serde_json::Error> for IndexError {
    fn from(err: serde_json::Error) -> Self {
        IndexError::Parse(err)
    }
}

// --- raw `cargo metadata` JSON shape (format-version 1) --------------------

#[derive(Debug, Deserialize)]
struct RawMetadata {
    version: u32,
    #[serde(default)]
    packages: Vec<RawPackage>,
    #[serde(default)]
    workspace_members: Vec<String>,
    workspace_root: String,
    target_directory: String,
}

#[derive(Debug, Deserialize)]
struct RawPackage {
    id: String,
    name: String,
    version: String,
    manifest_path: String,
    license: Option<String>,
    #[serde(default)]
    targets: Vec<RawTarget>,
    #[serde(default)]
    dependencies: Vec<RawDependency>,
    #[serde(default)]
    features: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RawTarget {
    name: String,
    #[serde(default)]
    kind: Vec<String>,
    #[serde(default)]
    crate_types: Vec<String>,
    src_path: String,
    edition: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawDependency {
    name: String,
    req: String,
    kind: Option<String>,
    rename: Option<String>,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    uses_default_features: bool,
    #[serde(default)]
    features: Vec<String>,
    target: Option<String>,
}

/// Kind of dependency edge (matches `cargo metadata`'s `kind` field).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    /// Normal `[dependencies]` entry.
    Normal,
    /// `[dev-dependencies]` entry.
    Development,
    /// `[build-dependencies]` entry.
    Build,
}

/// One declared dependency of a package.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyIndex {
    /// Actual package name.
    pub name: String,
    /// Name the dependency is renamed to in code, when `rename`/`package`
    /// is used.
    pub rename: Option<String>,
    /// Version requirement as written, e.g. `^1`.
    pub req: String,
    pub kind: DependencyKind,
    /// `true` when the dependency is feature-gated (`optional = true`).
    pub optional: bool,
    pub uses_default_features: bool,
    pub features: Vec<String>,
    /// `cfg(...)` expression this dependency applies to, when
    /// target-specific.
    pub target: Option<String>,
}

impl DependencyIndex {
    /// Name as written in code (`rename` when set, otherwise the package
    /// name).
    pub fn code_name(&self) -> &str {
        self.rename.as_deref().unwrap_or(&self.name)
    }
}

/// One build target (lib/bin/test/example/bench/build script) of a package.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetIndex {
    pub name: String,
    /// Target kinds as reported by cargo (`lib`, `bin`, `test`, `example`,
    /// `bench`, `custom-build`, ...).
    pub kinds: Vec<String>,
    pub crate_types: Vec<String>,
    pub src_path: PathBuf,
    pub edition: Option<String>,
}

/// One indexed package (crate).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageIndex {
    /// Opaque cargo package id (useful for cache keys, spec.md section 15).
    pub id: String,
    pub name: String,
    pub version: String,
    pub manifest_path: PathBuf,
    pub license: Option<String>,
    pub is_workspace_member: bool,
    pub targets: Vec<TargetIndex>,
    pub dependencies: Vec<DependencyIndex>,
    /// Feature name to the features it enables (includes `dep:` and
    /// `crate/feature` entries verbatim).
    pub features: BTreeMap<String, Vec<String>>,
}

impl PackageIndex {
    /// Dependencies declared as `optional = true`.
    pub fn optional_dependencies(&self) -> impl Iterator<Item = &DependencyIndex> {
        self.dependencies.iter().filter(|dep| dep.optional)
    }

    /// Dependencies of the given kind.
    pub fn dependencies_of_kind(
        &self,
        kind: DependencyKind,
    ) -> impl Iterator<Item = &DependencyIndex> {
        self.dependencies.iter().filter(move |dep| dep.kind == kind)
    }
}

/// Deterministic projection of `cargo metadata` (spec.md section 8.1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CargoIndex {
    pub workspace_root: PathBuf,
    pub target_directory: PathBuf,
    /// Names of workspace-member packages (sorted, deduplicated).
    pub workspace_members: Vec<String>,
    /// All packages reported by the metadata document.
    pub packages: Vec<PackageIndex>,
}

impl CargoIndex {
    /// Runs `cargo metadata --no-deps` with `project_dir` as working
    /// directory and projects the result.
    pub fn load(project_dir: impl AsRef<Path>) -> Result<Self, IndexError> {
        let output = Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--no-deps"])
            .current_dir(project_dir.as_ref())
            .output()?;
        if !output.status.success() {
            return Err(IndexError::CommandFailed {
                status: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Self::from_metadata_json(&String::from_utf8_lossy(&output.stdout))
    }

    /// Parses a `cargo metadata --format-version 1` document.
    pub fn from_metadata_json(json: &str) -> Result<Self, IndexError> {
        let raw: RawMetadata = serde_json::from_str(json)?;
        if raw.version != 1 {
            return Err(IndexError::UnsupportedFormat(raw.version));
        }
        let member_ids: HashSet<&str> =
            raw.workspace_members.iter().map(String::as_str).collect();

        let mut workspace_members: Vec<String> = Vec::new();
        let mut packages = Vec::new();
        for pkg in raw.packages {
            let is_member = member_ids.contains(pkg.id.as_str());
            if is_member {
                workspace_members.push(pkg.name.clone());
            }
            packages.push(PackageIndex {
                id: pkg.id,
                name: pkg.name,
                version: pkg.version,
                manifest_path: PathBuf::from(pkg.manifest_path),
                license: pkg.license,
                is_workspace_member: is_member,
                targets: pkg
                    .targets
                    .into_iter()
                    .map(|target| TargetIndex {
                        name: target.name,
                        kinds: target.kind,
                        crate_types: target.crate_types,
                        src_path: PathBuf::from(target.src_path),
                        edition: target.edition,
                    })
                    .collect(),
                dependencies: pkg
                    .dependencies
                    .into_iter()
                    .map(|dep| DependencyIndex {
                        name: dep.name,
                        rename: dep.rename,
                        req: dep.req,
                        kind: match dep.kind.as_deref() {
                            Some("dev") => DependencyKind::Development,
                            Some("build") => DependencyKind::Build,
                            _ => DependencyKind::Normal,
                        },
                        optional: dep.optional,
                        uses_default_features: dep.uses_default_features,
                        features: dep.features,
                        target: dep.target,
                    })
                    .collect(),
                features: pkg.features,
            });
        }
        workspace_members.sort();
        workspace_members.dedup();
        Ok(Self {
            workspace_root: PathBuf::from(raw.workspace_root),
            target_directory: PathBuf::from(raw.target_directory),
            workspace_members,
            packages,
        })
    }

    /// Looks up a package by name.
    pub fn package(&self, name: &str) -> Option<&PackageIndex> {
        self.packages.iter().find(|pkg| pkg.name == name)
    }

    /// Iterates workspace-member packages.
    pub fn workspace_packages(&self) -> impl Iterator<Item = &PackageIndex> {
        self.packages.iter().filter(|pkg| pkg.is_workspace_member)
    }

    /// Every optional (`optional = true`) dependency with its package
    /// (todo.md Phase 2: "Detect optional dependencies").
    pub fn optional_dependencies(
        &self,
    ) -> impl Iterator<Item = (&PackageIndex, &DependencyIndex)> {
        self.packages
            .iter()
            .flat_map(|pkg| pkg.optional_dependencies().map(move |dep| (pkg, dep)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "version": 1,
        "workspace_root": "C:/repos/demo",
        "target_directory": "C:/repos/demo/target",
        "workspace_members": ["path+file:///C:/repos/demo#0.1.0"],
        "packages": [{
            "id": "path+file:///C:/repos/demo#0.1.0",
            "name": "demo",
            "version": "0.1.0",
            "manifest_path": "C:/repos/demo/Cargo.toml",
            "license": "MIT",
            "targets": [{
                "name": "demo",
                "kind": ["lib"],
                "crate_types": ["lib"],
                "src_path": "C:/repos/demo/src/lib.rs",
                "edition": "2024"
            }],
            "dependencies": [
                {"name": "serde", "req": "^1", "kind": null, "rename": null,
                 "optional": true, "uses_default_features": false,
                 "features": ["derive"], "target": null},
                {"name": "criterion", "req": "^0.5", "kind": "dev", "rename": null,
                 "optional": false, "uses_default_features": true,
                 "features": [], "target": null},
                {"name": "cc", "req": "^1", "kind": "build", "rename": "compiler",
                 "optional": false, "uses_default_features": true,
                 "features": [], "target": "cfg(unix)"}
            ],
            "features": {"default": ["std"], "std": ["serde"], "gpu": ["dep:vk"]}
        }]
    }"#;

    #[test]
    fn parses_fixture_fields() {
        let index = CargoIndex::from_metadata_json(FIXTURE).expect("fixture parses");
        assert_eq!(index.workspace_members, ["demo"]);
        assert_eq!(index.workspace_root, PathBuf::from("C:/repos/demo"));

        let pkg = index.package("demo").expect("demo package");
        assert!(pkg.is_workspace_member);
        assert_eq!(pkg.targets[0].kinds, ["lib"]);
        assert_eq!(pkg.targets[0].edition.as_deref(), Some("2024"));

        // Optional / dev / build / renamed / target-specific detection.
        let optional: Vec<&DependencyIndex> = pkg.optional_dependencies().collect();
        assert_eq!(optional.len(), 1);
        assert_eq!(optional[0].name, "serde");
        assert_eq!(index.optional_dependencies().count(), 1);

        let dev = pkg
            .dependencies_of_kind(DependencyKind::Development)
            .next()
            .expect("dev dep");
        assert_eq!(dev.name, "criterion");
        let build = pkg
            .dependencies_of_kind(DependencyKind::Build)
            .next()
            .expect("build dep");
        assert_eq!(build.rename.as_deref(), Some("compiler"));
        assert_eq!(build.code_name(), "compiler");
        assert_eq!(build.target.as_deref(), Some("cfg(unix)"));

        // Feature map preserved verbatim.
        assert_eq!(pkg.features["std"], ["serde"]);
        assert_eq!(pkg.features["gpu"], ["dep:vk"]);
    }

    #[test]
    fn rejects_wrong_format_version_and_bad_json() {
        let err = CargoIndex::from_metadata_json(
            r#"{"version": 2, "workspace_root": "", "target_directory": ""}"#,
        )
        .expect_err("version 2 rejected");
        assert!(matches!(err, IndexError::UnsupportedFormat(2)));

        let err = CargoIndex::from_metadata_json("{not json").expect_err("bad json");
        assert!(matches!(err, IndexError::Parse(_)));
    }
}



