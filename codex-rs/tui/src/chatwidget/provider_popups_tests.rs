use super::*;
use pretty_assertions::assert_eq;

fn entries() -> Vec<ProviderEntry> {
    vec![
        ProviderEntry {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            base_url: None,
        },
        ProviderEntry {
            id: "my-proxy".to_string(),
            name: "My Proxy".to_string(),
            base_url: Some("https://example.com/v1".to_string()),
        },
    ]
}

#[test]
fn the_picker_lists_providers_with_the_configured_one_selected() {
    let params = build_provider_picker_params(
        entries(),
        /*current_id*/ Some("my-proxy"),
        /*current_model*/ Some("my-model"),
    );

    assert_eq!(
        params
            .items
            .iter()
            .map(|item| (item.name.clone(), item.description.clone(), item.is_current))
            .collect::<Vec<_>>(),
        vec![
            ("OpenAI".to_string(), None, false),
            (
                "My Proxy".to_string(),
                Some("https://example.com/v1".to_string()),
                true
            ),
        ]
    );
    assert_eq!(params.initial_selected_idx, Some(1));
}

#[test]
fn an_unconfigured_current_provider_leaves_the_picker_unselected() {
    let params = build_provider_picker_params(
        entries(),
        /*current_id*/ Some("removed"),
        /*current_model*/ None,
    );

    assert_eq!(params.initial_selected_idx, None);
}
