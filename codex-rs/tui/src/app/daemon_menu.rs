//! Local daemon maintenance only records an update action after explicit confirmation.
//! The CLI executes it in the foreground after the TUI restores the terminal.

use super::*;
use crate::bottom_pane::SelectionItem;
use crate::bottom_pane::SelectionViewParams;
use crate::update_action::DaemonUpdateSource;
use crate::wrapping::word_wrap_lines;
use ratatui::buffer::Buffer;
use ratatui::widgets::Paragraph;

struct DaemonMenuHeader(Vec<Line<'static>>);

impl Renderable for DaemonMenuHeader {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        Renderable::render(
            &Paragraph::new(word_wrap_lines(&self.0, usize::from(area.width))),
            area,
            buf,
        );
    }

    fn desired_height(&self, width: u16) -> u16 {
        word_wrap_lines(&self.0, usize::from(width)).len() as u16
    }
}

impl App {
    pub(super) fn open_daemon_menu(&mut self) {
        let status = self
            .chat_widget
            .remote_connection
            .as_ref()
            .filter(|_| matches!(self.app_server_target, AppServerTarget::LocalDaemon { .. }))
            .map(|connection| format!("后台服务正在运行：{}", connection.version))
            .unwrap_or_else(|| "未连接到本地后台服务器。".to_string());
        let mut header = vec![Line::from("后台服务".bold()), Line::from(status.dim())];
        let unavailable = if matches!(self.app_server_target, AppServerTarget::Remote { .. }) {
            Some("请在主机上管理此服务器。远程连接无法执行本地后台服务更新。")
        } else if self.daemon_cli_executable.is_none() {
            Some("运行 Codex CLI，即可从此菜单管理后台服务。")
        } else {
            None
        };
        if let Some(guidance) = unavailable {
            header.push(Line::from(guidance.dim()));
        }
        let has_package = self.daemon_cli_executable.as_ref().is_some_and(|path| {
            codex_install_context::InstallContext::from_exe(
                cfg!(target_os = "macos"),
                Some(path.as_path()),
                /*method_override*/ None,
            )
            .package_layout
            .is_some()
        });
        let items = [
            (DaemonUpdateSource::PublicStable, "安装最新公开稳定版"),
            (DaemonUpdateSource::ThisCli, "使用当前 CLI 构建"),
        ]
        .into_iter()
        .map(|(source, name)| SelectionItem {
            name: name.to_string(),
            is_disabled: unavailable.is_some(),
            disabled_reason: (unavailable.is_none()
                && source == DaemonUpdateSource::ThisCli
                && !has_package)
                .then(|| "此 CLI 没有可复制的本地软件包。".to_string()),
            actions: vec![Box::new(move |tx| {
                tx.send(AppEvent::ConfirmDaemonUpdate(source));
            })],
            dismiss_on_select: true,
            ..Default::default()
        })
        .collect();
        self.chat_widget.show_selection_view(SelectionViewParams {
            header: Box::new(DaemonMenuHeader(header)),
            items,
            ..Default::default()
        });
    }

    pub(super) fn confirm_daemon_update(&mut self, source: DaemonUpdateSource) {
        let Some(executable) = &self.daemon_cli_executable else {
            return;
        };
        if matches!(self.app_server_target, AppServerTarget::Remote { .. }) {
            return;
        }
        let mut explanation = match source {
            DaemonUpdateSource::PublicStable => "安装最新公开稳定版（运行命令时解析版本）。这将恢复正式版更新资格，并保留自动更新设置。".to_string(),
            DaemonUpdateSource::ThisCli => {
                let version = codex_install_context::InstallContext::current()
                    .package_manifest()
                    .map_or_else(|| CODEX_CLI_VERSION.to_string(), |manifest| manifest.version.to_string());
                format!("使用来自 {} 的当前 CLI 软件包 v{version}。完整的本地软件包将被复制并固定，不再自动更新。", executable.display())
            }
        };
        explanation.push_str("\n后台服务会在需要时重启，正在进行或排队的工作可能被中断。\nCodex 将退出并在此终端中执行更新，随后返回 shell。之后请重新启动 Codex。");
        let mut header = vec![Line::from("更新后台服务并退出 Codex？".bold())];
        header.extend(explanation.lines().map(|line| Line::from(line.to_owned())));
        self.chat_widget.show_selection_view(SelectionViewParams {
            header: Box::new(DaemonMenuHeader(header)),
            items: vec![
                SelectionItem {
                    name: "取消".to_string(),
                    dismiss_on_select: true,
                    ..Default::default()
                },
                SelectionItem {
                    name: "更新并退出".to_string(),
                    actions: vec![Box::new(move |tx| {
                        tx.send(AppEvent::RunDaemonUpdate(source))
                    })],
                    require_explicit_confirmation: true,
                    dismiss_on_select: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
    }
}

#[cfg(test)]
#[path = "daemon_menu_tests.rs"]
mod tests;
