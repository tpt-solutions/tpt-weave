//! Lazy file-source access (todo.md Phase 15 "Reduce allocations" /
//! "Profile CPU"): `LazySources` discovers `*.rs` paths without reading and
//! defers each read until first use.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use tpt_weave_context::{LazySources, SourceProvider, Sources};
use tpt_weave_core::PrivacyConfig;

static FIXTURE_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Creates a unique temporary directory tree exercising the directory-walk
/// exclusions. Returns `(root, tracked relative path)`.
fn fixture() -> PathBuf {
    let n = FIXTURE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "tpt-weave-sources-test-{}-{}-{}",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(dir.join("src")).expect("mkdir");
    fs::create_dir_all(dir.join("target/debug")).expect("mkdir target");
    fs::create_dir_all(dir.join(".git/objects")).expect("mkdir git");
    fs::create_dir_all(dir.join(".tpt-weave")).expect("mkdir weave");
    fs::create_dir_all(dir.join("secrets")).expect("mkdir secrets");

    fs::write(dir.join("src/lib.rs"), "pub fn tracked() {}\n").expect("lib.rs");
    fs::write(dir.join("src/other.rs"), "pub fn other() {}\n").expect("other.rs");
    fs::write(dir.join("notes.txt"), "not rust\n").expect("notes.txt");
    fs::write(dir.join("target/ignored.rs"), "fn ignored() {}\n").expect("target");
    fs::write(dir.join(".git/ignored.rs"), "fn ignored() {}\n").expect("git");
    fs::write(dir.join(".tpt-weave/ignored.rs"), "fn ignored() {}\n").expect("weave");
    fs::write(dir.join("secrets/hidden.rs"), "fn hidden() {}\n").expect("secrets");
    dir
}

fn relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .expect("under root")
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn discovers_rust_files_with_the_same_exclusions_as_sources() {
    let root = fixture();
    let eager = Sources::load_dir(&root).expect("eager load");
    let lazy = LazySources::open(&root).expect("lazy open");

    // Identical file sets: target/, .git/ and .tpt-weave/ excluded, non-`.rs`
    // files excluded, contents equal for every discovered path.
    let eager_paths: Vec<&str> = vec!["src/lib.rs", "src/other.rs"];
    for path in eager_paths {
        assert_eq!(
            lazy.source(path),
            eager.source(path),
            "content mismatch for {path}"
        );
    }
    assert_eq!(lazy.source("notes.txt"), None);
    assert_eq!(lazy.source("target/ignored.rs"), None);
    assert_eq!(lazy.source(".git/ignored.rs"), None);
    assert_eq!(lazy.source(".tpt-weave/ignored.rs"), None);
    assert_eq!(eager.source("secrets/hidden.rs"), None);
    assert_eq!(lazy.source("secrets/hidden.rs"), None);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn reads_are_deferred_until_first_request_then_cached() {
    let root = fixture();
    let lib = root.join("src/lib.rs");
    let tracked = relative(&lib, &root);

    let lazy = LazySources::open(&root).expect("lazy open");
    // Nothing read yet: the file changed on disk after `open`, and the first
    // request observes the new content (proof the read was deferred).
    fs::write(&lib, "pub fn updated() {}\n").expect("rewrite");
    assert_eq!(lazy.source(&tracked), Some("pub fn updated() {}\n"));

    // Cached thereafter: a later on-disk change is invisible.
    fs::write(&lib, "pub fn third() {}\n").expect("rewrite");
    assert_eq!(lazy.source(&tracked), Some("pub fn updated() {}\n"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn private_absolute_paths_are_excluded_before_discovery() {
    let root = fixture();
    let mut privacy = PrivacyConfig::default();
    privacy
        .private_paths
        .push(root.to_string_lossy().into_owned());
    let lazy = LazySources::open_with_privacy(&root, &privacy).expect("lazy open");
    assert_eq!(lazy.source("src/lib.rs"), None);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn missing_and_deleted_files_return_none() {
    let root = fixture();
    let lazy = LazySources::open(&root).expect("lazy open");

    assert_eq!(lazy.source("src/never-existed.rs"), None);

    // Deleted after discovery but *before* the first read: discovery knew the
    // path, the deferred read fails, and the failure is cached (no retry).
    let other = relative(&root.join("src/other.rs"), &root);
    fs::remove_file(root.join("src/other.rs")).expect("remove");
    assert_eq!(lazy.source(&other), None);
    assert_eq!(lazy.source(&other), None);
    let _ = fs::remove_dir_all(&root);
}
