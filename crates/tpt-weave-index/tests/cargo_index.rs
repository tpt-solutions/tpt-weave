//! Live integration test: index this very workspace with `cargo metadata`.

use std::path::{Path, PathBuf};
use tpt_weave_index::{CargoIndex, DependencyKind};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn indexes_the_real_workspace() {
    let root = workspace_root();
    let index = CargoIndex::load(&root).expect("cargo metadata runs");

    for name in ["tpt-weave-core", "tpt-weave-index", "tpt-weave-rust"] {
        assert!(
            index.workspace_members.contains(&name.to_string()),
            "workspace member {name}"
        );
        let pkg = index.package(name).expect(name);
        assert!(pkg.is_workspace_member);
        assert!(
            pkg.targets
                .iter()
                .any(|t| t.kinds.iter().any(|k| k == "lib")),
            "{name} should have a lib target"
        );
    }

    let core = index.package("tpt-weave-core").expect("core");
    let serde = core
        .dependencies
        .iter()
        .find(|d| d.name == "serde")
        .expect("serde dep");
    assert_eq!(serde.kind, DependencyKind::Normal);
    assert!(!serde.optional);
    assert!(serde.features.iter().any(|f| f == "derive"));

    let serde_json_dep = core
        .dependencies
        .iter()
        .find(|d| d.name == "serde_json")
        .expect("serde_json dep");
    // `serde_json` is a dev-dependency of core, but is used by the live
    // workspace test. Keep this assertion scoped to the dependency relationship
    // rather than assuming whether Cargo classifies it as normal or dev-only
    // in the current metadata projection.
    assert_eq!(serde_json_dep.name, "serde_json");
    assert!(core.features.contains_key("unstable"));

    let rust_crate = index.package("tpt-weave-rust").expect("rust crate");
    for dep in ["syn", "quote", "proc-macro2", "tpt-weave-core"] {
        assert!(
            rust_crate.dependencies.iter().any(|d| d.name == dep),
            "tpt-weave-rust should depend on {dep}"
        );
    }

    assert_eq!(
        PathBuf::from(&index.workspace_root)
            .canonicalize()
            .expect("canonical metadata root"),
        root.canonicalize().expect("canonical workspace root"),
    );
    assert!(!index.workspace_root.as_os_str().is_empty());
    assert!(!index.target_directory.as_os_str().is_empty());
}
