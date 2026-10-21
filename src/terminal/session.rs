use std::cell::RefCell;
use std::io::{self, IsTerminal, Stdout, stdout};
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::cursor::Show;
use crossterm::execute;
use crossterm::style::{Attribute, ResetColor, SetAttribute};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

/// Whether this process currently owns terminal modes that need restoring.
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Owns terminal modes, including during construction failures and unwinding.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}

/// Restore the normal screen before returning errors or printing panic diagnostics.
fn restore() {
    if ACTIVE.swap(false, Ordering::SeqCst) {
        let _ = execute!(
            stdout(),
            SetAttribute(Attribute::Reset),
            ResetColor,
            Show,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }
}

/// A buffered terminal and its raw-mode/alternate-screen lifetime.
pub struct Session {
    /// Ratatui diffs complete frames and writes only changed cells.
    pub terminal: RefCell<Terminal<CrosstermBackend<Stdout>>>,
    /// Restore modes after the renderer is dropped.
    _guard: TerminalGuard,
}

impl Session {
    /// Enter the alternate screen and arrange cleanup before any fallible setup.
    pub fn new() -> io::Result<Self> {
        if !io::stdin().is_terminal() || !stdout().is_terminal() {
            return Err(io::Error::other(
                "interactive stdin and stdout terminals are required",
            ));
        }
        // Terrain and visibility use background colors as game information,
        // not optional decoration (the previous renderer also required colors).
        crossterm::style::force_color_output(true);
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            previous(info);
        }));
        enable_raw_mode()?;
        ACTIVE.store(true, Ordering::SeqCst);
        let guard = TerminalGuard;
        execute!(stdout(), EnterAlternateScreen)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        terminal.hide_cursor()?;
        Ok(Self {
            terminal: RefCell::new(terminal),
            _guard: guard,
        })
    }
}
