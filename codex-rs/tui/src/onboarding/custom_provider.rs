//! Onboarding adapter around the shared third-party provider form.
//!
//! Onboarding cannot await the write inline, so the save runs on a spawned task
//! that writes its result back into the shared `SignInState`. A locally minted
//! token guards against a slow reply clobbering a newer state, mirroring
//! `start_bedrock_setup`.

use codex_app_server_protocol::WriteStatus;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;
use std::sync::PoisonError;
use uuid::Uuid;

use crate::config_update::build_custom_provider_edits;
use crate::config_update::format_config_error;
use crate::config_update::write_config_batch;
use crate::onboarding::auth::AuthModeWidget;
use crate::onboarding::auth::SignInState;
use crate::provider_form::ProviderDraft;
use crate::provider_form::ProviderForm;
use crate::provider_form::ProviderFormAction;
use crate::render::renderable::Renderable;

/// Provider form plus the in-flight save, if any.
#[derive(Clone)]
pub(crate) struct CustomProviderState {
    form: ProviderForm,
    /// Identifies the save this state is waiting on; `None` while editing.
    pending_save: Option<Uuid>,
}

impl CustomProviderState {
    fn new() -> Self {
        Self {
            form: ProviderForm::new(),
            pending_save: None,
        }
    }

    pub(super) fn is_text_entry_active(&self) -> bool {
        self.pending_save.is_none()
    }

    pub(super) fn render(&self, area: Rect, buf: &mut Buffer) {
        if self.pending_save.is_some() {
            let lines: Vec<Line> = vec![
                Line::from(vec!["> ".into(), "正在保存第三方模型提供商…".bold()]),
                "".into(),
            ];
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .render(area, buf);
            return;
        }
        self.form.render(area, buf);
    }
}

impl AuthModeWidget {
    pub(super) fn start_custom_provider_entry(&mut self) {
        *self
            .sign_in_state
            .write()
            .unwrap_or_else(PoisonError::into_inner) =
            SignInState::CustomProvider(CustomProviderState::new());
        *self.error.write().unwrap_or_else(PoisonError::into_inner) = None;
        self.request_frame.schedule_frame();
    }

    pub(super) fn handle_custom_provider_key_event(&mut self, key_event: &KeyEvent) -> bool {
        let action = {
            let mut guard = self
                .sign_in_state
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            let SignInState::CustomProvider(state) = &mut *guard else {
                return false;
            };
            if state.pending_save.is_some() {
                return true;
            }
            *self.error.write().unwrap_or_else(PoisonError::into_inner) = None;
            state.form.handle_key_event(*key_event)
        };

        match action {
            Some(ProviderFormAction::Submitted(draft)) => self.start_custom_provider_write(draft),
            Some(ProviderFormAction::Cancelled) => {
                *self
                    .sign_in_state
                    .write()
                    .unwrap_or_else(PoisonError::into_inner) = SignInState::PickMode;
            }
            None => {}
        }
        self.request_frame.schedule_frame();
        true
    }

    pub(super) fn handle_custom_provider_paste(&mut self, pasted: &str) -> bool {
        let mut guard = self
            .sign_in_state
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        let SignInState::CustomProvider(state) = &mut *guard else {
            return false;
        };
        if state.pending_save.is_some() || !state.form.handle_paste(pasted) {
            return false;
        }
        drop(guard);
        self.request_frame.schedule_frame();
        true
    }

    fn start_custom_provider_write(&mut self, draft: ProviderDraft) {
        let save_id = Uuid::new_v4();
        let fallback = {
            let mut guard = self
                .sign_in_state
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            let SignInState::CustomProvider(state) = &mut *guard else {
                return;
            };
            let fallback = state.form.clone();
            state.pending_save = Some(save_id);
            fallback
        };

        let edits = build_custom_provider_edits(
            &draft.id,
            &draft.name,
            &draft.base_url,
            draft.api_key.as_deref().map(String::as_str),
            &draft.model,
        );
        let request_handle = self.app_server_request_handle.clone();
        let sign_in_state = self.sign_in_state.clone();
        let request_frame = self.request_frame.clone();
        tokio::spawn(async move {
            let result = match write_config_batch(request_handle, edits).await {
                Ok(response) if response.status == WriteStatus::OkOverridden => Err(format!(
                    "提供商已保存但未应用：{}",
                    response
                        .overridden_metadata
                        .as_ref()
                        .map(|metadata| metadata.message.as_str())
                        .unwrap_or("有效配置被更高优先级的配置层覆盖")
                )),
                Ok(_) => Ok(()),
                Err(error) => Err(format!("保存失败：{}", format_config_error(&error))),
            };

            let mut guard = sign_in_state
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            if !matches!(
                &*guard,
                SignInState::CustomProvider(state) if state.pending_save == Some(save_id)
            ) {
                return;
            }
            match result {
                Ok(()) => *guard = SignInState::CustomProviderConfigured,
                Err(message) => {
                    let mut form = fallback;
                    form.set_error(message);
                    *guard = SignInState::CustomProvider(CustomProviderState {
                        form,
                        pending_save: None,
                    });
                }
            }
            drop(guard);
            request_frame.schedule_frame();
        });
        self.request_frame.schedule_frame();
    }
}

#[cfg(test)]
#[path = "custom_provider_tests.rs"]
mod tests;
