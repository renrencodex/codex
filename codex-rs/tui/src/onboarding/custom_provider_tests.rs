use super::*;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use pretty_assertions::assert_eq;

fn render_visible(state: &CustomProviderState) -> String {
    let area = Rect::new(0, 0, 72, 24);
    let mut buffer = Buffer::empty(area);
    state.render(area, &mut buffer);
    let mut rows = (area.top()..area.bottom())
        .map(|row| {
            (area.left()..area.right())
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>();
    while rows.last().is_some_and(String::is_empty) {
        rows.pop();
    }
    rows.join("\n")
}

#[test]
fn the_form_is_shown_until_a_save_is_in_flight() {
    let mut state = CustomProviderState::new();

    insta::assert_snapshot!("custom_provider_form", render_visible(&state));
    assert_eq!(state.is_text_entry_active(), true);

    state.pending_save = Some(Uuid::nil());

    insta::assert_snapshot!("custom_provider_saving", render_visible(&state));
    assert_eq!(state.is_text_entry_active(), false);
}

#[test]
fn typing_reaches_the_embedded_form() {
    let mut state = CustomProviderState::new();

    state
        .form
        .handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

    assert!(render_visible(&state).contains('x'));
}
