//! Model, collaboration, and reasoning popups for `ChatWidget`.
//!
//! These surfaces are tightly related because changing one often redirects
//! into another, especially while Plan mode is active.

use super::*;
use crate::model_catalog::LUNA_RESERVE_MODEL;

const ULTRA_REASONING_CONCURRENCY_WARNING_THRESHOLD: usize = 8;
pub(super) const MODEL_SELECTION_VIEW_ID: &str = "model-selection";
pub(super) const ALL_MODELS_SELECTION_VIEW_ID: &str = "all-models-selection";

impl ChatWidget {
    /// Open a popup to choose a quick auto model. Selecting "All models"
    /// opens the full picker with every available preset.
    pub(crate) fn open_model_popup(&mut self) {
        if !self.is_session_configured() {
            self.add_info_message("启动完成前无法选择模型。".to_string(), /*hint*/ None);
            return;
        }

        let presets: Vec<ModelPreset> = match self.model_catalog.try_list_models() {
            Ok(models) => models,
            Err(_) => {
                self.add_info_message(
                    "模型正在更新，请稍后再次尝试 /model。".to_string(),
                    /*hint*/ None,
                );
                return;
            }
        };
        let request_id = uuid::Uuid::new_v4();
        self.model_popup_request_id = Some(request_id);
        self.open_model_popup_with_presets(presets);
        // Show cached choices immediately and update any still-present picker when the reply arrives.
        self.app_event_tx.send(AppEvent::FetchModels { request_id });
    }

    pub(super) fn model_menu_header(&self, title: &str, subtitle: &str) -> Box<dyn Renderable> {
        let title = title.to_string();
        let subtitle = subtitle.to_string();
        let mut header = vec![Line::from(title.bold())];
        if !subtitle.is_empty() {
            header.push(Line::from(subtitle.dim()));
        }
        if let Some(warning) = self.model_menu_warning_line() {
            header.push(warning);
        }
        Box::new(Paragraph::new(header).wrap(Wrap { trim: false }))
    }

    fn model_menu_warning_line(&self) -> Option<Line<'static>> {
        let base_url = self.custom_openai_base_url()?;
        let warning = format!(
            "警告：OpenAI 基础 URL 已覆盖为 {base_url}。模型选择可能不受支持或无法正常工作。"
        );
        Some(Line::from(warning.red()))
    }

    fn custom_openai_base_url(&self) -> Option<String> {
        if !self.config.model_provider.is_openai() {
            return None;
        }

        let base_url = self.config.model_provider.base_url.as_ref()?;
        let trimmed = base_url.trim();
        if trimmed.is_empty() {
            return None;
        }

        let normalized = trimmed.trim_end_matches('/');
        if normalized == DEFAULT_OPENAI_BASE_URL {
            return None;
        }

        Some(trimmed.to_string())
    }

    pub(crate) fn open_model_popup_with_presets(&mut self, presets: Vec<ModelPreset>) {
        if self.restrict_model_picker_to_luna_reserve() {
            self.open_luna_reserve_model_popup(presets, MODEL_SELECTION_VIEW_ID);
            return;
        }
        let presets: Vec<ModelPreset> = presets
            .into_iter()
            .filter(|preset| preset.show_in_picker)
            .collect();

        let current_model = self.current_model();
        let current_label = presets
            .iter()
            .find(|preset| preset.model.as_str() == current_model)
            .map(|preset| preset.display_name.clone())
            .unwrap_or_else(|| self.model_display_name().to_string());

        let (mut auto_presets, other_presets): (Vec<ModelPreset>, Vec<ModelPreset>) = presets
            .into_iter()
            .partition(|preset| Self::is_auto_model(&preset.model));

        if auto_presets.is_empty() {
            self.open_all_models_popup_with_view_id(other_presets, MODEL_SELECTION_VIEW_ID);
            return;
        }

        auto_presets.sort_by_key(|preset| Self::auto_model_order(&preset.model));
        let mut model_ids: Vec<String> = auto_presets
            .iter()
            .map(|preset| preset.model.clone())
            .collect();
        let mut items: Vec<SelectionItem> = auto_presets
            .into_iter()
            .map(|preset| {
                let description =
                    (!preset.description.is_empty()).then_some(preset.description.clone());
                let model = preset.model.clone();
                let requires_advanced_selection =
                    Self::is_advanced_reasoning_effort(&preset.default_reasoning_effort)
                        || preset
                            .supported_reasoning_efforts
                            .iter()
                            .any(|option| Self::is_advanced_reasoning_effort(&option.effort));
                let actions: Vec<SelectionAction> = if requires_advanced_selection {
                    let preset_for_action = preset.clone();
                    vec![Box::new(move |tx| {
                        tx.send(AppEvent::OpenReasoningPopup {
                            model: preset_for_action.clone(),
                        });
                    })]
                } else {
                    let should_prompt_plan_mode_scope = self
                        .should_prompt_plan_mode_reasoning_scope(
                            model.as_str(),
                            Some(preset.default_reasoning_effort.clone()),
                        );
                    self.model_selection_actions(
                        model.clone(),
                        Some(preset.default_reasoning_effort.clone()),
                        should_prompt_plan_mode_scope,
                    )
                };
                SelectionItem {
                    name: preset.display_name.clone(),
                    description,
                    is_current: model.as_str() == current_model,
                    is_default: preset.is_default,
                    secondary_action: if requires_advanced_selection {
                        None
                    } else {
                        self.session_model_selection_action(
                            model.clone(),
                            Some(preset.default_reasoning_effort),
                        )
                    },
                    actions,
                    dismiss_on_select: !requires_advanced_selection,
                    dismiss_parent_on_child_accept: requires_advanced_selection,
                    ..Default::default()
                }
            })
            .collect();

        if !other_presets.is_empty() {
            model_ids.push("所有模型".to_string());
            let actions: Vec<SelectionAction> = vec![Box::new(|tx| {
                tx.send(AppEvent::OpenAllModelsPopup);
            })];

            let is_current = !items.iter().any(|item| item.is_current);
            let description = Some(format!("选择特定模型和推理级别（当前：{current_label}）"));

            items.push(SelectionItem {
                name: "所有模型".to_string(),
                description,
                is_current,
                actions,
                dismiss_on_select: true,
                ..Default::default()
            });
        }

        let header = self.model_menu_header("选择模型", "选择快速自动模式或浏览所有模型。");
        self.show_model_selection_view(
            model_ids,
            SelectionViewParams {
                view_id: Some(MODEL_SELECTION_VIEW_ID),
                items,
                header,
                ..SelectionViewParams::picker()
            },
        );
    }

    pub(super) fn is_auto_model(model: &str) -> bool {
        model.starts_with("codex-auto-")
    }

    fn auto_model_order(model: &str) -> usize {
        match model {
            "codex-auto-fast" => 0,
            "codex-auto-balanced" => 1,
            "codex-auto-thorough" => 2,
            _ => 3,
        }
    }

    pub(crate) fn open_all_models_popup(&mut self) {
        if self.restrict_model_picker_to_luna_reserve() {
            self.open_luna_reserve_model_popup(
                self.model_catalog.try_list_models().unwrap_or_default(),
                ALL_MODELS_SELECTION_VIEW_ID,
            );
            return;
        }
        let presets = self
            .model_catalog
            .try_list_models()
            .unwrap_or_default()
            .into_iter()
            .filter(|preset| preset.show_in_picker && !Self::is_auto_model(&preset.model))
            .collect();
        self.open_all_models_popup_with_view_id(presets, ALL_MODELS_SELECTION_VIEW_ID);
    }

    fn open_all_models_popup_with_view_id(
        &mut self,
        presets: Vec<ModelPreset>,
        view_id: &'static str,
    ) {
        if presets.is_empty() {
            self.bottom_pane.dismiss_view_by_id(view_id);
            self.add_info_message("目前没有其他可用模型。".to_string(), /*hint*/ None);
            return;
        }

        let mut items: Vec<SelectionItem> = Vec::new();
        let model_ids = presets.iter().map(|preset| preset.model.clone()).collect();
        for preset in presets.into_iter() {
            let description =
                (!preset.description.is_empty()).then_some(preset.description.to_string());
            let is_current = preset.model.as_str() == self.current_model();
            let direct_effort = match preset.supported_reasoning_efforts.as_slice() {
                [] => Some(preset.default_reasoning_effort.clone()),
                [option] => Some(option.effort.clone()),
                _ => None,
            }
            .filter(|effort| !Self::is_advanced_reasoning_effort(effort));
            let single_supported_effort = direct_effort.is_some();
            let preset_for_action = preset.clone();
            let actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
                let preset_for_event = preset_for_action.clone();
                tx.send(AppEvent::OpenReasoningPopup {
                    model: preset_for_event,
                });
            })];
            items.push(SelectionItem {
                name: preset.display_name.clone(),
                description,
                is_current,
                is_default: preset.is_default,
                secondary_action: direct_effort.and_then(|effort| {
                    self.session_model_selection_action(preset.model.clone(), Some(effort))
                }),
                actions,
                dismiss_on_select: single_supported_effort,
                dismiss_parent_on_child_accept: !single_supported_effort,
                ..Default::default()
            });
        }

        let header = self.model_menu_header("选择模型和推理强度", "");
        self.show_model_selection_view(
            model_ids,
            SelectionViewParams {
                view_id: Some(view_id),
                items,
                header,
                ..SelectionViewParams::picker()
            },
        );
    }

    fn model_selection_actions(
        &self,
        model_for_action: String,
        effort_for_action: Option<ReasoningEffortConfig>,
        should_prompt_plan_mode_scope: bool,
    ) -> Vec<SelectionAction> {
        let warning = effort_for_action
            .as_ref()
            .and_then(|effort| self.ultra_reasoning_concurrency_warning(effort));
        let thread_id = self.thread_id();
        let sparkle_thread = self.sparkle_thread_for_picker_action(&model_for_action);
        vec![Box::new(move |tx| {
            if model_for_action == LUNA_RESERVE_MODEL {
                // Reserve is temporary: update the active task without persisting a model default.
                if let Some(thread_id) = thread_id {
                    tx.send(AppEvent::UpdateLunaReserveReasoning {
                        thread_id,
                        effort: effort_for_action.clone(),
                    });
                }
            } else if effort_for_action == Some(ReasoningEffortConfig::Ultra) {
                tx.send(
                    AstraModelPickerAction::ApplyAdvancedReasoning {
                        effort: ReasoningEffortConfig::Ultra,
                    }
                    .into_picker_event(sparkle_thread, model_for_action.clone()),
                );
            } else if should_prompt_plan_mode_scope {
                tx.send(AppEvent::OpenPlanReasoningScopePrompt {
                    model: model_for_action.clone(),
                    effort: effort_for_action.clone(),
                });
            } else {
                tx.send(
                    AstraModelPickerAction::UpdateModel
                        .into_picker_event(sparkle_thread, model_for_action.clone()),
                );
                tx.send(AppEvent::UpdateReasoningEffort(effort_for_action.clone()));
                tx.send(AppEvent::PersistModelSelection {
                    model: model_for_action.clone(),
                    effort: effort_for_action.clone(),
                });
            }
            if let Some(warning) = warning.clone() {
                tx.send(AppEvent::InsertHistoryCell(Box::new(
                    history_cell::new_warning_event(warning),
                )));
            }
        })]
    }

    fn should_prompt_plan_mode_reasoning_scope(
        &self,
        selected_model: &str,
        selected_effort: Option<ReasoningEffortConfig>,
    ) -> bool {
        if !self.collaboration_modes_enabled()
            || selected_model == LUNA_RESERVE_MODEL
            || self.active_mode_kind() != ModeKind::Plan
            || selected_model != self.current_model()
        {
            return false;
        }

        // Prompt whenever the selection is not a true no-op for both:
        // 1) the active Plan-mode effective reasoning, and
        // 2) the stored global defaults that would be updated by the fallback path.
        selected_effort != self.effective_reasoning_effort()
            || selected_model != self.current_collaboration_mode.model()
            || selected_effort != self.current_collaboration_mode.reasoning_effort()
    }

    pub(crate) fn open_plan_reasoning_scope_prompt(
        &mut self,
        model: String,
        effort: Option<ReasoningEffortConfig>,
    ) {
        let reasoning_phrase = match effort.as_ref() {
            Some(ReasoningEffortConfig::None) => "不使用推理".to_string(),
            Some(selected_effort) => {
                format!(
                    "{}推理",
                    Self::reasoning_effort_sentence_label(selected_effort)
                )
            }
            None => "所选推理强度".to_string(),
        };
        let plan_only_description = format!("在计划模式中始终使用{reasoning_phrase}。");
        let plan_reasoning_source = if let Some(plan_override) =
            self.config.plan_mode_reasoning_effort.as_ref()
        {
            format!(
                "用户选择的计划模式覆盖值（{}）",
                Self::reasoning_effort_sentence_label(plan_override)
            )
        } else if let Some(plan_mask) = collaboration_modes::plan_mask(self.model_catalog.as_ref())
        {
            match plan_mask
                .reasoning_effort
                .as_ref()
                .and_then(|effort| effort.as_ref())
            {
                Some(plan_effort) => format!(
                    "内置计划模式默认值（{}）",
                    Self::reasoning_effort_sentence_label(plan_effort)
                ),
                None => "内置计划模式默认值（不使用推理）".to_string(),
            }
        } else {
            "内置计划模式默认值".to_string()
        };
        let all_modes_description = format!(
            "设置全局默认推理级别和计划模式覆盖值。这将替换当前的{plan_reasoning_source}。"
        );
        let subtitle = format!("选择{reasoning_phrase}的应用范围。");
        let warning = effort
            .as_ref()
            .and_then(|effort| self.ultra_reasoning_concurrency_warning(effort));
        let sparkle_thread = self.sparkle_thread_for_picker_action(&model);

        let plan_only_actions: Vec<SelectionAction> = vec![Box::new({
            let model = model.clone();
            let effort = effort.clone();
            let warning = warning.clone();
            move |tx| {
                tx.send(
                    AstraModelPickerAction::UpdateModel
                        .into_picker_event(sparkle_thread, model.clone()),
                );
                tx.send(AppEvent::UpdatePlanModeReasoningEffort(effort.clone()));
                tx.send(AppEvent::PersistPlanModeReasoningEffort(effort.clone()));
                if let Some(warning) = warning.clone() {
                    tx.send(AppEvent::InsertHistoryCell(Box::new(
                        history_cell::new_warning_event(warning),
                    )));
                }
            }
        })];
        let all_modes_actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
            tx.send(
                AstraModelPickerAction::UpdateModel
                    .into_picker_event(sparkle_thread, model.clone()),
            );
            tx.send(AppEvent::UpdateReasoningEffort(effort.clone()));
            tx.send(AppEvent::UpdatePlanModeReasoningEffort(effort.clone()));
            tx.send(AppEvent::PersistPlanModeReasoningEffort(effort.clone()));
            tx.send(AppEvent::PersistModelSelection {
                model: model.clone(),
                effort: effort.clone(),
            });
            if let Some(warning) = warning.clone() {
                tx.send(AppEvent::InsertHistoryCell(Box::new(
                    history_cell::new_warning_event(warning),
                )));
            }
        })];

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some(PLAN_MODE_REASONING_SCOPE_TITLE.to_string()),
            subtitle: Some(subtitle),
            items: vec![
                SelectionItem {
                    name: PLAN_MODE_REASONING_SCOPE_PLAN_ONLY.to_string(),
                    description: Some(plan_only_description),
                    actions: plan_only_actions,
                    dismiss_on_select: true,
                    ..Default::default()
                },
                SelectionItem {
                    name: PLAN_MODE_REASONING_SCOPE_ALL_MODES.to_string(),
                    description: Some(all_modes_description),
                    actions: all_modes_actions,
                    dismiss_on_select: true,
                    ..Default::default()
                },
            ],
            ..SelectionViewParams::picker()
        });
        self.notify(Notification::PlanModePrompt {
            title: PLAN_MODE_REASONING_SCOPE_TITLE.to_string(),
        });
    }

    /// Open a popup to choose the standard reasoning effort for the given model.
    ///
    /// Max and Ultra require an explicit second step so expensive efforts cannot
    /// be selected accidentally while moving through the normal effort scale.
    pub(crate) fn open_reasoning_popup(&mut self, preset: ModelPreset) {
        let default_effort = preset.default_reasoning_effort.clone();
        let supported = &preset.supported_reasoning_efforts;
        let in_plan_mode =
            self.collaboration_modes_enabled() && self.active_mode_kind() == ModeKind::Plan;

        let warn_effort = if supported
            .iter()
            .any(|option| option.effort == ReasoningEffortConfig::XHigh)
        {
            Some(ReasoningEffortConfig::XHigh)
        } else if supported
            .iter()
            .any(|option| option.effort == ReasoningEffortConfig::High)
        {
            Some(ReasoningEffortConfig::High)
        } else {
            None
        };
        let warning_text = warn_effort.as_ref().map(|effort| {
            let effort_label = Self::reasoning_effort_label(effort);
            format!("⚠ {effort_label}推理强度可能很快消耗 Plus 套餐的速率限额。")
        });
        let warn_for_model = preset.model.starts_with("gpt-5.1-codex")
            || preset.model.starts_with("gpt-5.1-codex-max")
            || preset.model.starts_with("gpt-5.2");

        let mut all_choices: Vec<ReasoningEffortConfig> = supported
            .iter()
            .map(|option| option.effort.clone())
            .collect();
        if all_choices.is_empty() {
            all_choices.push(default_effort.clone());
        }
        let (choices, advanced_choices): (Vec<_>, Vec<_>) = all_choices
            .into_iter()
            .partition(|effort| !Self::is_advanced_reasoning_effort(effort));

        if choices.len() == 1 && advanced_choices.is_empty() {
            let selected_effort = choices.first().cloned();
            let selected_model = preset.model;
            if self
                .should_prompt_plan_mode_reasoning_scope(&selected_model, selected_effort.clone())
            {
                self.app_event_tx
                    .send(AppEvent::OpenPlanReasoningScopePrompt {
                        model: selected_model,
                        effort: selected_effort,
                    });
            } else {
                self.apply_model_and_effort(selected_model, selected_effort);
            }
            return;
        }

        let default_choice = choices
            .contains(&default_effort)
            .then(|| default_effort.clone());

        let model_slug = preset.model.to_string();
        let model_label = preset.display_name.clone();
        let is_current_model = self.current_model() == preset.model.as_str();
        let highlight_choice = if is_current_model {
            if in_plan_mode {
                self.config
                    .plan_mode_reasoning_effort
                    .clone()
                    .or_else(|| self.effective_reasoning_effort())
            } else {
                self.effective_reasoning_effort()
            }
        } else {
            default_choice.clone().or_else(|| choices.first().cloned())
        };
        let selection_choice = highlight_choice.clone().or_else(|| default_choice.clone());
        let initial_selected_idx = choices
            .iter()
            .position(|choice| Some(choice) == selection_choice.as_ref());
        let mut items: Vec<SelectionItem> = Vec::new();
        for choice in choices.iter() {
            let effort = choice.clone();
            let mut effort_label = Self::reasoning_effort_label(&effort);
            if Some(choice) == default_choice.as_ref() {
                effort_label.push_str("（默认）");
            }

            let description = supported
                .iter()
                .find(|option| option.effort == effort)
                .map(|option| option.description.to_string())
                .filter(|text| !text.is_empty());

            let show_warning = warn_for_model && warn_effort.as_ref() == Some(&effort);
            let selected_description = if show_warning {
                warning_text.as_ref().map(|warning_message| {
                    description.as_ref().map_or_else(
                        || warning_message.clone(),
                        |d| format!("{d}\n{warning_message}"),
                    )
                })
            } else {
                None
            };

            let choice_effort = Some(effort);
            let should_prompt_plan_mode_scope = self.should_prompt_plan_mode_reasoning_scope(
                model_slug.as_str(),
                choice_effort.clone(),
            );
            let actions = self.model_selection_actions(
                model_slug.clone(),
                choice_effort.clone(),
                should_prompt_plan_mode_scope,
            );

            items.push(SelectionItem {
                name: effort_label,
                description,
                selected_description,
                is_current: is_current_model && Some(choice) == highlight_choice.as_ref(),
                secondary_action: self
                    .session_model_selection_action(model_slug.clone(), choice_effort),
                actions,
                dismiss_on_select: true,
                ..Default::default()
            });
        }

        if !advanced_choices.is_empty() {
            let advanced_label = advanced_choices
                .iter()
                .map(Self::reasoning_effort_label)
                .collect::<Vec<_>>()
                .join("和");
            let preset_for_action = preset;
            let actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
                tx.send(AppEvent::OpenAdvancedReasoningPopup {
                    model: preset_for_action.clone(),
                });
            })];
            items.push(SelectionItem {
                name: "更高推理强度…".to_string(),
                description: Some(format!("{advanced_label}会更快消耗用量限额")),
                is_current: is_current_model
                    && highlight_choice
                        .as_ref()
                        .is_some_and(Self::is_advanced_reasoning_effort),
                actions,
                dismiss_parent_on_child_accept: true,
                ..Default::default()
            });
        }

        let header = Paragraph::new(Line::from(format!("选择 {model_label} 的推理级别").bold()))
            .wrap(Wrap { trim: false });

        self.bottom_pane.show_selection_view(SelectionViewParams {
            header: Box::new(header),
            items,
            initial_selected_idx,
            ..SelectionViewParams::picker()
        });
    }

    /// Open the explicit Max/Ultra effort picker for the given model.
    pub(crate) fn open_advanced_reasoning_popup(&mut self, preset: ModelPreset) {
        let mut choices = preset
            .supported_reasoning_efforts
            .iter()
            .map(|option| option.effort.clone())
            .filter(Self::is_advanced_reasoning_effort)
            .collect::<Vec<_>>();
        if choices.is_empty()
            && Self::is_advanced_reasoning_effort(&preset.default_reasoning_effort)
        {
            choices.push(preset.default_reasoning_effort.clone());
        }
        choices.sort_by_key(|effort| matches!(effort, ReasoningEffortConfig::Ultra));
        if choices.is_empty() {
            return;
        }

        let model_slug = preset.model.to_string();
        let is_current_model = self.current_model() == preset.model.as_str();
        let highlight_choice = is_current_model
            .then(|| self.effective_reasoning_effort())
            .flatten();
        let mut items = Vec::new();
        for effort in choices {
            let description = match &effort {
                ReasoningEffortConfig::Max => "适合质量比速度更重要的难题 · 用量较高",
                ReasoningEffortConfig::Ultra => "适合使用多个智能体的高要求工作 · 用量最高",
                _ => unreachable!("advanced choices are limited to Max and Ultra"),
            };
            let should_prompt_plan_mode_scope = self
                .should_prompt_plan_mode_reasoning_scope(model_slug.as_str(), Some(effort.clone()));
            let actions = self.model_selection_actions(
                model_slug.clone(),
                Some(effort.clone()),
                should_prompt_plan_mode_scope,
            );

            items.push(SelectionItem {
                name: Self::reasoning_effort_label(&effort),
                description: Some(description.to_string()),
                is_current: is_current_model && Some(&effort) == highlight_choice.as_ref(),
                secondary_action: self
                    .session_model_selection_action(model_slug.clone(), Some(effort.clone())),
                actions,
                dismiss_on_select: true,
                ..Default::default()
            });
        }

        let header = Paragraph::new(vec![
            Line::from("高级推理".bold()),
            Line::from("⚠ 会更快消耗用量限额".cyan()),
        ])
        .wrap(Wrap { trim: false });
        self.bottom_pane.show_selection_view(SelectionViewParams {
            header: Box::new(header),
            items,
            ..SelectionViewParams::picker()
        });
    }

    pub(super) fn is_advanced_reasoning_effort(effort: &ReasoningEffortConfig) -> bool {
        matches!(
            effort,
            ReasoningEffortConfig::Max | ReasoningEffortConfig::Ultra
        )
    }

    pub(super) fn reasoning_effort_label(effort: &ReasoningEffortConfig) -> String {
        match effort {
            ReasoningEffortConfig::None => "无".to_string(),
            ReasoningEffortConfig::Minimal => "最低".to_string(),
            ReasoningEffortConfig::Low => "低".to_string(),
            ReasoningEffortConfig::Medium => "中".to_string(),
            ReasoningEffortConfig::High => "高".to_string(),
            ReasoningEffortConfig::XHigh => "超高".to_string(),
            ReasoningEffortConfig::Max => "最大".to_string(),
            ReasoningEffortConfig::Ultra => "极致".to_string(),
            ReasoningEffortConfig::Persistent => "持续".to_string(),
            ReasoningEffortConfig::Custom(value) => value.clone(),
        }
    }

    pub(super) fn reasoning_effort_sentence_label(effort: &ReasoningEffortConfig) -> String {
        match effort {
            ReasoningEffortConfig::Custom(value) => value.clone(),
            effort => Self::reasoning_effort_label(effort).to_lowercase(),
        }
    }

    pub(super) fn ultra_reasoning_concurrency_warning(
        &self,
        effort: &ReasoningEffortConfig,
    ) -> Option<String> {
        if effort != &ReasoningEffortConfig::Ultra {
            return None;
        }

        let max_threads = self
            .config
            .multi_agent_v2
            .max_concurrent_threads_per_session;
        if max_threads < ULTRA_REASONING_CONCURRENCY_WARNING_THRESHOLD {
            return None;
        }

        let max_subagents = max_threads.saturating_sub(1);
        Some(format!(
            "Ultra 推理可能会主动使用多个代理。此会话配置了 {max_threads} 个并发会话，\
             最多包含 {max_subagents} 个子代理，这可能会迅速增加用量。建议将 \
             features.multi_agent_v2.max_concurrent_threads_per_session 设置为低于 8。"
        ))
    }

    fn apply_model_and_effort(&self, model: String, effort: Option<ReasoningEffortConfig>) {
        for action in self
            .model_selection_actions(model, effort, /*should_prompt_plan_mode_scope*/ false)
        {
            action(&self.app_event_tx);
        }
    }
}
