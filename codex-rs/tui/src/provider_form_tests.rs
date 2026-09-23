use super::*;
use pretty_assertions::assert_eq;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn type_text(form: &mut ProviderForm, text: &str) -> Option<ProviderFormAction> {
    let mut action = None;
    for character in text.chars() {
        action = form.handle_key_event(key(KeyCode::Char(character)));
    }
    action
}

/// Fills every field and returns the action produced by the final Enter.
fn fill_and_submit(api_key: &str) -> Option<ProviderFormAction> {
    let mut form = ProviderForm::new();
    for value in [
        "my-proxy",
        "My Proxy",
        "https://example.com/v1",
        api_key,
        "my-model",
    ] {
        type_text(&mut form, value);
        if let Some(action) = form.handle_key_event(key(KeyCode::Enter)) {
            return Some(action);
        }
    }
    None
}

fn render_to_string(form: &ProviderForm, width: u16) -> String {
    let height = form.desired_height(width);
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    form.render(area, &mut buf);
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn submits_a_draft_once_every_required_field_is_filled() {
    assert_eq!(
        fill_and_submit("secret-key"),
        Some(ProviderFormAction::Submitted(ProviderDraft {
            id: "my-proxy".to_string(),
            name: "My Proxy".to_string(),
            base_url: "https://example.com/v1".to_string(),
            api_key: Some(RedactedString::from("secret-key")),
            model: "my-model".to_string(),
        }))
    );
}

#[test]
fn an_empty_api_key_is_omitted_from_the_draft() {
    assert_eq!(
        fill_and_submit(""),
        Some(ProviderFormAction::Submitted(ProviderDraft {
            id: "my-proxy".to_string(),
            name: "My Proxy".to_string(),
            base_url: "https://example.com/v1".to_string(),
            api_key: None,
            model: "my-model".to_string(),
        }))
    );
}

#[test]
fn enter_does_not_advance_past_an_empty_required_field() {
    let mut form = ProviderForm::new();

    assert_eq!(form.handle_key_event(key(KeyCode::Enter)), None);
    assert_eq!(form.error, Some("标识符不能为空。".to_string()));
    // Still on the id field, so typed characters land there.
    type_text(&mut form, "my-proxy");

    assert_eq!(form.value(ProviderField::Id), "my-proxy");
    assert_eq!(form.value(ProviderField::Name), "");
}

#[test]
fn a_rejected_draft_is_restored_into_the_form() {
    let draft = ProviderDraft {
        id: "my-proxy".to_string(),
        name: "My Proxy".to_string(),
        base_url: "https://example.com/v1".to_string(),
        api_key: Some(RedactedString::from("secret-key")),
        model: "my-model".to_string(),
    };

    let mut form = ProviderForm::from_draft(draft.clone());
    for _ in 1..FIELD_COUNT {
        assert_eq!(form.handle_key_event(key(KeyCode::Enter)), None);
    }

    assert_eq!(
        form.handle_key_event(key(KeyCode::Enter)),
        Some(ProviderFormAction::Submitted(draft))
    );
}

#[test]
fn tab_cycles_through_every_field_and_wraps() {
    let mut form = ProviderForm::new();

    type_text(&mut form, "my-proxy");
    for _ in 0..FIELD_COUNT {
        form.handle_key_event(key(KeyCode::Tab));
    }
    type_text(&mut form, "-suffix");

    assert_eq!(form.value(ProviderField::Id), "my-proxy-suffix");
}

#[test]
fn shift_tab_moves_to_the_previous_field() {
    let mut form = ProviderForm::new();

    form.handle_key_event(key(KeyCode::Tab));
    type_text(&mut form, "My Proxy");
    form.handle_key_event(key(KeyCode::BackTab));
    type_text(&mut form, "my-proxy");

    assert_eq!(form.value(ProviderField::Id), "my-proxy");
    assert_eq!(form.value(ProviderField::Name), "My Proxy");
}

#[test]
fn an_id_containing_a_dot_is_rejected_before_submitting() {
    let mut form = ProviderForm::new();
    for value in ["my.proxy", "My Proxy", "https://example.com/v1", "", "m"] {
        type_text(&mut form, value);
        assert_eq!(form.handle_key_event(key(KeyCode::Enter)), None);
    }

    assert_eq!(
        form.error,
        Some("标识符不能包含 . 或 \" 字符。".to_string())
    );
}

#[test]
fn esc_cancels_the_form() {
    let mut form = ProviderForm::new();

    assert_eq!(
        form.handle_key_event(key(KeyCode::Esc)),
        Some(ProviderFormAction::Cancelled)
    );
}

#[test]
fn pasting_appends_to_the_focused_field() {
    let mut form = ProviderForm::new();

    assert!(form.handle_paste("  my-proxy  "));
    assert!(!form.handle_paste("   "));

    assert_eq!(form.value(ProviderField::Id), "my-proxy");
}

#[test]
fn the_api_key_is_masked_while_the_field_has_focus() {
    let mut form = ProviderForm::new();
    for _ in 0..ProviderField::ApiKey.index() {
        form.handle_key_event(key(KeyCode::Tab));
    }
    type_text(&mut form, "secret");

    let rendered = render_to_string(&form, /*width*/ 60);

    assert!(
        rendered.contains("•••••t"),
        "expected a masked api key, got:\n{rendered}"
    );
    assert!(
        !rendered.contains("secret"),
        "expected the api key to stay hidden, got:\n{rendered}"
    );
}
