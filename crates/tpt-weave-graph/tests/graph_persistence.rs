//! Graph persistence and invalidation (todo.md Phase 3).

use tpt_weave_core::{RepositoryId, Revision, SCHEMA_VERSION};
use tpt_weave_graph::{graph_path, GraphBuilder, GraphError, RepositoryGraph};
use tpt_weave_index::CargoIndex;
use tpt_weave_rust::{parse_file, FileInput};

const METADATA: &str = r#"{
    "version": 1,
    "workspace_root": "C:/repos/demo",
    "target_directory": "C:/repos/demo/target",
    "workspace_members": ["path+file:///C:/repos/demo#0.1.0"],
    "packages": [{
        "id": "path+file:///C:/repos/demo#0.1.0",
        "name": "demo",
        "version": "0.1.0",
        "manifest_path": "C:/repos/demo/Cargo.toml",
        "license": null,
        "targets": [],
        "dependencies": [],
        "features": {}
    }]
}"#;

fn build() -> RepositoryGraph {
    let cargo = CargoIndex::from_metadata_json(METADATA).expect("fixture");
    let repository = RepositoryId::new("demo-repo");
    let input = FileInput {
        repository: &repository,
        package: "demo",
        path: "src/lib.rs",
        module_prefix: &[],
    };
    let parsed = parse_file(&input, "pub fn run() -> u8 { 1 }").expect("parses");
    let revision = Revision::new("2222222222222222222222222222222222222222");
    GraphBuilder::new("demo-repo", revision, cargo)
        .add_file("demo", parsed)
        .build()
}

#[test]
fn json_roundtrip_is_lossless_and_deterministic() {
    let graph = build();
    let json = graph.to_json().expect("serialise");
    let back = RepositoryGraph::from_json(&json).expect("parse");
    assert_eq!(back, graph);
    // Deterministic: serialising again yields the same bytes.
    assert_eq!(back.to_json().expect("re-serialise"), json);
}

#[test]
fn rejects_foreign_schema_and_tampered_documents() {
    let graph = build();
    let json = graph.to_json().expect("serialise");
    let bumped = json.replace(
        &format!("\"schema\": {SCHEMA_VERSION}"),
        "\"schema\": 999",
    );
    let err = RepositoryGraph::from_json(&bumped).expect_err("schema 999 rejected");
    assert!(matches!(err, GraphError::SchemaMismatch { found: 999, .. }), "{err:?}");

    assert!(RepositoryGraph::from_json("{ not json").is_err());
}

#[test]
fn invalidates_on_schema_or_revision_change() {
    let graph = build();
    assert!(!graph.is_stale(&Revision::new("2222222222222222222222222222222222222222")));

    // New revision => stale (source changed since the graph was built).
    let changed = Revision::new("3333333333333333333333333333333333333333");
    assert!(graph.is_stale(&changed));

    // Schema drift => stale regardless of revision.
    let mut schema_drift = graph.clone();
    schema_drift.schema = SCHEMA_VERSION + 1;
    assert!(schema_drift.is_stale(&Revision::new(
        "2222222222222222222222222222222222222222"
    )));
}

#[test]
fn saves_and_loads_through_the_tpt_weave_directory() {
    let graph = build();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "tpt-weave-graph-{}-{nanos}",
        std::process::id()
    ));
    let path = graph_path(&dir);
    assert!(path.to_string_lossy().replace('\\', "/").ends_with("/.tpt-weave/graph.json"));

    graph.save(&path).expect("save");
    let loaded = RepositoryGraph::load(&path).expect("load");
    assert_eq!(loaded, graph);

    std::fs::remove_dir_all(&dir).ok();
}
