use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::ring_buf::RingBuf;
use std;
use std::ffi::AsOsStr;

use ncurses as nc;

use super::Action;
use game;
use game::area;
use actor::{self, Behavior};
use ui;

use hex2d::{Angle, IntegerSpacing, Coordinate, ToCoordinate, Position};

use game::tile;

use std::fmt;
use std::fmt::Writer;

mod locale {
    use libc::{c_int, c_char};
    pub const LC_ALL: c_int = 6;
    extern "C" {
        pub fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    }
}

//        . . .
//       . . . .
//      . . . . .
//       . . . .
//        . . .
static SPACING: IntegerSpacing<i32> = IntegerSpacing::PointyTop(2, 1);

const NORMAL_DOT : &'static str = ".";
const UNICODE_DOT : &'static str = "·";

pub mod color {
    use std::collections::HashMap;
    use std::collections::hash_map::Entry;
    use ncurses as nc;

    pub const GRAY : [u8; 26] = [
        16, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243,
        244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 15
    ];
    pub const BLACK : u8 = GRAY[0];
    pub const WHITE : u8 = GRAY[25];

    pub const BACKGROUND_BG : u8 = GRAY[2];
    pub const MAP_BACKGROUND_BG : u8 = GRAY[2];

    pub const VISIBLE_FG : u8 = WHITE;

    // in light, shaded (barely visible), out of sight
    pub const EMPTY_FG : [u8; 3] = [GRAY[17], GRAY[12] , GRAY[5]];
    pub const EMPTY_BG : [u8; 3] = [GRAY[23], GRAY[20] , GRAY[6]];
    pub const WALL_FG : [u8; 3] = [BLACK, GRAY[1] , GRAY[2]];
    pub const WALL_BG : [u8; 3] = [GRAY[14], GRAY[8] , GRAY[4]];
    pub const CHAR_FG : [u8; 3] = [GRAY[1], GRAY[8] , GRAY[4]];
    pub const CHAR_BG : [u8; 3] = EMPTY_BG;
    pub const TREE_FG : [u8; 3] = CHAR_FG;
    pub const TREE_BG : [u8; 3] = EMPTY_BG;

    pub const TARGET_FG : u8 = 196;
    pub const TARGET_SELF_FG : u8 = 20;
    pub const TARGET_ENEMY_FG : u8 = 196;
    pub const LIGHTSOURCE : u8 = 227;
    pub const LOG_1_FG : u8 = GRAY[25];
    pub const LOG_2_FG : u8 = GRAY[21];
    pub const LOG_3_FG : u8 = GRAY[17];
    pub const LOG_4_FG : u8 = GRAY[13];
    pub const LOG_5_FG : u8 = GRAY[9];

    pub struct Allocator {
        map : HashMap<(u8, u8), i16>,
        cur : i16,
    }

    impl Allocator {
        pub fn new() -> Allocator {
            Allocator {
                cur: 1i16, /* 0 is reserved for defaults */
                map: HashMap::new(),
            }
        }

        pub fn get(&mut self, fg : u8, bg : u8) -> i16 {
            match self.map.entry((fg, bg)) {
                Entry::Occupied(i) => *i.get(),
                Entry::Vacant(i) => {
                    assert!((self.cur as i32) < nc::COLOR_PAIRS, "curses run out of color pairs!");
                    let ret = self.cur;
                    i.insert(self.cur);
                    nc::init_pair(ret, fg as i16, bg as i16);
                    self.cur += 1;
                    ret
                }
            }
        }
    }
}

pub struct Window {
    pub window : nc::WINDOW,
}


pub struct LogEntry {
    turn : u64,
    text : String,
}


impl Window {
    pub fn new(w : i32, h : i32, x : i32, y : i32) -> Window {
        Window {
            window : nc::subwin(nc::stdscr, h, w, y, x),
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        nc::delwin(self.window);
    }
}

pub struct CursesUI {
    calloc : RefCell<color::Allocator>,
    map_window : Window,
    log_window : Window,
    stats_window : Window,
    fs_window : Window, /* full screen */
    mode : Mode,
    log : RingBuf<LogEntry>,
    examine_pos : Option<Position>,
    dot : &'static str,
}

impl CursesUI {

    pub fn new() -> CursesUI {

        let term_ok = std::env::var_os("TERM").as_ref()
            .and_then(|s| s.as_os_str().to_str())
            .map_or(false, |s| s.ends_with("-256color"));

        let term_putty = std::env::var_os("TERM").as_ref()
            .and_then(|s| s.as_os_str().to_str())
            .map_or(false, |s| s.starts_with("putty"));

        if !term_ok {
            panic!("Your TERM environment variable must end with -256color, sorry, stranger from the past. It is curable. Google it, fix it, try again.");
        }

        unsafe {
            let _ = locale::setlocale(locale::LC_ALL, b"en_US.UTF-8".as_ptr() as *const i8);
        }

        nc::initscr();
        nc::start_color();
        nc::keypad(nc::stdscr, true);
        nc::noecho();
        nc::raw();
        nc::timeout(0);
        nc::flushinp();
        nc::curs_set(nc::CURSOR_VISIBILITY::CURSOR_INVISIBLE);

        assert!(nc::has_colors());

        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(nc::stdscr, &mut max_y, &mut max_x);

        let mid_x = max_x * 3 / 5;
        let mid_y = max_y * 4 / 5;

        let map_window = Window::new(
                mid_x, max_y, 0, 0
                );
        let stats_window = Window::new(
                max_x - mid_x, mid_y, mid_x, 0
                );
        let log_window = Window::new(
                max_x - mid_x, max_y - mid_y, mid_x, mid_y
                );
        let fs_window = Window::new(
                max_x, max_y, 0, 0
                );

        CursesUI {
            calloc: RefCell::new(color::Allocator::new()),
            map_window: map_window,
            stats_window: stats_window,
            log_window: log_window,
            fs_window: fs_window,
            mode : Mode::Normal,
            examine_pos : None,
            dot: if term_putty { NORMAL_DOT } else { UNICODE_DOT },
            log : RingBuf::new(),
        }
    }

    pub fn log(&mut self, s : String, gstate : &game::State) {
        self.log.push_front(LogEntry{text: s, turn: gstate.turn});
    }

    pub fn display_intro(&mut self) {
        self.mode = Mode::Intro;
    }

    fn draw_map(
        &mut self,
        astate : &actor::State, gstate : &game::State,
        )
    {
        let mut calloc = self.calloc.borrow_mut();

        let window = self.map_window.window;

        let actors_aheads : HashMap<Coordinate, Coordinate> =
            gstate.actors.iter().map(|(_, a)| (a.pos.coord + a.pos.dir, a.pos.coord)).collect();
        let astate_ahead = astate.pos.coord + astate.pos.dir;

        /* Get the screen bounds. */
        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(window, &mut max_y, &mut max_x);

        let mid_x = max_x / 2;
        let mid_y = max_y / 2;

        let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::MAP_BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);

        let center = match self.mode {
            Mode::Examine => {
                match self.examine_pos {
                    None => {
                        self.examine_pos = Some(astate.pos);
                        astate.pos.coord
                    },
                    Some(pos) => {
                        pos.coord
                    },
                }
            },
            _ => {
                astate.pos.coord
            }
        };

        let (vpx, vpy) = center.to_pixel_integer(SPACING);

        for vy in range(0, max_y) {
            for vx in range(0, max_x) {
                let (rvx, rvy) = (vx - mid_x, vy - mid_y);

                let (cvx, cvy) = (rvx + vpx, rvy + vpy);

                let (c, off) = Coordinate::from_pixel_integer(SPACING, (cvx, cvy));

                let is_proper_coord = off == (0, 0);

                let (visible, mut draw, tt, t, light) = if is_proper_coord {

                    let t = gstate.map.get(&c);

                    let tt = match t {
                        Some(t) => t.type_,
                        None => tile::Wall,
                    };

                    (astate.sees(c), astate.knows(c), Some(tt), t, gstate.light(c))
                } else {
                    // Paint a glue characters between two real characters
                    let c1 = c;
                    let (c2, _) = Coordinate::from_pixel_integer(SPACING, (cvx + 1, cvy));

                    let knows = astate.knows(c1) && astate.knows(c2);

                    let (e1, e2) = (
                        gstate.tile_map_or(c1, tile::Wall, |t| t.type_).ascii_expand(),
                        gstate.tile_map_or(c2, tile::Wall, |t| t.type_).ascii_expand()
                        );

                    let c = Some(if e1 > e2 { c1 } else { c2 });

                    let tt = c.map_or(None, |c| gstate.tile_map_or(c, Some(tile::Wall), |t| Some(t.type_)));

                    let visible = astate.sees(c1) && astate.sees(c2);

                    (visible, knows, tt, None, (gstate.light(c1) + gstate.light(c2)) / 2)
                };

                let (fg, bg, mut glyph) =
                    if is_proper_coord && astate.sees(c) && gstate.actors.contains_key(&c) {
                        (color::CHAR_FG, color::CHAR_BG, "@")
                    } else {
                        match tt {
                            Some(tile::Empty) => {
                                (
                                    color::EMPTY_FG, color::EMPTY_BG,
                                    if is_proper_coord { self.dot } else { " " }
                                 )
                            },
                            Some(tile::Wall) => {
                                (color::WALL_FG, color::WALL_BG, "#")
                            },
                            Some(tile::Door(open)) => {
                                (color::WALL_FG, color::WALL_BG,
                                 if open { "_" } else { "+" })
                            },
                            Some(tile::Tree) => {
                                (color::TREE_FG, color::TREE_BG, "T")
                            },
                            None => {
                                (color::EMPTY_FG, color::EMPTY_BG, " ")
                            },
                        }
                    };


                let (mut fg, mut bg) = if !visible {
                    (fg[2], bg[2])
                } else if light < 3 {
                    (fg[1], bg[1])
                } else {
                    (fg[0], bg[0])
                };

                if let Some(t) = t {
                    if visible && t.light > 0 {
                        fg = color::LIGHTSOURCE;
                    }
                }

                if is_proper_coord && visible && gstate.actor_map_or(c, 0, &|a| a.light) > 0u32 {
                    bg = color::LIGHTSOURCE;
                }

                if self.mode == Mode::Examine {
                    if is_proper_coord && self.examine_pos.unwrap().coord == c {
                            if astate.knows(c) {
                                fg = color::TARGET_SELF_FG;
                            } else {
                                draw = true;
                                glyph = " ";
                                bg = color::TARGET_SELF_FG;
                            }
                    }
                } else {
                    if is_proper_coord && actors_aheads.contains_key(&c) {
                        if astate.sees(*actors_aheads.get(&c).unwrap()) {
                            let color = if c == astate_ahead {
                                color::TARGET_SELF_FG
                            } else {
                                color::TARGET_ENEMY_FG
                            };

                            if astate.knows(c) {
                                fg = color;
                            } else {
                                draw = true;
                                glyph = " ";
                                bg = color;
                            }
                        }
                    }
                }

                if draw {
                    let cpair = nc::COLOR_PAIR(calloc.get(fg, bg));

                    if visible {
                        nc::wattron(window, nc::A_BOLD() as i32);
                    }

                    nc::wattron(window, cpair as i32);
                    nc::mvwaddstr(window, vy, vx, glyph);
                    nc::wattroff(window, cpair as i32);

                    if visible {
                        nc::wattroff(window, nc::A_BOLD() as i32);
                    }
                }

            }
        }

        nc::wnoutrefresh(window);
    }

    fn draw_stats(&mut self, astate : &actor::State, gstate : &game::State) {
        let window = self.stats_window.window;

        let mut calloc = self.calloc.borrow_mut();
        let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
        nc::wmove(window, 0, 0);

        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(window, &mut max_y, &mut max_x);

        let mut turn_str = String::new();
        write!(&mut turn_str, "Turn: {}", gstate.turn).unwrap();

        nc::mvwaddstr(window, max_y - 1, 0 , turn_str.as_slice());
        nc::mvwaddstr(window, 2, 0,
                      &format!("STR: {:2>} HP: {:>4}/{}", astate.stats.str_,
                               astate.stats.hp, astate.stats.max_hp));
        nc::mvwaddstr(window, 3, 0,
                      &format!("DEX: {} MP: {:>4}/{}", astate.stats.dex,
                               astate.stats.mp, astate.stats.max_mp));
        nc::mvwaddstr(window, 4, 0,
                      &format!("INT: {}", astate.stats.int));

        nc::wnoutrefresh(window);
    }
    // TODO: Consider the distance to the Item to print something
    // like "you see x in the distance", "you find yourself in x".
    fn format_areas<I>(&self, mut i : I) -> Option<String>
        where I : Iterator, <I as Iterator>::Item : fmt::Display
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

    fn turn_to_color(
        &self, turn : u64, calloc : &RefCell<color::Allocator>,
        gstate : &game::State) -> Option<i16>
    {
        let mut calloc = calloc.borrow_mut();

        let dturn = gstate.turn - turn;

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

    fn tile_description(&self, coord : Coordinate,
                        astate : &actor::State, gstate : &game::State
                        ) -> String
    {
        if !astate.knows(coord) {
            return "Unknown".to_string();
        }

        let tile_type = gstate.tile_map_or(coord, tile::Wall, |t| t.type_);
        let tile = gstate.tile_map_or(coord, None, |t| Some(t.clone()));
        let actor =
            if astate.sees(coord) {
                gstate.actor_map_or(coord, None, &|a| Some(match a.behavior {
                    Behavior::Pony => "A pony",
                    Behavior::Grue => "Toothless Grue",
                    Behavior::Player => "Yourself",
                }.to_string())
                )
            } else {
                None
            };

        match (tile_type, actor) {
            (tile::Wall, _) => {
                "a wall".to_string()
            },
            (tile::Door(_), _) => {
                "door".to_string()
            },
            (tile::Empty, None) => {
                match tile.and_then(|t| t.area).and_then(|a| Some(a.type_)) {
                    Some(area::Room(_)) => "room".to_string(),
                    None => "nothing".to_string()
                }
            },
            (tile::Empty, Some(descr)) => {
                descr
            },
            _ => {
                "Indescribable".to_string()
            },
        }
    }

    fn draw_examine(&self, astate : &actor::State, gstate : &game::State) {
        let window = self.log_window.window;

        let pos = self.examine_pos.expect("examine_pos should have not been None");

        let cpair = nc::COLOR_PAIR(self.calloc.borrow_mut().get(color::VISIBLE_FG, color::BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
        nc::wmove(window, 0, 0);

        let descr = self.tile_description(pos.coord, astate, gstate);
        nc::waddstr(window, descr.as_slice());

        nc::wnoutrefresh(window);
    }

    fn draw_log(&mut self, _ : &actor::State, gstate : &game::State) {
        let window = self.log_window.window;

        let cpair = nc::COLOR_PAIR(self.calloc.borrow_mut().get(color::VISIBLE_FG, color::BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
        nc::wmove(window, 0, 0);

        for i in &self.log {

            if nc::getcury(window) == nc::getmaxy(window) - 1 {
                break;
            }

            if let Some(color) = self.turn_to_color(i.turn, &self.calloc, gstate) {
                let cpair = nc::COLOR_PAIR(color);
                nc::wattron(window, cpair as i32);
                nc::waddstr(window, i.text.as_slice());
                nc::waddstr(window, "\n");
            }
        }

        nc::wnoutrefresh(window);
    }

    fn draw_intro(&mut self)
    {
        let window = self.fs_window.window;
        let mut calloc = self.calloc.borrow_mut();
        let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
        nc::wmove(window, 0, 0);

        nc::waddstr(window, "Long, long ago in a galaxy far, far away...\n\n");
        nc::waddstr(window, "You can press '?' in the game for help.\n\n");
        nc::waddstr(window, "Press anything to start.");
        nc::wnoutrefresh(window);
    }

    fn draw_help( &mut self) {
        let window = self.fs_window.window;
        let mut calloc = self.calloc.borrow_mut();
        let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
        nc::wmove(window, 0, 0);

        nc::waddstr(window, "This game have no point (yet) and is incomplete. Sorry for that.\n\n");
        nc::waddstr(window, "Just press one of: hjklui, or o. Especially o because it's cool.\n\n");
        nc::wnoutrefresh(window);
        nc::wnoutrefresh(window);
    }

}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Mode {
    Normal,
    Examine,
    Help,
    Intro,
}

impl ui::UiFrontend for CursesUI {

    fn update(&mut self, astate : &actor::State, gstate : &game::State) {
        let discoviered_areas = astate.discovered_areas.iter()
            .filter_map(|coord| gstate.tile_at(*coord))
            .filter_map(|tile| tile.area.as_ref())
            ;

        if let Some(s) = self.format_areas(discoviered_areas.map(|area| area.type_)) {
            self.log(s, gstate);
        }
    }

    fn draw(&mut self, astate : &actor::State, gstate : &game::State) {
        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(nc::stdscr, &mut max_y, &mut max_x);

        match self.mode {
            Mode::Normal|Mode::Examine => {
                self.draw_map(astate, gstate);
                if self.mode == Mode::Normal {
                    self.draw_log(astate, gstate);
                } else {
                    self.draw_examine(astate, gstate);
                }
                self.draw_stats(astate, gstate);
            },
            Mode::Help => {
                self.draw_help();
            },
            Mode::Intro => {
                self.draw_intro();
            },
        }

        nc::mv(max_y - 1, max_x - 1);
    }

    fn input(&mut self) -> Option<Action> {
        loop {
            let ch = nc::getch();
            match self.mode {
                Mode::Intro => match ch {
                    -1 => return None,
                    _ => {
                        self.mode = Mode::Normal;
                        return Some(Action::Redraw);
                    }
                },
                Mode::Normal => {
                    return Some(match (ch as u8) as char {
                        'q' => Action::Exit,
                        'h' =>  Action::Game(game::Action::Turn(Angle::Left)),
                        'l' => Action::Game(game::Action::Turn(Angle::Right)),
                        'k'|'K' => Action::Game(game::Action::Move(Angle::Forward)),
                        'H'|'J' => Action::Game(game::Action::Move(Angle::Left)),
                        'u' => Action::Game(game::Action::Spin(Angle::Left)),
                        'i' => Action::Game(game::Action::Spin(Angle::Right)),
                        'L' => Action::Game(game::Action::Move(Angle::Right)),
                        'j' => Action::Game(game::Action::Move(Angle::Back)),
                        '.' => Action::Game(game::Action::Wait),
                        'o' => Action::AutoExplore,
                        'x' =>  {
                            self.examine_pos = None;
                            self.mode = Mode::Examine;
                            return Some(Action::Redraw);
                        },
                        '?' => {
                            self.mode = Mode::Help;
                            return Some(Action::Redraw);
                        },
                        _ => { return None }
                    })
                },
                Mode::Help => match ch {
                    -1 => return None,
                    _ => {
                        self.mode = Mode::Normal;
                        return Some(Action::Redraw);
                    }
                },
                Mode::Examine => {
                    if ch == -1 {
                        return None;
                    }

                    let pos = self.examine_pos.unwrap();

                    match ch as u8 as char {
                        'x' | 'q' => {
                            self.examine_pos = None;
                            self.mode = Mode::Normal;
                        },
                        'h' => {
                            self.examine_pos = Some(pos + Angle::Left);
                        },
                        'l' => {
                            self.examine_pos = Some(pos + Angle::Right);
                        },
                        'j' => {
                            self.examine_pos = Some(pos + (pos.dir + Angle::Back).to_coordinate());
                        },
                        'k' => {
                            self.examine_pos = Some(pos + pos.dir.to_coordinate());
                        },
                        'K' => {
                            self.examine_pos = Some(pos + pos.dir.to_coordinate().scale(5));
                        },
                        'J' => {
                            self.examine_pos = Some(pos + (pos.dir + Angle::Back).to_coordinate().scale(5));
                        },
                        _ => {
                            return None;
                        }
                    }
                    return Some(Action::Redraw);
                }
            }
        }
    }

    fn event(&mut self, event : ui::Event, gstate : &game::State) {
        match event {
            ui::Event::Log(logev) => match logev {
                ui::LogEvent::AutoExploreDone => self.log("Nothing else to explore.".to_string(), gstate),
            }
        }
    }
}

impl Drop for CursesUI {
    fn drop(&mut self) {
        nc::clear();
        nc::refresh();
        nc::endwin();
    }
}
