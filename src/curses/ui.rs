use std::cell::RefCell;
use std::collections::{VecDeque, HashMap, HashSet};
use std::env;
use std;
use std::{thread, cmp, fmt};
use std::io::Write;
use std::fmt::Write as FmtWrite;

use chrono;
use num::integer::Integer;

use game::Action::*;

use ncurses as nc;
use hex2d::{Position, Coordinate, Angle, Left, Right, Forward, Back, ToCoordinate};

use hex2dext::algo::bfs;

use super::consts::*;
use super::color;
use super::{LogEntry, AutoMoveType, AutoMoveAction, LogEvent, Event, GoToType};
use super::Result;

use game::{actor, Location, Actor, item, area};
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

pub struct Window {
    pub window: nc::WINDOW,
}

impl Window {
    pub fn new(w: i32, h: i32, x: i32, y: i32) -> Window {
        Window { window: nc::subwin(nc::stdscr, h, w, y, x) }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        nc::delwin(self.window);
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
        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(nc::stdscr, &mut max_y, &mut max_x);

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
enum InvMode {
    View,
    Equip,
    Drop_,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum FSMode {
    Help,
    Intro,
    PickRace,
    Quit,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum TargetMode {
    Ranged,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Mode {
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
    dot: char,

    label_color: nc::attr_t,
    text_color: nc::attr_t,
    text_gray_color: nc::attr_t,
    red_color: nc::attr_t,
    green_color: nc::attr_t,

    input_mode : InputMode,

    engine: game::Engine,
    exit: bool,
    needs_redraw: bool,
    spawned: bool,

    automoving: Option<AutoMoveType>,
    automoving_stopped_turn: u64,

    after_action_delay: u32,
    game_action_queue: VecDeque<game::Action>,
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

        nc::doupdate();

        let mut ui = Ui {
            calloc: RefCell::new(calloc),
            windows: Windows::after_resize(),
            mode: Mode::FullScreen(FSMode::Intro),
            target_pos: None,
            dot: UNICODE_DOT,
            log: RefCell::new(VecDeque::new()),

            label_color: label_color,
            text_color: text_color,
            text_gray_color: text_gray_color,
            red_color: red_color,
            green_color: green_color,

            input_mode : InputMode::Vi,

            exit: false,
            needs_redraw: true,
            spawned: false,

            engine: engine,
            automoving: None,
            automoving_stopped_turn: 0,

            after_action_delay: 0,

            game_action_queue: VecDeque::new(),
        };
        ui.display_intro();
        Ok(ui)
    }

    pub fn initial_spawn(&mut self, race: actor::Race) {
        self.engine.initial_spawn(race);
        let player_id = self.engine.current_location().player_id();
        self.engine_change(player_id);
        self.spawned = true;
    }

    pub fn screen_size(&self) -> (i32, i32) {
        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(nc::stdscr, &mut max_y, &mut max_x);

        (max_y, max_y)
    }

    pub fn resize(&mut self) {
        self.windows = Windows::after_resize();
        self.redraw();
    }

    /// Mark the screen for redraw
    pub fn redraw(&mut self) {
        self.needs_redraw = true;
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

        let (max_x, max_y) = self.screen_size();

        nc::mv(max_y - 1, max_x - 1);
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

    pub fn goto_action(&self, gototype: GoToType) -> AutoMoveAction {
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
        self.update();

        if self.automoving.is_some() {
            if self.automoving_stopped_turn != self.engine.turn() && self.should_stop_automoving() {
                self.automoving_stop();
            }
        }

        self.after_action_delay += {
            let cur_loc = self.current_location();
            let player_id = cur_loc.player_id();

            if actor_id == player_id {
                if self.is_automoving() {
                    20
                } else {
                    0
                }
            } else {
                0
            }
        };

        self.redraw();
    }

    pub fn player(&self) -> &Actor {
        let player_id = self.engine.current_location().player_id();
        &self.engine.current_location().actors_byid[&player_id]
    }
    pub fn current_location(&self) -> &Location {
        self.engine.current_location()
    }

    pub fn run_once(&mut self) {
        if self.after_action_delay > 0 {
            self.after_action_delay -= 1;
        } else if self.spawned {
            let player_id = self.current_location().player_id();

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
                            self.redraw();
                        }
                        AutoMoveAction::Action(action) => {
                            self.engine.player_act(action);
                            self.engine_change(player_id);
                        }
                        AutoMoveAction::Finish => {
                            match movetype {
                                AutoMoveType::Explore => {
                                    self.event(Event::Log(LogEvent::AutoExploreDone));
                                }
                                _ => {}
                            }
                            self.automoving_stop();
                            self.redraw();
                        }
                    }
                } else if let Some(action) = self.game_action_queue.pop_front() {
                    self.engine.player_act(action);
                    self.engine_change(player_id);
                }
            } else {
                let actor_id = self.engine.one_actor_tick();
                self.engine_change(actor_id);
            }
        }
        {
            self.input_handle();
            if self.needs_redraw {
                self.needs_redraw = false;
                self.redraw_now();
            }
        }
    }

    pub fn run(&mut self) {
        while !self.exit {
            let start = chrono::Local::now();
            self.run_once();
            let msdelta = chrono::Local::now() - start;
            if msdelta < chrono::Duration::milliseconds(1) {
                let nanosecs = (chrono::Duration::milliseconds(1) - msdelta)
                                   .num_nanoseconds()
                                   .unwrap();
                assert!(nanosecs > 0);
                thread::sleep(std::time::Duration::new(0, nanosecs as u32));
            }
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

    pub fn queue_turn(&mut self, angle: Angle) {
        self.action_push(game::Action::Turn(angle))
    }

    pub fn queue_spin(&mut self, angle: Angle) {
        self.action_push(game::Action::Spin(angle))
    }

    pub fn queue_move(&mut self, angle: Angle) {
        self.action_push(game::Action::Move(angle))
    }

    pub fn queue_charge(&mut self) {
        self.action_push(game::Action::Charge)
    }

    pub fn queue_pick(&mut self) {
        self.action_push(game::Action::Pick)
    }

    pub fn queue_ranged(&mut self, coord: Coordinate) {
        self.action_push(game::Action::Ranged(coord))
    }

    pub fn queue_wait(&mut self) {
        self.action_push(game::Action::Wait)
    }

    pub fn queue_descend(&mut self) {
        self.action_push(game::Action::Descend)
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

    pub fn key_to_action_normal(&self, key : i32) -> Option<Action> {
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
            (KEY_LOWC, true) => Game(Charge),
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

    pub fn key_to_action_global(&self, key : i32) -> Option<GlobalAction> {
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
                        self.mode = Mode::Normal;
                    }
                    KEY_LOWH => {
                        self.target_pos = Some(pos + Angle::Left);
                    }
                    KEY_LOWL => {
                        self.target_pos = Some(pos + Angle::Right);
                    }
                    KEY_LOWJ => {
                        self.target_pos = Some(pos + (pos.dir + Angle::Back).to_coordinate());
                    }
                    KEY_LOWK => {
                        self.target_pos = Some(pos + pos.dir.to_coordinate());
                    }
                    KEY_CAPK => {
                        self.target_pos = Some(pos + pos.dir.to_coordinate().scale(5));
                    }
                    KEY_CAPJ => {
                        self.target_pos = Some(pos +
                                               (pos.dir + Angle::Back).to_coordinate().scale(5));
                    }
                    _ => {}
                }
                self.redraw();
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
                    }
                    KEY_LOWL => {
                        self.target_pos = Some(util::circular_move(center, pos, Angle::Right));
                    }
                    KEY_LOWJ => {
                        self.target_pos = Some(util::circular_move(center, pos, Angle::Back));
                    }
                    KEY_LOWK => {
                        self.target_pos = Some(util::circular_move(center, pos, Angle::Forward));
                    }
                    _ => {}
                }
                self.redraw();
            }
            Mode::GoTo => {
                match key {
                    KEY_DESCEND => self.automoving = Some(AutoMoveType::GoTo(GoToType::Stairs)),
                    _ => {}
                }
                self.mode_switch_to(Mode::Normal);
            }
        }
    }
    pub fn input_handle_key(&mut self, key : i32) {
        if let Some(global_action) = self.key_to_action_global(key) {
            self.input_mode.toggle();
            self.redraw();
        } else {
            self.input_handle_key_mode_specific(key)
        }
    }

    // TODO: break into smaller stuff?
    fn update(&mut self) {

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
                                    .item_map_or("".to_string(), |i| i.description().to_string());
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
            text: s.to_string(),
            turn: turn,
        });
    }

    pub fn display_intro(&mut self) {
        self.mode = Mode::FullScreen(FSMode::Intro);
    }

    fn draw_map(&self) {
        let player = self.player();
        let cur_loc = self.current_location();

        let mut calloc = self.calloc.borrow_mut();

        let window = self.windows.map.window;

        let actors_aheads: HashMap<Coordinate, Coordinate> =
            cur_loc.actors_byid
                   .iter()
                   .filter(|&(_, a)| !a.is_dead())
                   .map(|(_, a)| (a.head(), a.pos.coord))
                   .collect();
        let player_ahead = player.pos.coord + player.pos.dir;

        // Get the screen bounds.
        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(window, &mut max_y, &mut max_x);

        let mid_x = max_x / 2;
        let mid_y = max_y / 2;

        let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::MAP_BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);

        let (center, head) = match self.mode {
            Mode::Examine => {
                match self.target_pos {
                    None => (player.pos.coord, player.pos.coord + player.pos.dir),
                    Some(pos) => (pos.coord, pos.coord + pos.dir),
                }
            }
            Mode::Target(_) => {
                match self.target_pos {
                    None => (player.pos.coord, player.pos.coord + player.pos.dir),
                    Some(pos) => (player.pos.coord, pos.coord),
                }
            }
            _ => (player.pos.coord, player.pos.coord + player.pos.dir),
        };

        let mut target_line = HashSet::new();
        if let Mode::Target(_) = self.mode {
            center.for_each_in_line_to(head, |c| {
                target_line.insert(c);
            });
        }

        let (vpx, vpy) = center.to_pixel_integer(SPACING);

        for vy in 0..max_y {
            for vx in 0..max_x {
                let (rvx, rvy) = (vx - mid_x, vy - mid_y);

                let (cvx, cvy) = (rvx + vpx, rvy + vpy);

                let (c, off) = Coordinate::from_pixel_integer(SPACING, (cvx, cvy));

                let is_proper_coord = off == (0, 0);

                let (visible, _in_los, knows, tt, t, light) = if is_proper_coord {

                    let t = cur_loc.map[c].clone();
                    let tt = t.type_;

                    let visible = player.sees(c) || player.is_dead();
                    let light = if visible {
                        cur_loc.at(c).light_as_seen_by(player)
                    } else {
                        0
                    };

                    (visible,
                     player.in_los(c) || player.is_dead(),
                     player.knows(c),
                     Some(tt),
                     Some(t),
                     light)
                } else {
                    // Paint a glue characters between two real characters
                    let c1 = c;
                    let (c2, _) = Coordinate::from_pixel_integer(SPACING, (cvx + 1, cvy));

                    let low_opaq1 = player.sees(c1) && cur_loc.at(c1).tile().opaqueness() <= 1;
                    let low_opaq2 = player.sees(c2) && cur_loc.at(c2).tile().opaqueness() <= 1;

                    let knows = (player.knows(c1) && player.knows(c2)) ||
                                (player.knows(c1) && low_opaq1) ||
                                (player.knows(c2) && low_opaq2);

                    let (e1, e2) = (cur_loc.at(c1).tile().ascii_expand(),
                                    cur_loc.at(c2).tile().ascii_expand());

                    let c = Some(if e1 > e2 {
                        c1
                    } else {
                        c2
                    });

                    let tt = c.map_or(None, |c| Some(cur_loc.at(c).tile().type_));

                    let visible = player.is_dead() || (player.sees(c1) && player.sees(c2)) ||
                                  (player.sees(c1) && low_opaq1) ||
                                  (player.sees(c2) && low_opaq2);

                    let in_los = player.is_dead() || (player.in_los(c1) && player.in_los(c2)) ||
                                 (player.in_los(c1) && low_opaq1) ||
                                 (player.in_los(c2) && low_opaq2);

                    let light = if visible {
                        let (light1, light2) = (cur_loc.at(c1).light_as_seen_by(player),
                                                cur_loc.at(c2).light_as_seen_by(player));


                        if player.is_dead() {
                            (light1 + light2) / 2
                        } else {
                            match (player.sees(c1), player.sees(c2)) {
                                (true, true) => (light1 + light2) / 2,
                                (true, false) => light1,
                                (false, true) => light2,
                                (false, false) => 0,
                            }
                        }
                    } else {
                        0
                    };

                    (visible, in_los, knows, tt, None, light)
                };

                let mut draw = knows;

                debug_assert!(!visible || knows || player.is_dead());

                let mut bold = false;
                let occupied = cur_loc.at(c).is_occupied();
                let (fg, bg, mut glyph) = if is_proper_coord && visible && occupied {
                    let (fg, glyph) = match cur_loc.at(c).actor_map_or(Race::Rat, |a| a.race) {
                        Race::Human | Race::Elf | Race::Dwarf => (color::CHAR_SELF_FG, '@'),
                        Race::Rat => (color::CHAR_ENEMY_FG, 'r'),
                        Race::Goblin => (color::CHAR_ENEMY_FG, 'g'),
                        Race::Troll => (color::CHAR_ENEMY_FG, 'T'),
                    };
                    (fg, color::CHAR_BG, glyph)
                } else if is_proper_coord && visible &&
                                             cur_loc.at(c).item().is_some() {
                    let item = cur_loc.at(c).item().unwrap();
                    let s = item_to_char(item.category());
                    if player.discovered.contains(&c) {
                        bold = true;
                    }
                    (color::WALL_FG, color::EMPTY_BG, s)
                } else if knows {
                    match tt {
                        Some(tile::Empty) => {
                            let mut fg = color::STONE_FG;
                            let mut bg = color::EMPTY_BG;
                            let mut glyph = ' ';

                            if is_proper_coord {
                                match t.and_then(|t| t.feature) {
                                    None => {
                                        glyph = self.dot;
                                        fg = color::EMPTY_FG;
                                        bg = color::EMPTY_BG;
                                    }
                                    Some(tile::Door(open)) => {
                                        if open {
                                            glyph = DOOR_OPEN_CH;
                                        } else {
                                            glyph = DOOR_CLOSED_CH;
                                            bg = color::WALL_BG;
                                        }
                                    }
                                    Some(tile::Statue) => glyph = STATUE_CH,
                                    Some(tile::Stairs) => glyph = STAIRS_DOWN_CH,
                                }
                            }

                            (fg, bg, glyph)
                        }
                        Some(tile::Wall) => {
                            bold = true;
                            (color::WALL_FG, color::WALL_BG, WALL_CH)
                        }
                        Some(tile::Water) => (color::WATER_FG, color::WATER_BG, WATER_CH),
                        None => (color::EMPTY_FG, color::EMPTY_BG, '?'),
                    }
                } else {
                    (color::EMPTY_FG, color::EMPTY_BG, NOTHING_CH)
                };


                let (mut fg, mut bg) = if !visible || light == 0 {
                    if visible {
                        (fg[2], bg[2])
                    } else {
                        (fg[3], bg[3])
                    }
                } else if light < 3 {
                    (fg[1], bg[1])
                } else {
                    (fg[0], bg[0])
                };

                if let Some(t) = t {
                    if visible && t.light > 0 {
                        if !occupied {
                            fg = color::LIGHTSOURCE;
                            bold = true;
                        }
                    }
                }

                if is_proper_coord && visible &&
                   cur_loc.at(c).actor_map_or(0, |a| a.light_emision()) > 0u32 {
                    bg = color::LIGHTSOURCE;
                }

                if is_proper_coord && actors_aheads.contains_key(&c) &&
                   player.sees(*actors_aheads.get(&c).unwrap()) {
                    bold = true;
                    let color = if c == player_ahead {
                        color::TARGET_SELF_FG
                    } else {
                        color::TARGET_ENEMY_FG
                    };

                    if player.knows(c) {
                        if occupied {
                            bg = color;
                        } else {
                            fg = color;
                        }
                    } else {
                        draw = true;
                        glyph = ' ';
                        bg = color;
                    }
                }

                if is_proper_coord && c != center && !visible && player.hears(c) {
                    bg = color::NOISE_BG;
                    draw = true;
                }

                if self.mode == Mode::Examine {
                    if is_proper_coord && center == c {
                        glyph = '@';
                        fg = color::CHAR_GRAY_FG;
                        draw = true;
                    } else if is_proper_coord && c == head {
                        bold = true;
                        if player.knows(c) {
                            fg = color::TARGET_SELF_FG;
                        } else {
                            draw = true;
                            glyph = ' ';
                            bg = color::TARGET_SELF_FG;
                        }
                    }
                } else if let Mode::Target(_) = self.mode {
                    if is_proper_coord && target_line.contains(&c) {
                        glyph = '*';
                        draw = true;
                        if c == head {
                            fg = color::TARGET_SELF_FG;
                        }
                        if !cur_loc.at(c).tile().is_passable() {
                            bg = color::BLOCKED_BG;
                        }
                    }
                }


                if draw {
                    let cpair = nc::COLOR_PAIR(calloc.get(fg, bg));
                    if glyph <= (127u8 as char) {
                        let ch = (glyph as nc::chtype) | cpair;
                        nc::mvwaddch(window, vy, vx, if bold { ch|nc::A_BOLD() } else { ch });
                    } else {
                        let attrflag = if bold { cpair|nc::A_BOLD() } else { cpair };
                        nc::wattron(window, attrflag as i32);
                        nc::mvwaddstr(window, vy, vx, &format!("{}", glyph));
                        nc::wattroff(window, attrflag as i32);
                    }
                }

            }
        }

        nc::wnoutrefresh(window);
    }

    fn draw_stats_bar(&self, window: nc::WINDOW, y: i32, name: &str, cur: i32, prev: i32, max: i32) {

        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(window, &mut max_y, &mut max_x);

        if max_x < 6 || max_y < 1 {
            // Don't draw anything on too small window
            return;
        }

        let cur = cmp::max(cur, 0) as u32;
        let prev = cmp::max(prev, 0) as u32;
        let max = cmp::max(max, 1) as u32;

        nc::wmove(window, y, 0);
        nc::wattron(window, self.label_color as i32);
        nc::waddstr(window, &format!("{}: ", name));

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
        nc::wattron(window, self.label_color as i32);
        nc::waddstr(window, &format!("{}: ", label));

        nc::wattron(window, self.text_color as i32);
        nc::waddstr(window, &format!("{:<8}", val));
    }

    fn draw_val<T>(&self, window: nc::WINDOW, label: &str, val: T)
        where T: Integer + fmt::Display
    {
        nc::wattron(window, self.label_color as i32);
        nc::waddstr(window, &format!("{}:", label));

        nc::wattron(window, self.text_color as i32);
        nc::waddstr(window, &format!("{:>2} ", val));
    }

    fn draw_label(&self, window: nc::WINDOW, label: &str) {
        nc::wattron(window, self.label_color as i32);
        nc::waddstr(window, &format!("{}:", label));
    }

    fn draw_item(&self, window: nc::WINDOW, astate: &Actor, label: &str, slot: Slot) {
        self.draw_label(window, label);

        if slot == Slot::RHand && !astate.can_attack() {
            nc::wattron(window, self.text_gray_color as i32);
        } else {
            nc::wattron(window, self.text_color as i32);
        }

        let item = if let Some(&(_, ref item)) = astate.items_equipped.get(&slot) {
            nc::waddstr(window, &format!("{:^13}", item.description()));
        } else {
            nc::waddch(window, '-' as nc::chtype);
        };

        // let item = item.slice_chars(0, cmp::min(item.char_len(), 13));
    }

    fn draw_inventory(&self) {
        let window = self.windows.map.window;
        let player = self.player();

        let cpair = self.text_color;
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);

        nc::werase(window);
        nc::wmove(window, 0, 0);
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
        let turn = self.engine.turn();
        let window = self.windows.stats.window;
        let player = self.player();
        let cur_loc = self.current_location();

        let (ac, ev) = (player.stats.base.ac, player.stats.base.ev);
        let (dmg, acc) = (player.stats.melee_dmg, player.stats.melee_acc);

        let cpair = self.text_color;
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);

        nc::werase(window);
        nc::wmove(window, 0, 0);

        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(window, &mut max_y, &mut max_x);

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
        nc::wattron(window, self.text_color as i32);
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
            return "Unknown".to_string();
        }

        let tile_type = gstate.at(coord).tile().type_;
        let tile = gstate.at(coord).tile();
        let feature_descr = tile.feature.map(|f| f.description().to_string());
        let item_descr = gstate.at(coord).item_map_or(None, |i| Some(i.description().to_string()));

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
                     .to_string())
            })
        } else {
            None
        };

        match (tile_type, feature_descr, actor_descr, item_descr) {

            (_, _, Some(a_descr), _) => a_descr,
            (_, _, _, Some(i_descr)) => i_descr,
            (_, Some(f_descr), _, _) => f_descr.to_string(),
            (tile::Wall, _, _, _) => "a wall".to_string(),
            (tile::Empty, _, _, _) => {
                match tile.area.and_then(|a| Some(a.type_)) {
                    Some(area::Room(_)) => "room".to_string(),
                    None => "nothing".to_string(),
                }
            }
            _ => tile.type_.description().to_string(),
        }
    }

    fn draw_log(&self) {
        let window = self.windows.log.window;

        let cpair = nc::COLOR_PAIR(self.calloc
                                       .borrow_mut()
                                       .get(color::VISIBLE_FG, color::BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
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

        for i in self.log.borrow().iter() {
            if nc::getcury(window) == nc::getmaxy(window) - 1 {
                break;
            }
            if let Some(color) = self.turn_to_color(i.turn, &self.calloc) {
                let cpair = nc::COLOR_PAIR(color);
                nc::wattron(window, cpair as i32);
                nc::waddstr(window, &i.text);
            }
            nc::waddch(window, '\n' as nc::chtype);
        }

        nc::wnoutrefresh(window);
    }

    fn draw_intro(&mut self) {
        let window = self.windows.full.window;
        let mut calloc = self.calloc.borrow_mut();
        let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
        nc::wmove(window, 0, 0);

        nc::waddstr(window,
                    "A long time ago in a dungeon deep, deep underground...\n\n");
        nc::waddstr(window,
                    &format!("You can press {} in the game for help.\n\n",
                             KEY_HELP as u8 as char));
        nc::waddstr(window, "Press anything to start.");
        nc::wnoutrefresh(window);
    }

    fn draw_pickrace(&mut self) {
        let window = self.windows.full.window;
        let mut calloc = self.calloc.borrow_mut();
        let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
        nc::wmove(window, 0, 0);

        nc::waddstr(window, "Pick your race\n\n");
        nc::waddstr(window, "a) Human\n");
        nc::waddstr(window, "b) Elf\n");
        nc::waddstr(window, "c) Dwarf\n");

        nc::wnoutrefresh(window);
    }

    fn draw_help(&mut self) {
        let window = self.windows.full.window;
        let mut calloc = self.calloc.borrow_mut();
        let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
        nc::wmove(window, 0, 0);

        nc::waddstr(window, "This game is still incomplete. Sorry for that.\n\n");
        nc::waddstr(window, &format!("Current input mode: {} (F2 to change)\n\n", self.input_mode));
        if self.in_vi_input_mode() {
            nc::waddstr(window, "Move/attack: hjklui\n");
            nc::waddstr(window, "Strafe/attack: Shift + h/l\n");
        } else {
            nc::waddstr(window, "Move/attack: awsdqe\n");
        }
        nc::waddstr(window, "Charge: c\n");
        nc::waddstr(window, "Wait: .\n");
        nc::waddstr(window, "Descend: >\n");
        nc::waddstr(window, "Autoexplore: o\n");
        nc::waddstr(window, "Automove: shift + k\n");
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
        let window = self.windows.full.window;
        let mut calloc = self.calloc.borrow_mut();
        let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG));

        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(nc::stdscr, &mut max_y, &mut max_x);
        let text = "Quit. Are you sure?";

        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
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

//        . . .
//       . . . .
//      . . . . .
//       . . . .
//        . . .
pub fn item_to_char(t: item::Category) -> char {
    match t {
        item::Category::Weapon => ')',
        item::Category::RangedWeapon => '}',
        item::Category::Armor => '[',
        item::Category::Misc => '"',
        item::Category::Consumable => '%',
    }
}
