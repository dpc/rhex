//! Pipe bridge: Python owns the PTY, vt100 alone interprets its output.

use std::io::{self, BufRead, Write};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn color(color: vt100::Color) -> String {
    match color {
        vt100::Color::Default => "-".into(),
        vt100::Color::Idx(index) => index.to_string(),
        vt100::Color::Rgb(r, g, b) => format!("rgb:{r}:{g}:{b}"),
    }
}

/// One parser per PTY, with zero scrollback and bounded input/dimensions.
#[test]
#[ignore = "pipe helper driven by the Python PTY tests"]
fn terminal_parser_bridge() {
    let mut parser = vt100::Parser::new(36, 120, 0);
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        assert!(line.len() <= 131_080);
        let mut args = line.split_whitespace();
        match args.next().unwrap() {
            "data" => {
                let encoded = args.next().unwrap();
                let data: Vec<_> = encoded
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect();
                parser.process(&data);
            }
            "resize" => {
                let rows = args.next().unwrap().parse::<u16>().unwrap();
                let cols = args.next().unwrap().parse::<u16>().unwrap();
                assert!((1..=100).contains(&rows) && (1..=200).contains(&cols));
                parser.screen_mut().set_size(rows, cols);
            }
            other => panic!("unknown parser command {other}"),
        }
        let screen = parser.screen();
        let (rows, cols) = screen.size();
        let mut cells = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                let cell = screen.cell(row, col).unwrap();
                cells.push(format!(
                    "{},{},{},{},{},{}",
                    hex(cell.contents().as_bytes()),
                    color(cell.fgcolor()),
                    color(cell.bgcolor()),
                    u8::from(cell.bold()),
                    u8::from(cell.is_wide()),
                    u8::from(cell.is_wide_continuation()),
                ));
            }
        }
        println!(
            "SCREEN\t{}\t{}\t{}\t{}\t{},{},{}\t{}",
            rows,
            cols,
            screen.alternate_screen(),
            screen.hide_cursor(),
            color(screen.fgcolor()),
            color(screen.bgcolor()),
            u8::from(screen.bold()),
            cells.join(";")
        );
        io::stdout().flush().unwrap();
    }
}
