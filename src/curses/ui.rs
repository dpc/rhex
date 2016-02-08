use std::cell::RefCell;
use std::collections::{VecDeque};
use std::env;
use std;
use std::{thread, cmp, fmt};
use std::io::Write;
use std::fmt::Write as FmtWrite;

use chrono::{self, Duration};
use num::integer::Integer;

use game::Action::*;

use ncurses as nc;
use hex2d::{Position, Coordinate, Angle, Left, Right, Forward, Back, ToCoordinate};

use hex2dext::algo::bfs;

use super::consts::*;
use super::{color, Window};
use super::{LogEntry, AutoMoveType, AutoMoveAction, LogEvent, Event, GoToType};
use super::Result;
use super::map::MapRenderer;

use game::{actor, Location, Actor, area};
use game;
use game::actor::{Race, Slot};
use game::tile;
use util;

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
            InputMode::Normal => write!(f, "Normal")
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
    fn after_resize() -> Self {
        let (max_y, max_x) = Ui::screen_size();

        let mid_x = max_x - 30;
        let mid_y = 12;

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

pub struct Ui {
    calloc: RefCell<color::Allocator>,

    windows: Windows,

    mode: Mode,
    log: RefCell<VecDeque<LogEntry>>,
    target_pos: Option<Position>,

    label_color: nc::attr_t,
    text_color: nc::attr_t,
    text_gray_color: nc::attr_t,
    red_color: nc::attr_t,
    green_color: nc::attr_t,

    input_mode : InputMode,

    engine: game::Engine,
    exit: bool,
    spawned: bool,

    automoving: Option<AutoMoveType>,
    automoving_stopped_turn: u64,

    anim_frame_count: u32,
    next_turn_ts:  chrono::datetime::DateTime<chrono::offset::local::Local>,
    next_anim_frame_ts:  chrono::datetime::DateTime<chrono::offset::local::Local>,

    game_action_queue: VecDeque<game::Action>,

    map_renderer: MapRenderer,
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
    pub fn new() -> Result<Self> {

        if env::var_os("ESCDELAY").is_none() {
            env::set_var("ESCDELAY", "25");
        }

        nc::setlocale(nc::constants::LcCategory::all, "");

        nc::initscr();
        nc::start_color();
        nc::keypad(nc::stdscr, true);
        nc::noecho();
        nc::raw();
        nc::timeout(0);
        nc::flushinp();
        nc::curs_set(nc::CURSOR_VISIBILITY::CURSOR_INVISIBLE);

        assert!(nc::has_colors());

        let mut calloc = color::Allocator::new();
        let label_color = nc::COLOR_PAIR(calloc.get(color::LABEL_FG, color::BACKGROUND_BG));
        let text_color = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG));
        let text_gray_color = nc::COLOR_PAIR(calloc.get(color::GRAY[10], color::BACKGROUND_BG));
        let green_color = nc::COLOR_PAIR(calloc.get(color::GREEN_FG, color::BACKGROUND_BG));
        let red_color = nc::COLOR_PAIR(calloc.get(color::RED_FG, color::BACKGROUND_BG));

        let engine = game::Engine::new();


        let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG));
        nc::bkgd(cpair);
        nc::doupdate();

        let mut ui = Ui {
            calloc: RefCell::new(calloc),
            windows: Windows::after_resize(),
            mode: Mode::FullScreen(FSMode::Intro),
            target_pos: None,
            log: RefCell::new(VecDeque::new()),

            label_color: label_color,
            text_color: text_color,
            text_gray_color: text_gray_color,
            red_color: red_color,
            green_color: green_color,

            input_mode : InputMode::Normal,

            exit: false,
            spawned: false,

            engine: engine,
            automoving: None,
            automoving_stopped_turn: 0,

            anim_frame_count: 0,
            next_turn_ts: chrono::Local::now(),
            next_anim_frame_ts: chrono::Local::now(),
            game_action_queue: VecDeque::new(),
            map_renderer: MapRenderer::new(UNICODE_DOT),
        };
        ui.display_intro();
        ui.resize();
        Ok(ui)
    }

    pub fn initial_spawn(&mut self, race: actor::Race) {
        self.engine.initial_spawn(race);
        let player_id = self.engine.current_location().player_id();
        self.engine_change(player_id);
        self.update_changes();
        self.spawned = true;
        self.next_turn_ts = chrono::Local::now()
    }

    pub fn screen_size() -> (i32, i32) {
        Ui::window_size(nc::stdscr)
    }

    pub fn window_size(window: nc::WINDOW) -> (i32, i32) {
        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(window, &mut max_y, &mut max_x);

        (max_y, max_x)
    }

    pub fn resize(&mut self) {
        self.windows = Windows::after_resize();
        let window = self.windows.map.window;
        let (y, x) = Ui::window_size(window);
        self.map_renderer.resize(x as usize, y as usize);
        self.redraw();
    }

    /// Mark the screen for redraw
    pub fn redraw(&mut self) {
        self.anim_frame_count = 0;
        self.next_anim_frame_ts = chrono::Local::now();
    }

    pub fn redraw_now(&mut self) {
        match self.mode {
            Mode::Normal | Mode::Examine | Mode::Inventory(_) | Mode::Target(_) | Mode::GoTo => {
                if let Mode::Inventory(_) = self.mode {
                    self.draw_inventory();
                } else {
                    self.draw_map();
                }

                self.draw_log();

                self.draw_stats();
            }
            Mode::FullScreen(fs_mode) => {
                match fs_mode {
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
                }
            }
        }

        let (max_y, max_x) = Ui::screen_size();

        nc::mv(max_y - 1, max_x - 1);
        nc::doupdate();
    }

    pub fn is_automoving(&self) -> bool {
        self.automoving.is_some()
    }

    pub fn should_stop_automoving(&self) -> bool {
        let player = self.player();
        let cur_loc = self.current_location();

        !player.was_attacked_by.is_empty() || player.discovered_areas.iter().any(|_| true) ||
        player.visible.iter().any(|&coord| {
            cur_loc.at(coord)
                   .actor_map_or(false, |a| a.race == actor::Race::Rat)
        }) ||
        player.discovered.iter().any(|&coord| {
            cur_loc.at(coord)
                   .item_map_or(false, |_| true)
        }) ||
        player.heard
              .iter()
              .any(|(c, _)| !player.sees(*c)) || player.discovered_stairs(cur_loc)
    }

    pub fn automove_action(&self, movetype: AutoMoveType) -> AutoMoveAction {
        let player = self.player();
        let cur_loc = self.current_location();
        match movetype {
            AutoMoveType::Explore => self.autoexplore_action(),
            AutoMoveType::GoTo(gototype) => self.goto_action(gototype),
            AutoMoveType::Walk => {
                if cur_loc.at(player.head()).tile().is_passable() {
                    AutoMoveAction::Action(game::Action::Move(Angle::Forward))
                } else {
                    AutoMoveAction::Finish
                }
            }
        }
    }

    fn goto_action(&self, _gototype: GoToType) -> AutoMoveAction {
        let player = self.player();
        let cur_loc = self.current_location();

        let start = player.pos.coord;

        let mut bfs = bfs::Traverser::new(|c| {
                                              c == start ||
                                              (cur_loc.at(c).tile().is_passable() &&
                                               player.knows(c))
                                          },
                                          |c| {
                                              cur_loc.at(c).tile().feature ==
                                              Some(tile::Feature::Stairs)
                                          },
                                          start);

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

        let mut bfs = bfs::Traverser::new(|c| c == start || cur_loc.at(c).tile().is_passable(),
                                          |c| !player.knows(c),
                                          start);

        if let Some(dst) = bfs.find() {
            if let Some(neigh) = bfs.backtrace_last(dst) {

                let ndir = player.pos.coord.direction_to_cw(neigh).expect("bfs gave me trash");
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
        if self.automoving.is_some() &&
            self.automoving_stopped_turn != self.engine.turn() && self.should_stop_automoving() {
                self.automoving_stop();
            }

        let player_id = self.current_location().player_id();

        if actor_id == player_id {
            if self.is_automoving() {
                self.next_turn_ts = chrono::Local::now() + Duration::milliseconds(50);
            } else if self.player().is_dead() {
                self.next_turn_ts = chrono::Local::now() + Duration::milliseconds(50);
            }
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
        self.next_turn_ts <= chrono::Local::now()
    }

    pub fn is_next_anim_frame_time(&self) -> bool {
        self.next_anim_frame_ts <= chrono::Local::now()
    }

    pub fn run_engine_turn(&mut self) {
        if self.spawned && self.is_next_turn_time() {
            let player_id = self.current_location().player_id();
            let mut player_acted = false;

            if self.engine.is_turn_done() {
                self.engine.start_turn()
            }

            if self.engine.needs_player_input() {
                if let Some(movetype) = self.automoving {
                    match self.automove_action(movetype) {
                        AutoMoveAction::Blocked => {
                            match movetype {
                                AutoMoveType::Explore | AutoMoveType::GoTo(_) => {
                                    self.event(Event::Log(LogEvent::AutoExploreBlocked));
                                }
                                _ => {}
                            }
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
                } else if let Some(action) = self.game_action_queue.pop_front() {
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
                self.update_changes();
                self.redraw();
            }
        }
    }

    pub fn maybe_redraw_now(&mut self) {
        if self.is_next_anim_frame_time() {
            self.redraw_now();
            self.anim_frame_count += 1;
            self.next_anim_frame_ts = chrono::Local::now() + Duration::milliseconds(100);
        } else if self.game_action_queue.is_empty() {
            thread::sleep(std::time::Duration::new(0, 10_000_000));
        }
    }

    pub fn run_once(&mut self) {
        self.run_engine_turn();
        self.input_handle();
        self.maybe_redraw_now();
    }

    pub fn run(&mut self) {
        while !self.exit {
            self.run_once();
        }
    }

    /// Handle input.
    pub fn input_handle(&mut self) {
        loop {
            let ch = nc::getch();
            if ch == nc::KEY_RESIZE {
                self.resize();
                self.redraw();
            }
            if ch == -1 {
                return;
            }
            if self.automoving.is_some() {
                self.automoving_stop();
                continue;
            }

            self.input_handle_key(ch);
        }
    }

    pub fn action_push(&mut self, action: game::Action) {
        self.game_action_queue.push_back(action);
    }

    fn mode_switch_to(&mut self, mode: Mode) {
        self.mode = mode;
        self.redraw();
    }

    pub fn queue_ranged(&mut self, coord: Coordinate) {
        self.action_push(game::Action::Ranged(coord))
    }

    pub fn queue_action(&mut self, action : game::Action) {
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

    fn key_to_action_normal(&self, key : i32) -> Option<Action> {
        let action = match (key, self.in_vi_input_mode()) {
            (KEY_LOWH, true) |
            (KEY_LOWA, false) |
            (nc::KEY_LEFT, _) => Game(Turn(Left)),
            (KEY_LOWL, true) |
            (KEY_LOWD, false) |
            (nc::KEY_RIGHT, _) => Game(Turn(Right)),
            (KEY_LOWK, true) |
            (KEY_LOWW, false) |
            (nc::KEY_UP, _) => Game(Move(Forward)),
            (KEY_LOWC, _) => Game(Charge),
            (KEY_LOWU, true) => Game(Spin(Left)),
            (KEY_LOWI, true) => Game(Spin(Right)),
            (KEY_CAPH, true) |
            (KEY_LOWQ, false) => Game(Move(Left)),
            (KEY_CAPL, true) |
            (KEY_LOWE, false) => Game(Move(Right)),
            (KEY_LOWJ, true) |
            (KEY_LOWS, false) |
            (nc::KEY_DOWN, _) => Game(Move(Back)),
            (KEY_DOT, _) => Game(Wait),
            (KEY_COMMA, _) => Game(Pick),
            (KEY_DESCEND, _) => Game(Descend),
            (KEY_LOWO, _) => AutoMove(AutoMoveType::Explore),
            (KEY_CAPK, true) => AutoMove(AutoMoveType::Walk),
            (KEY_CAPW, false) => AutoMove(AutoMoveType::Walk),
            (KEY_LOWQ, true) |
            (KEY_ESC, _) => ModeSwitch(Mode::FullScreen(FSMode::Quit)),
            (KEY_CAPI, _) => ModeSwitch(Mode::Inventory(InvMode::View)),
            (KEY_CAPE, _) => ModeSwitch(Mode::Inventory(InvMode::Equip)),
            (KEY_CAPD, _) => ModeSwitch(Mode::Inventory(InvMode::Drop_)),
            (KEY_LOWX, _) => ModeSwitch(Mode::Examine),
            (KEY_LOWF, _) => ModeSwitch(Mode::Target(TargetMode::Ranged)),
            (KEY_HELP, _) => ModeSwitch(Mode::FullScreen(FSMode::Help)),
            (KEY_GOTO, _) => ModeSwitch(Mode::GoTo),
            _ => { return None}
        };
        Some(action)
    }

    fn key_to_action_global(&self, key : i32) -> Option<GlobalAction> {
        let action = match key {
            nc::KEY_F2 => ToggleInputMode,
            _ => return None,
        };

        Some(action)
    }

    pub fn input_handle_key_mode_specific(&mut self, key : i32) {
        match self.mode {
            Mode::FullScreen(fs_mode) => {
                match fs_mode {
                    FSMode::Quit => {
                        match key {
                            KEY_LOWY | KEY_CAPY => self.exit = true,
                            _ => self.mode_switch_to(Mode::Normal),
                        }
                    }
                    FSMode::Intro => {
                        match key {
                            _ => self.mode_switch_to(Mode::FullScreen(FSMode::PickRace)),
                        }
                    }
                    FSMode::PickRace => {
                        match key {
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
                        }
                    }
                    _ => {
                        match key {
                            _ => self.mode_switch_to(Mode::Normal),
                        }
                    }
                }
            }
            Mode::Normal => if let Some(action) = self.key_to_action_normal(key) {
                match action {
                    Game(action) => self.queue_action(action),
                    ModeSwitch(mode @ Mode::Target(TargetMode::Ranged)) => {
                        if self.player().can_attack_ranged() {
                            self.target_pos = None;
                            self.mode_switch_to(mode);
                        }
                    },
                    ModeSwitch(mode) => {
                        self.target_pos = None;
                        self.mode_switch_to(mode);
                    }
                    AutoMove(automove_type) => self.automoving = Some(automove_type),
                }
            },
            Mode::Inventory(InvMode::Equip) => {
                let ch = key as u8 as char;
                match key {
                    KEY_ESC => self.mode_switch_to(Mode::Normal),
                    _ => match ch {
                        'a'...'z' | 'A'...'Z' => {
                            if self.player().item_letter_taken(ch) {
                                self.queue_equip(ch)
                            }
                        }
                        _ => {}
                    }
                }
            }
            Mode::Inventory(InvMode::View) => {
                let ch = key as u8 as char;
                match key {
                    KEY_ESC => self.mode_switch_to(Mode::Normal),
                    _ => match ch {
                        'a'...'z' | 'A'...'Z' => {}
                        _ => {}
                    }
                }
            }
            Mode::Inventory(InvMode::Drop_) => {
                let ch = key as u8 as char;
                match key {
                    KEY_ESC => self.mode_switch_to(Mode::Normal),
                    _ => match ch {
                        'a'...'z' | 'A'...'Z' => {
                            if self.player().item_letter_taken(ch as u8 as char) {
                                self.queue_drop(ch as u8 as char)
                            }
                        }
                        _ => {}
                    }
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
                        self.target_pos = Some(pos + (pos.dir + Angle::Back).to_coordinate());
                        self.redraw();
                    }
                    KEY_LOWK => {
                        self.target_pos = Some(pos + pos.dir.to_coordinate());
                        self.redraw();
                    }
                    KEY_CAPK => {
                        self.target_pos = Some(pos + pos.dir.to_coordinate().scale(5));
                        self.redraw();
                    }
                    KEY_CAPJ => {
                        self.target_pos = Some(pos +
                                               (pos.dir + Angle::Back).to_coordinate().scale(5));
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
                        let target = self.target_pos.unwrap();
                        self.target_pos = None;
                        self.mode_switch_to(Mode::Normal);
                        self.queue_ranged(target.coord);
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
                if let KEY_DESCEND = key {
                    self.automoving = Some(AutoMoveType::GoTo(GoToType::Stairs))
                }
                self.mode_switch_to(Mode::Normal);
            }
        }
    }

    pub fn input_handle_key(&mut self, key : i32) {
        if let Some(_) = self.key_to_action_global(key) {
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

        let discovered_areas = player.discovered_areas
                                      .iter()
                                      .filter_map(|coord| cur_loc.at(*coord).tile().area);

        if let Some(s) = self.format_areas(discovered_areas.map(|area| area.type_)) {
            self.log(&s);
        }

        for item_coord in player.discovered
                                .iter()
                                .filter(|&coord| cur_loc.at(*coord).item_map_or(false, |_| true)) {
            let item_descr = cur_loc.at(*item_coord)
                                    .item_map_or("".to_owned(), |i| i.description());
            self.log(&format!("You've found {}.", item_descr));
        }

        if player.discovered_stairs(cur_loc) {
            self.log("You've found stairs.");
        }

        for res in &player.was_attacked_by {
            if res.success {
                self.log(&format!("{} hit you {}for {} dmg.",
                                  res.who,
                                  if res.behind {
                                      "from behind "
                                  } else {
                                      ""
                                  },
                                  res.dmg));
            } else {
                self.log(&format!("{} missed you.", res.who));
            }
        }

        for res in &player.did_attack {
            if res.success {
                self.log(&format!("You hit {} {}for {} dmg.",
                                  res.who,
                                  if res.behind {
                                      "from behind "
                                  } else {
                                      ""
                                  },
                                  res.dmg));
            } else {
                self.log(&format!("You missed {}.", res.who));
            }
        }

        let noises = player.heard
                           .iter()
                           .filter(|&(c, _)| *c != player.pos.coord)
                           .filter(|&(c, _)| !player.sees(*c));

        for (_, &noise) in noises {
            self.log(&format!("You hear {}.", noise.description()));
        }
    }


    pub fn log(&self, s: &str) {
        let turn = self.engine.turn();
        self.log.borrow_mut().push_front(LogEntry {
            text: s.to_owned(),
            turn: turn,
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
            ref calloc,
            anim_frame_count,
            ..
        } = *self;

        map_renderer.update(engine.current_location(), mode, target_pos, anim_frame_count);
        map_renderer.draw_into(&windows.map, &calloc);
        nc::wnoutrefresh(windows.map.window);
    }

    fn draw_stats_bar(&self, window: nc::WINDOW, y: i32, name: &str, cur: i32, prev: i32, max: i32) {

        let (max_y, max_x) = Ui::window_size(window);

        if max_x < 6 || max_y < 1 {
            // Don't draw anything on too small window
            return;
        }

        let cur = cmp::max(cur, 0) as u32;
        let prev = cmp::max(prev, 0) as u32;
        let max = cmp::max(max, 1) as u32;

        nc::wattron(window, self.label_color);
        nc::mvwaddstr(window, y, 0, &format!("{}: ", name));

        let width = max_x as u32 - 4 - name.chars().count() as u32;
        let cur_w = cur * width / max;
        let prev_w = prev * width / max;

        nc::waddch(window, self.text_color | ('[' as nc::chtype));
        for i in 0..width {
            let ch = match (i < cur_w, i < prev_w) {
                (true, true) => self.text_color | ('=' as nc::chtype),
                (false, true) => self.red_color | ('-' as nc::chtype),
                (true, false) => self.green_color | ('+' as nc::chtype),
                (false, false) => break,
            };
            nc::waddch(window, ch);
        }
        nc::mvwaddch(window, y, max_x-1, self.text_color | (']' as nc::chtype));
    }

    fn draw_turn<T>(&self, window: nc::WINDOW, label: &str, val: T)
        where T: Integer + fmt::Display
    {
        self.draw_label(window, label);

        nc::wattron(window, self.text_color);
        nc::waddstr(window, &format!(" {:<8}", val));
    }

    fn draw_val<T>(&self, window: nc::WINDOW, label: &str, val: T)
        where T: Integer + fmt::Display
    {
        self.draw_label(window, label);

        nc::wattron(window, self.text_color);
        nc::waddstr(window, &format!("{:>2} ", val));
    }

    fn draw_label(&self, window: nc::WINDOW, label: &str) {
        nc::wattron(window, self.label_color);
        nc::waddstr(window, &format!("{}:", label));
    }

    fn draw_item(&self, window: nc::WINDOW, astate: &Actor, label: &str, slot: Slot) {
        self.draw_label(window, label);

        if slot == Slot::RHand && !astate.can_attack() {
            nc::wattron(window, self.text_gray_color);
        } else {
            nc::wattron(window, self.text_color);
        }

        if let Some(&(_, ref item)) = astate.items_equipped.get(&slot) {
            nc::waddstr(window, &format!("{:^13}", item.description()));
        } else {
            nc::waddch(window, '-' as nc::chtype);
        }
    }

    fn draw_inventory(&self) {
        self.windows.map.clear(&self.calloc);
        let window = self.windows.map.window;
        let player = self.player();

        nc::wmove(window, 0, 0);

        nc::wattron(window, self.text_color);
        if !player.items_equipped.is_empty() {
            nc::waddstr(window, &format!("Equipped: \n"));
            for (slot, &(ref ch, ref i)) in &player.items_equipped {
                nc::waddstr(window,
                            &format!(" {} - {} [{:?}]\n", ch, i.description(), slot));
            }
            nc::waddstr(window, &format!("\n"));
        }

        nc::waddstr(window, &format!("Inventory: \n"));
        if !player.items_backpack.is_empty() {
            for (ch, i) in &player.items_backpack {
                nc::waddstr(window, &format!(" {} - {}\n", ch, i.description()));
            }
        }

        nc::waddstr(window, &format!("\n[ESC] to close...\n"));

        nc::wnoutrefresh(window);
    }

    fn draw_stats(&self) {
        self.windows.stats.clear(&self.calloc);
        let turn = self.engine.turn();
        let window = self.windows.stats.window;
        let player = self.player();
        let cur_loc = self.current_location();

        let (ac, ev) = (player.stats.base.ac, player.stats.base.ev);
        let (dmg, acc) = (player.stats.melee_dmg, player.stats.melee_acc);

        nc::wmove(window, 0, 0);

        let mut y = 0;
        nc::wmove(window, y, 0);
        self.draw_val(window, "Str", player.stats.base.str_);
        nc::wmove(window, y, 7);
        self.draw_val(window, "DMG", dmg);
        nc::wmove(window, y, 15);
        self.draw_val(window, "ACC", acc);

        y += 1;
        nc::wmove(window, y, 0);
        self.draw_val(window, "Int", player.stats.base.int);
        nc::wmove(window, y, 7);
        self.draw_val(window, " AC", ac);
        nc::wmove(window, y, 15);
        self.draw_val(window, "EV", ev);

        y += 1;
        nc::wmove(window, y, 0);
        self.draw_val(window, "Dex", player.stats.base.dex);

        y += 1;
        self.draw_stats_bar(window, y,
                            "HP",
                            player.hp,
                            player.saved_hp,
                            player.stats.base.max_hp);

        y += 1;
        self.draw_stats_bar(window, y,
                            "MP",
                            player.mp,
                            player.saved_mp,
                            player.stats.base.max_mp);

        y += 1;
        self.draw_stats_bar(window, y,
                            "SP",
                            player.sp,
                            player.saved_sp,
                            player.stats.base.max_sp);

        let slots = [("R", Slot::RHand),
                     ("L", Slot::LHand),
                     ("F", Slot::Feet),
                     ("B", Slot::Body),
                     ("H", Slot::Head),
                     ("C", Slot::Cloak),
                     ("Q", Slot::Quick)];

        for (i, &(string, slot)) in slots.iter().enumerate() {
            if i & 1 == 0 {
                y += 1;
                nc::wmove(window, y, 0);
            } else {
                nc::wmove(window, y, 14);
            }

            self.draw_item(window, player, string, slot);
        }

        y += 1;
        nc::wmove(window, y, 0);

        let pos = if self.mode == Mode::Examine {
            self.target_pos.unwrap_or(self.player().pos)
        } else {
            player.pos
        };

        let head = pos.coord + pos.dir;
        let descr = self.tile_description(head, player, cur_loc);
        self.draw_label(window, "In front");
        nc::wattron(window, self.text_color);
        nc::waddstr(window, &format!(" {}", descr));

        y += 1;
        nc::wmove(window, y, 0);
        self.draw_turn(window, "Turn", turn);
        self.draw_turn(window, "Level", cur_loc.level);

        nc::wnoutrefresh(window);
    }

    // TODO: Consider the distance to the Item to print something
    // like "you see x in the distance", "you find yourself in x".
    fn format_areas<I>(&self, mut i: I) -> Option<String>
        where I: Iterator,
              <I as Iterator>::Item: fmt::Display
    {
        if let Some(descr) = i.next() {
            let mut s = String::new();
            write!(&mut s, "{}", "You see: ").unwrap();
            write!(&mut s, "{}", descr).unwrap();

            for ref descr in i {
                write!(&mut s, ", ").unwrap();
                write!(&mut s, "{}", descr).unwrap();
            }

            write!(&mut s, ".").unwrap();
            Some(s)
        } else {
            None
        }
    }

    fn turn_to_color(&self, turn: u64, calloc: &RefCell<color::Allocator>) -> Option<i16> {
        let mut calloc = calloc.borrow_mut();

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

        fg.map(|fg| calloc.get(fg, color::BACKGROUND_BG))
    }

    fn tile_description(&self,
                        coord: Coordinate,
                        astate: &Actor,
                        gstate: &game::Location)
                        -> String {
        if !astate.knows(coord) {
            return "Unknown".to_owned();
        }

        let tile_type = gstate.at(coord).tile().type_;
        let tile = gstate.at(coord).tile();
        let feature_descr = tile.feature.map(|f| f.description().to_owned());
        let item_descr = gstate.at(coord).item_map_or(None, |i| Some(i.description().to_owned()));

        let actor_descr = if astate.sees(coord) || astate.is_dead() {
            gstate.at(coord).actor_map_or(None, |a| {
                Some(match a.race {
                         // Race::Pony => "A Pony",
                         Race::Rat => "A rat",
                         Race::Goblin => "Goblin",
                         Race::Troll => "Troll",
                         Race::Human => "Human",
                         Race::Elf => "Elf",
                         Race::Dwarf => "Dwarf",
                     }
                     .to_owned())
            })
        } else {
            None
        };

        match (tile_type, feature_descr, actor_descr, item_descr) {

            (_, _, Some(a_descr), _) => a_descr,
            (_, _, _, Some(i_descr)) => i_descr,
            (_, Some(f_descr), _, _) => f_descr.to_owned(),
            (tile::Wall, _, _, _) => "a wall".to_owned(),
            (tile::Empty, _, _, _) => {
                match tile.area.and_then(|a| Some(a.type_)) {
                    Some(area::Room(_)) => "room".to_owned(),
                    None => "nothing".to_owned(),
                }
            }
            _ => tile.type_.description().to_owned(),
        }
    }

    fn draw_log(&self) {
        self.windows.log.clear(&self.calloc);
        let window = self.windows.log.window;

        nc::wmove(window, 0, 0);

        match self.mode {
            Mode::GoTo => {
                nc::waddstr(window, &format!("Go to where?\n"));
            }
            Mode::Inventory(InvMode::Drop_) => {
                nc::waddstr(window, &format!("Drop what?\n"));
            }
            Mode::Inventory(InvMode::Equip) => {
                nc::waddstr(window, &format!("Equip/use what?\n"));
            }
            _ => {}
        }

        let cury = nc::getcury(window);
        let maxy = nc::getmaxy(window) - 1;
        if cury < maxy {
            let mut prevcolor = -1;
            for i in self.log.borrow().iter().take((maxy - cury) as usize) {
                if let Some(color) = self.turn_to_color(i.turn, &self.calloc) {
                    if prevcolor != color {
                        prevcolor = color;
                        let cpair = nc::COLOR_PAIR(color);
                        nc::wattron(window, cpair);
                    }
                    nc::waddstr(window, &i.text);
                }
                nc::waddch(window, '\n' as nc::chtype);
            }
        }

        nc::wnoutrefresh(window);
    }

    fn draw_intro(&mut self) {
        self.windows.full.clear(&self.calloc);

        let window = self.windows.full.window;

        nc::wmove(window, 0, 0);

        nc::waddstr(window,
                    "This game is in very early development. We are looking for feedback and contributors!\n\n");
        nc::waddstr(window,
                    "See help: https://github.com/dpc/rhex/wiki/Help-How-to-Play\n");
        nc::waddstr(window,
                    "Join chat: https://gitter.im/dpc/rhex\n");
        nc::waddstr(window,
                    "Project page: https://github.com/dpc/rhex\n\n");
        nc::waddstr(window,
                    &format!("You can press {} in the game for offline help.\n\n",
                             KEY_HELP as u8 as char));
        nc::waddstr(window, "Press anything to start.");
        nc::wnoutrefresh(window);
    }

    fn draw_pickrace(&mut self) {
        self.windows.full.clear(&self.calloc);
        let window = self.windows.full.window;

        nc::wmove(window, 0, 0);

        nc::waddstr(window, "Pick your race\n\n");
        nc::waddstr(window, "a) Human\n");
        nc::waddstr(window, "b) Elf\n");
        nc::waddstr(window, "c) Dwarf\n");

        nc::wnoutrefresh(window);
    }

    fn draw_help(&mut self) {
        self.windows.full.clear(&self.calloc);
        let window = self.windows.full.window;
        nc::wmove(window, 0, 0);

        nc::waddstr(window, &format!("Current input mode: {} (F2 to change)\n\n", self.input_mode));
        if self.in_vi_input_mode() {
            nc::waddstr(window, "Move/attack: hjklui\n");
            nc::waddstr(window, "Strafe/attack: Shift + h/l\n");
            nc::waddstr(window, "Automove: shift + k\n");
        } else {
            nc::waddstr(window, "Move/attack: awsdqe\n");
            nc::waddstr(window, "Automove: shift + w\n");
        }
        nc::waddstr(window, "Charge: c\n");
        nc::waddstr(window, "Wait: .\n");
        nc::waddstr(window, "Descend: >\n");
        nc::waddstr(window, "Autoexplore: o\n");
        nc::waddstr(window, "Go to: G (only '>' follow-up implemented)\n");
        nc::waddstr(window, "Examine: x\n");
        nc::waddstr(window, "Pick item in front: ,\n");
        nc::waddstr(window, "Look at Inventory: I\n");
        nc::waddstr(window, "Equip/Use: E\n");
        nc::waddstr(window, "Drop: D\n");
        nc::waddstr(window, "Ranged/Throw: f (not fully working)\n");
        nc::waddstr(window, "Quit: ESC/q\n");
        nc::wnoutrefresh(window);
    }

    fn draw_quit(&mut self) {
        self.windows.full.clear(&self.calloc);
        let window = self.windows.full.window;
        let mut calloc = self.calloc.borrow_mut();

        let (max_y, max_x) = Ui::screen_size();
        let text = "Quit. Are you sure?";

        nc::wmove(window, max_y / 2, (max_x - text.chars().count() as i32) / 2);

        nc::waddstr(window, text);
        nc::wnoutrefresh(window);
    }

    fn event(&mut self, event: Event) {
        match event {
            Event::Log(logev) => {
                match logev {
                    LogEvent::AutoExploreDone => self.log("Nothing else to explore."),
                    LogEvent::AutoExploreBlocked => self.log("Can't get there."),
                }
            }
        }
    }
}


impl Drop for Ui {
    fn drop(&mut self) {
        nc::endwin();
    }
}


