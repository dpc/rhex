# ARCH-rhex: Terminal roguelike architecture

Rhex is a single-process, turn-based hexagonal roguelike. One Rust binary
contains both the simulation and the terminal interface. Game state lives in
memory; there is no save/load layer. The [README](../README.md) covers running
the game, controls, terminal requirements, and development commands.

## Ownership and boundaries

- [`terminal::Runner`](../src/terminal/runner.rs) owns the terminal session and
  interactive loop. It supplies terminal events, dimensions, and scheduling
  instants to the terminal-independent [`Ui`](../src/terminal/ui.rs).
  `Ui` owns the game engine, menus, targeting, and the player-facing log. It
  translates input into game actions or interface mode changes, queues player
  actions, handles automatic movement, and controls animation and turn pacing.
- [`game::Engine`](../src/game/engine.rs) owns the locations and turn progression.
  It coordinates player and AI actions and transfers the player into a newly
  generated location on descent.
- [`game::Location`](../src/game/location.rs) owns a level's terrain, actors,
  ground items, and lighting. It applies actions and coordinates their effects
  with the actor and item models. Actor state includes perception and discovery
  information for the player's view.
- [`generate`](../src/generate.rs) builds levels from game types;
  [`ai`](../src/ai.rs) chooses actions from the current location and actor state.
  Neither these modules nor `game` depend on the terminal interface. Shared
  hexagonal coordinates and spatial algorithms come from `hex2d` and
  `hex2d-dpcext`.

## Simulation and presentation

The interface submits manual and automatic movement through the same game
action boundary. This turn path omits presentation callbacks:

```text
terminal::Ui::run_engine_turn():
    game::Engine::start_turn()                 // if the previous turn ended
    game::Engine::player_act(action)           // when player input is needed
    // Otherwise, skip the player's action.
    game::Engine::one_actor_tick()             // repeat until the turn ends
    terminal::Ui::update_changes()
    terminal::Ui::redraw()
```

The simulation does not run on a separate thread or wall-clock timer.
The interface advances it and renders its state; animation timing differs
from game turns. A terminal below the minimum supported size pauses simulation.

Locations retain geometry for observed, resolved hits until `Ui` consumes it
after each actor action. The map renderer owns active animations independently
of actor lifetimes and advances them using `Ui`'s elapsed presentation time.
Effects at the same impact cell play in resolution order; effects at different
cells play concurrently. Visibility gates both capture and display, and descent
clears old effects. Animation playback neither advances turns nor consumes game RNG.

[`terminal::Session`](../src/terminal/session.rs) owns raw mode and the alternate
screen, using Crossterm for terminal operations and Ratatui for buffered output.
Its guard restores terminal modes on drop, and its panic hook restores them
before diagnostics. The map renderer projects hexagonal game state into
terminal cells. Buffered windows compose the map, statistics, log, and
full-screen views into a frame. Input translation and terminal cleanup remain
outside the simulation.

Production drawing and headless tests share composed buffer rendering and an
event dispatcher. Hand-built test levels isolate controller and cell-buffer
expectations from procedural generation.

The engine owns the random stream for level seeds, player facing, and AI.
Each location owns its tactical stream; rendering is deterministic and
does not consume randomness. Explicit seeds reproduce worlds and action
outcomes within the current RNG implementation,
not across versions or platforms.

The player acts first, followed by living actors in ascending spawn-ID order.
Descent transfers the player and discards the old location's pending schedule.
Coordinate order resolves AI cue ties and generation dependencies; inventory
letters determine row and corpse-loot placement order. These orders replace
hash-table iteration wherever it could change behavior.

The [PTY integration test](../tests/terminal.rs) exercises the real binary and
session failure probes through [a Python driver](../tests/terminal.py), covering
interactive input, resizing, rendering, and terminal restoration.
