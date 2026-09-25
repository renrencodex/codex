//! Destination and filename prompts for on-demand transcript exports.

use super::*;
use crate::app_event::TranscriptExportDestination;
use crate::bottom_pane::popup_consts::picker_hint_line_for_keymap;

impl ChatWidget {
    pub(crate) fn copy_transcript_to_clipboard(&mut self, markdown: &str) {
        match crate::clipboard_copy::copy_to_clipboard(
            markdown,
            crate::clipboard_copy::CopyFormat::PlainText,
        ) {
            Ok(outcome) => {
                let status = outcome.store(&mut self.clipboard_lease);
                self.add_info_message(status.message("conversation"), /*hint*/ None);
            }
            Err(error) => self.add_error_message(format!("复制失败：{error}")),
        }
    }

    pub(super) fn show_transcript_export_popup(&mut self) {
        self.show_selection_view(SelectionViewParams {
            header: Box::new(
                Paragraph::new(vec![
                    Line::from("导出对话".bold()),
                    Line::from("将完整对话保存为 Markdown".dim()),
                ])
                .wrap(Wrap { trim: false }),
            ),
            footer_hint: Some(picker_hint_line_for_keymap(&self.bottom_pane.list_keymap())),
            items: vec![
                SelectionItem {
                    name: "复制到剪贴板".to_string(),
                    description: Some("复制完整的 Markdown 对话记录".to_string()),
                    is_disabled: cfg!(target_os = "android"),
                    actions: vec![Box::new(|tx| {
                        tx.send(AppEvent::ExportTranscript {
                            destination: TranscriptExportDestination::Clipboard,
                        });
                    })],
                    dismiss_on_select: true,
                    ..Default::default()
                },
                SelectionItem {
                    name: "保存到文件".to_string(),
                    description: Some("选择 Markdown 文件名".to_string()),
                    actions: vec![Box::new(|tx| {
                        tx.send(AppEvent::OpenTranscriptExportFilePrompt);
                    })],
                    dismiss_on_select: true,
                    ..Default::default()
                },
            ],
            ..SelectionViewParams::picker()
        });
        self.defer_input_until_settings_applied();
        self.request_redraw();
    }

    pub(crate) fn show_transcript_export_file_prompt(&mut self) {
        let tx = self.app_event_tx.clone();
        let filename = self.thread_id().map_or_else(
            || "codex-session.md".to_string(),
            |thread_id| format!("codex-session-{thread_id}.md"),
        );
        let view = CustomPromptView::new(
            "保存对话".to_string(),
            "输入文件名并按 Enter".to_string(),
            filename,
            /*context_label*/ None,
            Box::new(move |filename| {
                tx.send(AppEvent::ExportTranscript {
                    destination: TranscriptExportDestination::File(PathBuf::from(filename)),
                });
            }),
        );
        self.bottom_pane.show_text_prompt(view);
        self.request_redraw();
    }
}
