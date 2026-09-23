//! Shared entry form for adding a third-party model provider.
//!
//! Both the onboarding auth step and the `/provider` popup embed this form, so
//! it stays independent of either host: it owns its own error state (the
//! `Renderable::render` signature has nowhere to pass one in) and it avoids the
//! onboarding-private `keys` module.

use codex_utils_redacted_string::RedactedString;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use crate::key_hint;
use crate::render::renderable::Renderable;
use crate::wrapping::word_wrap_lines;

/// A single text field, listed in tab order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProviderField {
    Id,
    Name,
    BaseUrl,
    ApiKey,
    Model,
}

const FIELDS: [ProviderField; 5] = [
    ProviderField::Id,
    ProviderField::Name,
    ProviderField::BaseUrl,
    ProviderField::ApiKey,
    ProviderField::Model,
];

const FIELD_COUNT: usize = FIELDS.len();

impl ProviderField {
    const fn index(self) -> usize {
        match self {
            Self::Id => 0,
            Self::Name => 1,
            Self::BaseUrl => 2,
            Self::ApiKey => 3,
            Self::Model => 4,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Id => "标识符",
            Self::Name => "显示名称",
            Self::BaseUrl => "基础 URL",
            Self::ApiKey => "API 密钥（可选）",
            Self::Model => "模型",
        }
    }

    fn placeholder(self) -> &'static str {
        match self {
            Self::Id => "my-proxy",
            Self::Name => "My Proxy",
            Self::BaseUrl => "https://example.com/v1",
            Self::ApiKey => "留空则不写入密钥",
            Self::Model => "provider 支持的模型名",
        }
    }

    fn is_secret(self) -> bool {
        matches!(self, Self::ApiKey)
    }

    fn is_required(self) -> bool {
        !matches!(self, Self::ApiKey)
    }
}

/// Values collected once every required field is filled.
///
/// `api_key` is a [`RedactedString`] because the draft travels through
/// `AppEvent`, which derives `Debug` and can end up in traces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProviderDraft {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: Option<RedactedString>,
    pub model: String,
}

/// Outcome of a key event that ends the form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ProviderFormAction {
    Submitted(ProviderDraft),
    Cancelled,
}

/// Editable state for the provider entry form.
#[derive(Clone, Default)]
pub(crate) struct ProviderForm {
    values: [String; FIELD_COUNT],
    selected_field: usize,
    error: Option<String>,
}

impl ProviderForm {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Restores a draft the server rejected so the user edits the offending
    /// field instead of retyping all five.
    pub(crate) fn from_draft(draft: ProviderDraft) -> Self {
        let mut form = Self::new();
        form.values[ProviderField::Id.index()] = draft.id;
        form.values[ProviderField::Name.index()] = draft.name;
        form.values[ProviderField::BaseUrl.index()] = draft.base_url;
        form.values[ProviderField::ApiKey.index()] = draft
            .api_key
            .map(RedactedString::into_inner)
            .unwrap_or_default();
        form.values[ProviderField::Model.index()] = draft.model;
        form
    }

    pub(crate) fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }

    pub(crate) fn handle_key_event(&mut self, key_event: KeyEvent) -> Option<ProviderFormAction> {
        match key_event.code {
            KeyCode::Esc => return Some(ProviderFormAction::Cancelled),
            KeyCode::Up | KeyCode::BackTab => self.move_selection(/*delta*/ FIELD_COUNT - 1),
            KeyCode::Down | KeyCode::Tab => self.move_selection(/*delta*/ 1),
            KeyCode::Enter => return self.confirm(),
            KeyCode::Backspace => {
                self.values[self.selected_field].pop();
                self.error = None;
            }
            KeyCode::Char(character)
                if matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat)
                    && !key_event.modifiers.intersects(
                        KeyModifiers::SUPER | KeyModifiers::CONTROL | KeyModifiers::ALT,
                    ) =>
            {
                self.values[self.selected_field].push(character);
                self.error = None;
            }
            _ => {}
        }
        None
    }

    pub(crate) fn handle_paste(&mut self, pasted: &str) -> bool {
        let pasted = pasted.trim();
        if pasted.is_empty() {
            return false;
        }
        self.values[self.selected_field].push_str(pasted);
        self.error = None;
        true
    }

    fn move_selection(&mut self, delta: usize) {
        self.selected_field = (self.selected_field + delta) % FIELD_COUNT;
    }

    fn value(&self, field: ProviderField) -> &str {
        self.values[field.index()].trim()
    }

    /// Enter walks the form and only submits from the last field, so a partially
    /// filled form cannot be written to `config.toml` by an accidental keypress.
    fn confirm(&mut self) -> Option<ProviderFormAction> {
        if self.selected_field + 1 < FIELD_COUNT {
            let field = FIELDS[self.selected_field];
            if field.is_required() && self.values[self.selected_field].trim().is_empty() {
                let label = field.label();
                self.error = Some(format!("{label}不能为空。"));
                return None;
            }
            self.selected_field += 1;
            return None;
        }

        match self.validate() {
            Ok(draft) => Some(ProviderFormAction::Submitted(draft)),
            Err((field, message)) => {
                self.selected_field = field.index();
                self.error = Some(message);
                None
            }
        }
    }

    /// Only checks what the server cannot report back usefully. Reserved ids,
    /// duplicate names, and malformed URLs surface as `ConfigValidationError`
    /// from `config/batchWrite` instead.
    fn validate(&self) -> Result<ProviderDraft, (ProviderField, String)> {
        for field in FIELDS {
            if field.is_required() && self.value(field).is_empty() {
                let label = field.label();
                return Err((field, format!("{label}不能为空。")));
            }
        }

        // Limited to TOML bare-key characters so the id never needs quoting in
        // `config.toml` or in a `-c model_providers.<id>...` override.
        let id = self.value(ProviderField::Id);
        if !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        {
            return Err((
                ProviderField::Id,
                "标识符只能包含英文字母、数字、- 和 _。".to_string(),
            ));
        }

        let api_key = self.value(ProviderField::ApiKey);
        Ok(ProviderDraft {
            id: id.to_string(),
            name: self.value(ProviderField::Name).to_string(),
            base_url: self.value(ProviderField::BaseUrl).to_string(),
            api_key: (!api_key.is_empty()).then(|| RedactedString::from(api_key)),
            model: self.value(ProviderField::Model).to_string(),
        })
    }

    fn display_value(&self, field: ProviderField) -> Line<'static> {
        let value = &self.values[field.index()];
        if value.is_empty() {
            return field.placeholder().dim().into();
        }
        if !field.is_secret() {
            return value.clone().into();
        }
        // Reveal only the most recent character, and only while focused.
        let mut masked = "•".repeat(value.chars().count().saturating_sub(1));
        if field.index() == self.selected_field
            && let Some(character) = value.chars().last()
        {
            masked.push(character);
        } else {
            masked.push('•');
        }
        masked.into()
    }

    fn lines(&self) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = vec![
            Line::from(vec!["> ".into(), "添加第三方模型提供商".bold()]),
            "".into(),
            "  填写 OpenAI 兼容接口的连接信息。".into(),
            "".into(),
        ];

        for field in FIELDS {
            let selected = field.index() == self.selected_field;
            let marker = if selected { ">" } else { " " };
            let label = field.label();
            let mut spans = vec![format!("{marker} {label}: ").into()];
            spans.extend(self.display_value(field).spans);
            let line = Line::from(spans);
            lines.push(if selected { line.cyan() } else { line });
        }

        lines.push("".into());
        lines.push("  保存后对新会话生效。".dim().into());
        lines.push("".into());
        lines.push(Line::from(vec![
            "  按 ".dim(),
            key_hint::plain(KeyCode::Enter).into(),
            " 继续，".dim(),
            key_hint::plain(KeyCode::Tab).into(),
            " 切换字段，".dim(),
            key_hint::plain(KeyCode::Esc).into(),
            " 返回".dim(),
        ]));

        if let Some(error) = &self.error {
            lines.push("".into());
            lines.push(error.clone().red().into());
        }

        lines
    }
}

impl Renderable for ProviderForm {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let lines = word_wrap_lines(self.lines(), usize::from(area.width));
        Paragraph::new(lines).render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        let lines = word_wrap_lines(self.lines(), usize::from(width));
        u16::try_from(lines.len()).unwrap_or(u16::MAX)
    }
}

#[cfg(test)]
#[path = "provider_form_tests.rs"]
mod tests;
