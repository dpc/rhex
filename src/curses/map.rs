use std::collections::{HashMap};
use std::cell::RefCell;

use super::ui::Mode;
use super::color::{self, Color};
use super::consts::*;
use super::Window;

use game::{Location, tile, item};
use game::actor::{self, Race};
use hex2d::{Coordinate, Direction, Position};

use rand::thread_rng;
use rand::prelude::SliceRandom;
use ncurses as nc;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct DisplayCharacter {
    glyph : char,
    bg : Color,
    fg : Color,
    bold : bool,
}

impl Default for DisplayCharacter {
    fn default() -> Self {
        DisplayCharacter {
            glyph : ' ',
            fg : color::MAP_BACKGROUND_BG.into(),
            bg : color::MAP_BACKGROUND_BG.into(),
            bold : false,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
struct ExtendedCharacter {
    base : DisplayCharacter,
    known: bool,
    visible : bool,
    light : u32,
}

#[derive(Clone)]
struct Array2d<T> {
    array : Vec<T>, // 2d array
    width : usize,
    height : usize,
}

impl<T> Array2d<T>
where T : Clone+Default {
    pub fn new(width : usize, height : usize) -> Self {
        Array2d {
            array : vec![Default::default(); width * height],
            width: width,
            height: height,
        }
    }

    pub fn reset(&mut self) {
        for ch in &mut self.array {
            *ch = Default::default();
        }
    }

    pub fn resize(&mut self, width : usize, height : usize) {
        *self = Array2d::new(width, height);
    }

    pub fn at(&self, vx : usize, vy: usize) -> &T{
            &self.array[vy * self.width + vx]
    }

    pub fn at_mut(&mut self, vx: usize, vy: usize) -> &mut T {
            &mut self.array[vy * self.width + vx]
    }
}

pub struct MapRenderer {
    base : Array2d<ExtendedCharacter>,
    items : Array2d<Option<DisplayCharacter>>,
    actors : Array2d<Option<DisplayCharacter>>,
    effects : Array2d<Vec<DisplayCharacter>>,

    mode : Mode,
    coord_center : Coordinate,
    coord_head : Coordinate,

    actors_aheads: HashMap<Coordinate, Coordinate>,
    enemies_prev_pos: HashMap<Coordinate, Coordinate>,
    enemies_prev_head: HashMap<Coordinate, Coordinate>,

    dot : char,
    anim_frame_count: u32,
}

impl MapRenderer {
    pub fn new(dot : char) -> Self {
        MapRenderer {
            base : Array2d::new(0, 0),
            items: Array2d::new(0, 0),
            actors: Array2d::new(0, 0),
            effects: Array2d::new(0, 0),
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
        self.items.resize(width, height);
        self.actors.resize(width, height);
        self.effects.resize(width, height);
    }

    pub fn update(&mut self, cur_loc: &Location, mode : Mode,
                  target_pos : Option<Position>, anim_frame_count : u32) {
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

        if anim_frame_count == 0 {
            self.base.reset();
            self.items.reset();
            self.actors.reset();
            self.update_turn_changed(cur_loc);
            self.for_each_glyph(cur_loc, Self::draw_map_base_glyph);
            self.for_each_glyph(cur_loc, Self::draw_items);
            self.draw_actors(cur_loc);
        }

        self.effects.reset();
        self.draw_effects(cur_loc);
    }


    pub fn update_turn_changed(&mut self, cur_loc: &Location) {
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
                .map(|(_, a)| (a.pre_pos.unwrap_or(a.pos).coord, a.pos.coord))
                .filter(|&(pre_pos, pos)| pre_pos != pos)
                .collect();

            self.enemies_prev_head =
                cur_loc.actors_byid
                .iter()
                .filter(|&(_, a)| !a.is_dead())
                .filter(|&(_, a)| !a.is_player())
                .filter(|&(_, a)| a.pre_pos.unwrap_or(a.pos).coord == a.pos.coord)
                .map(|(_, a)| (a.pre_head.unwrap_or(a.head()), a.head()))
                .filter(|&(pre, now)| pre != now)
                .collect();
    }

    fn draw_map_base_glyph(&mut self, cur_loc : &Location,
                           vx : usize, vy : usize,
                           c : Coordinate, is_proper_coord: bool) {
        let player = cur_loc.player();

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

            let low_opaq1 = cur_loc.at(c1).tile().opaqueness() <= 1;
            let low_opaq2 = cur_loc.at(c2).tile().opaqueness() <= 1;

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

        debug_assert!(!visible || knows || player.is_dead());

        if !knows {
            return;
        }

        let mut bold = false;
        let (fg, bg, glyph) = match tt {
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

                    if visible && cur_loc.at(c).tile().light > 0 {
                        fg = [
                            color::LIGHTSOURCE[0].to_u8(),
                            color::LIGHTSOURCE[1].to_u8(),
                            color::LIGHTSOURCE[2].to_u8(),
                            color::LIGHTSOURCE[3].to_u8(),
                        ]
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
        };


        let fg = color_by_visibility(fg, visible, light);
        let bg = color_by_visibility(bg, visible, light);

        *self.base.at_mut(vx, vy) = ExtendedCharacter {
            base : DisplayCharacter {
                glyph: glyph,
                fg: Color::from(fg),
                bg: Color::from(bg),
                bold: bold,
            },
            light: light,
            visible: visible,
            known: knows,
        }
    }

    fn draw_items(&mut self, cur_loc : &Location,
                 vx : usize, vy : usize,
                 c : Coordinate, is_proper_coord: bool) {

        if !is_proper_coord {
            return;
        }

        let player = cur_loc.player();

        let base = &self.base.at(vx, vy);

        if !base.known || !base.visible {
            return;
        }

        if cur_loc.at(c).item().is_some() {
            let item = cur_loc.at(c).item().unwrap();

            let color = color::WALL_FG;
            let fg = color_by_visibility(color, base.visible, base.light);

            *self.items.at_mut(vx, vy) = Some(DisplayCharacter {
                fg: fg,
                bg: base.base.bg,
                bold: player.discovered.contains(&c),
                glyph: item_to_char(item.category())
            });
        }
    }

    fn draw_actors(&mut self, cur_loc : &Location) {
        let mut actors : Vec<(actor::Id, Position)> =
                cur_loc.actors_byid
                .iter()
                .filter(|&(_, a)| !a.is_dead())
                .map(|(id, a)| (*id, a.pos))
                .collect();

        let player = cur_loc.player();

        actors.shuffle(&mut thread_rng());

        for &(id, _) in &actors {
            let actor = &cur_loc.actors_byid[&id];
            let race = actor.race;

            let actor_coord = actor.pos.coord;
            let actor_head = actor.pos.coord + Coordinate::from(actor.pos.dir);

            if player.sees(actor_coord) {
                if let Some((vx, vy)) = self.coord_to_glyph_xy(actor_coord) {
                    let base = self.base.at(vx, vy);
                    debug_assert!(base.known);
                    let fg_palete = race_to_palete(race);
                    let mut fg = color_by_visibility(fg_palete, base.visible, base.light);

                    let mut bg = color_by_visibility(color::CHAR_BG, base.visible, base.light);
                    if self.actors_aheads.contains_key(&actor_coord) &&
                     player.sees(self.actors_aheads[&actor_coord]) {
                         bg = Color::from(if actor_coord == player.head() {
                                 color::SELF_HEAD_ACTOR_BG
                             } else {
                                 color::ENEMY_HEAD_ACTOR_BG
                             });
                        fg = Color::from(color::ACTOR_HEAD_ACTOR_FG);
                    }
                    *self.actors.at_mut(vx, vy) = Some(DisplayCharacter {
                        fg: Color::from(fg),
                        bg: bg,
                        bold: base.base.bold,
                        glyph: race_to_char(race),
                    });
                }

                if let Some((vx, vy)) = self.coord_to_glyph_xy(actor_head) {
                    let base = self.base.at(vx, vy);
                    let items = self.items.at(vx, vy);
                    let actors = self.actors.at_mut(vx, vy);
                    if base.known {
                        let palete = race_to_palete(race);
                        let color = palete[0];

                        if !cur_loc.at(actor_head).is_occupied() {
                            *actors = Some(DisplayCharacter {
                                fg: Color::from(color),
                                bg: base.base.bg,
                                bold: true,
                                glyph: items.unwrap_or(base.base).glyph,
                            });
                        }
                    } else {
                        let palete = race_to_palete(race);
                        let bg = palete[0];
                        *actors = Some(DisplayCharacter {
                            fg: base.base.fg,
                            bg: Color::from(bg),
                            bold: true,
                            glyph: ' ',
                        });
                    }
                }
            }
        }
    }

    fn draw_effects_normal(&mut self, cur_loc : &Location) {
         let mut actors : Vec<(actor::Id, Position)> =
                cur_loc.actors_byid
                .iter()
                .filter(|&(_, a)| !a.is_dead())
                .map(|(id, a)| (*id, a.pos))
                .collect();

        let player = cur_loc.player();

        actors.shuffle(&mut thread_rng());

        for &(id, _) in &actors {
            let actor = &cur_loc.actors_byid[&id];

            let cur_pos = actor.pos;
            let prev_pos = actor.prev_pos();

            if !player.could_have_seen(actor) {
                continue;
            }

            let prev_and_cur = if  prev_pos.coord != cur_pos.coord {
                Some((prev_pos.coord, cur_pos.coord))
            } else if prev_pos.dir != cur_pos.dir {
                Some((prev_pos.coord + Coordinate::from(prev_pos.dir),
                    cur_pos.coord + Coordinate::from(cur_pos.dir)))
            } else {
                None
            };

            if let Some((prev_coord, cur_coord)) = prev_and_cur {
                if let Some((tail_vx, tail_vy)) = self.coord_to_glyph_xy(prev_coord) {
                    let base = self.base.at(tail_vx, tail_vy);
                    if self.anim_frame_count < 6 {
                        let fg : Color = if actor.is_player() {
                            color::RGB::new(0, 0, 5)
                        } else {
                            color::RGB::new(5, 0, 0)
                        }.into();

                        // TODO: use effective fg, not base one
                        let fg : Color = fg.mix(base.base.bg, self.anim_frame_count as u8);

                        let glyph = match cur_coord.direction_to_cw(prev_coord) {
                            Some(Direction::XY) | Some(Direction::YX) => '-',
                            Some(Direction::ZY) | Some(Direction::YZ) => '\\',
                            Some(Direction::ZX) | Some(Direction::XZ) => '/',
                            None => panic!(),
                        };
                        self.effects.at_mut(tail_vx, tail_vy).push(
                            DisplayCharacter {
                                fg: Color::from(fg),
                                bg: base.base.bg,
                                bold: false,
                                glyph: glyph,
                            });
                    }
                }
            }
        }
    }

    fn draw_effects_examine(&mut self, _cur_loc : &Location) {
        let coord_center = self.coord_center;
        let coord_head = self.coord_head;
        if let Some((vx, vy)) = self.coord_to_glyph_xy(coord_center) {
            let base = self.base.at(vx, vy);
            self.effects.at_mut(vx, vy).push(
                DisplayCharacter {
                    fg: Color::from(color::CHAR_GRAY_FG),
                    bg: base.base.bg,
                    bold: false,
                    glyph: '@',
                });
        }

        if let Some((vx, vy)) = self.coord_to_glyph_xy(coord_head) {
            let base = self.base.at(vx, vy);
            self.effects.at_mut(vx, vy).push(
                DisplayCharacter {
                    fg: Color::from(color::CHAR_GRAY_FG),
                    bg: base.base.bg,
                    bold: true,
                    glyph: self.dot,
                });
        }
    }

    fn draw_effects_target(&mut self, cur_loc : &Location) {

        let mut target_line = Vec::new();
        for c in self.coord_center.line_to_iter(self.coord_head) {
            target_line.push(c);
        }

        for &c in &target_line {
            if let Some((vx, vy)) = self.coord_to_glyph_xy(c) {
                let base = self.base.at(vx, vy);

                self.effects.at_mut(vx, vy).push(
                    if cur_loc.at(c).tile().is_passable() && base.known {
                        DisplayCharacter {
                            fg: color_by_visibility(color::CHAR_SELF_FG.into(), base.visible, base.light),
                            bg: base.base.bg,
                            bold: base.base.bold,
                            glyph: '*',
                        }
                    } else {
                        DisplayCharacter {
                            fg: base.base.fg,
                            bg: color::BLOCKED_BG.into(),
                            bold: true,
                            glyph: base.base.glyph,
                        }
                    });
            }
        }
    }

    fn draw_effects(&mut self, cur_loc : &Location) {
        match self.mode {
            Mode::Normal|Mode::GoTo => self.draw_effects_normal(cur_loc),
            Mode::Examine => self.draw_effects_examine(cur_loc),
            Mode::Target(_) => self.draw_effects_target(cur_loc),
            _ => panic!("wrong mode"),
        }
    }

    pub fn draw_into(&self, window : &Window, calloc : &RefCell<color::Allocator>) {
        window.clear(calloc);

        let (max_x, max_y) = (self.base.width, self.base.height);
        for vx in 0..max_x {
            for vy in 0..max_y {
                let ch = if let Some(ch) = self.effects.at(vx, vy).choose(&mut thread_rng()) {
                    ch
                } else if let Some(ref ch) = *self.actors.at(vx, vy) {
                    ch
                } else if let Some(ref ch) = *self.items.at(vx, vy) {
                    ch
                } else {
                    &self.base.at(vx, vy).base
                };
                let cpair = nc::COLOR_PAIR(calloc.borrow_mut().get(ch.fg, ch.bg));
                if ch.glyph <= (127u8 as char) {
                    let c = (ch.glyph as nc::chtype) | cpair;
                    nc::mvwaddch(window.window, vy as i32, vx as i32, if ch.bold { c|nc::A_BOLD() } else { c });
                } else {
                    let attrflag = if ch.bold { cpair|nc::A_BOLD() } else { cpair };
                    nc::wattron(window.window, attrflag);
                    nc::mvwaddstr(window.window, vy as i32, vx as i32, &format!("{}", ch.glyph));
                    nc::wattroff(window.window, attrflag);
                }
            }
        }
        nc::wrefresh(window.window);
    }


    fn coord_to_glyph_xy(&mut self, coord : Coordinate) -> Option<(usize, usize)> {
        let (center_vx, center_vy) = self.coord_center.to_pixel_integer(SPACING);
        let (coord_vx, coord_vy) = coord.to_pixel_integer(SPACING);

        let (rel_vx, rel_vy) = (coord_vx - center_vx, coord_vy - center_vy);
        let (max_x, max_y) = (self.base.width as isize, self.base.height as isize);

        let mid_x = max_x as isize / 2;
        let mid_y = max_y as isize / 2;

        let vx = rel_vx as isize + mid_x;
        let vy = rel_vy as isize + mid_y;

        if vx >= 0 && vx < max_x &&
            vy >= 0 && vy < max_y {
                Some((vx as usize, vy as usize))
            } else {
                None
            }
    }

    fn for_each_glyph<F>(&mut self, cur_loc: &Location, f : F)
        where F : Fn(&mut MapRenderer, &Location, usize, usize, Coordinate, bool) {
            let (vpx, vpy) = self.coord_center.to_pixel_integer(SPACING);
            let (max_x, max_y) = (self.base.width as i32, self.base.height as i32);

            let mid_x = max_x / 2;
            let mid_y = max_y / 2;

            for vx in 0i32..max_x {
                for vy in 0i32..max_y {
                    let (rvx, rvy) = (vx - mid_x, vy - mid_y);

                    let (cvx, cvy) = (rvx + vpx, rvy + vpy);

                    let (c, off) = Coordinate::nearest_with_offset(SPACING, (cvx as i32, cvy as i32));

                    let is_proper_coord = off == (0, 0);

                    f(self, cur_loc, vx as usize, vy as usize, c, is_proper_coord)
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

pub fn race_to_char(race: Race) -> char {
    match race {
        Race::Human | Race::Elf | Race::Dwarf => '@',
        Race::Rat =>  'r',
        Race::Goblin => 'g',
        Race::Troll => 'T',
    }
}

pub fn color_by_visibility(color : [u8; 4], visible: bool, light : u32) -> Color {
    let fg = if !visible || light == 0 {
        if visible {
            color[2]
        } else {
            color[3]
        }
    } else if light < 3 {
        color[1]
    } else {
        color[0]
    };

    Color::from(fg)
}

// TODO: actor to palete, not race, and use is_player()
pub fn race_to_palete(race : Race) -> [u8; 4] {
    match race {
        Race::Human | Race::Elf | Race::Dwarf => color::CHAR_SELF_FG,
        _ => color::CHAR_ENEMY_FG,
    }
}
