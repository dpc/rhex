# Testing coverage and terminal checks

The main correctness checks use deterministic controller, engine, and styled-cell
examples. Bounded generated sequences and three text snapshots complement them.
PTY tests check the real terminal boundary, not the correctness of a random dungeon.

## Commands and failure evidence

```sh
cargo test --locked
cargo clippy --locked --all-targets
timeout 180s env PROPTEST_CASES=1024 cargo test --locked --bin rhex
selfci check -c <change-id>
```

The full selfci gate includes Nix builds and nextest/PTY checks. Python 3 comes
from the Nix development environment. The Unix-only PTY tests include one
native-binary draw-failure driver; to run only pure headless examples,
skip `ui_draw_failure_restores_terminal_before_diagnostics`. The ignored parser
and failure probes are helpers; run them through their drivers.

selfci invokes `nix build --keep-failed -L .#ci.tests` to retain the failed Nix
sandbox, including artifacts outside the source or game working directory.
Nix prints the retained build directory; artifact paths under `/build` refer to
locations inside that directory. Use `--keep-failed` for direct Nix test runs
too; default Nix cleanup removes those diagnostics. Remove retained failed build
trees by hand. These checks promise no automatic artifact export or retrieval
from remote builders.

See [PROPERTIES.md](PROPERTIES.md) for property shrinking, source-embedded replay,
writable diagnostics and snapshot review. Generator sweeps provide bounded
evidence, not a termination proof; use an external timeout for stress tests.
RNG replay depends on the locked implementation and is not portable across
versions.

PTY failures retain a unique directory under `RHEX_PTY_DIAGNOSTICS`, or the
system temporary directory by default. The driver reports its path on standard
error before removing the game's temporary working directory:

- `output.bin`: the last 1 MiB of raw terminal output.
- `screen.json`: parsed cells, indexed/RGB styles, wide-cell flags, cursor and
  alternate-screen state, and current output attributes.
- `trace.json`: failure text, the last 4096 timestamped input/resize events,
  command, initial dimensions and `TERM`, exit status before/after harness cleanup,
  output byte counts, and terminal settings before/after.

During Nix builds only, the driver sets the newly created diagnostic directory to
`0755` and its three completed files to `0644` for host inspection after
`--keep-failed` maps the builder UID. These artifacts contain only synthetic game
output and executable/working-directory paths, not environment or user-session
data. Ordinary host runs, including `nix shell`, keep artifacts private.
Permissions on an override parent directory or restrictions on a remote builder
may still prevent access.

The real-game trace labels its seed as entropy: this is a transport
test, not deterministic gameplay replay. Native fixture/property failures supply
their own seeds/actions. The driver kills a running child only to stop a failed
test; this does not establish application cleanup on `SIGKILL`.

For a safe artifact-path check, copy the source to an isolated temporary tree,
use a separate `CARGO_TARGET_DIR`, and run its terminal integration test with
`RHEX_TEST_FORCE_ARTIFACT_FAILURE=1` and a writable diagnostics directory. The
test fails after tiny-start recovery, records input/resize and screen evidence,
and must exit nonzero. Never mutate or run intentional
negative probes against the live game's working directory or its build target.

## Coverage audit

This matrix maps the five-stage testing plan to the test suite. It
describes bounded evidence, not exhaustive state-space or platform coverage.

| Area | Evidence |
| --- | --- |
| Stage 1: input safety | `input_tests.rs` covers modifiers, key kinds, Shift, unsupported Unicode and keys; `ui_input_tests.rs` covers normal/Vi mapping and opposite h/l/arrow rotations at all six facings. |
| Stage 1: buffer and geometry | `window_tests.rs`, `color_tests.rs`, and `util_tests.rs` cover clipping, wrapping, clear/style reset, empty regions, palette and hand-calculated hex/radius cases. |
| Stage 2: shared production paths | `Runner` owns the session; headless `Ui` consumes events, dimensions and explicit time. Production and tests call the same dispatch and composition methods. |
| Stage 2: modes and queues | `ui_state_tests.rs` checks exact actions/target coordinates, inventory ownership, race/quit, examine, modal burst order, auto-move cancellation and stairs selection. |
| Stage 2: small-size and recovery | Boundary sizes, zero buffers, mode/target preservation, pre-shrink queue retention, discarded small-mode input, and recovered cell/style equality. |
| Stages 2–3: rendering | `map_tests.rs` checks exact symbols/styles, light/glue, signed projections, concealment, authorized head/trail/death exceptions, target blockage, layer precedence, occupied heads and reset. |
| Stages 2–3: composition and time | Long inventory/multiline log overflow remains inside panels, clears and survives shrink/grow. Explicit 49/50 ms dead/auto-move and 99/100 ms render deadlines; `cooldown` progression is separate from idle behavior. |
| Stage 3: independent randomness | Identical seeded simulation traces with zero/one/seven renders compare normalized outcomes and next tactical draw; combat must occur. |
| Stage 3: tactics | Engine/location/AI examples cover exact actor order, descent, occupancy, six-direction movement, doors/charge/digging, ownership/equipment, seeded combat, perception/discovery and AI priorities/fallback. |
| Stages 3–4: generation | Fixed 20-case corpus and 16 generated seed/level pairs check free spawn, door-aware reachable stairs, placement, occupancy and repeat generation. Approachable statue loot remains valid. |
| Stage 4: sequences and snapshots | UI/tactical vectors check a small independent model and invariants; embedded regression seeds replay without source writes. Three text snapshots retain separate style oracles. |
| Stage 5: output fidelity | Tests feed real Crossterm backend frames to `vt100` byte by byte and assert selected symbols/styles, stale diffs, wide-cell cleanup and terminal modes. Python handles PTY allocation and delegates all output parsing to the same parser. |
| Stage 5: transport | Real arrow/F2/modifier encodings, fragmented UTF-8/escape prefixes, a burst with an explicit eventual Help command, tiny startup, resize recovery, and a coarse Linux paused-CPU smoke test. |
| Stage 5: failure cleanup | Isolated normal/error/panic/partial-setup sessions and an actual UI-loop backend write error check terminal settings, parsed modes/attributes and cleanup before diagnostics; non-TTY refusal stays covered. |
| Stage 5: diagnostics/manual scope | Bounded persistent artifacts and the release checklist below. Automated checks do not establish physical terminal/font behavior. |

The audit identified gaps in the first four stages: long composed content,
dead-player pacing and occupied-head colors. Explicit examples now cover them.
Existing h/l regressions remain intact. The report that both keys turned right
was not reproduced; the earlier mutation-build explanation remains unconfirmed.

Unchanged policies: modal screens need not freeze queued actions;
pre-shrink work survives recovery; ranged attacks gain no new range/occlusion
rule; HP may cross zero; authorized perception cues are not treated as leaks.
Arbitrary grapheme/ZWJ support, cross-version saves/replay, exhaustive scheduler
models, and new platform support are not claims of this suite. Wide-cell tests
exercise the terminal backend, not universal Unicode support in game text.

## Manual release/dependency-upgrade checklist

**Not performed as part of this implementation.** Record the date, OS, actual
terminal/emulator and version, font, locale, and multiplexer/remote versions for
each real run. Changing `TERM` alone is not another emulator or platform.

- [ ] One actual Linux UTF-8/256-color terminal.
- [ ] A second terminal implementation.
- [ ] `tmux`, if used.
- [ ] SSH, if used, including remote resize propagation.
- [ ] Resize while targeting and auto-moving, shrink below minimum, recover.
- [ ] Long inventory/log content; bullet and door glyphs; visibility contrast,
      facing/occupied-head cues and fading trails.
- [ ] Normal quit and panic recovery: shell echo/canonical input, visible cursor,
      original screen and reset colors.

The suite claims no Windows or macOS coverage. Neither the guard nor these tests
promise restoration after `SIGKILL`, abort, or arbitrary fatal signals.
