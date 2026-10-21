use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::consts::{KEY_ENTER, KEY_ESC};

/// Non-character game key codes live above the Unicode range.
pub const LEFT: i32 = 0x110000;
/// Turn right.
pub const RIGHT: i32 = LEFT + 1;
/// Move forward.
pub const UP: i32 = LEFT + 2;
/// Move backward.
pub const DOWN: i32 = LEFT + 3;
/// Toggle normal/Vi controls.
pub const F2: i32 = LEFT + 4;

/// Translate terminal events without interpreting Alt/Ctrl shortcuts as game moves.
pub fn game_key(key: KeyEvent) -> Option<i32> {
    if key.kind == KeyEventKind::Release
        || key.modifiers.intersects(
            KeyModifiers::CONTROL
                | KeyModifiers::ALT
                | KeyModifiers::SUPER
                | KeyModifiers::HYPER
                | KeyModifiers::META,
        )
    {
        return None;
    }
    Some(match key.code {
        KeyCode::Char(ch) if ch.is_ascii() => {
            if key.modifiers.contains(KeyModifiers::SHIFT) && ch.is_ascii_lowercase() {
                ch.to_ascii_uppercase() as i32
            } else {
                ch as i32
            }
        }
        KeyCode::Esc => KEY_ESC,
        KeyCode::Enter => KEY_ENTER,
        KeyCode::Left if key.modifiers.is_empty() => LEFT,
        KeyCode::Right if key.modifiers.is_empty() => RIGHT,
        KeyCode::Up if key.modifiers.is_empty() => UP,
        KeyCode::Down if key.modifiers.is_empty() => DOWN,
        KeyCode::F(2) if key.modifiers.is_empty() => F2,
        _ => return None,
    })
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
