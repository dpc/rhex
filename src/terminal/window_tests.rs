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
            assert_eq!(cell.bg, Color::Indexed(232));
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
