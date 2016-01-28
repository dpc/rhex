use std::default;
use std::collections::{HashMap, HashSet};

use super::ui::Mode;
use super::color::{self, Color};
use super::consts::*;

use game::{Location, tile, item};
use game::actor::Race;
use hex2d::{Coordinate, Direction, Position};

use ncurses as nc;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
struct Character {
    glyph : char,
    bg : Color,
    fg : Color,
    bold : bool,
}


#[derive(Clone)]
struct CharArray {
    array : Vec<Option<Character>>, // 2d array
    width : usize,
    height : usize,
}

impl CharArray {
    pub fn new(width : usize, height : usize) -> Self {
        CharArray {
            array : vec![Default::default(); width * height],
            width: width,
            height: height,
        }
    }

    pub fn reset(&mut self) {
        for mut ch in self.array.iter_mut() {
            *ch = Default::default();
        }
    }

    pub fn resize(&mut self, width : usize, height : usize) {
        *self = CharArray::new(width, height);
    }
}

pub struct MapRenderer {
    base : CharArray,
    anim : CharArray,

    mode : Mode,
    coord_center : Coordinate,
    coord_head : Coordinate,

    actors_aheads: HashMap<Coordinate, Coordinate>,
    enemies_prev_pos: HashMap<Coordinate, Coordinate>,
    enemies_prev_head: HashMap<Coordinate, Coordinate>,

    dot : char,
    anim_frame_count: u32,
}

impl MapRenderer{
    pub fn new(dot : char) -> Self {
        MapRenderer {
            base : CharArray::new(0, 0),
            anim : CharArray::new(0, 0),
            mode : Mode::Normal,
            coord_center: Coordinate::new(0, 0),
            coord_head: Coordinate::new(0, 0),

            actors_aheads: Default::default(),
            enemies_prev_pos: Default::default(),
            enemies_prev_head: Default::default(),

            dot : dot,
            anim_frame_count: 0,
        }
    }

    pub fn resize(&mut self, width : usize, height : usize) {
        self.base.resize(width, height);
        self.anim.resize(width, height);
    }

    pub fn update(&mut self, cur_loc: &Location, mode : Mode, target_pos : Option<Position>, anim_frame_count : u32) {
        self.base.reset();
        let player = cur_loc.player();

        self.mode = mode;
        self.anim_frame_count = anim_frame_count;

        let (center, head) = match mode {
            Mode::Examine => {
                match target_pos {
                    None => (player.pos.coord, player.pos.coord + player.pos.dir),
                    Some(pos) => (pos.coord, pos.coord + pos.dir),
                }
            }
            Mode::Target(_) => {
                match target_pos {
                    None => (player.pos.coord, player.pos.coord + player.pos.dir),
                    Some(pos) => (player.pos.coord, pos.coord),
                }
            }
            _ => (player.pos.coord, player.pos.coord + player.pos.dir),
        };

        self.coord_center = center;
        self.coord_head = head;

        self.actors_aheads =
            cur_loc.actors_byid
            .iter()
            .filter(|&(_, a)| !a.is_dead())
            .map(|(_, a)| (a.head(), a.pos.coord))
            .collect();

        self.enemies_prev_pos =
            cur_loc.actors_byid
            .iter()
            .filter(|&(_, a)| !a.is_dead())
            .filter(|&(_, a)| !a.is_player())
            .map(|(id, a)| (a.pre_pos.unwrap_or(a.pos).coord, a.pos.coord))
            .filter(|&(pre_pos, pos)| pre_pos != pos)
            .collect();

        self.enemies_prev_head =
            cur_loc.actors_byid
            .iter()
            .filter(|&(_, a)| !a.is_dead())
            .filter(|&(_, a)| !a.is_player())
            .filter(|&(_, a)| a.pre_pos.unwrap_or(a.pos).coord == a.pos.coord)
            .map(|(id, a)| (a.pre_head.unwrap_or(a.head()), a.head()))
            .filter(|&(pre, now)| pre != now)
            .collect();

        self.for_each_glyph(cur_loc, Self::draw_map_glyph);
    }

    fn draw_map_glyph(&mut self, cur_loc : &Location, vx : usize, vy : usize, c : Coordinate, is_proper_coord: bool) {
        let player = cur_loc.player();
        let player_ahead = player.pos.coord + player.pos.dir;

        let mut target_line = HashSet::new();
        if let Mode::Target(_) = self.mode {
            self.coord_center.for_each_in_line_to(self.coord_head, |c| {
                target_line.insert(c);
            });
        }

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
            let c2 = c + Direction::XY;

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
                    fg = color::LIGHTSOURCE.to_u8();
                    bold = true;
                }
            }
        }

        if is_proper_coord && visible &&
            cur_loc.at(c).actor_map_or(0, |a| a.light_emision()) > 0u32 {
                bg = color::LIGHTSOURCE.to_u8();
            }

        if is_proper_coord && self.actors_aheads.contains_key(&c) &&
            player.sees(*self.actors_aheads.get(&c).unwrap()) {
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

        if is_proper_coord && c != self.coord_center && !visible && player.hears(c) {
            bg = color::NOISE_BG.to_u8();
            draw = true;
        }

        // changed position animation
        if self.mode == Mode::Normal &&
            is_proper_coord && visible &&
                self.enemies_prev_pos.contains_key(&c) &&
                self.anim_frame_count < 6
                {
                    let cur_pos = self.enemies_prev_pos[&c];
                    let r = 5 - self.anim_frame_count;
                    fg = color::RGB::new(r as u8, 0, 0).to_u8();
                    match cur_pos.direction_to_cw(c) {
                        Some(Direction::XY) | Some(Direction::YX) => glyph = '-',
                        Some(Direction::ZY) | Some(Direction::YZ) => glyph = '\\',
                        Some(Direction::ZX) | Some(Direction::XZ) => glyph = '/',
                        None => {},
                    }
                }

        // changed direction animation
        if self.mode == Mode::Normal &&
            is_proper_coord && visible &&
                self.enemies_prev_head.contains_key(&c) &&
                self.anim_frame_count < 6
                {
                    let cur_pos = self.enemies_prev_head[&c];
                    let r = 5 - self.anim_frame_count;
                    fg = color::RGB::new(r as u8, 0, 0).to_u8();
                    glyph = self.dot;
                }

        if self.mode == Mode::Examine {
            if is_proper_coord && self.coord_center == c {
                glyph = '@';
                fg = color::CHAR_GRAY_FG;
                draw = true;
            } else if is_proper_coord && c == self.coord_head {
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
                if c == self.coord_head {
                    fg = color::TARGET_SELF_FG;
                }
                if !cur_loc.at(c).tile().is_passable() {
                    bg = color::BLOCKED_BG;
                }
            }
        }


        if draw {
            self.base.array[vy * self.base.width + vx] = Some(Character {
                glyph: glyph,
                fg: Color::from(fg),
                bg: Color::from(bg),
                bold: bold,
            })
        }
    }

    pub fn draw_into(&self, window : nc::WINDOW, calloc : &mut color::Allocator) {
        let cpair = nc::COLOR_PAIR(calloc.get(color::VISIBLE_FG, color::MAP_BACKGROUND_BG));
        nc::wbkgd(window, ' ' as nc::chtype | cpair as nc::chtype);
        nc::werase(window);

        let (max_x, max_y) = (self.base.width, self.base.height);
        for vx in 0..max_x {
            for vy in 0..max_y {
                match self.base.array[vy * max_x as usize + vx] {
                    Some(ch) => {
                        let cpair = nc::COLOR_PAIR(calloc.get(ch.fg, ch.bg));
                        if ch.glyph <= (127u8 as char) {
                            let c = (ch.glyph as nc::chtype) | cpair;
                            nc::mvwaddch(window, vy as i32, vx as i32, if ch.bold { c|nc::A_BOLD() } else { c });
                        } else {
                            let attrflag = if ch.bold { cpair|nc::A_BOLD() } else { cpair };
                            nc::wattron(window, attrflag as i32);
                            nc::mvwaddstr(window, vy as i32, vx as i32, &format!("{}", ch.glyph));
                            nc::wattroff(window, attrflag as i32);
                        }

                    }
                    None => { }
                }
            }
        }
    }


    fn for_each_glyph<F>(&mut self, cur_loc: &Location, f : F)
    where F : Fn(&mut MapRenderer, &Location, usize, usize, Coordinate, bool) {
        let (vpx, vpy) = self.coord_center.to_pixel_integer(SPACING);
        let (max_x, max_y) = (self.base.width, self.base.height);

        let mid_x = max_x / 2;
        let mid_y = max_y / 2;


        for vx in 0..max_x {
            for vy in 0..max_y {
                let (rvx, rvy) = (vx - mid_x, vy - mid_y);

                let (cvx, cvy) = (rvx + vpx as usize, rvy + vpy as usize);

                let (c, off) = Coordinate::from_pixel_integer(SPACING, (cvx as i32, cvy as i32));

                let is_proper_coord = off == (0, 0);

                f(self, cur_loc, vx, vy, c, is_proper_coord)
            }
        }
    }
}

pub fn item_to_char(t: item::Category) -> char {
    match t {
        item::Category::Weapon => ')',
        item::Category::RangedWeapon => '}',
        item::Category::Armor => '[',
        item::Category::Misc => '"',
        item::Category::Consumable => '%',
    }
}
