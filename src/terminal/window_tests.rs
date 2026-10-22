use super::*;
use ratatui::style::{Color, Modifier};

fn filled(area: Rect) -> Buffer {
    let mut buffer = Buffer::empty(area);
    for cell in &mut buffer.content {
        cell.set_char('!');
    }
    buffer
}

#[test]
fn fresh_windows_and_partial_styles_use_black_without_a_clear() {
    let window = Window::new(3, 2, 0, 0);
    window.set_style(Style::default().fg(Color::Red));
    window.write("x");
    window.put(1, 1, 'y', Style::default().fg(Color::Green));
    let mut frame = filled(Rect::new(0, 0, 3, 2));
    window.render(&mut frame);
    assert_eq!(frame[(0, 0)].fg, Color::Red);
    assert_eq!(frame[(1, 1)].fg, Color::Green);
    for cell in &frame.content {
        assert_eq!(cell.bg, Color::Indexed(16));
    }
}

#[test]
fn local_coordinates_and_clipping_preserve_surrounding_cells() {
    let window = Window::new(3, 2, 2, 1);
    window.clear();
    window.write("abcDEF");
    for (y, x) in [(-1, 0), (0, -1), (2, 0), (0, 3)] {
        window.put(y, x, 'X', Style::default());
        window.move_to(y, x);
        window.write("X");
    }
    // The target clips the window's right edge and bottom row.
    let mut frame = filled(Rect::new(1, 0, 3, 2));
    window.render(&mut frame);
    for (y, row) in ["!!!", "!ab"].into_iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            assert_eq!(frame[(x as u16 + 1, y as u16)].symbol(), ch.to_string());
        }
    }
}

#[test]
fn wrapping_newlines_and_map_glyphs_keep_the_text_cursor() {
    let window = Window::new(4, 3, 0, 0);
    window.write("ab");
    window.put(
        2,
        3,
        '•',
        Style::default()
            .fg(Color::Green)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    window.write("cdE\nF");
    assert_eq!(window.cursor_y(), 2);
    let mut frame = Buffer::empty(Rect::new(0, 0, 4, 3));
    window.render(&mut frame);
    assert_eq!(frame[(3, 2)].fg, Color::Green);
    assert_eq!(frame[(3, 2)].bg, Color::Yellow);
    assert_eq!(frame[(3, 2)].modifier, Modifier::BOLD);
    let rows: Vec<String> = (0..3)
        .map(|y| (0..4).map(|x| frame[(x, y)].symbol()).collect())
        .collect();
    assert_eq!(rows, ["abcd", "E   ", "F  •"]);
    window.move_to(2, 0);
    window.write("lastignored");
    window.render(&mut frame);
    assert_eq!(frame[(3, 2)].symbol(), "t");
    assert_eq!(window.cursor_y(), 3);
}

#[test]
fn clear_removes_old_glyphs_attributes_and_resets_text_state() {
    let window = Window::new(3, 2, 0, 0);
    window.set_style(
        Style::default()
            .fg(Color::Red)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
    );
    window.write("ABCDEF");
    let mut frame = filled(Rect::new(0, 0, 3, 2));
    window.render(&mut frame);
    for cell in &frame.content {
        assert_eq!(cell.fg, Color::Red);
        assert_eq!(cell.bg, Color::Blue);
        assert_eq!(cell.modifier, Modifier::BOLD | Modifier::REVERSED);
    }
    window.clear();
    window.write("x");
    window.render(&mut frame);
    for y in 0..2 {
        for x in 0..3 {
            let cell = &frame[(x, y)];
            assert_eq!(cell.symbol(), if (x, y) == (0, 0) { "x" } else { " " });
            assert_eq!(cell.fg, Color::Indexed(231));
            assert_eq!(cell.bg, Color::Indexed(16));
            assert_eq!(cell.modifier, Modifier::empty());
        }
    }
}

#[test]
fn empty_and_disjoint_regions_are_safe_no_ops() {
    for (width, height) in [(0, 0), (0, 2), (2, 0), (-1, 2), (2, -1)] {
        let window = Window::new(width, height, -2, -3);
        assert_eq!(window.size(), (height.max(0), width.max(0)));
        window.clear();
        window.write("text\nmore");
        window.put(0, 0, 'x', Style::default());
        let mut frame = filled(Rect::new(0, 0, 2, 2));
        let expected = frame.clone();
        window.render(&mut frame);
        assert_eq!(frame, expected);
    }
    let window = Window::new(2, 2, 10, 10);
    window.write("test");
    for area in [Rect::new(0, 0, 2, 2), Rect::new(0, 0, 0, 0)] {
        let mut frame = filled(area);
        let expected = frame.clone();
        window.render(&mut frame);
        assert_eq!(frame, expected);
    }
}

#[test]
fn full_lines_consume_one_newline_and_preserve_explicit_blank_lines() {
    for (text, expected) in [
        ("abc\nZ", ["abc ", "Z   ", "    ", "    "]),
        ("abcd\nZ", ["abcd", "Z   ", "    ", "    "]),
        ("abcde\nZ", ["abcd", "e   ", "Z   ", "    "]),
        ("abcdZ", ["abcd", "Z   ", "    ", "    "]),
        ("abcd\n\nZ", ["abcd", "    ", "Z   ", "    "]),
        ("abcd\n\n\nZ", ["abcd", "    ", "    ", "Z   "]),
        ("•é@b\nZ", ["•é@b", "Z   ", "    ", "    "]),
        ("界界\nZ", ["界 界 ", "Z   ", "    ", "    "]),
    ] {
        let window = Window::new(4, 4, 0, 0);
        // Exercise separate write calls, as the log appends its newline separately.
        for ch in text.chars() {
            window.write(&ch.to_string());
        }
        let mut frame = Buffer::empty(Rect::new(0, 0, 4, 4));
        window.render(&mut frame);
        let rows: Vec<String> = (0..4)
            .map(|y| (0..4).map(|x| frame[(x, y)].symbol()).collect())
            .collect();
        assert_eq!(rows, expected, "{text:?}");
    }
}

#[test]
fn moving_or_clearing_after_a_full_line_resets_newline_handling() {
    for clear in [false, true] {
        let window = Window::new(4, 3, 0, 0);
        window.write("abcd");
        if clear {
            window.clear();
        } else {
            window.move_to(0, 0);
        }
        window.write("\nZ");
        let mut frame = Buffer::empty(Rect::new(0, 0, 4, 3));
        window.render(&mut frame);
        assert_eq!(frame[(0, 1)].symbol(), "Z");
    }
}
