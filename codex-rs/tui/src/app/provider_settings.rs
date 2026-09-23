//! Reads and writes the model provider through the owning app server.
//!
//! The provider table is only exposed to `config/read` as untyped JSON under
//! `additional`, and every write has to go out as one batch so `model_provider`
//! never names a provider that `Config::load` cannot resolve.

use super::*;
use crate::chatwidget::ProviderEntry;
use codex_app_server_protocol::WriteStatus;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::built_in_model_providers;
use codex_model_provider_info::merge_configured_model_providers;
use std::collections::HashMap;

/// Built-in providers are compiled in rather than stored in `config.toml`, so
/// they have to be merged back in for the picker to offer a way home.
fn provider_entries(config: &codex_app_server_protocol::Config) -> Result<Vec<ProviderEntry>> {
    let configured = match config.additional.get("model_providers") {
        None | Some(serde_json::Value::Null) => HashMap::new(),
        Some(value) => serde_json::from_value::<HashMap<String, ModelProviderInfo>>(value.clone())?,
    };
    let providers = merge_configured_model_providers(
        built_in_model_providers(/*openai_base_url*/ None),
        configured,
    )
    .map_err(|message| color_eyre::eyre::eyre!(message))?;

    let mut entries: Vec<ProviderEntry> = providers
        .into_iter()
        .map(|(id, provider)| ProviderEntry {
            name: if provider.name.is_empty() {
                id.clone()
            } else {
                provider.name
            },
            base_url: provider.base_url,
            id,
        })
        .collect();
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(entries)
}

impl App {
    /// The selected provider and model come from the same `config/read` as the
    /// list, not from the local `Config`, so the marked entry always describes
    /// the file the picker is about to write.
    pub(super) async fn open_provider_picker(&mut self, app_server: &AppServerSession) {
        let response = crate::config_update::read_effective_config_if_supported(
            app_server.request_handle(),
            &self.chat_widget.config_ref().cwd,
        )
        .await;
        let picker = response.and_then(|config| {
            let config = config.ok_or_else(|| {
                color_eyre::eyre::eyre!("this server cannot report its configured providers")
            })?;
            let entries = provider_entries(&config)?;
            Ok((entries, config.model_provider, config.model))
        });
        match picker {
            Ok((entries, current_id, current_model)) => {
                self.chat_widget
                    .open_provider_picker(entries, current_id, current_model);
            }
            Err(error) => self
                .chat_widget
                .add_error_message(format!("读取模型提供商失败：{error}")),
        }
    }

    pub(super) async fn persist_provider_selection(
        &mut self,
        app_server: &mut AppServerSession,
        provider_id: String,
        model: String,
    ) {
        let edits = crate::config_update::build_provider_selection_edits(&provider_id, &model);
        let response = match crate::config_update::write_config_batch(
            app_server.request_handle(),
            edits,
        )
        .await
        {
            Ok(response) => response,
            Err(error) => {
                self.chat_widget.add_error_message(format!(
                    "保存模型提供商失败：{}",
                    crate::config_update::format_config_error(&error)
                ));
                return;
            }
        };

        if response.status == WriteStatus::OkOverridden {
            let message = super::config_persistence::overridden_write_message(&response);
            tracing::warn!(
                message,
                "model provider config write was overridden by effective config"
            );
            self.chat_widget
                .add_error_message(format!("模型提供商更改已保存但未应用：{message}"));
            if let Some(effective) = self
                .read_effective_config_after_overridden_write(app_server, "模型提供商更改")
                .await
                && let Some(provider) = effective.config.model_provider
            {
                self.chat_widget.add_info_message(
                    format!("当前生效的模型提供商为 {provider}。"),
                    /*hint*/ None,
                );
            }
            return;
        }

        self.chat_widget.add_info_message(
            format!("模型提供商已设置为 {provider_id}，模型为 {model}。"),
            Some("使用 /new 开始新会话后生效".to_string()),
        );
    }
}

#[cfg(test)]
#[path = "provider_settings_tests.rs"]
mod tests;
