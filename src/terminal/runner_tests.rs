use super::*;

#[test]
fn ui_draw_failure_restores_terminal_before_diagnostics() {
    let output = std::process::Command::new("python3")
        .args([
            "-c",
            include_str!("../../tests/terminal.py"),
            "--runner-probes",
        ])
        .arg(std::env::current_exe().unwrap())
        .output()
        .expect("Python 3 is required for PTY tests");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Process-isolated real UI loop and backend failure; never run without a PTY.
#[test]
#[ignore = "subprocess driven by ui_draw_failure_restores_terminal_before_diagnostics"]
fn ui_draw_failure_probe() {
    let session = Session::new().unwrap();
    session
        .terminal
        .borrow_mut()
        .draw(|frame| {
            frame
                .buffer_mut()
                .set_string(0, 0, "Session ready", ratatui::style::Style::default());
        })
        .unwrap();
    event::read().unwrap();
    session.fail_next_write();
    let mut runner = Runner {
        session,
        ui: Ui::new(
            Engine::fixture(super::super::fixtures::room()),
            (36, 120),
            Instant::now(),
        ),
    };
    // Runner::run calls Ui::render_into and Terminal::draw, whose actual writer fails.
    let error = runner.run().unwrap_err();
    assert_eq!(error.to_string(), "injected UI draw error");
    drop(runner);
    eprintln!("{error}");
}
