//! Shared fixture for context tests (compiled into each test binary).

#![allow(dead_code)]

use tpt_weave_context::Sources;
use tpt_weave_core::{RepositoryId, Revision};
use tpt_weave_graph::{GraphBuilder, RepositoryGraph};
use tpt_weave_index::CargoIndex;
use tpt_weave_rust::{FileInput, ParsedFile, parse_file};

/// Two workspace packages: `demo` (with `tests/integration.rs`) depends on
/// internal `tpt-math`, unlinked external `serde`, and linkable external
/// `tpt-cv`.
pub const METADATA: &str = r#"{
    "version": 1,
    "workspace_root": "C:/repos/demo",
    "target_directory": "C:/repos/demo/target",
    "workspace_members": [
        "path+file:///C:/repos/demo#0.1.0",
        "path+file:///C:/repos/tpt-math#0.1.0"
    ],
    "packages": [
        {
            "id": "path+file:///C:/repos/demo#0.1.0",
            "name": "demo",
            "version": "0.1.0",
            "manifest_path": "C:/repos/demo/Cargo.toml",
            "license": null,
            "targets": [],
            "dependencies": [
                {"name": "tpt-math", "req": "*", "kind": null, "rename": null,
                 "optional": false, "uses_default_features": true,
                 "features": [], "target": null},
                {"name": "serde", "req": "^1", "kind": null, "rename": null,
                 "optional": false, "uses_default_features": true,
                 "features": [], "target": null},
                {"name": "tpt-cv", "req": "*", "kind": null, "rename": null,
                 "optional": false, "uses_default_features": true,
                 "features": [], "target": null}
            ],
            "features": {}
        },
        {
            "id": "path+file:///C:/repos/tpt-math#0.1.0",
            "name": "tpt-math",
            "version": "0.1.0",
            "manifest_path": "C:/repos/tpt-math/Cargo.toml",
            "license": null,
            "targets": [],
            "dependencies": [],
            "features": {}
        }
    ]
}"#;

pub const SRC_LIB: &str = r#"pub struct Pixel {
    pub value: u8,
}

pub fn make() -> Pixel {
    Pixel { value: 0 }
}

pub fn consume(p: Pixel) -> u8 {
    helper(p.value)
}

fn helper(v: u8) -> u8 {
    v
}

pub trait Draw {
    fn draw(&self);
}

impl Draw for Pixel {
    fn draw(&self) {
        make();
    }
}

pub mod inner {
    pub fn outer() {
        crate::make();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make() {
        let p = make();
        assert_eq!(p.value, 0);
    }
}
"#;

pub const SRC_TEST: &str = r#"#[test]
fn integration_consume() {
    consume(Pixel { value: 1 });
}
"#;

pub const SRC_MATH: &str = r#"pub fn square(x: i64) -> i64 {
    x * x
}
"#;

pub fn parse(package: &str, path: &str, src: &str) -> ParsedFile {
    let repository = RepositoryId::new("demo-repo");
    let input = FileInput {
        repository: &repository,
        package,
        path,
        module_prefix: &[],
    };
    parse_file(&input, src).expect("parses")
}

pub fn build() -> RepositoryGraph {
    let cargo = CargoIndex::from_metadata_json(METADATA).expect("fixture");
    let revision = Revision::new("1111111111111111111111111111111111111111").with_branch("main");
    GraphBuilder::new("demo-repo", revision, cargo)
        .add_file("demo", parse("demo", "src/lib.rs", SRC_LIB))
        .add_file("demo", parse("demo", "tests/integration.rs", SRC_TEST))
        .add_file(
            "tpt-math",
            parse("tpt-math", "crates/tpt-math/src/lib.rs", SRC_MATH),
        )
        .link_cross_repository("tpt-cv", "tpt-cv")
        .build()
}

pub fn sources() -> Sources {
    Sources::new()
        .with("src/lib.rs", SRC_LIB)
        .with("tests/integration.rs", SRC_TEST)
        .with("crates/tpt-math/src/lib.rs", SRC_MATH)
}

pub fn key_of(graph: &RepositoryGraph, name: &str) -> String {
    graph
        .find_symbols(name)
        .first()
        .unwrap_or_else(|| panic!("missing symbol {name}"))
        .id
        .canonical_key()
}
