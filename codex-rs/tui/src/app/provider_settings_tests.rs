use super::*;
use pretty_assertions::assert_eq;

fn config_with_providers(model_providers: serde_json::Value) -> codex_app_server_protocol::Config {
    serde_json::from_value(serde_json::json!({ "model_providers": model_providers }))
        .expect("config/read payloads keep unknown keys in `additional`")
}

#[test]
fn configured_providers_are_merged_with_the_built_in_ones() {
    let config = config_with_providers(serde_json::json!({
        "my-proxy": {
            "name": "My Proxy",
            "base_url": "https://example.com/v1",
            "wire_api": "responses",
        }
    }));

    let entries = provider_entries(&config).expect("provider table should parse");

    assert!(entries.contains(&ProviderEntry {
        id: "my-proxy".to_string(),
        name: "My Proxy".to_string(),
        base_url: Some("https://example.com/v1".to_string()),
    }));
    assert!(entries.iter().any(|entry| entry.id == "openai"));
}

#[test]
fn a_provider_without_a_name_falls_back_to_its_id() {
    let config = config_with_providers(serde_json::json!({
        "my-proxy": { "base_url": "https://example.com/v1" }
    }));

    let entries = provider_entries(&config).expect("provider table should parse");

    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.id == "my-proxy")
            .map(|entry| entry.name.clone()),
        Some("my-proxy".to_string())
    );
}

#[test]
fn built_in_providers_are_listed_when_none_are_configured() {
    let config = config_with_providers(serde_json::Value::Null);

    let entries = provider_entries(&config).expect("an absent provider table is not an error");

    assert!(entries.iter().any(|entry| entry.id == "openai"));
}
