//! Exercise the real game through a pseudo-terminal, including terminal cleanup.

#![cfg(unix)]

#[path = "../src/terminal/session.rs"]
mod session;

#[path = "support/backend.rs"]
mod backend;
#[path = "support/parser.rs"]
mod parser;

#[test]
fn terminal_gameplay_and_exit() {
    let output = std::process::Command::new("python3")
        .args([
            "-c",
            include_str!("terminal.py"),
            env!("CARGO_BIN_EXE_rhex"),
            std::env::current_exe().unwrap().to_str().unwrap(),
        ])
        .output()
        .expect("Python 3 is required for the PTY integration test (provided by nix develop)");
    assert!(
        output.status.success(),
        "PTY test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Run only under the PTY driver to exercise production cleanup on failure.
#[test]
#[ignore = "subprocess probe driven by terminal_gameplay_and_exit"]
fn terminal_session_failure_probe() {
    if std::env::var_os("RHEX_TEST_PARTIAL_SETUP").is_some() {
        let result = session::Session::with_setup(|| {
            Err(std::io::Error::other("injected partial setup error"))
        });
        let error = result.err().expect("partial setup must fail");
        assert_eq!(error.to_string(), "injected partial setup error");
        eprintln!("{error}");
        return;
    }
    fn fail_session() -> std::io::Result<()> {
        let session = session::Session::new()?;
        session.terminal.borrow_mut().draw(|frame| {
            frame
                .buffer_mut()
                .set_string(0, 0, "Session ready", ratatui::style::Style::default());
        })?;
        if let crossterm::event::Event::Key(key) = crossterm::event::read()?
            && key.code == crossterm::event::KeyCode::Char('p')
        {
            panic!("injected terminal panic");
        }
        Err(std::io::Error::other("injected terminal error"))
    }
    assert!(fail_session().is_err());
    eprintln!("injected terminal error");
}
