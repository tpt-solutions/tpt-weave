//! Manifest load/save/validate and privacy exclusion behaviour.

use tpt_weave_core::{manifest_path, ConfigError, Manifest, SCHEMA_VERSION};
use std::path::PathBuf;

fn temp_manifest_path(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tpt-weave-{}-{tag}-{nanos}",
        std::process::id()
    ))
    .join("manifest.toml")
}

#[test]
fn default_manifest_renders_and_reparses_identically() {
    let manifest = Manifest::new("tpt-cv");
    let rendered = manifest.render().expect("render");
    assert!(rendered.contains("schema = 1"), "{rendered}");
    assert!(rendered.contains("repository = \"tpt-cv\""), "{rendered}");
    assert!(rendered.contains("[features]"), "{rendered}");
    assert!(rendered.contains("default_budget = 12000"), "{rendered}");
    assert!(rendered.contains("maximum_budget = 32000"), "{rendered}");
    assert!(rendered.contains("remote_decisions = false"), "{rendered}");

    let reparsed: Manifest = toml::from_str(&rendered).expect("reparse");
    assert_eq!(reparsed, manifest);
}

#[test]
fn save_and_load_roundtrip() {
    let path = temp_manifest_path("roundtrip");
    let manifest = Manifest::new("tpt-cv");
    manifest.save(&path).expect("save");
    assert!(path.exists());

    let loaded = Manifest::load(&path).expect("load");
    assert_eq!(loaded, manifest);
    assert_eq!(loaded.schema, SCHEMA_VERSION);

    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn load_rejects_invalid_manifest() {
    let path = temp_manifest_path("invalid");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "schema = 999\nrepository = \"x\"\n").unwrap();

    let error = Manifest::load(&path).expect_err("schema 999 must be rejected");
    assert!(matches!(error, ConfigError::Invalid(_)), "{error:?}");
    assert!(error.to_string().contains("schema"), "{error}");

    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn validation_catches_bad_sections() {
    let mut manifest = Manifest::new("tpt-cv");
    manifest.context.default_budget = 50_000;
    manifest.context.maximum_budget = 1_000;
    assert!(manifest.validate().is_err());

    let mut manifest = Manifest::new("tpt-cv");
    manifest.jev.min_confidence = 1.5;
    assert!(manifest.validate().is_err());

    let mut manifest = Manifest::new("tpt-cv");
    manifest.provider.endpoint = "ftp://example".into();
    assert!(manifest.validate().is_err());

    let manifest = Manifest::new("");
    assert!(manifest.validate().is_err());
}

#[test]
fn privacy_exclusions_match_supported_pattern_forms() {
    let privacy = &Manifest::new("tpt-cv").privacy;

    // Exact segment (`.env`) anywhere in the tree.
    assert!(privacy.is_excluded(".env"));
    assert!(privacy.is_excluded("services/api/.env"));
    // Prefix form (`.env.*`).
    assert!(privacy.is_excluded(".env.local"));
    assert!(privacy.is_excluded("app/config/.env.production"));
    // Directory form (`secrets/`, `credentials/`).
    assert!(privacy.is_excluded("secrets/key.txt"));
    assert!(privacy.is_excluded("crates/core/credentials/token"));
    // Extension form (`*.pem`).
    assert!(privacy.is_excluded("certs/server.pem"));
    // Prefix form (`id_rsa*`).
    assert!(privacy.is_excluded("home/.ssh/id_rsa"));

    // Normal source files stay included.
    assert!(!privacy.is_excluded("src/main.rs"));
    assert!(!privacy.is_excluded("docs/README.md"));
    assert!(!privacy.is_excluded("Cargo.toml"));
}

#[test]
fn manifest_path_layout_matches_spec() {
    let path = manifest_path(std::path::Path::new("/repos/tpt-cv"));
    let text = path.to_string_lossy().replace('\\', "/");
    assert!(text.ends_with("/.tpt-weave/manifest.toml"), "{text}");
}
