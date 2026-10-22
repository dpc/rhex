# rhex

Rhex is an unfinished but playable ASCII/Unicode terminal roguelike with a
hexagonal map and tactical positioning, written in Rust. It uses
[hex2d](https://github.com/dpc/hex2d-rs).

![rhex v0.0.3 screenshot](http://i.imgur.com/M9fi1ri.png)

[Watch the gameplay recording](https://asciinema.org/a/34224).

See the [core game design](GAME_DESIGN.md) for the intended horror and stealth
experience and its foundations in the current game.

## Run with Nix

Use a UTF-8 locale and a terminal with at least 256 colors and a Unicode font.
For a comfortable layout, use at least 100 columns and 30 rows. Enable flakes and
the `nix-command` feature in Nix.

```console
nix run .
```

The terminal interface uses Crossterm for input and terminal modes, and Ratatui
for buffered rendering. You do not need an ncurses library, a terminfo database,
or a separate `tput` program. Run from a writable directory: the game writes
`debug.log` there. The game has no save/load support.

Press any key at the introduction, then choose a race (`a` for human).
In either input mode, use `h`/`l` or the Left/Right arrows to turn left/right, and
`k`/`j` or the Up/Down arrows to move forward/backward.
In either input mode, press Shift-H/Shift-L to strafe front-left/front-right
without changing your facing. Creatures use the same `Move` actions to approach
targets.
In normal input mode, you can also use `w`/`s` to move forward/backward, `a`/`d`
to turn, and `q`/`e` to sidestep. Press `.` to wait, `,` to interact in front,
`I` to open inventory, and `?` for help.
Press F2 to toggle additional Vi-style controls (`u`/`i` spin,
`K` walks automatically, and `q` quits instead of sidestepping).
Use these movement aliases during gameplay; in menus and inventory, use letters
to select entries.

Press comma to interact with the tile in front of you without moving or turning.
You pick up an item first; if there is no item, you open or close a door. This
matches the item-first “In front” readout. A full inventory leaves the item and
door unchanged; an actor in an open doorway prevents you from closing it.
Each permitted attempt takes one normal action, even when there is nothing to
do. Opening or closing a door makes the same noise as bump-opening it; pickup
and unsuccessful interactions are silent.
Moving into a closed door still opens it without entering. Comma does not clear
thickets or use items, exits, or objects underfoot. Use `E` for inventory items
and `<`/`>` for the passage you are standing on.

### Reading the sidebar

Read current/maximum HP, MP and SP in one row. Red values indicate a decrease
since the previous turn; green values indicate an increase.

Below the equipment, tile readouts and turn counter, use the compact sight
legend to identify entries by `X - name`. Each entry uses its ordinary map glyph
and lighting colors, without transient facing, targeting or animation effects.
The legend lists visible creatures, items, doors, passages, statues, water and
forest obstacles. It excludes the player and remembered or hidden contents.
It follows the map's topmost-occupant priority: an actor or item can obscure a
feature.

Entries with the same symbol and name share one legend entry, using the nearest
example's colors. Creatures come first, followed by items, features and unusual
terrain; within each group, nearer entries come first. Short names share a row.
Long names use the full width and clip with an ellipsis. `+N more` counts
additional distinct entries that do not fit. The map and log keep
their space, including at the minimum supported size of 60 columns by 24 rows.

### Forests and returning to levels

You start in a Dark Forest. Stand on its `>` exit and press `>` to enter a
Dungeon. Continue through onward exits to alternate between the two environments,
or use the `<` return passages to revisit previous levels. Press either `<` or `>`
while standing on a passage to use it. Check the tile readout for its destination.
Press `G`, then `>` or `<`, to route to the nearest known passage without using it.

Dark Forests have narrow paths through dense undergrowth and large brown tree
trunks, with small lit canopy openings, a winding creek and a pond. Green `%`
thickets block movement and sight. Hit them by moving forward or diagonally
forward into them. The hit consumes an action and makes noise, leaving ground
you can enter on the next move. You can clear thickets unarmed, subject to the
normal action and stamina checks. Charging also stops after clearing one thicket.
Brown `#` trunks block movement, sight and sound and cannot be cleared.
Blue `,` shallow water is transparent and walkable; blue `~` deep water is
transparent but impassable. Thickets absorb sound without completely blocking it.

Visited levels stay in memory: returning preserves enemies, awareness, items,
terrain changes and exploration. Inactive levels do not take turns. If an actor
occupies the return tile, arrival uses the nearest free connected walkable tile;
if that component is full, traversal leaves you where you are. This persistence
lasts only for the current game; there is still no save/load support.

To exit, press Escape, then `y` to confirm.
If your terminal has fewer than 60 columns or 24 rows, the game pauses and asks
you to resize it. Press Escape at this prompt to exit without confirmation.

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
cargo clippy --all-targets --all-features --locked -- -D warnings
nix build --keep-failed -L .#ci.tests
selfci check
```

The flake exposes the game as the default package. Run individual checks with
`ci.workspace`, `ci.clippy`, `ci.tests`, and `ci.cargoFmt`.
The `treefmt` package checks Rust, Nix, and TOML formatting.
CI rejects compiler and Clippy warnings across all test targets and features.
The project has no optional Cargo features at present. Nix retains failed test
build directories so PTY diagnostics remain available.

GitHub Actions runs on pushes to `master`, pull requests, and manual dispatch.
It runs the same checks as selfci: formatting, Vale, lockfile consistency,
workspace and package builds, strict Clippy, and the full test suite including
PTY tests. Run the hosted CI command locally with
`nix develop -c ./.config/selfci/check.sh`.

Without Nix, install a current Rust toolchain and a platform linker, then run
`cargo run --release --locked`. Builds do not need a terminal. The PTY integration
tests require Python 3 on Unix.

During the terminal migration, we updated all direct dependencies to
the latest stable release or upstream revision available at the time.
The in-house `hex2d-dpcext` extension still declares an unused `rand` 0.8
dependency, so its older random-number dependency chain remains in the lockfile
alongside the game's `rand` 0.10.
We keep the upstream algorithms rather than vendor them to remove that
dependency.

## Contributing

The game is not feature-complete. To suggest improvements or report bugs,
[open an issue](https://github.com/dpc/rhex/issues).
This project uses the MPL-2.0 license; see [LICENSE](LICENSE).

## AI usage disclosure

[I use LLMs when working on my projects.](https://dpc.pw/posts/he-went-to-the-slop-side/)
