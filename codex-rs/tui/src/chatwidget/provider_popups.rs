//! `/provider` popups: pick a configured model provider or add a third-party one.
//!
//! The provider list comes from the owning app server rather than the local
//! `Config`, because `config/batchWrite` targets the server's `config.toml` and
//! a remote workspace would otherwise show entries it is not writing to.

use super::*;
use crate::bottom_pane::SelectionItem;
use crate::bottom_pane::SelectionViewParams;
use crate::bottom_pane::popup_consts::standard_popup_hint_line;

/// A provider the user can switch to, as reported by `config/read`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProviderEntry {
    pub id: String,
    pub name: String,
    pub base_url: Option<String>,
}

pub(crate) fn build_provider_picker_params(
    providers: Vec<ProviderEntry>,
    current_id: Option<&str>,
    current_model: Option<&str>,
) -> SelectionViewParams {
    let mut initial_selected_idx = None;
    let items: Vec<SelectionItem> = providers
        .into_iter()
        .enumerate()
        .map(|(idx, provider)| {
            let is_current = current_id == Some(provider.id.as_str());
            if is_current {
                initial_selected_idx = Some(idx);
            }
            let model = current_model.unwrap_or_default().to_string();
            SelectionItem {
                name: provider.name.clone(),
                description: provider.base_url.clone(),
                is_current,
                dismiss_on_select: true,
                search_value: Some(provider.id.clone()),
                actions: vec![Box::new(move |tx| {
                    tx.send(AppEvent::OpenProviderModelPrompt {
                        provider_id: provider.id.clone(),
                        provider_name: provider.name.clone(),
                        model: model.clone(),
                    });
                })],
                ..Default::default()
            }
        })
        .collect();

    SelectionViewParams {
        title: Some("选择模型提供商".to_string()),
        subtitle: Some("更改将在下次会话开始时生效".to_string()),
        footer_hint: Some(standard_popup_hint_line()),
        items,
        is_searchable: true,
        search_placeholder: Some("输入文字以筛选提供商…".to_string()),
        initial_selected_idx,
        ..Default::default()
    }
}

impl ChatWidget {
    pub(crate) fn open_provider_picker(
        &mut self,
        providers: Vec<ProviderEntry>,
        current_id: Option<String>,
        current_model: Option<String>,
    ) {
        let params = build_provider_picker_params(
            providers,
            current_id.as_deref(),
            current_model.as_deref(),
        );
        self.bottom_pane.show_selection_view(params);
    }

    pub(crate) fn open_provider_model_prompt(
        &mut self,
        provider_id: String,
        provider_name: String,
        model: String,
    ) {
        let tx = self.app_event_tx.clone();
        // `model` is a single global config key rather than a per-provider one,
        // so switching providers has to restate it.
        let view = CustomPromptView::new(
            format!("{provider_name} 使用的模型"),
            "输入模型名并按 Enter".to_string(),
            model,
            /*context_label*/ None,
            Box::new(move |model: String| {
                let model = model.trim().to_string();
                if model.is_empty() {
                    return;
                }
                tx.send(AppEvent::PersistProviderSelection {
                    provider_id: provider_id.clone(),
                    model,
                });
            }),
        );
        self.bottom_pane.show_text_prompt(view);
    }
}

#[cfg(test)]
#[path = "provider_popups_tests.rs"]
mod tests;
