use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::collections::VecDeque;
use std::{self, cmp, env};
use std::io::Write;
use core::str::StrExt;
use ncurses as nc;

use num::integer::Integer;

use util::circular_move;

use super::Action;
use game;
use game::area;
use actor::{self, Race, Slot};
use ui;
use item;

use hex2d::{Angle, IntegerSpacing, Coordinate, ToCoordinate, Position};

use game::tile;

use std::fmt;
use std::fmt::Write as FmtWrite;

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

const KEY_ESC : i32 = 0x1b;
const KEY_ENTER: i32 = '\n' as i32;
const KEY_LOWX : i32 = 'x' as i32;
const KEY_LOWY : i32 = 'y' as i32;
const KEY_LOWH : i32 = 'h' as i32;
const KEY_LOWL : i32 = 'l' as i32;
const KEY_LOWK : i32 = 'k' as i32;
const KEY_LOWU : i32 = 'u' as i32;
const KEY_LOWC : i32 = 'c' as i32;
const KEY_LOWI : i32 = 'i' as i32;
const KEY_LOWO : i32 = 'o' as i32;
const KEY_LOWQ : i32 = 'q' as i32;
const KEY_LOWJ : i32 = 'j' as i32;
const KEY_LOWF : i32 = 'f' as i32;
const KEY_CAPY : i32 = 'Y' as i32;
const KEY_CAPH : i32 = 'H' as i32;
const KEY_CAPL : i32 = 'L' as i32;
const KEY_CAPE : i32 = 'E' as i32;
const KEY_CAPI : i32 = 'I' as i32;
const KEY_CAPK : i32 = 'K' as i32;
const KEY_CAPJ : i32 = 'J' as i32;
const KEY_DOT  : i32 = '.' as i32;
const KEY_COMMA   : i32 = ',' as i32;
const KEY_HELP    : i32 = '?' as i32;
const KEY_DESCEND : i32 = '>' as i32;

pub fn item_to_str(t : item::Category) -> &'static str {
    match t {
        item::Category::Weapon => ")",
        item::Category::Armor => "[",
        item::Category::Misc => "\"",
        item::Category::Consumable => "%",
    }
}

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
    pub const YELLOW : u8 = 226;
    pub const ORANGE : u8 = 3;

    pub const BACKGROUND_BG : u8 = GRAY[1];
    pub const MAP_BACKGROUND_BG : u8 = GRAY[1];

    pub const VISIBLE_FG : u8 = WHITE;

    pub const NOT_IN_LOS_FG : u8 = GRAY[16];
    pub const NOT_IN_LOS_BG : u8 = GRAY[1];

    // in light, shaded (barely visible), in LoS but not visible (dark), not in LoS
    pub const EMPTY_FG : [u8; 4] = [GRAY[17], GRAY[10], NOT_IN_LOS_FG, NOT_IN_LOS_FG];
    pub const EMPTY_BG : [u8; 4] = [GRAY[24], GRAY[16], GRAY[4], NOT_IN_LOS_BG];
    pub const WATER_FG: [u8; 4] = EMPTY_FG;
    pub const WATER_BG: [u8; 4] = [4, 74, 67, 67];
    pub const STONE_FG : [u8; 4] = [BLACK, GRAY[1] , NOT_IN_LOS_FG, NOT_IN_LOS_FG];
    pub const WALL_FG : [u8; 4] = STONE_FG;
    pub const WALL_BG : [u8; 4] = [GRAY[14], GRAY[8] , GRAY[4], NOT_IN_LOS_BG];
    pub const CHAR_SELF_FG : [u8; 4] = [19, 18, NOT_IN_LOS_FG, NOT_IN_LOS_FG];
    pub const CHAR_ALLY_FG : [u8; 4] = [28, 22, NOT_IN_LOS_FG, NOT_IN_LOS_FG];
    pub const CHAR_ENEMY_FG : [u8; 4] = [124, 88, NOT_IN_LOS_FG, NOT_IN_LOS_FG];
    pub const CHAR_GRAY_FG : u8= GRAY[17];
    pub const CHAR_BG : [u8; 4] = EMPTY_BG;

    pub const BLOCKED_BG :   u8 = 124;

    pub const LABEL_FG: u8 = 94;
    pub const GREEN_FG: u8 = 34;
    pub const RED_FG:   u8 = 124;
    pub const NOISE_BG : u8 = ORANGE;
    pub const TARGET_SELF_FG : u8 = 20;
    pub const TARGET_ENEMY_FG : u8 = 196;
    pub const LIGHTSOURCE : u8 = YELLOW;
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

    map_window : Option<Window>,
    log_window : Option<Window>,
    stats_window : Option<Window>,
    fs_window : Option<Window>,
    mode : Mode,
    log : VecDeque<LogEntry>,
    target_pos : Option<Position>,
    dot : &'static str,

    label_color: u64,
    text_color: u64,
    text_gray_color: u64,
    red_color: u64,
    green_color: u64,
}

impl CursesUI {

    pub fn new() -> CursesUI {

        let term_ok = env::var_os("TERM").as_ref()
            .and_then(|s| s.as_os_str().to_str())
            .map_or(false, |s| s.ends_with("-256color"));

        let term_putty = env::var_os("TERM").as_ref()
            .and_then(|s| s.as_os_str().to_str())
            .map_or(false, |s| s.starts_with("putty"));

        if !term_ok {
            panic!("Your TERM environment variable must end with -256color, sorry, stranger from the past. It is curable. Google it, fix it, try again.");
        }

        if env::var_os("ESCDELAY").is_none() {
            env::set_var("ESCDELAY", "25");
        }

        unsafe {
            let _ = locale::setlocale(locale::LC_ALL, b"en_US.UTF-8\0".as_ptr() as *const i8);
        }


        nc::initscr();
        nc::start_color();
        nc::keypad(nc::stdscr, true);
        nc::noecho();
        nc::raw();
        nc::timeout(0);
        nc::flushinp();

        assert!(nc::has_colors());

        let mut calloc = color::Allocator::new();
        let label_color = nc::COLOR_PAIR(
            calloc.get(color::LABEL_FG, color::BACKGROUND_BG)
            );
        let text_color = nc::COLOR_PAIR(
            calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG)
            );
        let text_gray_color = nc::COLOR_PAIR(
            calloc.get(color::GRAY[10], color::BACKGROUND_BG)
            );
        let green_color = nc::COLOR_PAIR(
            calloc.get(color::GREEN_FG, color::BACKGROUND_BG)
            );
        let red_color = nc::COLOR_PAIR(
            calloc.get(color::RED_FG, color::BACKGROUND_BG)
            );

        nc::doupdate();

        let mut ret = CursesUI {
            calloc: RefCell::new(calloc),
            map_window: None,
            stats_window: None,
            log_window: None,
            fs_window: None,
            mode: Mode::Normal,
            target_pos: None,
            dot: if term_putty { NORMAL_DOT } else { UNICODE_DOT },
            log: VecDeque::new(),
            label_color: label_color,
            text_color: text_color,
            text_gray_color: text_gray_color,
            red_color: red_color,
            green_color: green_color,
        };

        ret.resize();
        ret
    }

    fn resize(&mut self) {

        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(nc::stdscr, &mut max_y, &mut max_x);

        let mid_x = max_x - 30;
        let mid_y = 12;

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

        self.map_window = Some(map_window);
        self.stats_window = Some(stats_window);
        self.log_window = Some(log_window);
        self.fs_window = Some(fs_window);
    }

    pub fn log(&mut self, s : &str, gstate : &game::State) {
        self.log.push_front(LogEntry{
            text: s.to_string(), turn: gstate.turn
        });
    }

    pub fn display_intro(&mut self) {
        self.mode = Mode::FullScreen(FSMode::Intro);
    }

    fn draw_map(
        &mut self,
        astate : &actor::State, gstate : &game::State,
        )
    {
        let mut calloc = self.calloc.borrow_mut();

        let window = self.map_window.as_ref().unwrap().window;

        let actors_aheads : HashMap<Coordinate, Coordinate> =
            gstate.actors.iter().filter(|&(_, a)| !a.is_dead())
            .map(|(_, a)| (a.pos.coord + a.pos.dir, a.pos.coord)).collect();
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

        let (center, head) = match self.mode {
            Mode::Examine => {
                match self.target_pos {
                    None => {
                        self.target_pos = Some(astate.pos);
                        (astate.pos.coord, astate.pos.coord + astate.pos.dir)
                    },
                    Some(pos) => {
                        (pos.coord, pos.coord + pos.dir)
                    },
                }
            },
            Mode::Target(_) => {
                match self.target_pos {
                    None => {
                        self.target_pos = Some(astate.pos);
                        (astate.pos.coord, astate.pos.coord + astate.pos.dir)
                    },
                    Some(pos) => {
                        (astate.pos.coord, pos.coord)
                    },
                }
            },
            _ => {
                (astate.pos.coord, astate.pos.coord + astate.pos.dir)
            }
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

                    let t = gstate.map[c].clone();
                    let tt = t.type_;

                    (
                        astate.sees(c) || astate.is_dead(),
                        astate.in_los(c) || astate.is_dead(),
                        astate.knows(c),
                        Some(tt), Some(t),
                        gstate.at(c).light_as_seen_by(astate)
                    )
                } else {
                    // Paint a glue characters between two real characters
                    let c1 = c;
                    let (c2, _) = Coordinate::from_pixel_integer(SPACING, (cvx + 1, cvy));

                    let knows = astate.knows(c1) && astate.knows(c2);

                    let (e1, e2) = (
                        gstate.at(c1).tile().ascii_expand(),
                        gstate.at(c2).tile().ascii_expand(),
                        );

                    let c = Some(if e1 > e2 { c1 } else { c2 });

                    let tt = c.map_or(None, |c| Some(gstate.at(c).tile().type_));

                    let visible = (astate.sees(c1) && astate.sees(c2)) ||
                        astate.is_dead();
                    let in_los = (astate.in_los(c1) && astate.in_los(c2))
                        || astate.is_dead();

                    (
                        visible, in_los, knows,
                        tt, None,
                        cmp::max(
                            gstate.at(c1).light_as_seen_by(astate),
                            gstate.at(c2).light_as_seen_by(astate)
                            )
                    )
                };

                let mut draw = knows;

                let mut bold = false;
                let occupied = gstate.at(c).is_occupied();
                let (fg, bg, mut glyph) =
                    if is_proper_coord && visible && occupied {
                        let fg = match gstate.at(c).actor_map_or(Race::Grue, |a| a.race) {
                            Race::Human => color::CHAR_SELF_FG,
                            Race::Pony => color::CHAR_ALLY_FG,
                            Race::Grue => color::CHAR_ENEMY_FG,
                        };
                        (fg, color::CHAR_BG, "@")
                    } else if is_proper_coord && visible && gstate.at(c).item().is_some() {
                        let item = gstate.at(c).item().unwrap();
                        let s = item_to_str(item.category());
                        if astate.discovered.contains(&c) {
                            bold = true;
                        }
                        (color::WALL_FG, color::EMPTY_BG, s)
                    } else if knows {
                        match tt {
                            Some(tile::Empty) => {
                                let mut fg = color::STONE_FG;
                                let mut bg = color::EMPTY_BG;
                                let mut glyph = " ";

                                if is_proper_coord {
                                    match t.and_then(|t| t.feature) {
                                        None => {
                                            glyph = self.dot;
                                            fg = color::EMPTY_FG;
                                            bg = color::EMPTY_BG;
                                        }
                                        Some(tile::Door(open)) =>
                                            glyph = if open { "_" } else { "+" },
                                        Some(tile::Statue) => glyph = "&",
                                        Some(tile::Stairs) => glyph = ">",
                                    }
                                }

                                (fg, bg, glyph)
                            },
                            Some(tile::Wall) => {
                                (color::WALL_FG, color::WALL_BG, "#")
                            },
                            Some(tile::Water) => {
                                (color::WATER_FG, color::WATER_BG, "~")
                            },
                            None => {
                                (color::EMPTY_FG, color::EMPTY_BG, "?")
                            },
                        }
                    } else {
                        (color::EMPTY_FG, color::EMPTY_BG, " ")
                    };


                let (mut fg, mut bg) = if !visible || light == 0 {
                    if visible /* change to in_los for los debug */ {
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
                        fg = color::LIGHTSOURCE;
                        glyph = "*";
                    }
                }

                if is_proper_coord && visible && gstate.at(c).actor_map_or(0, |a| a.light_emision) > 0u32 {
                    bg = color::LIGHTSOURCE;
                }

                if self.mode == Mode::Examine {
                    if is_proper_coord && center == c {
                        glyph = "@";
                        fg = color::CHAR_GRAY_FG;
                        draw = true;
                    } else if is_proper_coord && c == head {
                        bold = true;
                        if astate.knows(c) {
                            fg = color::TARGET_SELF_FG;
                        } else {
                            draw = true;
                            glyph = " ";
                            bg = color::TARGET_SELF_FG;
                        }
                    }
                } else if let Mode::Target(_) = self.mode {
                    if is_proper_coord && target_line.contains(&c) {
                        glyph = "*";
                        draw = true;
                        if c == head {
                            fg = color::TARGET_SELF_FG;
                        }
                        if !gstate.at(c).tile().is_passable() {
                            bg = color::BLOCKED_BG;
                        }
                    }
                } else {
                    if is_proper_coord && actors_aheads.contains_key(&c) &&
                        astate.sees(*actors_aheads.get(&c).unwrap()) {
                        bold = true;
                        let color = if c == astate_ahead {
                            color::TARGET_SELF_FG
                        } else {
                            color::TARGET_ENEMY_FG
                        };

                        if astate.knows(c) {
                            if occupied {
                                bg = color;
                            } else {
                                fg = color;
                            }
                        } else {
                            draw = true;
                            glyph = " ";
                            bg = color;
                        }
                    }
                }

                if is_proper_coord && c != center && !visible && astate.hears(c) {
                    bg = color::NOISE_BG;
                    draw = true;
                }

                if draw {
                    let cpair = nc::COLOR_PAIR(calloc.get(fg, bg));

                    if bold {
                        nc::wattron(window, nc::A_BOLD() as i32);
                    }

                    nc::wattron(window, cpair as i32);
                    nc::mvwaddstr(window, vy, vx, glyph);
                    nc::wattroff(window, cpair as i32);

                    if bold {
                        nc::wattroff(window, nc::A_BOLD() as i32);
                    }
                }

            }
        }

        nc::wnoutrefresh(window);
    }

    fn draw_stats_bar(&mut self, window : nc::WINDOW, name : &str,
                      cur : i32, prev : i32, max : i32) {

        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(window, &mut max_y, &mut max_x);

        let cur = cmp::max(cur, 0) as u32;
        let prev = cmp::max(prev, 0) as u32;
        let max = cmp::max(max, 1) as u32;

        nc::wattron(window, self.label_color as i32);
        nc::waddstr(window, &format!("{}: ", name));

        let width = max_x as u32 - 4 - name.chars().count() as u32;
        let cur_w = cur * width / max;
        let prev_w = prev * width / max;

        nc::wattron(window, self.text_color as i32);
        nc::waddstr(window, "[");
        for i in 0..width {
            let (color, s) = match (i < cur_w, i < prev_w) {
                (true, true) => (self.text_color, "="),
                (false, true) => (self.red_color, "-"),
                (true, false) => (self.green_color, "+"),
                (false, false) => (self.text_color, " "),
            };
            nc::wattron(window, color as i32);
            nc::waddstr(window, s);
        }
        nc::wattron(window, self.text_color as i32);
        nc::waddstr(window, "]");
    }

    fn draw_turn<T>(&self, window : nc::WINDOW, label: &str, val: T)
        where T : Integer+fmt::Display
    {
        nc::wattron(window, self.label_color as i32);
        nc::waddstr(window, &format!("{}: ", label));

        nc::wattron(window, self.text_color as i32);
        nc::waddstr(window, &format!("{:<8}", val));
    }

    fn draw_val<T>(&self, window : nc::WINDOW, label: &str, val: T)
        where T : Integer+fmt::Display
    {
        nc::wattron(window, self.label_color as i32);
        nc::waddstr(window, &format!("{}:", label));

        nc::wattron(window, self.text_color as i32);
        nc::waddstr(window, &format!("{:>2} ", val));
    }

    fn draw_label(&self, window : nc::WINDOW, label: &str) {
        nc::wattron(window, self.label_color as i32);
        nc::waddstr(window, &format!("{}:", label));
    }

    fn draw_item(&self, window : nc::WINDOW, astate : &actor::State, label: &str, slot : actor::Slot) {
        self.draw_label(window, label);

        if slot == Slot::RHand && astate.melee_cd > 0 {
            nc::wattron(window, self.text_gray_color as i32);
        } else {
            nc::wattron(window, self.text_color as i32);
        }

        let item = if let Some(&(_, ref item)) = astate.items_equipped.get(&slot) {
            item.description().to_string()
        } else {
            "-".to_string()
        };

        //let item = item.slice_chars(0, cmp::min(item.char_len(), 13));
        nc::waddstr(window, &format!("{:^13}", item));
    }

    fn draw_inventory(&mut self, astate : &actor::State, _gstate : &game::State) {
        let window = self.map_window.as_ref().unwrap().window;

        let cpair = self.text_color;
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);

        nc::werase(window);
        nc::wmove(window, 0, 0);
        if !astate.items_equipped.is_empty() {
            nc::waddstr(window, &format!("Equipped: \n"));
            for (slot, &(ref ch, ref i)) in &astate.items_equipped {
                nc::waddstr(window, &format!(" {} - {} [{:?}]\n", ch, i.description(), slot));
            }
            nc::waddstr(window, &format!("\n"));
        }

        if !astate.items_backpack.is_empty() {
            nc::waddstr(window, &format!("Inventory: \n"));

            for (ch, i) in &astate.items_backpack {
                nc::waddstr(window, &format!(" {} - {}\n", ch, i.description()));
            }
            nc::waddstr(window, &format!("\n"));
        }

        nc::wnoutrefresh(window);
    }

    fn draw_stats(&mut self, astate : &actor::State, gstate : &game::State) {
        let window = self.stats_window.as_ref().unwrap().window;

        let (ac, ev) = (astate.stats.ac, astate.stats.ev);
        let (dmg, acc, cd) = (astate.stats.melee_dmg, astate.stats.melee_acc, astate.stats.melee_cd);

        let cpair = self.text_color;
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);

        nc::werase(window);
        nc::wmove(window, 0, 0);

        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(window, &mut max_y, &mut max_x);

        let mut y = 0;
        nc::wmove(window, y, 0);
        self.draw_val(window, "Str", astate.stats.str_);
        nc::wmove(window, y, 7);
        self.draw_val(window, "DMG", dmg);
        nc::wmove(window, y, 15);
        self.draw_val(window, "CD", cd);
        nc::wmove(window, y, 21);
        self.draw_val(window, "ACC", acc);

        y += 1;
        nc::wmove(window, y, 0);
        self.draw_val(window, "Int", astate.stats.int);
        nc::wmove(window, y, 7);
        self.draw_val(window, " AC", ac);
        nc::wmove(window, y, 15);
        self.draw_val(window, "EV", ev);

        y += 1;
        nc::wmove(window, y, 0);
        self.draw_val(window, "Dex", astate.stats.dex);

        y += 1;
        nc::wmove(window, y, 0);

        self.draw_stats_bar(window, "HP",
                            astate.hp, astate.prev_hp,
                            astate.stats.max_hp);

        y += 1;
        nc::wmove(window, y, 0);
        self.draw_stats_bar(window, "MP",
                            astate.mp, astate.prev_mp,
                            astate.stats.max_mp);

        y += 1;
        nc::wmove(window, y, 0);
        self.draw_stats_bar(window, "SP",
                            astate.sp, astate.prev_sp,
                            astate.stats.max_sp);

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
                nc::wmove(window, y, 0);
            } else {
                nc::wmove(window, y, 14);
            }

            self.draw_item(window, astate, string, slot);
        }

        y += 1;
        nc::wmove(window, y, 0);

        let pos = if self.mode == Mode::Examine {
            self.target_pos.unwrap()
        } else {
            astate.pos
        };

        let head = pos.coord + pos.dir;
        let descr = self.tile_description(head, astate, gstate);
        self.draw_label(window, "In front");
        nc::wattron(window, self.text_color as i32);
        nc::waddstr(window, &format!(" {} ({} {} {})", descr,
                        gstate.at(head).light_as_seen_by(astate) as usize,
                        astate.sees(head) as usize,
                        astate.in_los(head) as usize
        ));

        y += 1;
        nc::wmove(window, y, 0);
        self.draw_turn(window, "Turn", gstate.turn);
        self.draw_turn(window, "Level", gstate.level);

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

        let tile_type = gstate.at(coord).tile().type_;
        let tile = gstate.at(coord).tile();
        let feature_descr = tile.feature.map(|f| f.description());
        let item_descr = gstate.at(coord).item_map_or(None, |i| Some(i.description().to_string()));

        let actor_descr =
            if astate.sees(coord) || astate.is_dead() {
                gstate.at(coord).actor_map_or(None, |a| Some(match a.race{
                    Race::Pony => "A Pony",
                    Race::Grue => "A Grue",
                    Race::Human => "Human",
                }.to_string())
                )
            } else {
                None
            };

        match (tile_type, feature_descr, actor_descr, item_descr) {

            (_, _, Some(a_descr), _) => a_descr,
            (_, _, _, Some(i_descr)) => i_descr,
            (_, Some(f_descr), _, _) => f_descr.to_string(),
            (tile::Wall, _, _, _) => {
                "a wall".to_string()
            },
            (tile::Empty, _, _, _) => {
                match tile.area.and_then(|a| Some(a.type_)) {
                    Some(area::Room(_)) => "room".to_string(),
                    None => "nothing".to_string()
                }
            },
            _ => {
                "Indescribable".to_string()
            },
        }
    }

    fn draw_log(&mut self, _ : &actor::State, gstate : &game::State) {
        let window = self.log_window.as_ref().unwrap().window;

        let cpair = nc::COLOR_PAIR(self.calloc.borrow_mut().get(color::VISIBLE_FG, color::BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
        nc::wmove(window, 0, 0);
        let mut last_turn = None;

        for i in &self.log {
            if let Some(last_turn) = last_turn {
                if last_turn != i.turn && nc::getcurx(window) != 0 {
                    nc::waddstr(window, "\n");
                }
            }
            last_turn = Some(i.turn);

            if nc::getcury(window) == nc::getmaxy(window) - 1 {
                break;
            }

            if let Some(color) = self.turn_to_color(i.turn, &self.calloc, gstate) {
                let cpair = nc::COLOR_PAIR(color);
                nc::wattron(window, cpair as i32);
                nc::waddstr(window, &format!(
                        "{} ", i.text.as_str()
                        ));
            }
        }

        nc::wnoutrefresh(window);
    }

    fn draw_intro(&mut self) {
        let window = self.fs_window.as_ref().unwrap().window;
        let mut calloc = self.calloc.borrow_mut();
        let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
        nc::wmove(window, 0, 0);

        nc::waddstr(window, "Long, long ago in a galaxy far, far away...\n\n");
        nc::waddstr(window, &format!("You can press {} in the game for help.\n\n",  KEY_HELP as u8 as char));
        nc::waddstr(window, "Press anything to start.");
        nc::wnoutrefresh(window);
    }

    fn draw_help( &mut self) {
        let window = self.fs_window.as_ref().unwrap().window;
        let mut calloc = self.calloc.borrow_mut();
        let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
        nc::wmove(window, 0, 0);

        nc::waddstr(window, "This game is still incomplete. Sorry for that.\n\n");
        nc::waddstr(window, "= (more or less) Implemented actions = \n\n");
        nc::waddstr(window, "Move: hjklui\n");
        nc::waddstr(window, "Wait: .\n");
        nc::waddstr(window, "Pick: ,\n");
        nc::waddstr(window, "Fire/Throw: f\n");
        nc::waddstr(window, "Autoexplore: O\n");
        nc::waddstr(window, "Examine: x\n");
        nc::waddstr(window, "Equip: E\n");
        nc::waddstr(window, "Inventory: I\n");
        nc::waddstr(window, "Quit: q\n");
        nc::wnoutrefresh(window);
    }

    fn draw_quit( &mut self) {
        let window = self.fs_window.as_ref().unwrap().window;
        let mut calloc = self.calloc.borrow_mut();
        let cpair = nc::COLOR_PAIR(
            calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG)
            );

        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(nc::stdscr, &mut max_y, &mut max_x);
        let text = "Quit. Are you sure?";

        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);
        nc::wmove(window, max_y / 2, (max_x  - text.chars().count() as i32) / 2);

        nc::waddstr(window, text);
        nc::wnoutrefresh(window);
    }

}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum InvMode {
    View,
    Equip,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum FSMode {
    Help,
    Intro,
    Quit,
}
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum TargetMode {
    Fire,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Mode {
    Normal,
    Examine,
    Target(TargetMode),
    FullScreen(FSMode),
    Inventory(InvMode),
}

impl ui::UiFrontend for CursesUI {

    fn update(&mut self, astate : &actor::State, gstate : &game::State) {
        if astate.is_dead() {
            return;
        }

        let discoviered_areas = astate.discovered_areas.iter()
            .filter_map(|coord| gstate.at(*coord).tile().area)
            ;

        if let Some(s) = self.format_areas(discoviered_areas.map(|area| area.type_)) {
            self.log(&s, gstate);
        }

        for item_coord in astate.discovered.iter().filter(|&coord|
                                      gstate.at(*coord).item_map_or(false, |_| true)
                                      ) {
            let item_descr = gstate.at(*item_coord).item_map_or("".to_string(), |i| i.description().to_string());
            self.log(&format!("You've found {}.", item_descr), gstate);
        }

        if astate.discovered_stairs(gstate) {
            self.log("You've found stairs.", gstate);
        }

        for res in &astate.was_attacked_by {
            if res.success {
                self.log(&format!(
                        "{} hit you {}for {} dmg.",
                        res.who,
                        if res.behind { "from behind " } else { "" },
                        res.dmg
                        ), gstate);
            } else {
                self.log(&format!("{} missed you.", res.who), gstate);
            }
        }

        for res in &astate.did_attack {
            if res.success {
                self.log(&format!(
                        "You hit {} {}for {} dmg.",
                        res.who,
                        if res.behind { "from behind " } else { "" },
                        res.dmg
                        ), gstate);
            } else {
                self.log(&format!("You missed {}.", res.who), gstate);
            }
        }

        let noises = astate.heared.iter()
            .filter(|&(c, _) | *c != astate.pos.coord)
            .filter(|&(c, _) | !astate.sees(*c));

        for (_, &noise) in noises {
            self.log(&format!("You hear {}.", noise.description()), gstate);
        }
    }

    fn draw(&mut self, astate : &actor::State, gstate : &game::State) {
        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(nc::stdscr, &mut max_y, &mut max_x);

        match self.mode {
            Mode::Normal|Mode::Examine|Mode::Inventory(_)|Mode::Target(_) => {
                if let Mode::Inventory(_) = self.mode {
                    self.draw_inventory(astate, gstate);
                } else {
                    self.draw_map(astate, gstate);
                }

                self.draw_log(astate, gstate);

                self.draw_stats(astate, gstate);
            },
            Mode::FullScreen(fs_mode) => match fs_mode {
                FSMode::Help => {
                    self.draw_help();
                },
                FSMode::Quit => {
                    self.draw_quit();
                },
                FSMode::Intro => {
                    self.draw_intro();
                },
            },
        }

        nc::mv(max_y - 1, max_x - 1);
        let _ = std::io::stdout().flush();
    }

    fn input(&mut self, astate : Option<&actor::State>) -> Option<Action> {
        loop {
            let ch = nc::getch();
            if ch == nc::KEY_RESIZE {
                self.resize();
                return Some(Action::Redraw);
            }
            if ch == -1 {
                return None;
            }
            match self.mode {
                Mode::FullScreen(fs_mode) => match fs_mode {
                    FSMode::Quit => match ch {
                        KEY_LOWY|KEY_CAPY => return Some(Action::Exit),
                        _ => {
                            self.mode = Mode::Normal;
                            return Some(Action::Redraw);
                        },
                    },
                    _ => match ch {
                        _ => {
                            self.mode = Mode::Normal;
                            return Some(Action::Redraw);
                        },
                    },
                },
                Mode::Normal => {
                    return Some(match ch {
                        KEY_LOWH => Action::Game(game::Action::Turn(Angle::Left)),
                        KEY_LOWL => Action::Game(game::Action::Turn(Angle::Right)),
                        KEY_LOWK => Action::Game(game::Action::Move(Angle::Forward)),
                        KEY_LOWC => Action::Game(game::Action::Charge),
                        KEY_LOWU => Action::Game(game::Action::Spin(Angle::Left)),
                        KEY_LOWI => Action::Game(game::Action::Spin(Angle::Right)),
                        KEY_CAPH => Action::Game(game::Action::Move(Angle::Left)),
                        KEY_CAPL => Action::Game(game::Action::Move(Angle::Right)),
                        KEY_LOWJ => Action::Game(game::Action::Move(Angle::Back)),
                        KEY_DOT => Action::Game(game::Action::Wait),
                        KEY_COMMA => Action::Game(game::Action::Pick),
                        KEY_DESCEND => Action::Game(game::Action::Descend),
                        KEY_LOWO => Action::AutoExplore,
                        KEY_LOWQ => {
                            self.mode = Mode::FullScreen(FSMode::Quit);
                            return Some(Action::Redraw);
                        },
                        KEY_CAPI =>  {
                            self.mode = Mode::Inventory(InvMode::View);
                            return Some(Action::Redraw);
                        },
                        KEY_CAPE =>  {
                            self.mode = Mode::Inventory(InvMode::Equip);
                            return Some(Action::Redraw);
                        },
                        KEY_LOWX =>  {
                            self.target_pos = None;
                            self.mode = Mode::Examine;
                            return Some(Action::Redraw);
                        },
                        KEY_LOWF =>  {
                            self.target_pos = None;
                            self.mode = Mode::Target(TargetMode::Fire);
                            return Some(Action::Redraw);
                        },
                        KEY_HELP => {
                            self.mode = Mode::FullScreen(FSMode::Help);
                            return Some(Action::Redraw);
                        },
                        _ => { return None }
                    })
                },
                Mode::Inventory(InvMode::Equip) => match ch {
                    ch => match ch as u8 as char {
                        'a'...'z'|'A'...'Z' => {
                            if let Some(astate) = astate {
                                if astate.item_letter_taken(ch as u8 as char) {
                                    return Some(Action::Game(game::Action::Equip(ch as u8 as char)))
                                }
                            }
                            return None
                        },
                        '\x1b' => {
                            self.mode = Mode::Normal;
                            return Some(Action::Redraw);
                        },
                        _ => {},
                    }
                },
                Mode::Inventory(InvMode::View) => match ch {
                    ch => match ch as u8 as char {
                        'a'...'z'|'A'...'Z' => {
                        },
                        '\x1b' => {
                            self.mode = Mode::Normal;
                            return Some(Action::Redraw);
                        },
                        _ => {},
                    }
                },
                Mode::Examine => {
                    let pos = self.target_pos.unwrap();

                    match ch {
                        KEY_ESC | KEY_LOWX | KEY_LOWQ => {
                            self.target_pos = None;
                            self.mode = Mode::Normal;
                        },
                        KEY_LOWH => {
                            self.target_pos = Some(pos + Angle::Left);
                        },
                        KEY_LOWL => {
                            self.target_pos = Some(pos + Angle::Right);
                        },
                        KEY_LOWJ => {
                            self.target_pos = Some(pos + (pos.dir + Angle::Back).to_coordinate());
                        },
                        KEY_LOWK => {
                            self.target_pos = Some(pos + pos.dir.to_coordinate());
                        },
                        KEY_CAPK => {
                            self.target_pos = Some(pos + pos.dir.to_coordinate().scale(5));
                        },
                        KEY_CAPJ => {
                            self.target_pos = Some(pos + (pos.dir + Angle::Back).to_coordinate().scale(5));
                        },
                        _ => {
                            return None;
                        }
                    }
                    return Some(Action::Redraw);
                },
                Mode::Target(_) => {
                    let pos = self.target_pos.unwrap();
                    let astate = astate.unwrap();
                    let center = astate.pos;

                    match ch  {
                        KEY_ESC | KEY_LOWX | KEY_LOWQ => {
                            self.target_pos = None;
                            self.mode = Mode::Normal;
                        },
                        KEY_ENTER | KEY_LOWF => {
                            let target = self.target_pos.unwrap();
                            self.target_pos = None;
                            self.mode = Mode::Normal;
                            return Some(Action::Game(game::Action::Fire(target.coord)));
                        },
                        KEY_LOWH => {
                            self.target_pos = Some(
                                circular_move(center, pos, Angle::Left)
                                );
                        },
                        KEY_LOWL => {
                            self.target_pos = Some(
                                circular_move(center, pos, Angle::Right)
                                );
                        },
                        KEY_LOWJ => {
                            self.target_pos = Some(
                                circular_move(center, pos, Angle::Back)
                                );
                        },
                        KEY_LOWK => {
                            self.target_pos = Some(
                                circular_move(center, pos, Angle::Forward)
                                );
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
                ui::LogEvent::AutoExploreDone => self.log("Nothing else to explore.", gstate),
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
