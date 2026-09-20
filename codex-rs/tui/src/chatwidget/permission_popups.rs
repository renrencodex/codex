//! Permission and approval popup flows for `ChatWidget`.
//!
//! This module owns the generic permission pickers and confirmation surfaces;
//! Windows-specific sandbox prompting lives beside it in
//! `windows_sandbox_prompts`.

use super::*;
use codex_protocol::openai_models::MODEL_SPECIALTY_CYBER;

impl ChatWidget {
    /// Open the permissions popup.
    pub(crate) fn open_approvals_popup(&mut self) {
        self.open_permissions_popup();
    }

    /// Open a popup to choose the permissions mode.
    pub(crate) fn open_permissions_popup(&mut self) {
        if self.config.explicit_permission_profile_mode
            || self.permission_profiles_menu_opened
            || self
                .config
                .permissions
                .active_permission_profile()
                .is_some_and(|profile| !profile.id.starts_with(':'))
        {
            self.request_permission_profiles();
            return;
        }

        self.open_legacy_permissions_popup();
    }

    pub(super) fn open_legacy_permissions_popup(&mut self) {
        let include_read_only = cfg!(target_os = "windows");
        let current_approval =
            AskForApproval::from(self.config.permissions.approval_policy.value());
        let current_permission_profile = self.config.permissions.permission_profile().clone();
        let guardian_approval_enabled = self.config.features.enabled(Feature::GuardianApproval);
        let current_review_policy = self.config.approvals_reviewer;
        let mut items: Vec<SelectionItem> = Vec::new();
        let presets: Vec<ApprovalPreset> = builtin_approval_presets();

        #[cfg(target_os = "windows")]
        let windows_sandbox_level = self.windows_sandbox_config.level();
        #[cfg(target_os = "windows")]
        let windows_degraded_sandbox_enabled =
            matches!(windows_sandbox_level, WindowsSandboxLevel::RestrictedToken)
                && self.windows_sandbox_local_server
                && self.windows_sandbox_host == crate::app::WindowsSandboxHost::Local;
        #[cfg(not(target_os = "windows"))]
        let windows_degraded_sandbox_enabled = false;

        let show_elevate_sandbox_hint = windows_degraded_sandbox_enabled
            && self
                .windows_sandbox_config
                .allows(WindowsSandboxSetupMode::Elevated)
            && presets.iter().any(|preset| preset.id == "auto");

        let guardian_disabled_reason = |enabled: bool| {
            let mut next_features = self.config.features.get().clone();
            next_features.set_enabled(Feature::GuardianApproval, enabled);
            self.config
                .features
                .can_set(&next_features)
                .err()
                .map(|err| err.to_string())
        };

        for preset in presets.into_iter() {
            if !include_read_only && preset.id == "read-only" {
                continue;
            }
            let base_name = if preset.id == "auto" && windows_degraded_sandbox_enabled {
                format!("{ASK_FOR_APPROVAL_LABEL}（非管理员沙箱）")
            } else if preset.id == "auto" {
                ASK_FOR_APPROVAL_LABEL.to_string()
            } else {
                match preset.id {
                    "full-access" => "完全访问".to_string(),
                    "read-only" => "只读".to_string(),
                    _ => preset.label.to_string(),
                }
            };
            let base_description = Some(match preset.id {
                "auto" => "允许在工作区内读写和运行命令；需要时请求审批。".to_string(),
                "full-access" => "允许不受沙箱限制地访问系统。".to_string(),
                "read-only" => "仅允许读取；修改文件或执行受限操作时请求审批。".to_string(),
                _ => preset.description.replace(" (Identical to Agent mode)", ""),
            });
            let approval_disabled_reason = match self
                .config
                .permissions
                .approval_policy
                .can_set(&preset.approval)
            {
                Ok(()) => None,
                Err(err) => Some(err.to_string()),
            };
            let default_disabled_reason = approval_disabled_reason
                .clone()
                .or_else(|| guardian_disabled_reason(false));
            let default_actions = self.permission_mode_actions(
                &preset,
                base_name.clone(),
                ApprovalsReviewer::User,
                /*profile_selection*/ None,
                /*return_to_permissions*/ !include_read_only,
            );
            if preset.id == "auto" {
                items.push(SelectionItem {
                    name: base_name.clone(),
                    description: base_description.clone(),
                    is_current: current_review_policy == ApprovalsReviewer::User
                        && Self::preset_matches_current(
                            current_approval,
                            &current_permission_profile,
                            self.config.cwd.as_path(),
                            &preset,
                        ),
                    actions: default_actions,
                    dismiss_on_select: true,
                    disabled_reason: default_disabled_reason,
                    ..Default::default()
                });

                if guardian_approval_enabled {
                    items.push(SelectionItem {
                        name: APPROVE_FOR_ME_LABEL.to_string(),
                        description: Some(AUTO_REVIEW_DESCRIPTION.to_string()),
                        is_current: current_review_policy == ApprovalsReviewer::AutoReview
                            && (Self::preset_matches_current(
                                current_approval,
                                &current_permission_profile,
                                self.config.cwd.as_path(),
                                &preset,
                            ) || (current_approval == AskForApproval::OnRequest
                                && self
                                    .config
                                    .config_layer_stack
                                    .requirements()
                                    .auto_review_required_for_model(self.current_model()))),
                        actions: self.permission_mode_actions(
                            &preset,
                            APPROVE_FOR_ME_LABEL.to_string(),
                            ApprovalsReviewer::AutoReview,
                            /*profile_selection*/ None,
                            /*return_to_permissions*/ !include_read_only,
                        ),
                        dismiss_on_select: true,
                        disabled_reason: approval_disabled_reason
                            .or_else(|| guardian_disabled_reason(true)),
                        ..Default::default()
                    });
                }
            } else {
                items.push(SelectionItem {
                    name: base_name,
                    description: base_description,
                    is_current: Self::preset_matches_current(
                        current_approval,
                        &current_permission_profile,
                        self.config.cwd.as_path(),
                        &preset,
                    ),
                    actions: default_actions,
                    dismiss_on_select: true,
                    disabled_reason: default_disabled_reason,
                    ..Default::default()
                });
            }
        }

        let footer_note = show_elevate_sandbox_hint.then(|| {
            vec![
                "非管理员沙箱通常可以保护文件并阻止网络访问，但受到提示注入时风险更高。要升级到默认沙箱，请运行 ".dim(),
                "/setup-default-sandbox".cyan(),
                ".".dim(),
            ]
            .into()
        });

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("更新模型权限".to_string()),
            footer_note,
            footer_hint: Some(standard_popup_hint_line()),
            items,
            header: Box::new(()),
            ..Default::default()
        });
    }

    pub(crate) fn open_auto_review_denials_popup(&mut self) {
        if self.review.recent_auto_review_denials.is_empty() {
            self.add_info_message(
                "此会话中没有最近被自动审查拒绝的操作。".to_string(),
                Some("自动审查拒绝操作后会在此记录。".to_string()),
            );
            return;
        }
        let Some(thread_id) = self.thread_id() else {
            self.add_error_message("该会话已不可用。".to_string());
            return;
        };

        let mut items = vec![SelectionItem {
            name: "操作".to_string(),
            description: Some("理由".to_string()),
            is_disabled: true,
            search_value: Some(String::new()),
            ..Default::default()
        }];
        items.extend(
            self.review
                .recent_auto_review_denials
                .entries()
                .map(|event| {
                    let id = event.id.clone();
                    let summary = auto_review_denials::action_summary(&event.action);
                    let rationale = event.rationale.as_deref().unwrap_or("自动审查未提供理由。");
                    SelectionItem {
                        name: summary.clone(),
                        description: Some(rationale.to_string()),
                        selected_description: Some(rationale.to_string()),
                        search_value: Some(format!("{summary} {rationale}")),
                        actions: vec![Box::new(move |tx| {
                            tx.send(AppEvent::ApproveRecentAutoReviewDenial {
                                thread_id,
                                id: id.clone(),
                            });
                        })],
                        dismiss_on_select: true,
                        ..Default::default()
                    }
                }),
        );

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("自动审查拒绝记录".to_string()),
            subtitle: Some("选择要批准重试的操作。".to_string()),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            is_searchable: true,
            col_width_mode: ColumnWidthMode::AutoAllRows,
            ..Default::default()
        });
        self.request_redraw();
    }

    pub(crate) fn approve_recent_auto_review_denial(&mut self, thread_id: ThreadId, id: String) {
        let Some(event) = self.review.recent_auto_review_denials.take(&id) else {
            self.add_error_message("该自动审查拒绝记录已不可用。".to_string());
            return;
        };

        self.app_event_tx.send(AppEvent::SubmitThreadOp {
            thread_id,
            op: AppCommand::approve_guardian_denied_action(event),
        });
        self.add_info_message(
            "已批准重试一次所选的自动审查拒绝操作。".to_string(),
            Some("模型会看到审批上下文；重试仍会经过自动审查。".to_string()),
        );
    }

    pub(super) fn approval_preset_actions(
        approval: AskForApproval,
        permission_profile: PermissionProfile,
        active_permission_profile: ActivePermissionProfile,
        label: String,
        approvals_reviewer: ApprovalsReviewer,
    ) -> Vec<SelectionAction> {
        vec![Box::new(move |tx| {
            tx.send(AppEvent::CodexOp(AppCommand::override_turn_context(
                /*cwd*/ None,
                Some(approval),
                Some(approvals_reviewer),
                Some(permission_profile.clone()),
                Some(active_permission_profile.clone()),
                /*model*/ None,
                /*effort*/ None,
                /*summary*/ None,
                /*service_tier*/ None,
                /*collaboration_mode*/ None,
                /*personality*/ None,
            )));
            tx.send(AppEvent::UpdateAskForApprovalPolicy(approval));
            tx.send(AppEvent::UpdateActivePermissionProfile(
                active_permission_profile.clone(),
            ));
            tx.send(AppEvent::UpdateApprovalsReviewer(approvals_reviewer));
            tx.send(AppEvent::InsertHistoryCell(Box::new(
                history_cell::new_info_event(format!("权限已更新为{label}"), /*hint*/ None),
            )));
        })]
    }

    pub(super) fn permission_profile_selection_actions(
        selection: PermissionProfileSelection,
    ) -> Vec<SelectionAction> {
        vec![Box::new(move |tx| {
            tx.send(AppEvent::SelectPermissionProfile(selection.clone()));
        })]
    }

    pub(super) fn permission_mode_actions(
        &self,
        preset: &ApprovalPreset,
        label: String,
        approvals_reviewer: ApprovalsReviewer,
        profile_selection: Option<PermissionProfileSelection>,
        return_to_permissions: bool,
    ) -> Vec<SelectionAction> {
        let profile_selection = profile_selection.or_else(|| {
            self.thread_id.map(|_| PermissionProfileSelection {
                profile_id: preset.active_permission_profile.id.clone(),
                approval_policy: Some(AskForApproval::from(preset.approval)),
                approvals_reviewer: Some(approvals_reviewer),
                display_label: label.clone(),
            })
        });
        let apply_actions = || {
            profile_selection.clone().map_or_else(
                || {
                    Self::approval_preset_actions(
                        AskForApproval::from(preset.approval),
                        preset.permission_profile.clone(),
                        preset.active_permission_profile.clone(),
                        label.clone(),
                        approvals_reviewer,
                    )
                },
                Self::permission_profile_selection_actions,
            )
        };
        let requires_confirmation =
            approvals_reviewer == ApprovalsReviewer::User && preset.id == "full-access";
        #[cfg(target_os = "windows")]
        if preset.id == "auto"
            && matches!(
                self.windows_sandbox_host,
                crate::app::WindowsSandboxHost::Mixed | crate::app::WindowsSandboxHost::Unknown
            )
        {
            let preset = preset.clone();
            return vec![Box::new(move |tx| {
                tx.send(AppEvent::OpenWindowsSandboxEnablePrompt {
                    preset: preset.clone(),
                    profile_selection: profile_selection.clone(),
                });
            })];
        }
        if requires_confirmation {
            let preset = preset.clone();
            return vec![Box::new(move |tx| {
                tx.send(AppEvent::OpenFullAccessConfirmation {
                    preset: preset.clone(),
                    return_to_permissions,
                    profile_selection: profile_selection.clone(),
                });
            })];
        }
        if approvals_reviewer == ApprovalsReviewer::User && preset.id == "auto" {
            #[cfg(target_os = "windows")]
            {
                if self.windows_sandbox_host == crate::app::WindowsSandboxHost::Remote {
                    // The remote server owns the permission choice. Its executor
                    // cannot be set up from this TUI's Windows account.
                    return apply_actions();
                }
                if !self.windows_sandbox_config.is_enabled() {
                    let preset = preset.clone();
                    return vec![Box::new(move |tx| {
                        tx.send(AppEvent::OpenWindowsSandboxEnablePrompt {
                            preset: preset.clone(),
                            profile_selection: profile_selection.clone(),
                        });
                    })];
                }
            }
        }
        apply_actions()
    }

    pub(super) fn preset_matches_current(
        current_approval: AskForApproval,
        current_permission_profile: &PermissionProfile,
        cwd: &std::path::Path,
        preset: &ApprovalPreset,
    ) -> bool {
        let preset_approval = AskForApproval::from(preset.approval);
        if current_approval != preset_approval {
            return false;
        }

        match preset.id {
            "full-access" => matches!(current_permission_profile, PermissionProfile::Disabled),
            "read-only" => {
                let file_system_policy = current_permission_profile.file_system_sandbox_policy();
                matches!(
                    current_permission_profile,
                    PermissionProfile::Managed { .. }
                ) && !file_system_policy.has_full_disk_write_access()
                    && file_system_policy
                        .get_writable_roots_with_cwd(cwd)
                        .is_empty()
                    && current_permission_profile.network_sandbox_policy()
                        == preset.permission_profile.network_sandbox_policy()
            }
            "auto" => {
                let file_system_policy = current_permission_profile.file_system_sandbox_policy();
                matches!(
                    current_permission_profile,
                    PermissionProfile::Managed { .. }
                ) && file_system_policy.can_write_local_path_with_cwd(cwd, cwd)
                    && !file_system_policy.has_full_disk_write_access()
            }
            _ => current_permission_profile == &preset.permission_profile,
        }
    }

    pub(crate) fn open_full_access_confirmation(
        &mut self,
        preset: ApprovalPreset,
        return_to_permissions: bool,
        profile_selection: Option<PermissionProfileSelection>,
    ) {
        let selected_name = match preset.id {
            "full-access" => "完全访问".to_string(),
            "read-only" => "只读".to_string(),
            "auto" => ASK_FOR_APPROVAL_LABEL.to_string(),
            _ => preset.label.to_string(),
        };
        let approval = AskForApproval::from(preset.approval);
        let is_cyber_model = self.model_catalog.try_list_models().is_ok_and(|models| {
            models.iter().any(|model| {
                model.model == self.current_model()
                    && model.model_specialty.as_deref() == Some(MODEL_SPECIALTY_CYBER)
            })
        });
        let title_line = Line::from("要启用完全访问吗？").bold();
        let info_lines = if is_cyber_model {
            let recommendation = if auto_review_available(&self.config) {
                "强烈建议改选“代我审批”，并根据你的使用场景自定义审查策略。"
            } else {
                "强烈建议改选“请求审批”。"
            };
            vec![
                Line::default(),
                Line::from(
                    "Codex 以完全访问模式运行时，无需你的批准即可编辑计算机上的任何文件，并运行可访问网络的命令。",
                ),
                Line::default(),
                Line::from(vec![
                    "网络安全模型执行危险操作的风险更高。".red(),
                    " 授予完全访问前，请确保已采取适当的安全措施。".into(),
                    recommendation.into(),
                ]),
            ]
        } else {
            vec![Line::from(vec![
                "Codex 以完全访问模式运行时，无需你的批准即可编辑计算机上的任何文件，并运行可访问网络的命令。"
                    .into(),
                "启用完全访问时请谨慎，这会显著增加数据丢失、泄露或意外行为的风险。"
                    .red(),
            ])]
        };
        let header = Paragraph::new(
            std::iter::once(title_line)
                .chain(info_lines)
                .collect::<Vec<_>>(),
        )
        .wrap(Wrap { trim: false });

        let accept_actions = profile_selection.map_or_else(
            || {
                Self::approval_preset_actions(
                    approval,
                    preset.permission_profile,
                    preset.active_permission_profile,
                    selected_name,
                    ApprovalsReviewer::User,
                )
            },
            Self::permission_profile_selection_actions,
        );

        let deny_actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
            if return_to_permissions {
                tx.send(AppEvent::OpenPermissionsPopup);
            } else {
                tx.send(AppEvent::OpenApprovalsPopup);
            }
        })];

        let items = vec![
            SelectionItem {
                name: "是，仍然继续".to_string(),
                description: Some("为此会话应用完全访问".to_string()),
                actions: accept_actions,
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: "取消".to_string(),
                description: Some("返回且不启用完全访问".to_string()),
                actions: deny_actions,
                dismiss_on_select: true,
                ..Default::default()
            },
        ];

        self.bottom_pane.show_selection_view(SelectionViewParams {
            footer_hint: Some(standard_popup_hint_line()),
            items,
            header: Box::new(header),
            ..Default::default()
        });
    }
}
