use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Write as FmtWrite;
use std::time::{Duration, Instant};
use std::{cmp, fmt};

use num::integer::Integer;

use crate::game::Action::*;

use crossterm::event::Event as TerminalEvent;
use hex2d::{Angle, Back, Coordinate, Forward, Left, Position, Right};
use ratatui::buffer::Buffer;
use ratatui::style::Style;

use crate::hex2dext::algo::bfs;

use super::consts::*;
use super::input;
use super::map::MapRenderer;
use super::{AutoMoveAction, AutoMoveType, Event, GoToType, LogEntry, LogEvent};
use super::{Window, color};

use crate::game;
use crate::game::actor::{Race, Slot};
use crate::game::tile;
use crate::game::{Actor, Location, actor, area};
use crate::util;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum InputMode {
    Vi,
    Normal,
}

impl InputMode {
    fn toggle(&mut self) {
        *self = match *self {
            InputMode::Vi => InputMode::Normal,
            InputMode::Normal => InputMode::Vi,
        }
    }
}

impl fmt::Display for InputMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InputMode::Vi => write!(f, "Vi"),
            InputMode::Normal => write!(f, "Normal"),
        }
    }
}

struct Windows {
    map: Window,
    log: Window,
    stats: Window,
    full: Window,
}

impl Windows {
    fn after_resize((max_y, max_x): (i32, i32)) -> Self {
        let mid_x = (max_x - 30).max(0);
        let mid_y = 13.min(max_y);

        let map_window = Window::new(mid_x, max_y, 0, 0);
        let stats_window = Window::new(max_x - mid_x, mid_y, mid_x, 0);
        let log_window = Window::new(max_x - mid_x, max_y - mid_y, mid_x, mid_y);
        let fs_window = Window::new(max_x, max_y, 0, 0);

        Windows {
            map: map_window,
            stats: stats_window,
            log: log_window,
            full: fs_window,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum InvMode {
    View,
    /// Choose an action for an owned inventory letter.
    Actions(char),
    Equip,
    Drop_,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum FSMode {
    Help,
    Intro,
    PickRace,
    Quit,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum TargetMode {
    Ranged,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Mode {
    Normal,
    Examine,
    GoTo,
    Target(TargetMode),
    FullScreen(FSMode),
    Inventory(InvMode),
}

/// Terminal-independent controller and buffered presentation state.
pub struct Ui {
    windows: Windows,

    mode: Mode,
    log: RefCell<VecDeque<LogEntry>>,
    target_pos: Option<Position>,

    label_color: Style,
    text_color: Style,
    text_gray_color: Style,
    red_color: Style,
    green_color: Style,

    input_mode: InputMode,

    engine: game::Engine,
    exit: bool,
    spawned: bool,

    automoving: Option<AutoMoveType>,
    automoving_stopped_turn: u64,

    anim_frame_count: u32,
    next_turn_ts: Instant,
    next_anim_frame_ts: Instant,

    /// Gameplay actions in input order; inventory selections have a presentation deadline.
    game_action_queue: VecDeque<(game::Action, Option<Instant>)>,

    map_renderer: MapRenderer,
    /// Level owning the current effects; traversal invalidates their coordinates.
    animation_level: game::level::LevelId,
    /// Per-action observations staged until all actors in this turn have acted.
    pending_sounds: Vec<game::sound::ObservedSound>,

    /// Time supplied by the runner at each scheduling/event boundary.
    now: Instant,
    /// Stable presentation epoch; redraws and game turns never reset blink phase.
    presentation_started: Instant,
}

use self::Action::*;

enum Action {
    Game(game::Action),
    ModeSwitch(Mode),
    AutoMove(AutoMoveType),
}

use self::GlobalAction::*;

enum GlobalAction {
    ToggleInputMode,
}

impl Ui {
    /// Construct presentation state around an existing engine without opening a terminal.
    pub fn new(engine: game::Engine, size: (i32, i32), now: Instant) -> Self {
        let label_color = color::style(color::LABEL_FG, color::BACKGROUND_BG);
        let text_color = color::style(color::VISIBLE_FG, color::BACKGROUND_BG);
        let text_gray_color = color::style(color::GRAY[10], color::BACKGROUND_BG);
        let green_color = color::style(color::GREEN_FG, color::BACKGROUND_BG);
        let red_color = color::style(color::RED_FG, color::BACKGROUND_BG);

        let mut ui = Ui {
            windows: Windows::after_resize(size),
            mode: Mode::FullScreen(FSMode::Intro),
            target_pos: None,
            log: RefCell::new(VecDeque::new()),

            label_color,
            text_color,
            text_gray_color,
            red_color,
            green_color,

            input_mode: InputMode::Normal,

            exit: false,
            spawned: false,

            engine,
            automoving: None,
            automoving_stopped_turn: 0,

            anim_frame_count: 0,
            next_turn_ts: now,
            next_anim_frame_ts: now,
            game_action_queue: VecDeque::new(),
            map_renderer: MapRenderer::new(UNICODE_DOT),
            animation_level: game::level::LevelId(0),
            pending_sounds: Vec::new(),
            now,
            presentation_started: now,
        };
        ui.display_intro();
        ui.resize(size);
        ui
    }

    pub fn initial_spawn(&mut self, race: actor::Race) {
        self.game_action_queue.clear();
        self.map_renderer.animations.clear();
        self.pending_sounds.clear();
        self.engine.initial_spawn(race);
        let player_id = self.engine.current_location().player_id();
        self.engine_change(player_id);
        self.update_changes();
        self.spawned = true;
        self.next_turn_ts = self.now
    }

    pub fn window_size(window: &Window) -> (i32, i32) {
        window.size()
    }

    pub fn resize(&mut self, size: (i32, i32)) {
        self.windows = Windows::after_resize(size);
        let window = &self.windows.map;
        let (y, x) = Ui::window_size(window);
        self.map_renderer.resize(x as usize, y as usize);
        self.redraw();
    }

    /// Mark the screen for redraw
    pub fn redraw(&mut self) {
        self.anim_frame_count = 0;
        self.next_anim_frame_ts = self.now;
    }

    /// Compose the same complete frame for the terminal and headless tests.
    pub fn render_into(&mut self, buffer: &mut Buffer) {
        buffer.reset();
        buffer.set_style(buffer.area, self.text_color);
        if self.too_small() {
            self.windows.full.clear();
            self.windows
                .full
                .write("Terminal too small.\nResize to at least 60x24.\nEsc to quit.");
        } else {
            match self.mode {
                Mode::Normal
                | Mode::Examine
                | Mode::Inventory(_)
                | Mode::Target(_)
                | Mode::GoTo => {
                    if let Mode::Inventory(_) = self.mode {
                        self.draw_inventory();
                    } else {
                        self.draw_map();
                    }

                    self.draw_log();

                    self.draw_stats();
                }
                Mode::FullScreen(fs_mode) => match fs_mode {
                    FSMode::Help => {
                        self.draw_help();
                    }
                    FSMode::Quit => {
                        self.draw_quit();
                    }
                    FSMode::Intro => {
                        self.draw_intro();
                    }
                    FSMode::PickRace => {
                        self.draw_pickrace();
                    }
                },
            }
        }
        if self.too_small() || matches!(self.mode, Mode::FullScreen(_)) {
            self.windows.full.render(buffer);
        } else {
            self.windows.map.render(buffer);
            self.windows.stats.render(buffer);
            self.windows.log.render(buffer);
        }
    }

    /// Keep layout calculations and input safe when the terminal is too small.
    fn too_small(&self) -> bool {
        let (height, width) = self.windows.full.size();
        width < 60 || height < 24
    }

    pub fn is_automoving(&self) -> bool {
        self.automoving.is_some()
    }

    pub fn should_stop_automoving(&self) -> bool {
        let player = self.player();
        let cur_loc = self.current_location();

        !player.was_attacked_by.is_empty()
            || player.discovered_areas.iter().any(|_| true)
            || player.visible.iter().any(|&coord| {
                cur_loc
                    .at(coord)
                    .actor_map_or(false, |a| a.race == actor::Race::Rat)
            })
            || player
                .discovered
                .iter()
                .any(|&coord| cur_loc.at(coord).item_map_or(false, |_| true))
            || player.heard.iter().any(|(c, _)| !player.sees(*c))
            || player.discovered_stairs(cur_loc)
    }

    pub fn automove_action(&self, movetype: AutoMoveType) -> AutoMoveAction {
        match movetype {
            AutoMoveType::Explore => self.autoexplore_action(),
            AutoMoveType::GoTo(gototype) => self.goto_action(gototype),
        }
    }

    fn goto_action(&self, _gototype: GoToType) -> AutoMoveAction {
        let player = self.player();
        let cur_loc = self.current_location();

        let start = player.pos.coord;

        let mut bfs = bfs::Traverser::new(
            |c| c == start || (cur_loc.at(c).tile().is_passable() && player.knows(c)),
            |c| {
                cur_loc
                    .at(c)
                    .tile()
                    .feature
                    .is_some_and(tile::Feature::is_exit)
            },
            start,
        );

        if let Some(dst) = bfs.find() {
            if let Some(neigh) = bfs.backtrace_last(dst) {
                if let Some(ndir) = player.pos.coord.direction_to_cw(neigh) {
                    if ndir == player.pos.dir {
                        if cur_loc.at(neigh).is_occupied() {
                            AutoMoveAction::Blocked
                        } else {
                            AutoMoveAction::Action(game::Action::Move(Angle::Forward))
                        }
                    } else {
                        AutoMoveAction::Action(game::Action::Turn(ndir - player.pos.dir))
                    }
                } else {
                    AutoMoveAction::Finish
                }
            } else {
                AutoMoveAction::Finish
            }
        } else {
            AutoMoveAction::Blocked
        }
    }

    pub fn autoexplore_action(&self) -> AutoMoveAction {
        let player = self.player();
        let cur_loc = self.current_location();

        let start = player.pos.coord;

        let mut bfs = bfs::Traverser::new(
            |c| c == start || cur_loc.at(c).tile().is_passable(),
            |c| !player.knows(c),
            start,
        );

        if let Some(dst) = bfs.find() {
            if let Some(neigh) = bfs.backtrace_last(dst) {
                let ndir = player
                    .pos
                    .coord
                    .direction_to_cw(neigh)
                    .expect("bfs gave me trash");
                if ndir == player.pos.dir {
                    if cur_loc.at(neigh).is_occupied() {
                        AutoMoveAction::Blocked
                    } else {
                        AutoMoveAction::Action(game::Action::Move(Angle::Forward))
                    }
                } else {
                    AutoMoveAction::Action(game::Action::Turn(ndir - player.pos.dir))
                }
            } else {
                AutoMoveAction::Finish
            }
        } else {
            AutoMoveAction::Finish
        }
    }

    pub fn automoving_stop(&mut self) {
        self.automoving = None;
        self.automoving_stopped_turn = self.engine.turn()
    }

    fn engine_change(&mut self, actor_id: actor::Id) {
        let messages = std::mem::take(&mut self.engine.current_location_mut().messages);
        for message in messages {
            self.log(&message);
        }
        let level = self.current_location().id;
        if self.animation_level != level {
            self.map_renderer.animations.clear();
            self.pending_sounds.clear();
            self.animation_level = level;
            self.target_pos = None;
            self.automoving = None;
        }
        let events = std::mem::take(&mut self.engine.current_location_mut().combat_events);
        self.pending_sounds
            .append(&mut self.engine.current_location_mut().sound_events);
        let awareness = std::mem::take(&mut self.engine.current_location_mut().awareness_events);
        self.map_renderer.animations.enqueue_awareness(
            awareness,
            self.now
                .saturating_duration_since(self.presentation_started),
        );
        self.map_renderer.animations.enqueue(
            events,
            self.now
                .saturating_duration_since(self.presentation_started),
        );
        if self.automoving.is_some()
            && self.automoving_stopped_turn != self.engine.turn()
            && self.should_stop_automoving()
        {
            self.automoving_stop();
        }

        let player_id = self.current_location().player_id();

        if actor_id == player_id && (self.is_automoving() || self.player().is_dead()) {
            self.next_turn_ts = self.now + Duration::from_millis(50);
        }
    }

    pub fn player(&self) -> &Actor {
        let player_id = self.engine.current_location().player_id();
        &self.engine.current_location().actors_byid[&player_id]
    }

    pub fn current_location(&self) -> &Location {
        self.engine.current_location()
    }

    pub fn is_next_turn_time(&self) -> bool {
        self.next_turn_ts <= self.now
    }

    pub fn is_next_anim_frame_time(&self) -> bool {
        self.next_anim_frame_ts <= self.now
    }

    pub fn run_engine_turn(&mut self) {
        if self.exit {
            return;
        }
        if self.spawned && self.is_next_turn_time() {
            // Inventory selections leave time to see the map before their effect.
            // Keep this before start_turn: even the turn counter stays frozen.
            if self.game_action_queue.front().is_some_and(|(_, ready)| {
                ready.is_some_and(|ready| self.mode != Mode::Normal || self.now < ready)
            }) {
                return;
            }
            let player_id = self.current_location().player_id();
            let mut player_acted = false;

            if self.engine.is_turn_done() {
                self.engine.start_turn()
            }

            if self.engine.needs_player_input() {
                // An auto-move request during the lead-in must not strand the
                // already accepted inventory action behind automatic steps.
                if let Some(movetype) = self.automoving.filter(|_| {
                    !self
                        .game_action_queue
                        .iter()
                        .any(|(_, ready)| ready.is_some())
                }) {
                    match self.automove_action(movetype) {
                        AutoMoveAction::Blocked => {
                            self.event(Event::Log(LogEvent::AutoExploreBlocked));
                            self.automoving_stop();
                        }
                        AutoMoveAction::Action(action) => {
                            self.engine.player_act(action);
                            self.engine_change(player_id);
                            player_acted = true;
                        }
                        AutoMoveAction::Finish => {
                            if let AutoMoveType::Explore = movetype {
                                self.event(Event::Log(LogEvent::AutoExploreDone));
                            }
                            self.automoving_stop();
                        }
                    }
                } else if let Some((action, _)) = self.game_action_queue.pop_front() {
                    self.engine.player_act(action);
                    self.engine_change(player_id);
                    player_acted = true;
                }
            } else {
                self.engine.player_skip_act();
                self.engine_change(player_id);
                player_acted = true;
            }

            if player_acted {
                while !self.engine.is_turn_done() {
                    let actor_id = self.engine.one_actor_tick();
                    self.engine_change(actor_id);
                }
                self.map_renderer.animations.enqueue_sounds(
                    std::mem::take(&mut self.pending_sounds),
                    self.now
                        .saturating_duration_since(self.presentation_started),
                );
                self.update_changes();
                self.redraw();
            }
        }
    }

    /// Advance the animation schedule only after a successful draw.
    pub fn frame_drawn(&mut self, now: Instant) {
        self.anim_frame_count += 1;
        self.next_anim_frame_ts = now + Duration::from_millis(100);
    }

    /// Reconcile size and advance simulation before the runner drains input.
    pub fn advance(&mut self, size: (i32, i32), now: Instant) {
        self.now = now;
        self.map_renderer
            .animations
            .advance(now.saturating_duration_since(self.presentation_started));
        if self.windows.full.size() != size {
            self.resize(size);
        }
        if !self.too_small() {
            self.run_engine_turn();
        }
    }

    /// Report whether the runner should finish.
    pub fn exited(&self) -> bool {
        self.exit
    }

    /// Check rendering and idle pacing at the runner's current instant.
    pub fn frame_due(&mut self, now: Instant) -> bool {
        self.now = now;
        self.is_next_anim_frame_time()
    }

    /// Avoid spinning while paused or waiting for player input.
    pub fn should_idle(&self) -> bool {
        self.too_small()
            || self.game_action_queue.front().is_some_and(|(_, ready)| {
                ready.is_some_and(|ready| self.mode != Mode::Normal || self.now < ready)
            })
            || self.game_action_queue.is_empty()
    }

    /// Dispatch native events, including resize, pause and cancellation policy.
    pub fn handle_event(&mut self, event: TerminalEvent, now: Instant) {
        self.now = now;
        let ch = match event {
            TerminalEvent::Resize(width, height) => {
                self.resize((height.into(), width.into()));
                return;
            }
            TerminalEvent::Key(key) => match input::game_key(key) {
                Some(ch) => ch,
                None => return,
            },
            _ => return,
        };
        if self.too_small() {
            if ch == KEY_ESC {
                self.exit = true;
            }
            return;
        }
        if self.automoving.is_some() {
            self.automoving_stop();
            return;
        }

        self.input_handle_key(ch);
    }

    pub fn action_push(&mut self, action: game::Action) {
        let ready = (matches!(self.mode, Mode::Inventory(_))
            && matches!(action, game::Action::Equip(_) | game::Action::Drop_(_)))
        .then_some(self.now + Duration::from_millis(200));
        self.game_action_queue.push_back((action, ready));
    }

    fn mode_switch_to(&mut self, mode: Mode) {
        if self.mode != Mode::Normal && mode == Mode::Normal {
            // Give a pending inventory effect a visible lead-in after leaving a modal.
            for (_, ready) in &mut self.game_action_queue {
                if ready.is_some() {
                    *ready = Some(self.now + Duration::from_millis(200));
                }
            }
        }
        self.mode = mode;
        self.redraw();
    }

    pub fn queue_ranged(&mut self, coord: Coordinate) {
        self.action_push(game::Action::Ranged(coord))
    }

    pub fn queue_action(&mut self, action: game::Action) {
        self.action_push(action)
    }

    pub fn queue_equip(&mut self, ch: char) {
        self.action_push(game::Action::Equip(ch))
    }

    pub fn queue_drop(&mut self, ch: char) {
        self.action_push(game::Action::Drop_(ch))
    }

    pub fn in_vi_input_mode(&self) -> bool {
        self.input_mode == InputMode::Vi
    }

    fn key_to_action_normal(input_mode: InputMode, key: i32) -> Option<Action> {
        let action = match (key, input_mode == InputMode::Vi) {
            (KEY_LOWH, _) | (input::LEFT, _) => Game(Turn(Left)),
            (KEY_LOWL, _) | (input::RIGHT, _) => Game(Turn(Right)),
            (KEY_LOWK | KEY_CAPK, _) | (input::UP, _) => Game(Move(Forward)),
            (KEY_LOWC, _) => Game(Charge),
            (KEY_CAPH, _) => Game(Move(Left)),
            (KEY_CAPL, _) => Game(Move(Right)),
            (KEY_LOWJ | KEY_CAPJ, _) | (input::DOWN, _) => Game(Move(Back)),
            (KEY_DOT, _) => Game(Wait),
            (KEY_COMMA, _) => Game(Interact),
            (KEY_DESCEND | KEY_RETURN, _) => Game(Descend),
            (KEY_LOWO, _) => AutoMove(AutoMoveType::Explore),
            (KEY_LOWQ, true) | (KEY_ESC, _) => ModeSwitch(Mode::FullScreen(FSMode::Quit)),
            (KEY_CAPI, _) => ModeSwitch(Mode::Inventory(InvMode::View)),
            (KEY_CAPE, _) => ModeSwitch(Mode::Inventory(InvMode::Equip)),
            (KEY_CAPD, _) => ModeSwitch(Mode::Inventory(InvMode::Drop_)),
            (KEY_LOWX, _) => ModeSwitch(Mode::Examine),
            (KEY_LOWF, _) => ModeSwitch(Mode::Target(TargetMode::Ranged)),
            (KEY_HELP, _) => ModeSwitch(Mode::FullScreen(FSMode::Help)),
            (KEY_GOTO, _) => ModeSwitch(Mode::GoTo),
            _ => return None,
        };
        Some(action)
    }

    fn key_to_action_global(key: i32) -> Option<GlobalAction> {
        let action = match key {
            input::F2 => ToggleInputMode,
            _ => return None,
        };

        Some(action)
    }

    pub fn input_handle_key_mode_specific(&mut self, key: i32) {
        match self.mode {
            Mode::FullScreen(fs_mode) => match fs_mode {
                FSMode::Quit => match key {
                    KEY_LOWY | KEY_CAPY => self.exit = true,
                    _ => self.mode_switch_to(Mode::Normal),
                },
                FSMode::Intro => self.mode_switch_to(Mode::FullScreen(FSMode::PickRace)),
                FSMode::PickRace => match key {
                    KEY_LOWA => {
                        self.initial_spawn(Race::Human);
                        self.mode_switch_to(Mode::Normal);
                    }
                    KEY_LOWB => {
                        self.initial_spawn(Race::Elf);
                        self.mode_switch_to(Mode::Normal);
                    }
                    KEY_LOWC => {
                        self.initial_spawn(Race::Dwarf);
                        self.mode_switch_to(Mode::Normal);
                    }
                    _ => {}
                },
                _ => self.mode_switch_to(Mode::Normal),
            },
            Mode::Normal => {
                if let Some(action) = Self::key_to_action_normal(self.input_mode, key) {
                    match action {
                        Game(action) => self.queue_action(action),
                        ModeSwitch(mode @ Mode::Target(TargetMode::Ranged)) => {
                            if self.player().can_attack_ranged() {
                                self.target_pos = None;
                                self.mode_switch_to(mode);
                            }
                        }
                        ModeSwitch(mode) => {
                            self.target_pos = None;
                            self.mode_switch_to(mode);
                        }
                        AutoMove(automove_type) => self.automoving = Some(automove_type),
                    }
                }
            }
            Mode::Inventory(InvMode::Equip) => {
                let ch = char::from_u32(key as u32).unwrap_or('\0');
                match key {
                    KEY_ESC => self.mode_switch_to(Mode::Normal),
                    _ => match ch {
                        'a'..='z' | 'A'..='Z' if self.player().item_letter_taken(ch) => {
                            self.queue_equip(ch);
                            self.mode_switch_to(Mode::Normal);
                        }
                        _ => {}
                    },
                }
            }
            Mode::Inventory(InvMode::View) => {
                let ch = char::from_u32(key as u32).unwrap_or('\0');
                match key {
                    KEY_ESC => self.mode_switch_to(Mode::Normal),
                    _ => match ch {
                        'a'..='z' | 'A'..='Z' if self.player().item_letter_taken(ch) => {
                            self.mode_switch_to(Mode::Inventory(InvMode::Actions(ch)));
                        }
                        _ => {}
                    },
                }
            }
            Mode::Inventory(InvMode::Actions(ch)) => {
                if key == KEY_ESC {
                    self.mode_switch_to(Mode::Inventory(InvMode::View));
                } else if let Some((_, _, action)) = self
                    .inventory_actions(ch)
                    .into_iter()
                    .find(|(shortcut, _, _)| *shortcut as i32 == key)
                {
                    self.queue_action(action);
                    self.mode_switch_to(Mode::Normal);
                }
            }
            Mode::Inventory(InvMode::Drop_) => {
                let ch = char::from_u32(key as u32).unwrap_or('\0');
                match key {
                    KEY_ESC => self.mode_switch_to(Mode::Normal),
                    _ => match ch {
                        'a'..='z' | 'A'..='Z' if self.player().item_letter_taken(ch) => {
                            self.queue_drop(ch);
                            self.mode_switch_to(Mode::Normal);
                        }
                        _ => {}
                    },
                }
            }
            Mode::Examine => {
                let pos = self.target_pos.unwrap_or(self.player().pos);

                match key {
                    KEY_ESC | KEY_LOWX | KEY_LOWQ => {
                        self.target_pos = None;
                        self.mode_switch_to(Mode::Normal);
                    }
                    KEY_LOWH => {
                        self.target_pos = Some(pos + Angle::Left);
                        self.redraw();
                    }
                    KEY_LOWL => {
                        self.target_pos = Some(pos + Angle::Right);
                        self.redraw();
                    }
                    KEY_LOWJ => {
                        self.target_pos = Some(pos + Coordinate::from(pos.dir + Angle::Back));
                        self.redraw();
                    }
                    KEY_LOWK => {
                        self.target_pos = Some(pos + Coordinate::from(pos.dir));
                        self.redraw();
                    }
                    KEY_CAPK => {
                        self.target_pos = Some(pos + Coordinate::from(pos.dir).scale(5));
                        self.redraw();
                    }
                    KEY_CAPJ => {
                        self.target_pos =
                            Some(pos + Coordinate::from(pos.dir + Angle::Back).scale(5));
                        self.redraw();
                    }
                    _ => {}
                }
            }
            Mode::Target(_) => {
                let center = self.player().pos;
                let pos = self.target_pos.unwrap_or(center);

                match key {
                    KEY_ESC | KEY_LOWX | KEY_LOWQ => {
                        self.target_pos = None;
                        self.mode_switch_to(Mode::Normal);
                    }
                    KEY_ENTER | KEY_LOWF => {
                        match self.target_pos {
                            Some(target) => {
                                self.queue_ranged(target.coord);
                            }
                            None => { /* Nothing */ }
                        }
                        self.target_pos = None;
                        self.mode_switch_to(Mode::Normal);
                    }
                    KEY_LOWH => {
                        self.target_pos = Some(util::circular_move(center, pos, Angle::Left));
                        self.redraw();
                    }
                    KEY_LOWL => {
                        self.target_pos = Some(util::circular_move(center, pos, Angle::Right));
                        self.redraw();
                    }
                    KEY_LOWJ => {
                        self.target_pos = Some(util::circular_move(center, pos, Angle::Back));
                        self.redraw();
                    }
                    KEY_LOWK => {
                        self.target_pos = Some(util::circular_move(center, pos, Angle::Forward));
                        self.redraw();
                    }
                    _ => {}
                }
            }
            Mode::GoTo => {
                if let KEY_DESCEND | KEY_RETURN = key {
                    self.automoving = Some(AutoMoveType::GoTo(GoToType::Stairs))
                }
                self.mode_switch_to(Mode::Normal);
            }
        }
    }

    pub fn input_handle_key(&mut self, key: i32) {
        if Self::key_to_action_global(key).is_some() {
            self.input_mode.toggle();
            self.redraw();
        } else {
            self.input_handle_key_mode_specific(key)
        }
    }

    // Notice changed between start turn and it's end
    fn update_changes(&mut self) {
        let cur_loc = self.current_location();
        let player = self.player();

        if player.is_dead() {
            return;
        }

        let mut area_coords: Vec<_> = player.discovered_areas.iter().collect();
        area_coords.sort_by_key(|c| (c.x, c.y));
        let discovered_areas = area_coords
            .into_iter()
            .filter_map(|coord| cur_loc.at(*coord).tile().area);

        if let Some(s) = self.format_areas(discovered_areas.map(|area| area.type_)) {
            self.log(&s);
        }

        let mut discovered: Vec<_> = player.discovered.iter().collect();
        discovered.sort_by_key(|c| (c.x, c.y));
        for item_coord in discovered
            .into_iter()
            .filter(|&coord| cur_loc.at(*coord).item_map_or(false, |_| true))
        {
            let item_descr = cur_loc
                .at(*item_coord)
                .item_map_or("".to_owned(), |i| i.description());
            self.log(&format!("You've found {}.", item_descr));
        }

        if player.discovered_stairs(cur_loc) {
            self.log("You've found an exit.");
        }

        for res in &player.was_attacked_by {
            if res.success {
                self.log(&format!(
                    "{} hit you {}for {} dmg.",
                    res.who,
                    if res.behind { "from behind " } else { "" },
                    res.dmg
                ));
            } else {
                self.log(&format!("{} missed you.", res.who));
            }
        }

        for res in &player.did_attack {
            if res.success {
                self.log(&format!(
                    "You hit {} {}for {} dmg.",
                    res.who,
                    if res.behind { "from behind " } else { "" },
                    res.dmg
                ));
            } else {
                self.log(&format!("You missed {}.", res.who));
            }
        }

        let mut noises: Vec<_> = player
            .heard
            .iter()
            .filter(|&(c, _)| *c != player.pos.coord)
            .filter(|&(c, _)| !player.sees(*c))
            .collect();
        noises.sort_unstable_by_key(|(c, _)| (c.x, c.y));

        for (_, &noise) in noises {
            self.log(&format!("You hear {}.", noise.description()));
        }
    }

    pub fn log(&self, s: &str) {
        let turn = self.engine.turn();
        self.log.borrow_mut().push_front(LogEntry {
            text: s.to_owned(),
            turn,
        });
    }

    pub fn display_intro(&mut self) {
        self.mode = Mode::FullScreen(FSMode::Intro);
    }

    fn draw_map(&mut self) {
        let Ui {
            ref mut map_renderer,
            ref engine,
            mode,
            target_pos,
            ref windows,
            anim_frame_count,
            now,
            presentation_started,
            ..
        } = *self;

        map_renderer.update(
            engine.current_location(),
            mode,
            target_pos,
            anim_frame_count,
            now.saturating_duration_since(presentation_started),
        );
        map_renderer.draw_into(&windows.map);
    }

    /// Keep exact resource values on one row; color changes as the old bars did.
    fn draw_resources(&self, window: &Window, y: i32) {
        let player = self.player();
        for (index, (label, current, previous, maximum)) in [
            ("HP", player.hp, player.saved_hp, player.stats.base.max_hp),
            ("MP", player.mp, player.saved_mp, player.stats.base.max_mp),
            ("SP", player.sp, player.saved_sp, player.stats.base.max_sp),
        ]
        .into_iter()
        .enumerate()
        {
            let x = index as i32 * 10;
            window.move_to(y, x);
            self.draw_label(window, label);
            let style = match current.cmp(&previous) {
                cmp::Ordering::Less => self.red_color,
                cmp::Ordering::Greater => self.green_color,
                cmp::Ordering::Equal => self.text_color,
            };
            Self::put_clipped(window, y, x + 3, 7, &format!("{current}/{maximum}"), style);
        }
    }

    /// Clip a single-line field explicitly rather than allowing Window wrapping.
    fn put_clipped(window: &Window, y: i32, x: i32, width: usize, text: &str, style: Style) {
        let length = text.chars().count();
        for (offset, ch) in text.chars().take(width).enumerate() {
            let ch = if offset + 1 == width && length > width {
                '…'
            } else {
                ch
            };
            window.put(y, x + offset as i32, ch, style);
        }
    }

    /// Pack short sight entries in pairs, retaining a visible overflow count.
    fn draw_sight(&self, window: &Window, first_y: i32) {
        let entries = self.map_renderer.sight_entries(self.current_location());
        let width = window.size().1 as usize;
        let half = (width - 1) / 2;
        let mut rows: Vec<Vec<&super::sight_entry::SightEntry>> = Vec::new();
        for entry in &entries {
            if entry.name.chars().count() + 4 <= half
                && let Some(row) = rows.last_mut()
                && row.len() == 1
                && row[0].name.chars().count() + 4 <= half
            {
                row.push(entry);
            } else {
                rows.push(vec![entry]);
            }
        }
        let available = (window.size().0 - first_y).max(0) as usize;
        let mut displayed = 0;
        for (offset, row) in rows.iter().take(available).enumerate() {
            let y = first_y + offset as i32;
            for (column, entry) in row.iter().enumerate() {
                let x = if column == 0 { 0 } else { half + 1 };
                let field_width = if row.len() == 1 { width } else { half };
                if offset + 1 == available && column + 1 == row.len() && rows.len() > available {
                    Self::put_clipped(
                        window,
                        y,
                        x as i32,
                        field_width,
                        &format!("+{} more", entries.len() - displayed),
                        self.text_gray_color,
                    );
                    break;
                }
                window.put(y, x as i32, entry.glyph, entry.style);
                Self::put_clipped(
                    window,
                    y,
                    x as i32 + 1,
                    field_width - 1,
                    &format!(" - {}", entry.name),
                    self.text_color,
                );
                displayed += 1;
            }
        }
    }

    fn draw_turn<T>(&self, window: &Window, label: &str, val: T)
    where
        T: Integer + fmt::Display,
    {
        self.draw_label(window, label);

        window.set_style(self.text_color);
        window.write(&format!(" {:<8}", val));
    }

    fn draw_val<T>(&self, window: &Window, label: &str, val: T)
    where
        T: Integer + fmt::Display,
    {
        self.draw_label(window, label);

        window.set_style(self.text_color);
        window.write(&format!("{:>2} ", val));
    }

    fn draw_label(&self, window: &Window, label: &str) {
        window.set_style(self.label_color);
        window.write(&format!("{}:", label));
    }

    fn draw_item(&self, window: &Window, astate: &Actor, label: &str, slot: Slot) {
        self.draw_label(window, label);

        if slot == Slot::RHand && !astate.can_attack() {
            window.set_style(self.text_gray_color);
        } else {
            window.set_style(self.text_color);
        }

        if let Some((_, item)) = astate.items_equipped.get(&slot) {
            window.write(&format!("{:^13}", item.description()));
        } else {
            window.write_char('-');
        }
    }

    /// Derive both menu rows and accepted input from current item capabilities.
    fn inventory_actions(&self, ch: char) -> Vec<(char, &'static str, game::Action)> {
        let player = self.player();
        let mut actions = Vec::new();
        if let Some(item) = player.items_backpack.get(&ch) {
            if item.is_usable() {
                actions.push(('u', item.use_label(), Equip(ch)));
            } else if item.slot().is_some() {
                actions.push(('e', "Equip", Equip(ch)));
            }
        } else if player.item_letter_taken(ch) {
            actions.push(('e', "Unequip", Equip(ch)));
        } else {
            return actions;
        }
        actions.push(('d', "Drop", Drop_(ch)));
        actions
    }

    fn draw_inventory(&self) {
        self.windows.map.clear();
        let window = &self.windows.map;
        let player = self.player();

        window.move_to(0, 0);

        window.set_style(self.text_color);
        if let Mode::Inventory(InvMode::Actions(ch)) = self.mode {
            let item = player.items_backpack.get(&ch).or_else(|| {
                player
                    .items_equipped
                    .values()
                    .find_map(|(letter, item)| (*letter == ch).then_some(item))
            });
            if let Some(item) = item {
                window.write(&format!("{} - {}\n\n", ch, item.description()));
                for (shortcut, label, _) in self.inventory_actions(ch) {
                    window.write(&format!(" [{}] {}\n", shortcut, label));
                }
                if let Some(hint) = item.usage_hint() {
                    window.write(&format!("\n{hint}\n"));
                }
            }
            window.write("\n[ESC] back to inventory\n");
            return;
        }
        if !player.items_equipped.is_empty() {
            window.write("Equipped: \n");
            let mut equipped: Vec<_> = player.items_equipped.iter().collect();
            equipped.sort_by_key(|(_, (ch, _))| *ch);
            for (slot, (ch, i)) in equipped {
                window.write(&format!(" {} - {} [{:?}]\n", ch, i.description(), slot));
            }
            window.write("\n");
        }

        window.write("Inventory: \n");
        if !player.items_backpack.is_empty() {
            let mut backpack: Vec<_> = player.items_backpack.iter().collect();
            backpack.sort_by_key(|(ch, _)| **ch);
            for (ch, i) in backpack {
                window.write(&format!(" {} - {}\n", ch, i.description()));
            }
        }

        if self.mode == Mode::Inventory(InvMode::View) {
            window.write("\nLetter: actions | ESC: close\n");
        } else {
            window.write("\n[ESC] to close...\n");
        }
    }

    fn draw_stats(&self) {
        self.windows.stats.clear();
        let turn = self.engine.turn();
        let window = &self.windows.stats;
        let player = self.player();
        let cur_loc = self.current_location();

        let (ac, ev) = (player.stats.base.ac, player.stats.base.ev);
        let (dmg, acc) = (player.stats.melee_dmg, player.stats.melee_acc);

        window.move_to(0, 0);

        let mut y = 0;
        window.move_to(y, 0);
        self.draw_val(window, "Str", player.stats.base.str_);
        window.move_to(y, 7);
        self.draw_val(window, "DMG", dmg);
        window.move_to(y, 15);
        self.draw_val(window, "ACC", acc);

        y += 1;
        window.move_to(y, 0);
        self.draw_val(window, "Int", player.stats.base.int);
        window.move_to(y, 7);
        self.draw_val(window, " AC", ac);
        window.move_to(y, 15);
        self.draw_val(window, "EV", ev);

        y += 1;
        window.move_to(y, 0);
        self.draw_val(window, "Dex", player.stats.base.dex);

        y += 1;
        self.draw_resources(window, y);

        let slots = [
            ("R", Slot::RHand),
            ("L", Slot::LHand),
            ("F", Slot::Feet),
            ("B", Slot::Body),
            ("H", Slot::Head),
            ("C", Slot::Cloak),
            ("Q", Slot::Quick),
        ];

        for (i, &(string, slot)) in slots.iter().enumerate() {
            if i & 1 == 0 {
                y += 1;
                window.move_to(y, 0);
            } else {
                window.move_to(y, 14);
            }

            self.draw_item(window, player, string, slot);
        }

        y += 1;
        window.move_to(y, 0);

        let pos = if self.mode == Mode::Examine {
            self.target_pos.unwrap_or(self.player().pos)
        } else {
            player.pos
        };

        let head = pos.coord + pos.dir;
        let descr = self.tile_description(head, player, cur_loc);
        self.draw_tile_readout(window, "In front", &descr);

        y += 1;
        window.move_to(y, 0);
        // Describe the surface under the selected pose, never its occupant.
        self.draw_tile_readout(
            window,
            "Below",
            &Self::below_description(pos.coord, player, cur_loc),
        );

        y += 1;
        window.move_to(y, 0);
        self.draw_turn(window, "Turn", turn);
        self.draw_turn(window, "Level", cur_loc.level);

        self.draw_sight(window, y + 1);
    }

    /// Keep each tile readout on its own row without spilling into other stats.
    fn draw_tile_readout(&self, window: &Window, label: &str, description: &str) {
        self.draw_label(window, label);
        window.set_style(self.text_color);
        let available = (window.size().1 as usize).saturating_sub(label.len() + 2);
        let description: String = description.chars().take(available).collect();
        window.write(&format!(" {description}"));
    }

    // TODO: Consider the distance to the Item to print something
    // like "you see x in the distance", "you find yourself in x".
    fn format_areas<I>(&self, mut i: I) -> Option<String>
    where
        I: Iterator,
        <I as Iterator>::Item: fmt::Display,
    {
        match i.next() {
            Some(descr) => {
                let mut s = String::new();
                write!(&mut s, "You see: ").unwrap();
                write!(&mut s, "{}", descr).unwrap();

                for ref descr in i {
                    write!(&mut s, ", ").unwrap();
                    write!(&mut s, "{}", descr).unwrap();
                }

                write!(&mut s, ".").unwrap();
                Some(s)
            }
            _ => None,
        }
    }

    fn turn_to_color(&self, turn: u64) -> Option<Style> {
        let dturn = self.engine.turn() - turn;

        let fg = if dturn < 1 {
            Some(color::LOG_1_FG)
        } else if dturn < 4 {
            Some(color::LOG_2_FG)
        } else if dturn < 16 {
            Some(color::LOG_3_FG)
        } else if dturn < 32 {
            Some(color::LOG_4_FG)
        } else if dturn < 64 {
            Some(color::LOG_5_FG)
        } else {
            None
        };

        fg.map(|fg| color::style(fg, color::BACKGROUND_BG))
    }

    fn tile_description(
        &self,
        coord: Coordinate,
        astate: &Actor,
        gstate: &game::Location,
    ) -> String {
        if !astate.knows(coord) {
            return "Unknown".to_owned();
        }

        let actor_descr = if astate.sees(coord) || astate.is_dead() {
            gstate.at(coord).actor_map_or(None, |a| {
                Some(
                    match a.race {
                        // Race::Pony => "A Pony",
                        Race::Rat => "A rat",
                        Race::Goblin => "Goblin",
                        Race::Troll => "Troll",
                        Race::Human => "Human",
                        Race::Elf => "Elf",
                        Race::Dwarf => "Dwarf",
                        Race::Wolf => "Wolf",
                        Race::CaveBeetle => "Cave beetle",
                        Race::Skeleton => "Skeleton",
                    }
                    .to_owned(),
                )
            })
        } else {
            None
        };

        actor_descr.unwrap_or_else(|| Self::tile_surface_description(coord, gstate))
    }

    /// Describe the selected surface without reading live contents outside sight.
    fn below_description(coord: Coordinate, player: &Actor, location: &game::Location) -> String {
        if coord == player.pos.coord || player.sees(coord) {
            return Self::tile_surface_description(coord, location);
        }
        if !player.knows(coord) {
            return "Unknown".to_owned();
        }
        if location.remembered_chests.contains_key(&coord) {
            return "chest".to_owned();
        }
        // Terrain, features and loose items have no last-seen snapshot. Do not
        // substitute their current state for an observation of this hidden tile.
        "Out of sight".to_owned()
    }

    /// Describe ground contents with the same item, feature, then terrain priority.
    fn tile_surface_description(coord: Coordinate, gstate: &game::Location) -> String {
        let at = gstate.at(coord);
        let tile = at.tile();
        if let Some(item) = at.item() {
            return item.description();
        }
        if let Some(chest) = gstate.chests.get(&coord) {
            return if gstate.player().sees(coord) {
                chest.description()
            } else {
                "chest".to_owned()
            };
        }
        if let Some(feature) = tile.feature {
            if feature == tile::Entrance {
                return match gstate.exits.get(&coord) {
                    Some(link) => format!("return to level {} (<)", link.destination.0),
                    None => "return passage (<)".to_owned(),
                };
            }
            if feature.is_exit() {
                return format!("to {} (>)", gstate.biome.next().description());
            }
            return feature.description().to_owned();
        }
        match tile.type_ {
            tile::Wall => "a wall".to_owned(),
            tile::Empty => match tile.area.map(|a| a.type_) {
                Some(area::Room(_)) => "room".to_owned(),
                None => "nothing".to_owned(),
            },
            _ => tile.type_.description().to_owned(),
        }
    }

    fn draw_log(&self) {
        self.windows.log.clear();
        let window = &self.windows.log;

        window.move_to(0, 0);

        match self.mode {
            Mode::GoTo => {
                window.write("Go to where?\n");
            }
            Mode::Inventory(InvMode::Drop_) => {
                window.write("Drop what?\n");
            }
            Mode::Inventory(InvMode::Equip) => {
                window.write("Equip/use what?\n");
            }
            _ => {}
        }

        let cury = window.cursor_y();
        let maxy = window.size().0 - 1;
        if cury < maxy {
            for i in self.log.borrow().iter().take((maxy - cury) as usize) {
                if let Some(style) = self.turn_to_color(i.turn) {
                    window.set_style(style);
                    window.write(&i.text);
                }
                window.write_char('\n');
            }
        }
    }

    fn draw_intro(&mut self) {
        let window = &self.windows.full;
        window.clear();

        window.write("This game is in very early development. We are looking for feedback and contributors!\n\n");
        window.write("See help: https://github.com/dpc/rhex/wiki/Help-How-to-Play\n");
        window.write("Join chat: https://gitter.im/dpc/rhex\n");
        window.write("Project page: https://github.com/dpc/rhex\n\n");
        window.write(&format!(
            "You can press {} in the game for offline help.\n\n",
            KEY_HELP as u8 as char
        ));
        window.write("Press anything to start.");
    }

    fn draw_pickrace(&mut self) {
        let window = &self.windows.full;
        window.clear();
        window.write("Pick your race\n\n");
        window.write("a) Human\n");
        window.write("b) Elf\n");
        window.write("c) Dwarf\n");
    }

    fn draw_help(&mut self) {
        let window = &self.windows.full;
        window.clear();
        window.write(&format!(
            "Current input mode: {} (F2 to change)\n\n",
            self.input_mode
        ));
        window.write("Turn left/right: h/l or Left/Right arrows\n");
        window.write("Move forward/back: k/j or Up/Down arrows\n");
        window.write("  Shift + k/j also move forward/back.\n");
        window.write("Strafe front-left/right: Shift + h/l\n");
        window.write("  Keep facing; no attacks. Doors/thickets still work.\n");
        window.write("Charge: c\n");
        window.write("Wait: .\n");
        window.write("Use exit: > or < (on its tile)\n");
        window.write("Autoexplore: o\n");
        window.write("Go to exit: G, then > or <\n");
        window.write("Examine: x\n");
        window.write("Interact in front (item, chest, then door): ,\n");
        window.write("Inventory: I, item letter, then action (ESC goes back)\n");
        window.write("Equip/Use: E\n");
        window.write("Drop: D\n");
        window.write("Ranged/Throw: f (not fully working)\n");
        if self.in_vi_input_mode() {
            window.write("Quit: ESC/q\n");
        } else {
            window.write("Quit: ESC\n");
        }
    }

    fn draw_quit(&mut self) {
        let window = &self.windows.full;
        window.clear();

        let (max_y, max_x) = window.size();
        let text = "Quit. Are you sure?";

        window.move_to(max_y / 2, (max_x - text.chars().count() as i32) / 2);
        window.write(text);
    }

    fn event(&mut self, event: Event) {
        match event {
            Event::Log(logev) => match logev {
                LogEvent::AutoExploreDone => self.log("Nothing else to explore."),
                LogEvent::AutoExploreBlocked => self.log("Can't get there."),
            },
        }
    }
}

#[cfg(test)]
#[path = "ui_input_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "ui_state_tests.rs"]
mod state_tests;

#[cfg(test)]
#[path = "ui_property_tests.rs"]
mod property_tests;

#[cfg(test)]
#[path = "ui_snapshot_tests.rs"]
mod snapshot_tests;

#[cfg(test)]
#[path = "ui_sight_tests.rs"]
mod sight_tests;
