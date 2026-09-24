//! Standard integration registry and launch configuration tests.

use tpt_weave_core::{
    AgentEnvironment, IntegrationError, McpServerConfig, RegistryEntry, RepositoryRegistry,
    SCHEMA_VERSION,
};

#[test]
fn registry_round_trips_and_rejects_duplicates_or_schema_drift() {
    let mut registry = RepositoryRegistry::new();
    registry
        .upsert(RegistryEntry::new("tpt-cv", "../tpt-cv"))
        .expect("add cv");
    registry
        .upsert(RegistryEntry::new("tpt-math", "../tpt-math"))
        .expect("add math");
    let text = registry.to_json().expect("json");
    let back = RepositoryRegistry::from_json(&text).expect("parse");
    assert_eq!(back, registry);
    assert_eq!(
        back.get("tpt-cv").unwrap().path,
        std::path::PathBuf::from("../tpt-cv")
    );
    assert!(matches!(
        RepositoryRegistry::from_json(r#"{"schema":999,"repositories":[]}"#),
        Err(IntegrationError::Schema { found: 999, .. })
    ));

    let mut duplicate = RepositoryRegistry::new();
    duplicate
        .upsert(RegistryEntry::new("tpt-cv", "../one"))
        .unwrap();
    let result = duplicate.upsert(RegistryEntry::new("tpt-cv", "../two"));
    assert!(result.is_ok());
    assert_eq!(duplicate.repositories.len(), 1);
    assert_eq!(
        duplicate.get("tpt-cv").unwrap().path,
        std::path::PathBuf::from("../two")
    );
}

#[test]
fn agent_and_mcp_contracts_use_standard_environment_variables() {
    let agent = AgentEnvironment::new("/repo", "tpt-weave-mcp");
    let env = agent.to_env();
    assert_eq!(env.get("TPT_WEAVE_PATH").map(String::as_str), Some("/repo"));
    assert_eq!(
        env.get("TPT_WEAVE_TOOLS").map(String::as_str),
        Some("orient,symbol,context")
    );
    let mcp = McpServerConfig::new("tpt-weave-mcp", "/repo");
    let value: serde_json::Value = serde_json::from_str(&mcp.to_json().unwrap()).unwrap();
    assert_eq!(value["name"], "tpt-weave");
    assert_eq!(value["env"]["TPT_WEAVE_PATH"], "/repo");
    assert_eq!(SCHEMA_VERSION, 1);
}
