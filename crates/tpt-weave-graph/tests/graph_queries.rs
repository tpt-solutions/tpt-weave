//! Reference, caller/callee, trait-impl and public API queries (Phase 3).

mod common;

use common::{build, key_of};
use tpt_weave_graph::ReferenceKind;

#[test]
fn resolves_callers_callees_types_and_trait_impls() {
    let graph = build();
    let make = key_of(&graph, "make");
    let consume = key_of(&graph, "consume");
    let helper = key_of(&graph, "helper");
    let pixel = key_of(&graph, "Pixel");
    let draw = key_of(&graph, "Draw");
    let impl_draw = graph
        .impls_of("Pixel")
        .first()
        .expect("impl of Pixel")
        .id
        .canonical_key();

    // Callees: consume -> helper (Call); Pixel is a type, not a callee.
    let callees = graph.callees_of(&consume);
    assert!(callees.iter().any(|s| s.id.canonical_key() == helper));
    assert!(!callees.iter().any(|s| s.id.canonical_key() == pixel));

    // Type relationships: consume and make both reference Pixel.
    assert!(
        graph
            .type_relations_of(&consume)
            .iter()
            .any(|s| s.id.canonical_key() == pixel)
    );
    assert!(
        graph
            .type_relations_of(&make)
            .iter()
            .any(|s| s.id.canonical_key() == pixel)
    );

    // Callers of make: the trait impl method and inner::outer.
    let callers = graph.callers_of(&make);
    assert!(callers.iter().any(|s| s.id.name == "Pixel::draw"));
    assert!(callers.iter().any(|s| s.id.name == "outer"));
    assert_eq!(callers.len(), 2, "deduped callers: {callers:?}");

    // Trait implementations (impl -> trait edge, kind TraitImpl).
    let impls = graph.trait_impls_of(&draw);
    assert_eq!(impls.len(), 1);
    assert_eq!(impls[0].id.canonical_key(), impl_draw);
    assert!(
        graph
            .references_of(&impl_draw)
            .iter()
            .any(|(_, kind)| *kind == ReferenceKind::TraitImpl)
    );
}

#[test]
fn exposes_public_api_surface_and_relationships() {
    let graph = build();
    let api = graph.public_api("demo");
    let names: Vec<&str> = api.iter().map(|s| s.id.name.as_str()).collect();
    assert!(names.contains(&"make"));
    assert!(names.contains(&"Pixel"));
    assert!(names.contains(&"Draw"));
    assert!(!names.contains(&"helper"), "private symbol excluded");

    let edges = graph.public_api_edges("demo");
    assert!(!edges.is_empty());
    for edge in edges {
        let from = graph.symbol(&edge.from).expect("from endpoint");
        let to = graph.symbol(&edge.to).expect("to endpoint");
        assert!(from.visibility.is_public() && to.visibility.is_public());
        assert_eq!(from.id.package, "demo");
    }

    // Unresolved mentions (locals, primitives, `crate`) are counted, not
    // turned into edges.
    assert!(graph.unresolved_mentions > 0);
}
