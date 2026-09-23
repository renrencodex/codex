//! `/provider` popups: pick a configured model provider or add a third-party one.
//!
//! The provider list comes from the owning app server rather than the local
//! `Config`, because `config/batchWrite` targets the server's `config.toml` and
//! a remote workspace would otherwise show entries it is not writing to.

use super::*;
use crate::bottom_pane::BottomPaneView;
use crate::bottom_pane::SelectionItem;
use crate::bottom_pane::SelectionViewParams;
use crate::bottom_pane::popup_consts::standard_popup_hint_line;
use crate::provider_form::ProviderDraft;
use crate::provider_form::ProviderForm;
use crate::provider_form::ProviderFormAction;

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
    let mut items: Vec<SelectionItem> = providers
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

    items.push(SelectionItem {
        name: "添加第三方提供商…".to_string(),
        description: Some("填写 OpenAI 兼容接口的连接信息".to_string()),
        dismiss_on_select: true,
        actions: vec![Box::new(|tx| tx.send(AppEvent::OpenCustomProviderForm))],
        ..Default::default()
    });

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

/// Hosts [`ProviderForm`] in the bottom pane and submits the completed draft.
pub(crate) struct ProviderFormView {
    form: ProviderForm,
    app_event_tx: AppEventSender,
    complete: bool,
}

impl ProviderFormView {
    pub(crate) fn new(form: ProviderForm, app_event_tx: AppEventSender) -> Self {
        Self {
            form,
            app_event_tx,
            complete: false,
        }
    }
}

impl BottomPaneView for ProviderFormView {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.form.handle_key_event(key_event) {
            Some(ProviderFormAction::Submitted(draft)) => {
                self.app_event_tx
                    .send(AppEvent::PersistCustomProvider { draft });
                self.complete = true;
            }
            Some(ProviderFormAction::Cancelled) => self.complete = true,
            None => {}
        }
    }

    /// Esc closes the form rather than interrupting the turn, so it must reach
    /// `handle_key_event` instead of the cancellation path.
    fn prefer_esc_to_handle_key_event(&self) -> bool {
        true
    }

    fn on_ctrl_c(&mut self) -> CancellationEvent {
        self.complete = true;
        CancellationEvent::Handled
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn handle_paste(&mut self, pasted: String) -> bool {
        self.form.handle_paste(&pasted)
    }
}

impl Renderable for ProviderFormView {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.form.render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.form.desired_height(width)
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

    pub(crate) fn open_custom_provider_form(&mut self) {
        let view = ProviderFormView::new(ProviderForm::new(), self.app_event_tx.clone());
        self.bottom_pane.show_view(Box::new(view));
    }

    pub(crate) fn reopen_custom_provider_form(&mut self, draft: ProviderDraft, error: String) {
        let mut form = ProviderForm::from_draft(draft);
        form.set_error(error);
        let view = ProviderFormView::new(form, self.app_event_tx.clone());
        self.bottom_pane.show_view(Box::new(view));
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
