//! Exercise the real game through a pseudo-terminal, including terminal cleanup.

#[test]
fn terminal_gameplay_and_exit() {
    let output = std::process::Command::new("python3")
        .args([
            "-c",
            include_str!("terminal.py"),
            env!("CARGO_BIN_EXE_rhex"),
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
