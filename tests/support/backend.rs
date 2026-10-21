//! Deterministic real-backend output tests, without terminal ownership or a PTY.

use crossterm::{cursor, execute, style, terminal};
use ratatui::{
    Terminal, TerminalOptions, Viewport,
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
};
use std::{cell::RefCell, io, io::Write, rc::Rc};

/// Share captured bytes without enabling Ratatui's unstable writer accessors.
#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn backend_diffs_clear_stale_cells_styles_and_wide_continuations() {
    // Match Session startup even when the invoking shell sets NO_COLOR.
    style::force_color_output(true);
    let output = Capture::default();
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(output.clone()),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, 16, 4)),
        },
    )
    .unwrap();
    execute!(
        terminal.backend_mut(),
        terminal::EnterAlternateScreen,
        cursor::Hide
    )
    .unwrap();
    let mut parser = vt100::Parser::new(4, 16, 0);
    let mut offset = 0;
    for phase in 0..3 {
        terminal
            .draw(|frame| {
                let buffer = frame.buffer_mut();
                if phase == 0 {
                    buffer.set_string(
                        1,
                        1,
                        "•×界 stale",
                        Style::default()
                            .fg(Color::Indexed(94))
                            .bg(Color::Indexed(232))
                            .add_modifier(Modifier::BOLD),
                    );
                } else if phase == 1 {
                    buffer.set_string(1, 1, "ok", Style::default());
                } else {
                    buffer.set_string(3, 1, "X", Style::default().fg(Color::Indexed(2)));
                }
            })
            .unwrap();
        let bytes = output.0.borrow();
        // Deliberately split every UTF-8 scalar and escape sequence at every byte.
        for byte in &bytes[offset..] {
            parser.process(std::slice::from_ref(byte));
        }
        offset = bytes.len();
        let screen = parser.screen();
        assert!(screen.alternate_screen());
        assert!(screen.hide_cursor());
        let first = screen.cell(1, 1).unwrap();
        if phase == 0 {
            assert_eq!(first.contents(), "•");
            assert_eq!(first.fgcolor(), vt100::Color::Idx(94));
            assert_eq!(first.bgcolor(), vt100::Color::Idx(232));
            assert!(first.bold());
            assert_eq!(screen.cell(1, 2).unwrap().contents(), "×");
            assert!(screen.cell(1, 3).unwrap().is_wide());
            assert!(screen.cell(1, 4).unwrap().is_wide_continuation());
        } else {
            assert_eq!(first.contents(), if phase == 1 { "o" } else { " " });
            assert_eq!(first.fgcolor(), vt100::Color::Default);
            if phase == 2 {
                let replacement = screen.cell(1, 3).unwrap();
                assert_eq!(replacement.contents(), "X");
                assert_eq!(replacement.fgcolor(), vt100::Color::Idx(2));
            }
            for col in 1..12 {
                let cell = screen.cell(1, col).unwrap();
                assert!(!cell.bold(), "stale bold at column {col}, phase {phase}");
                assert_eq!(cell.bgcolor(), vt100::Color::Default);
                assert!(!cell.is_wide() && !cell.is_wide_continuation());
                if col >= 4 {
                    assert!(cell.contents().trim().is_empty(), "stale text at {col}");
                }
            }
        }
    }
    execute!(
        terminal.backend_mut(),
        style::SetAttribute(style::Attribute::Reset),
        style::ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    )
    .unwrap();
    let bytes = output.0.borrow();
    parser.process(&bytes[offset..]);
    assert!(!parser.screen().alternate_screen());
    assert!(!parser.screen().hide_cursor());
    assert_eq!(parser.screen().fgcolor(), vt100::Color::Default);
    assert_eq!(parser.screen().bgcolor(), vt100::Color::Default);
}
