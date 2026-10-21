use std::cell::RefCell;
use std::io::{self, IsTerminal, Write, stdout};
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
    pub terminal: RefCell<Terminal<CrosstermBackend<Output>>>,
    /// Restore modes after the renderer is dropped.
    _guard: TerminalGuard,
    /// Shared one-shot backend failure switch, absent from production builds.
    #[cfg(test)]
    #[allow(dead_code)] // The integration target reuses Session without the UI runner.
    fail_write: std::rc::Rc<std::cell::Cell<bool>>,
}

/// Standard output with a test-only, one-shot write failure at the real backend boundary.
pub struct Output {
    /// Fail the next backend write without interfering with guard cleanup.
    #[cfg(test)]
    fail_next_write: std::rc::Rc<std::cell::Cell<bool>>,
}

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        #[cfg(test)]
        if self.fail_next_write.replace(false) {
            return Err(io::Error::other("injected UI draw error"));
        }
        stdout().write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        stdout().flush()
    }
}

impl Session {
    /// Fail one real backend write; cleanup still uses unmodified standard output.
    #[cfg(test)]
    #[allow(dead_code)] // Used by the native UI probe, not the standalone session probes.
    pub(crate) fn fail_next_write(&self) {
        self.fail_write.set(true);
    }

    /// Enter the alternate screen and arrange cleanup before any fallible setup.
    pub fn new() -> io::Result<Self> {
        Self::with_setup(|| Ok(()))
    }

    /// Run a fallible setup step after acquiring modes, under the same guard.
    pub(crate) fn with_setup(setup: impl FnOnce() -> io::Result<()>) -> io::Result<Self> {
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
        setup()?;
        #[cfg(test)]
        let fail_write = std::rc::Rc::new(std::cell::Cell::new(false));
        let mut terminal = Terminal::new(CrosstermBackend::new(Output {
            #[cfg(test)]
            fail_next_write: fail_write.clone(),
        }))?;
        terminal.hide_cursor()?;
        Ok(Self {
            terminal: RefCell::new(terminal),
            _guard: guard,
            #[cfg(test)]
            fail_write,
        })
    }
}
