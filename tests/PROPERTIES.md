# Bounded properties and composed snapshots

Run the fast native suite with `cargo test --locked --bin rhex`. It includes
64 UI vectors of at most 80 events, 64 tactical vectors of at most 64 actions,
and 16 generated seed/level pairs (levels 0 through 8), plus reviewed example
traces and the fixed generator corpus. For a larger run:

```sh
PROPTEST_CASES=1024 cargo test --locked --bin rhex
```

The UI reference model covers normal, help, quit cancellation, inventory view,
radial ranged targeting, input-mode toggling, queue order, and minimum-size
pause/recovery. It does not execute queued actions at normal terminal size:
modal transitions must retain them, and confirmation must append the
chosen coordinate. Paused advances must leave the world and queue unchanged.
Other input mappings, all-six-facing h/l movement, target rotation, and
automatic movement retain their focused example tests.

Tactical vectors apply ready-player actions in a hand-built room with stationary
enemies, a closed door, and distinct non-consumable items. They check live
occupancy, item ownership/conservation, equipped slots and unique letters,
movement geometry, knowledge monotonicity, visibility/discovery, and recorded
attack damage. They do not model the scheduler, assume nonnegative
HP or impose a ranged range/occlusion policy. Engine scheduling and consumable
items retain focused examples.

## Failure persistence

The property runner shrinks named events/actions and prints the minimized trace and seed.
The shared test helper also writes a unique `rhex-*.txt` diagnostic into
`$RHEX_PROPTEST_DIAGNOSTICS`, or the system temporary directory by default.
Create that directory before running tests. Failed writes remain visible on
standard error with the seed and trace; the helper never falls back to writing source.

After investigating a failure, copy its seed line into the corresponding
`src/**/regressions/*.txt` file. Add an important minimized trace as a named
Rust example as well. The tests embed committed corpora with `include_str!`.
The existing Nix `src` filter includes them, and replay needs no writable source
tree, source-path lookup, or runtime repository access. Corpus comments distinguish
injected mutation failures from real game defects.
Replay depends on the current strategy and locked dependencies; it is not a
cross-version or cross-platform game replay format.

## Snapshot review

Three small text snapshots cover a composed tactical room, inventory view, and
minimum-size warning. Recovery must reproduce the inventory frame after a
shrink/grow transition. Row-numbered before/after diagnostics and the complete
actual frame support review. There is no update or auto-accept command:
check why the layout changed before editing expected files.

Snapshots trim trailing spaces and omit cell styles. Existing symbol, color,
modifier, visibility, and clearing assertions remain the style oracle. These
snapshots are not screenshots or a terminal emulator test.
