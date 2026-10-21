use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::game_key;

#[test]
fn unsupported_unicode_cannot_select_inventory_letters() {
    // Item commands use ASCII letters. In particular these scalars must not
    // reach the item dispatcher: narrowing them to u8 would select a/A.
    for ch in ['š', 'Ł', '🡡'] {
        assert_eq!(
            game_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)),
            None
        );
    }
    assert_eq!(
        game_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
        Some('a' as i32)
    );
}
