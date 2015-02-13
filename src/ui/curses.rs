use std::cell::RefCell;
use std::old_io::timer::sleep;
use std::time::duration::Duration;

use ncurses as nc;

use super::Action;
use game;
use actor;
use ui;

use hex2d::{Angle, IntegerSpacing};


mod locale {
    use libc::{c_int,c_char};
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

pub mod window {
    use ncurses as nc;
    use game;
    use actor;
    use hex2d::{Coordinate};
    use game::tile;
    use super::color;

    use super::SPACING;
    use super::color::Allocator;
    use std::collections::ring_buf::RingBuf;
    use std::cell::RefCell;
    use std::fmt;
    use std::fmt::Writer;

    pub trait Window {
        fn draw(
            &mut self,
            calloc : &RefCell<color::Allocator>,
            astate : &actor::State,
            gstate : &game::State
            );
    }

    pub struct Map {
        window : nc::WINDOW,
    }

    impl Map {
        pub fn new(w : i32, h : i32, x : i32, y : i32) -> Map {
            Map {
                window : nc::subwin(nc::stdscr, h, w, y, x),
            }
        }
    }

    impl Window for Map {
        fn draw(
            &mut self,
                calloc : &RefCell<color::Allocator>,
                astate : &actor::State, gstate : &game::State
                )
        {
            let mut calloc = calloc.borrow_mut();

            /* Get the screen bounds. */
            let mut max_x = 0;
            let mut max_y = 0;
            nc::getmaxyx(self.window, &mut max_y, &mut max_x);

            let mid_x = max_x / 2;
            let mid_y = max_y / 2;

            let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::MAP_BACKGROUND_BG));
            nc::wbkgd(self.window, ' ' as u32 | cpair as u32);
            nc::werase(self.window);

            let (vpx, vpy) = astate.pos.to_pixel_integer(SPACING);

            for vy in range(0, max_y) {
                for vx in range(0, max_x) {
                    let (rvx, rvy) = (vx - mid_x, vy - mid_y);

                    let (cvx, cvy) = (rvx + vpx, rvy + vpy);

                    let (c, off) = Coordinate::from_pixel_integer(SPACING, (cvx, cvy));

                    let is_proper_coord = off == (0, 0);

                    let (visible, tt, t, light) = if is_proper_coord {

                        if !astate.knows(c) {
                            continue;
                        }

                        let t = gstate.map.get(&c);

                        let tt = match t {
                            Some(t) => t.type_,
                            None => tile::Wall,
                        };

                        (astate.sees(c), Some(tt), t, gstate.light(c))
                    } else {
                        // Paint a glue characters between two real characters
                        let c1 = c;
                        let (c2, _) = Coordinate::from_pixel_integer(SPACING, (cvx + 1, cvy));

                        if !astate.knows(c1) || !astate.knows(c2) {
                            continue;
                        }

                        let (t1, t2) = match (
                            gstate.tile_map(c1, |t| t.type_),
                            gstate.tile_map(c2, |t| t.type_)
                            ) {
                            (Some(t1), Some(t2)) => (t1, t2),
                            (Some(t1), None) => (t1, t1),
                            (None, Some(t2)) => (t2, t2),
                            (None, None) => (tile::Wall, tile::Wall),
                        };

                        let tt = if !(t1.ascii_expand() && t2.ascii_expand()) {
                            None
                        } else {
                            Some(t1)
                        };

                        let visible = astate.sees(c1) && astate.sees(c2);

                        (visible, tt, None, (gstate.light(c1) + gstate.light(c2)) / 2)
                    };

                    let (fg, bg, glyph) =
                        if is_proper_coord && astate.sees(c) && gstate.actors.contains_key(&c) {
                            (color::CHAR_FG, color::CHAR_BG, "@")
                        } else {
                            match tt {
                                Some(tile::Empty) => {
                                    (color::EMPTY_FG, color::EMPTY_BG, ".")
                                },
                                Some(tile::Wall) => {
                                    (color::WALL_FG, color::WALL_BG, "#")
                                },
                                Some(tile::Tree) => {
                                    (color::TREE_FG, color::TREE_BG, "T")
                                },
                                None => {
                                    (color::EMPTY_FG, color::EMPTY_BG, " ")
                                },
                            }
                        };


                    let (mut fg, bg) = if !visible {
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

                    let cpair = nc::COLOR_PAIR(calloc.get(fg, bg));

                    if visible {
                        nc::attron(nc::A_BOLD());
                    }

                    nc::wattron(self.window, cpair);
                    nc::mvwaddstr(self.window, vy, vx, glyph);
                    nc::wattroff(self.window, cpair);

                    if visible {
                        nc::wattroff(self.window, nc::A_BOLD());
                    }

                }
            }

            nc::wnoutrefresh(self.window);
        }
    }

    pub struct LogEntry {
        turn : u64,
        text : String,
    }

    pub struct Log {
        window : nc::WINDOW,
        log : RingBuf<LogEntry>,
    }

    impl Log {
        pub fn new(w : i32, h : i32, x : i32, y : i32) -> Log {
            Log {
                window : nc::newwin(h, w, y, x),
                log : RingBuf::new(),
            }
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

        fn print_log(&self, calloc : &RefCell<color::Allocator>, gstate : &game::State) {
            for i in self.log.iter() {

                if nc::getcury(self.window) == nc::getmaxy(self.window) - 1 {
                    break;
                }

                if let Some(color) = self.turn_to_color(i.turn, calloc, gstate) {
                    let cpair = nc::COLOR_PAIR(color);
                    nc::wattron(self.window, cpair);
                    nc::waddstr(self.window, i.text.as_slice());
                    nc::waddstr(self.window, "\n");
                }
            }
        }

        pub fn log(&mut self, s : String, gstate : &game::State) {
            self.log.push_front(LogEntry{text: s, turn: gstate.turn});
        }
    }

    impl Window for Log {

        fn draw(
            &mut self,
            calloc : &RefCell<color::Allocator>,
            astate : &actor::State, gstate : &game::State
            )
        {

            let cpair = nc::COLOR_PAIR(calloc.borrow_mut().get(color::VISIBLE_FG, color::BACKGROUND_BG));
            nc::wbkgd(self.window, ' ' as u32 | cpair as u32);
            nc::werase(self.window);
            nc::wmove(self.window, 0, 0);

            let discoviered_areas = astate.discovered_areas.iter()
                .filter_map(|coord| gstate.tile_at(*coord))
                .filter_map(|tile| tile.area.as_ref())
                ;

            if let Some(s) = self.format_areas(discoviered_areas.map(|area| area.type_)) {
                self.log(s, gstate);
            }

            self.print_log(calloc, gstate);

            nc::wnoutrefresh(self.window);
        }
    }

    pub struct Stats {
        window : nc::WINDOW,
    }

    impl Stats {
        pub fn new(w : i32, h : i32, x : i32, y : i32) -> Stats {
            Stats {
                window : nc::newwin(h, w, y, x),
            }
        }
    }

    impl Window for Stats {

        fn draw(
            &mut self,
                calloc : &RefCell<color::Allocator>,
                _ : &actor::State, gstate : &game::State
                )
        {
            let mut calloc = calloc.borrow_mut();
            let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::BACKGROUND_BG));
            nc::wbkgd(self.window, ' ' as u32 | cpair as u32);
            nc::werase(self.window);
            nc::wmove(self.window, 0, 0);

            let mut turn_str = String::new();
            write!(&mut turn_str, "Turn: {}", gstate.turn).unwrap();

            nc::mvwaddstr(self.window, 0, 0, turn_str.as_slice());
            nc::mvwaddstr(self.window, 2, 0, "STR: 15");
            nc::mvwaddstr(self.window, 3, 0, "DEX: 15");
            nc::mvwaddstr(self.window, 4, 0, "INT: 15");

            nc::wnoutrefresh(self.window);
        }
    }

    impl Drop for Log {
        fn drop(&mut self) {
            nc::delwin(self.window);
        }
    }
}

pub struct CursesUI {
    calloc : RefCell<color::Allocator>,
    windows : Vec<Box<(window::Window + 'static)>>,
    log_window : Box<window::Log>,
}

impl CursesUI {

    pub fn new() -> CursesUI {
        unsafe {
            let _ = locale::setlocale(locale::LC_ALL, b"".as_ptr() as *const i8);
        }

        nc::initscr();
        nc::start_color();
        nc::raw();
        nc::keypad(nc::stdscr, true);
        nc::noecho();
        nc::timeout(0);
        nc::flushinp();
        nc::curs_set(nc::CURSOR_VISIBILITY::CURSOR_INVISIBLE);

        assert!(nc::has_colors());

        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(nc::stdscr, &mut max_y, &mut max_x);

        let mid_x = max_x * 3 / 5;
        let mid_y = max_y * 4 / 5;

        let mut windows : Vec<Box<window::Window>> = Vec::new();

        nc::refresh();
        // Workaround: For whatever reason ncurses sometimes does not display
        // anything at first and waits on getch() if this getch() is not delayed
        // enough. Seems like some weird i/o buffering interaction
        sleep(Duration::milliseconds(100));

        windows.push(Box::new(window::Map::new(
                mid_x, max_y, 0, 0
                )));
        windows.push(Box::new(window::Stats::new(
                max_x - mid_x, mid_y, mid_x, 0
                )));
        let log_window = Box::new(window::Log::new(
                max_x - mid_x, max_y - mid_y, mid_x, mid_y
                ));

        CursesUI {
            calloc: RefCell::new(color::Allocator::new()),
            windows: windows,
            log_window: log_window,
        }
    }
}

use self::window::Window;

impl ui::UiFrontend for CursesUI {

    fn draw(&mut self, astate : &actor::State, gstate : &game::State) {
        let mut max_x = 0;
        let mut max_y = 0;
        nc::getmaxyx(nc::stdscr, &mut max_y, &mut max_x);

        nc::mv(max_y - 1, max_x - 1);

        for w in self.windows.iter_mut() {
            w.draw(&self.calloc, astate, gstate);
        }
        (*self.log_window).draw(&self.calloc, astate, gstate);
        nc::doupdate();
    }

    fn input(&self) -> Option<Action> {
        Some(match (nc::getch() as u8) as char {
            'q' => Action::Exit,
            'h' =>  Action::Game(game::Action::Turn(Angle::Left)),
            'l' => Action::Game(game::Action::Turn(Angle::Right)),
            'k'|'K' => Action::Game(game::Action::Move(Angle::Forward)),
            'H'|'J' => Action::Game(game::Action::Move(Angle::Left)),
            'L' => Action::Game(game::Action::Move(Angle::Right)),
            'j' => Action::Game(game::Action::Move(Angle::Back)),
            '.' => Action::Game(game::Action::Wait),
            'o' => Action::AutoExplore,
            _ => { return None }
        })
    }

    fn event(&mut self, event : ui::Event, gstate : &game::State) {
        match event {
            ui::Event::Log(logev) => match logev {
                ui::LogEvent::AutoExploreDone => self.log_window.log("Nothing else to explore.".to_string(), gstate),
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
