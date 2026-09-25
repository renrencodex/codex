//! Shared popup-related constants for bottom pane widgets.

use ratatui::style::Stylize;
use ratatui::text::Line;

use crate::key_hint;
use crate::key_hint::ShortcutHint;
use crate::keymap::ListAction;
use crate::keymap::ListKeymap;
use crossterm::event::KeyCode;

/// Maximum number of rows any popup should attempt to display.
/// Keep this consistent across all popups for a uniform feel.
pub(crate) const MAX_POPUP_ROWS: usize = 8;

/// Standard footer hint text used by popups.
pub(crate) fn standard_popup_hint_line() -> Line<'static> {
    Line::from(vec![
        "按 ".into(),
        key_hint::plain(KeyCode::Enter).into(),
        " 确认，或按 ".into(),
        key_hint::plain(KeyCode::Esc).into(),
        " 返回".into(),
    ])
}

/// Compact footer for shared pickers, using only the configured list actions.
pub(crate) fn picker_hint_line_for_keymap(list_keymap: &ListKeymap) -> Line<'static> {
    let mut spans = Vec::new();
    for (action, label) in [(ListAction::Accept, "选择"), (ListAction::Cancel, "返回")] {
        if let Some(hint) = list_keymap.primary_hint(action) {
            if !spans.is_empty() {
                spans.push(" · ".dim());
            }
            spans.extend(hint.spans());
            spans.push(format!(" {label}").dim());
        }
    }
    spans.into()
}

pub(crate) fn accept_cancel_hint_line(
    accept: Option<ShortcutHint>,
    accept_label: &'static str,
    cancel: Option<ShortcutHint>,
    cancel_label: &'static str,
) -> Line<'static> {
    let mut spans = Vec::new();
    if let Some(accept) = accept {
        spans.push("按 ".dim());
        spans.extend(accept.spans());
        spans.push(format!(" {accept_label}").dim());
    }
    if let Some(cancel) = cancel {
        spans.push(if spans.is_empty() { "按 " } else { " 或 " }.dim());
        spans.extend(cancel.spans());
        spans.push(format!(" {cancel_label}").dim());
    }
    spans.into()
}
