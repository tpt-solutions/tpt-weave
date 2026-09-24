//! Symbol table, module graph and crate/dependency graphs (Phase 3).

mod common;

use common::{SRC, build, key_of};
use tpt_weave_core::RepositoryId;
use tpt_weave_graph::GraphBuilder;
use tpt_weave_index::CargoIndex;
use tpt_weave_rust::FileInput;

#[test]
fn builds_symbol_table_and_module_graph() {
    let graph = build();
    graph.validate().expect("no dangling edges");

    let make = key_of(&graph, "make");
    assert!(graph.symbol(&make).is_some());
    assert!(graph.symbol("does-not-exist").is_none());

    // Module graph: inner (top-level) and outer2::deep (nested).
    let modules = graph.modules_of("demo");
    assert!(
        modules
            .iter()
            .any(|m| m.path == "inner" && m.parent.is_none())
    );
    let deep = modules
        .iter()
        .find(|m| m.path == "outer2::deep")
        .expect("deep module node");
    assert_eq!(deep.parent.as_deref(), Some("outer2"));

    let in_deep = graph.symbols_in_module("demo", "outer2::deep");
    assert_eq!(in_deep.len(), 1);
    assert_eq!(in_deep[0].id.name, "leaf");
    let root_symbols = graph.symbols_in_module("demo", "");
    assert!(root_symbols.iter().any(|s| s.id.name == "make"));

    // Ingesting a second file extends the same table deterministically.
    let repository = RepositoryId::new("demo-repo");
    let input = FileInput {
        repository: &repository,
        package: "demo",
        path: "src/extra.rs",
        module_prefix: &[],
    };
    let extra = tpt_weave_rust::parse_file(&input, "pub fn extra() {}").expect("parses");
    let cargo = tpt_weave_index::CargoIndex::from_metadata_json(common::METADATA).expect("fx");
    let revision = tpt_weave_core::Revision::new("1111111111111111111111111111111111111111");
    let graph2 = tpt_weave_graph::GraphBuilder::new("demo-repo", revision, cargo)
        .add_file(
            "demo",
            tpt_weave_rust::parse_file(&input, SRC).expect("src"),
        )
        .add_file("demo", extra)
        .build();
    assert!(graph2.find_symbols("extra").len() == 1);
    assert!(graph2.find_symbols("make").len() == 1);
}

#[test]
fn incremental_graph_reuses_unchanged_symbols_and_recomputes_edges() {
    let cargo = CargoIndex::from_metadata_json(common::METADATA).expect("fixture");
    let revision = tpt_weave_core::Revision::new("1111111111111111111111111111111111111111");
    let previous = GraphBuilder::new("demo-repo", revision.clone(), cargo.clone())
        .add_file("demo", common::parse("demo", "src/lib.rs", common::SRC))
        .add_file(
            "demo",
            tpt_weave_rust::parse_file(
                &FileInput {
                    repository: &RepositoryId::new("demo-repo"),
                    package: "demo",
                    path: "src/other.rs",
                    module_prefix: &[],
                },
                "pub fn other() { make(); }",
            )
            .expect("other parses"),
        )
        .build();

    let updated_source = "pub fn make() -> u64 { 7 }";
    let changed = tpt_weave_rust::parse_file(
        &FileInput {
            repository: &RepositoryId::new("demo-repo"),
            package: "demo",
            path: "src/lib.rs",
            module_prefix: &[],
        },
        updated_source,
    )
    .expect("updated parses");
    let updated = GraphBuilder::new("demo-repo", revision, cargo)
        .add_file("demo", changed)
        .add_file(
            "demo",
            tpt_weave_rust::parse_file(
                &FileInput {
                    repository: &RepositoryId::new("demo-repo"),
                    package: "demo",
                    path: "src/other.rs",
                    module_prefix: &[],
                },
                "pub fn other() { make(); }",
            )
            .expect("other parses"),
        )
        .build_incremental(&previous, &["src/lib.rs".to_string()]);

    assert!(
        updated
            .find_symbols("make")
            .iter()
            .any(|s| s.signature.contains("u64"))
    );
    assert!(
        updated
            .references
            .iter()
            .any(|edge| { edge.to.ends_with("make#fn") && edge.from.ends_with("other#fn") })
    );
}

#[test]
fn builds_crate_and_dependency_graphs_with_cross_repo_links() {
    let graph = build();

    // Crate graph: only the internal edge resolves inside this graph.
    let internal = graph.crate_dependencies("demo");
    assert_eq!(internal.len(), 1);
    assert_eq!(internal[0].to_package, "tpt-math");
    assert!(internal[0].repository.is_none());

    // Dependency graph: everything declared, cross-repo link attached.
    let all = graph.dependencies_of("demo");
    assert_eq!(all.len(), 3);
    let cv = all
        .iter()
        .find(|e| e.to_package == "tpt-cv")
        .expect("tpt-cv edge");
    assert!(!cv.internal);
    assert_eq!(cv.repository, Some(RepositoryId::new("tpt-cv")));
    let serde = all
        .iter()
        .find(|e| e.to_package == "serde")
        .expect("serde edge");
    assert!(!serde.internal);
    assert!(serde.repository.is_none());

    assert_eq!(graph.linked_repositories(), [RepositoryId::new("tpt-cv")]);
    assert_eq!(graph.crates.len(), 2);
    assert!(
        graph
            .crates
            .iter()
            .any(|c| c.name == "tpt-math" && c.is_workspace_member)
    );
}
