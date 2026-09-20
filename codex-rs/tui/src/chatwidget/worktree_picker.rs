//! Worktree choices for local, feature-enabled session commands.

use super::*;
use crate::app_event::ManagedWorktreeMode;
use crate::worktree_browser::Action;
use crate::worktree_browser::Entry;
use crate::worktree_browser::Owner;
use crate::worktree_browser::Request;

const BROWSER_VIEW_ID: &str = "managed-worktrees";

impl ChatWidget {
    pub(super) fn managed_worktree_available(&self) -> bool {
        self.config.features.enabled(Feature::Worktrees)
            && self.local_worktree_operations
            && get_git_repo_root(self.config.cwd.as_path()).is_some()
    }

    pub(super) fn show_session_checkout_picker(
        &mut self,
        mode: ManagedWorktreeMode,
        name: Option<String>,
    ) {
        if !self.managed_worktree_available() {
            match mode {
                ManagedWorktreeMode::New => {
                    self.app_event_tx.send(AppEvent::NewSession { name });
                }
                ManagedWorktreeMode::Fork => {
                    self.app_event_tx
                        .send(AppEvent::ForkCurrentSession { name });
                }
            }
            return;
        }

        let title = match mode {
            ManagedWorktreeMode::New => "新对话应在哪里运行？",
            ManagedWorktreeMode::Fork => "分叉对话应在哪里运行？",
        };
        let current_name = name.clone();
        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some(title.to_string()),
            footer_hint: Some(standard_popup_hint_line()),
            items: vec![
                SelectionItem {
                    name: "当前检出".to_string(),
                    description: Some("继续使用当前工作目录".to_string()),
                    actions: vec![Box::new(move |tx| match mode {
                        ManagedWorktreeMode::New => {
                            tx.send(AppEvent::NewSession {
                                name: current_name.clone(),
                            });
                        }
                        ManagedWorktreeMode::Fork => {
                            tx.send(AppEvent::ForkCurrentSession {
                                name: current_name.clone(),
                            });
                        }
                    })],
                    dismiss_on_select: true,
                    ..Default::default()
                },
                SelectionItem {
                    name: "新建工作树".to_string(),
                    description: Some("创建隔离的托管检出".to_string()),
                    actions: vec![Box::new(move |tx| {
                        tx.send(AppEvent::StartManagedWorktree {
                            mode,
                            name: name.clone(),
                        });
                    })],
                    dismiss_on_select: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        self.request_redraw();
    }

    pub(super) fn show_managed_worktree_picker(&mut self) {
        if !self.config.features.enabled(Feature::Worktrees) {
            self.add_error_message("请在 Codex 配置中启用工作树后再创建工作树。".to_string());
            return;
        }
        if !self.managed_worktree_available() {
            self.add_error_message("托管工作树需要本地 Git 仓库。".to_string());
            return;
        }

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("工作树".to_string()),
            footer_hint: Some(standard_popup_hint_line()),
            items: vec![
                SelectionItem {
                    name: "继续当前对话".to_string(),
                    description: Some("在新检出中保留此对话".to_string()),
                    actions: vec![Box::new(|tx| {
                        tx.send(AppEvent::StartManagedWorktree {
                            mode: ManagedWorktreeMode::Fork,
                            name: None,
                        });
                    })],
                    dismiss_on_select: true,
                    ..Default::default()
                },
                SelectionItem {
                    name: "开始新对话".to_string(),
                    description: Some("在新检出中打开全新对话".to_string()),
                    actions: vec![Box::new(|tx| {
                        tx.send(AppEvent::StartManagedWorktree {
                            mode: ManagedWorktreeMode::New,
                            name: None,
                        });
                    })],
                    dismiss_on_select: true,
                    ..Default::default()
                },
                SelectionItem {
                    name: "浏览工作树".to_string(),
                    description: Some("恢复所属线程或复制工作目录".to_string()),
                    actions: vec![Box::new(|tx| tx.send(AppEvent::BrowseManagedWorktrees))],
                    dismiss_on_select: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        self.request_redraw();
    }

    pub(crate) fn request_managed_worktrees(&mut self) -> Option<Request> {
        if !self.managed_worktree_available() {
            return None;
        }
        let request = Request {
            id: uuid::Uuid::new_v4(),
            cwd: self.config.cwd.to_path_buf(),
            thread_id: self.thread_id,
        };
        self.bottom_pane.dismiss_view_by_id(BROWSER_VIEW_ID);
        self.worktree_popup_request_id = Some(request.id);
        self.bottom_pane.show_selection_view(SelectionViewParams {
            view_id: Some(BROWSER_VIEW_ID),
            title: Some("托管工作树".to_string()),
            items: vec![SelectionItem {
                name: "正在加载工作树…".to_string(),
                is_disabled: true,
                ..Default::default()
            }],
            footer_hint: Some(standard_popup_hint_line()),
            ..Default::default()
        });
        Some(request)
    }

    pub(crate) fn worktree_request_is_current(&self, request: &Request) -> bool {
        self.worktree_popup_request_id == Some(request.id)
            && request.cwd == self.config.cwd.as_path()
            && request.thread_id == self.thread_id
            && self.managed_worktree_available()
    }

    pub(crate) fn on_managed_worktrees_loaded(
        &mut self,
        request: Request,
        result: Result<Vec<Entry>, String>,
    ) {
        if self.worktree_popup_request_id != Some(request.id) {
            return;
        }
        if !self.worktree_request_is_current(&request)
            || !self.bottom_pane.dismiss_active_view_if_id(BROWSER_VIEW_ID)
        {
            self.worktree_popup_request_id = None;
            self.bottom_pane.dismiss_view_by_id(BROWSER_VIEW_ID);
            return;
        }
        let entries = match result {
            Ok(entries) => entries,
            Err(error) => {
                self.worktree_popup_request_id = None;
                self.add_error_message(format!("无法列出托管工作树：{error}"));
                return;
            }
        };
        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("托管工作树".to_string()),
            subtitle: Some(
                if entries.is_empty() {
                    "此仓库配置的池中没有工作树"
                } else {
                    "选择工作树以恢复、复制路径或删除"
                }
                .to_string(),
            ),
            is_searchable: true,
            items: entries
                .into_iter()
                .map(|entry| {
                    let request = request.clone();
                    let (name, description) = match &entry.owner {
                        Owner::None => (entry.cwd.display().to_string(), "未关联线程".to_string()),
                        Owner::Unavailable(_) => (
                            entry.cwd.display().to_string(),
                            "所属线程不可用".to_string(),
                        ),
                        Owner::Archived(thread) | Owner::Resumable(thread) => {
                            let status = match &entry.owner {
                                Owner::Archived(_) => "已归档 · ",
                                Owner::Resumable(_) => "",
                                Owner::None | Owner::Unavailable(_) => unreachable!(),
                            };
                            (
                                thread.title.clone(),
                                format!(
                                    "{status}更新于 {} · {}",
                                    worktree_updated_ago(
                                        thread.updated_at,
                                        chrono::Utc::now().timestamp()
                                    ),
                                    entry.cwd.display()
                                ),
                            )
                        }
                    };
                    SelectionItem {
                        name: name.clone(),
                        search_value: Some(format!("{name} {}", entry.cwd.display())),
                        description: Some(description),
                        actions: vec![Box::new(move |tx| {
                            tx.send(AppEvent::ShowManagedWorktreeActions {
                                request: request.clone(),
                                entry: entry.clone(),
                            })
                        })],
                        dismiss_on_select: true,
                        ..Default::default()
                    }
                })
                .collect(),
            footer_hint: Some(standard_popup_hint_line()),
            ..Default::default()
        });
    }

    pub(crate) fn managed_worktree_action(
        &self,
        request: &Request,
        action: Action,
    ) -> Option<AppEvent> {
        if !self.worktree_request_is_current(request) {
            return None;
        }
        Some(match action {
            Action::Resume(owner) => AppEvent::ResumeSessionByIdOrName(owner.to_string()),
            Action::Copy(cwd) => AppEvent::CopySelection {
                text: cwd.to_str()?.into(),
                label: "工作树工作目录".to_string(),
                format: crate::clipboard_copy::CopyFormat::PlainText,
            },
            Action::Remove(root) => AppEvent::RemoveManagedWorktree {
                request: request.clone(),
                root,
            },
        })
    }

    pub(crate) fn show_managed_worktree_actions(&mut self, request: Request, entry: Entry) {
        if !self.worktree_request_is_current(&request) {
            return;
        }
        let mut items = Vec::new();
        if let Owner::Resumable(thread) = &entry.owner {
            let owner = thread.id;
            let request = request.clone();
            items.push(SelectionItem {
                name: "恢复所属线程".to_string(),
                actions: vec![Box::new(move |tx| {
                    tx.send(AppEvent::ManagedWorktreeAction {
                        request: request.clone(),
                        action: Action::Resume(owner),
                    })
                })],
                dismiss_on_select: true,
                ..Default::default()
            });
        }
        let cwd = entry.cwd.clone();
        let copy_request = request.clone();
        items.push(SelectionItem {
            name: "复制工作目录".to_string(),
            is_disabled: entry.cwd.to_str().is_none(),
            disabled_reason: entry
                .cwd
                .to_str()
                .is_none()
                .then(|| "路径不是有效的 UTF-8".to_string()),
            actions: vec![Box::new(move |tx| {
                tx.send(AppEvent::ManagedWorktreeAction {
                    request: copy_request.clone(),
                    action: Action::Copy(cwd.clone()),
                })
            })],
            dismiss_on_select: true,
            ..Default::default()
        });
        let can_delete = !request.cwd.starts_with(&entry.root);
        let root = entry.root.clone();
        let delete_request = request;
        items.push(SelectionItem {
            name: "删除工作树".to_string(),
            is_disabled: !can_delete,
            disabled_reason: (!can_delete).then(|| "请先切换到其他检出再删除此工作树".to_string()),
            actions: vec![Box::new(move |tx| {
                tx.send(AppEvent::ConfirmManagedWorktreeRemoval {
                    request: delete_request.clone(),
                    root: root.clone(),
                })
            })],
            dismiss_on_select: true,
            ..Default::default()
        });
        let title = match &entry.owner {
            Owner::Archived(thread) | Owner::Resumable(thread) => {
                format!("工作树：{}", thread.title)
            }
            Owner::None | Owner::Unavailable(_) => "工作树".to_string(),
        };
        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some(title),
            subtitle: Some(entry.cwd.display().to_string()),
            items,
            footer_hint: Some(standard_popup_hint_line()),
            ..Default::default()
        });
    }

    pub(crate) fn confirm_managed_worktree_removal(&mut self, request: Request, root: PathBuf) {
        if !self.worktree_request_is_current(&request) || request.cwd.starts_with(&root) {
            return;
        }
        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("删除此工作树？".to_string()),
            subtitle: Some(root.display().to_string()),
            items: vec![
                SelectionItem {
                    name: "取消".to_string(),
                    actions: vec![],
                    dismiss_on_select: true,
                    ..Default::default()
                },
                SelectionItem {
                    name: "删除工作树".to_string(),
                    description: Some("保留线程历史；可能会影响其他会话".to_string()),
                    actions: vec![Box::new(move |tx| {
                        tx.send(AppEvent::ManagedWorktreeAction {
                            request: request.clone(),
                            action: Action::Remove(root.clone()),
                        })
                    })],
                    dismiss_on_select: true,
                    ..Default::default()
                },
            ],
            footer_hint: Some(standard_popup_hint_line()),
            ..Default::default()
        });
    }
}

fn worktree_updated_ago(updated_at: i64, now: i64) -> String {
    let seconds = now.saturating_sub(updated_at).max(0);
    if seconds < 60 {
        "刚刚".to_string()
    } else if seconds < 3_600 {
        format!("{} 分钟前", seconds / 60)
    } else if seconds < 86_400 {
        format!("{} 小时前", seconds / 3_600)
    } else {
        format!("{} 天前", seconds / 86_400)
    }
}
