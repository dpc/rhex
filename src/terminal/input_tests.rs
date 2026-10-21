use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::{DOWN, F2, LEFT, RIGHT, UP, game_key};
use crate::terminal::consts::{KEY_ENTER, KEY_ESC};

#[test]
fn accepted_keys_respect_shift_and_event_kind() {
    for kind in [
        KeyEventKind::Press,
        KeyEventKind::Repeat,
        KeyEventKind::Release,
    ] {
        for (code, modifiers, expected) in [
            (KeyCode::Char('a'), KeyModifiers::NONE, 'a' as i32),
            (KeyCode::Char('a'), KeyModifiers::SHIFT, 'A' as i32),
            (KeyCode::Char('A'), KeyModifiers::SHIFT, 'A' as i32),
            (KeyCode::Char('?'), KeyModifiers::SHIFT, '?' as i32),
            (KeyCode::Char('1'), KeyModifiers::SHIFT, '1' as i32),
            (KeyCode::Esc, KeyModifiers::NONE, KEY_ESC),
            (KeyCode::Esc, KeyModifiers::SHIFT, KEY_ESC),
            (KeyCode::Enter, KeyModifiers::NONE, KEY_ENTER),
            (KeyCode::Enter, KeyModifiers::SHIFT, KEY_ENTER),
            (KeyCode::Left, KeyModifiers::NONE, LEFT),
            (KeyCode::Right, KeyModifiers::NONE, RIGHT),
            (KeyCode::Up, KeyModifiers::NONE, UP),
            (KeyCode::Down, KeyModifiers::NONE, DOWN),
            (KeyCode::F(2), KeyModifiers::NONE, F2),
        ] {
            assert_eq!(
                game_key(KeyEvent::new_with_kind(code, modifiers, kind)),
                (kind != KeyEventKind::Release).then_some(expected),
                "{code:?} {modifiers:?} {kind:?}"
            );
        }
    }
}

#[test]
fn shortcuts_and_unsupported_keys_cannot_be_game_input() {
    for code in [
        KeyCode::Char('h'),
        KeyCode::Char('I'),
        KeyCode::Esc,
        KeyCode::Enter,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::F(2),
    ] {
        for forbidden in [
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
            KeyModifiers::SUPER,
            KeyModifiers::HYPER,
            KeyModifiers::META,
        ] {
            for modifiers in [forbidden, forbidden | KeyModifiers::SHIFT] {
                for kind in [
                    KeyEventKind::Press,
                    KeyEventKind::Repeat,
                    KeyEventKind::Release,
                ] {
                    assert_eq!(
                        game_key(KeyEvent::new_with_kind(code, modifiers, kind)),
                        None,
                        "{code:?} {modifiers:?} {kind:?}"
                    );
                }
            }
        }
    }
    for code in [
        KeyCode::Backspace,
        KeyCode::Delete,
        KeyCode::Insert,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Null,
        KeyCode::F(1),
        KeyCode::F(3),
        KeyCode::F(12),
    ] {
        for modifiers in [KeyModifiers::NONE, KeyModifiers::SHIFT] {
            assert_eq!(game_key(KeyEvent::new(code, modifiers)), None, "{code:?}");
        }
    }
    for code in [
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::F(2),
    ] {
        assert_eq!(game_key(KeyEvent::new(code, KeyModifiers::SHIFT)), None);
    }
}

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
