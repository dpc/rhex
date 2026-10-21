# rhex

Rhex is an unfinished but playable ASCII/Unicode terminal roguelike with a
hexagonal map and tactical positioning, written in Rust. It uses
[hex2d](https://github.com/dpc/hex2d-rs).

![rhex v0.0.3 screenshot](http://i.imgur.com/M9fi1ri.png)

[Watch the gameplay recording](https://asciinema.org/a/34224).

## Run with Nix

Use a UTF-8 locale and a terminal with at least 256 colors and a Unicode font.
For a comfortable layout, use at least 100 columns and 30 rows. Enable flakes and
the `nix-command` feature in Nix.

```console
nix run .
```

The flake supplies the Rust toolchain and native ncurses library; you do not need
a separate `tput` program. Run from a writable directory: the game writes
`debug.log` there. The game has no save/load support.

Press any key at the introduction, then choose a race (`a` for human).
In normal input mode, use `w`/`s` to move forward/backward, `a`/`d` to turn, and
`q`/`e` to sidestep. `.` waits, `,` picks up an item, `I` opens inventory,
`?` shows help, and F2 toggles Vi-style controls.
To exit, press Escape, then `y` to confirm.

## Develop

```console
nix develop
cargo run --release --locked
```

You can also activate the shell with `direnv allow`.
The project uses flakebox, treefmt, Vale, and selfci:

```console
treefmt
just vale
cargo clippy --all-targets --locked
nix build -L .#ci.tests
selfci check
```

The flake exposes the game as the default package. Run individual checks with
`ci.workspace`, `ci.clippy`, `ci.tests`, and `ci.cargoFmt`.
The `treefmt` package checks Rust, Nix, and TOML formatting.
Clippy reports existing legacy-code warnings without treating them as errors.

Without Nix, install a current Rust toolchain, a C compiler, pkg-config, and
the wide-character ncurses development library, then run
`cargo run --release --locked`. The ncurses Rust build script initializes a
terminal while generating constants; in a headless build, set
`TERM=xterm-256color` and install the corresponding terminfo entry.

## Contributing

The game is not feature-complete. To suggest improvements or report bugs,
[open an issue](https://github.com/dpc/rhex/issues).
This project uses the MPL-2.0 license; see [LICENSE](LICENSE).

## AI usage disclosure

[I use LLMs when working on my projects.](https://dpc.pw/posts/he-went-to-the-slop-side/)
