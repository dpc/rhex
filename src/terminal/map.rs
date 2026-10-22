use std::collections::HashMap;
use std::time::Duration;

use super::Window;
use super::color::{self, Color};
use super::consts::*;
use super::ui::Mode;

use crate::game::actor::{self, Race};
use crate::game::{Location, item, tile};
use hex2d::{Coordinate, Direction, Position};

use ratatui::style::Modifier;

#[cfg(test)]
#[path = "awareness_tests.rs"]
mod awareness_tests;
#[cfg(test)]
#[path = "hit_tests.rs"]
mod hit_tests;
#[cfg(test)]
#[path = "sound_tests.rs"]
mod sound_tests;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct DisplayCharacter {
    glyph: char,
    bg: Color,
    fg: Color,
    bold: bool,
}

impl Default for DisplayCharacter {
    fn default() -> Self {
        DisplayCharacter {
            glyph: ' ',
            fg: color::MAP_BACKGROUND_BG.into(),
            bg: color::MAP_BACKGROUND_BG.into(),
            bold: false,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
struct ExtendedCharacter {
    base: DisplayCharacter,
    known: bool,
    visible: bool,
    light: u32,
}

#[derive(Clone)]
struct Array2d<T> {
    array: Vec<T>, // 2d array
    width: usize,
    height: usize,
}

impl<T> Array2d<T>
where
    T: Clone + Default,
{
    pub fn new(width: usize, height: usize) -> Self {
        Array2d {
            array: vec![Default::default(); width * height],
            width,
            height,
        }
    }

    pub fn reset(&mut self) {
        for ch in &mut self.array {
            *ch = Default::default();
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        *self = Array2d::new(width, height);
    }

    pub fn at(&self, vx: usize, vy: usize) -> &T {
        &self.array[vy * self.width + vx]
    }

    pub fn at_mut(&mut self, vx: usize, vy: usize) -> &mut T {
        &mut self.array[vy * self.width + vx]
    }
}

pub struct MapRenderer {
    /// Presentation-owned hit lifecycle; never consumes simulation randomness.
    pub(super) animations: super::animations::Animations,
    base: Array2d<ExtendedCharacter>,
    items: Array2d<Option<DisplayCharacter>>,
    actors: Array2d<Option<DisplayCharacter>>,
    effects: Array2d<Vec<DisplayCharacter>>,

    mode: Mode,
    coord_center: Coordinate,
    coord_head: Coordinate,

    dot: char,
}

impl MapRenderer {
    pub fn new(dot: char) -> Self {
        MapRenderer {
            animations: Default::default(),
            base: Array2d::new(0, 0),
            items: Array2d::new(0, 0),
            actors: Array2d::new(0, 0),
            effects: Array2d::new(0, 0),
            mode: Mode::Normal,
            coord_center: Coordinate::new(0, 0),
            coord_head: Coordinate::new(0, 0),

            dot,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.base.resize(width, height);
        self.items.resize(width, height);
        self.actors.resize(width, height);
        self.effects.resize(width, height);
    }

    pub fn update(
        &mut self,
        cur_loc: &Location,
        mode: Mode,
        target_pos: Option<Position>,
        anim_frame_count: u32,
        presentation_elapsed: Duration,
    ) {
        let player = cur_loc.player();

        self.mode = mode;

        let (center, head) = match mode {
            Mode::Examine => match target_pos {
                None => (player.pos.coord, player.pos.coord + player.pos.dir),
                Some(pos) => (pos.coord, pos.coord + pos.dir),
            },
            Mode::Target(_) => match target_pos {
                None => (player.pos.coord, player.pos.coord + player.pos.dir),
                Some(pos) => (player.pos.coord, pos.coord),
            },
            _ => (player.pos.coord, player.pos.coord + player.pos.dir),
        };

        self.coord_center = center;
        self.coord_head = head;

        if anim_frame_count == 0 {
            self.base.reset();
            self.items.reset();
            self.for_each_glyph(cur_loc, Self::draw_map_base_glyph);
            self.for_each_glyph(cur_loc, Self::draw_items);
        }
        // Facing colors animate even while the simulation and terrain are idle.
        self.actors.reset();
        self.draw_actors(cur_loc, presentation_elapsed);
        self.animations.advance(presentation_elapsed);
        let glyphs: Vec<_> = self.animations.glyphs(presentation_elapsed).collect();
        for (coord, glyph) in glyphs {
            if !player.sees(coord) {
                continue;
            }
            if let Some((vx, vy)) = self.coord_to_glyph_xy(coord) {
                let mut ch = self
                    .actors
                    .at(vx, vy)
                    .or(*self.items.at(vx, vy))
                    .unwrap_or(self.base.at(vx, vy).base);
                ch.glyph = glyph;
                *self.actors.at_mut(vx, vy) = Some(ch);
            }
        }

        let sounds: Vec<_> = self.animations.sound_cells(presentation_elapsed).collect();
        for (coord, marker) in sounds {
            if marker == player.sees(coord) {
                continue;
            }
            if let Some((vx, vy)) = self.coord_to_glyph_xy(coord) {
                let mut ch = if marker {
                    self.base.at(vx, vy).base
                } else {
                    self.actors
                        .at(vx, vy)
                        .or(*self.items.at(vx, vy))
                        .unwrap_or(self.base.at(vx, vy).base)
                };
                ch.bold = true;
                if marker {
                    ch.glyph = '!';
                    ch.fg = color::RED_FG.into();
                }
                *self.actors.at_mut(vx, vy) = Some(ch);
            }
        }

        // Awareness wins over facing, hits and sound bolding; targeting remains above it.
        let aware: Vec<_> = self.animations.aware_actors(presentation_elapsed).collect();
        for id in aware {
            let Some(enemy) = cur_loc.actors_byid.get(&id) else {
                continue;
            };
            if player.is_dead()
                || enemy.is_dead()
                || enemy.is_player()
                || !player.sees(enemy.coord())
            {
                continue;
            }
            if let Some((vx, vy)) = self.coord_to_glyph_xy(enemy.coord())
                && let Some(ch) = self.actors.at_mut(vx, vy)
            {
                ch.glyph = '!';
                ch.fg = color::YELLOW.into();
                ch.bold = true;
            }
        }

        self.effects.reset();
        self.draw_effects(cur_loc);
    }

    fn draw_map_base_glyph(
        &mut self,
        cur_loc: &Location,
        vx: usize,
        vy: usize,
        c: Coordinate,
        is_proper_coord: bool,
    ) {
        let player = cur_loc.player();

        let (visible, _in_los, knows, tt, t, light) = if is_proper_coord {
            let t = cur_loc.map[c];
            let tt = t.type_;

            let visible = player.sees(c) || player.is_dead();
            let light = if visible {
                cur_loc.at(c).light_as_seen_by(player)
            } else {
                0
            };

            (
                visible,
                player.in_los(c) || player.is_dead(),
                player.knows(c),
                Some(tt),
                Some(t),
                light,
            )
        } else {
            // Paint a glue characters between two real characters
            let c1 = c;
            let c2 = c + Direction::XY;

            let low_opaq1 = cur_loc.at(c1).tile().opaqueness() <= 1;
            let low_opaq2 = cur_loc.at(c2).tile().opaqueness() <= 1;

            let knows = (player.knows(c1) && player.knows(c2))
                || (player.knows(c1) && low_opaq1)
                || (player.knows(c2) && low_opaq2);

            let (e1, e2) = (
                cur_loc.at(c1).tile().ascii_expand(),
                cur_loc.at(c2).tile().ascii_expand(),
            );

            let expanded = if e1 > e2 { c1 } else { c2 };
            let c = Some(expanded);

            let tt = c.map(|c| cur_loc.at(c).tile().type_);

            let visible = player.is_dead()
                || (player.sees(c1) && player.sees(c2))
                || (player.sees(c1) && low_opaq1)
                || (player.sees(c2) && low_opaq2);

            let in_los = player.is_dead()
                || (player.in_los(c1) && player.in_los(c2))
                || (player.in_los(c1) && low_opaq1)
                || (player.in_los(c2) && low_opaq2);

            let light = if visible {
                let (light1, light2) = (
                    cur_loc.at(c1).light_as_seen_by(player),
                    cur_loc.at(c2).light_as_seen_by(player),
                );

                // Shoreline glue extends the ground, not the water. Averaging
                // in the water's light can turn a lit grass bank black.
                let shoreline = tt == Some(tile::Ground)
                    && matches!(
                        cur_loc
                            .at(if expanded == c1 { c2 } else { c1 })
                            .tile()
                            .type_,
                        tile::Water | tile::ShallowWater
                    );
                if shoreline && (player.sees(expanded) || player.is_dead()) {
                    cur_loc.at(expanded).light_as_seen_by(player)
                } else if player.is_dead() {
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
            Some(tile::Empty | tile::Ground) => {
                let mut fg = color::STONE_FG;
                let mut bg = color::EMPTY_BG;
                let mut glyph = ' ';
                if tt == Some(tile::Ground) {
                    fg = color::GROUND_FG;
                    bg = color::FOREST_BG;
                }

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
                        Some(tile::ForestPath) => glyph = '>',
                        Some(tile::Entrance) => glyph = '<',
                    }

                    if tt == Some(tile::Ground) {
                        fg = color::GROUND_FG;
                        bg = color::FOREST_BG;
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
            Some(tile::ShallowWater) => {
                // Keep commas on real shallows, but leave continuous water
                // between two shallow hexes free of extra terrain glyphs.
                let glyph = if !is_proper_coord
                    && cur_loc.at(c).tile().type_ == tile::ShallowWater
                    && cur_loc.at(c + Direction::XY).tile().type_ == tile::ShallowWater
                {
                    ' '
                } else {
                    ','
                };
                (color::WATER_FG, color::WATER_BG, glyph)
            }
            Some(tile::TreeTrunk) => {
                bold = true;
                (color::TRUNK_FG, color::FOREST_BG, WALL_CH)
            }
            Some(tile::Thicket) => (color::THICKET_FG, color::FOREST_BG, '%'),
            None => (color::EMPTY_FG, color::EMPTY_BG, '?'),
        };

        let fg = color_by_visibility(fg, visible, light);
        let bg = color_by_visibility(bg, visible, light);

        *self.base.at_mut(vx, vy) = ExtendedCharacter {
            base: DisplayCharacter {
                glyph,
                fg,
                bg,
                bold,
            },
            light,
            visible,
            known: knows,
        }
    }

    fn draw_items(
        &mut self,
        cur_loc: &Location,
        vx: usize,
        vy: usize,
        c: Coordinate,
        is_proper_coord: bool,
    ) {
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
                fg,
                bg: base.base.bg,
                bold: player.discovered.contains(&c),
                glyph: item_to_char(item.category()),
            });
        }
    }

    fn draw_actors(&mut self, cur_loc: &Location, presentation_elapsed: Duration) {
        let mut actors: Vec<(actor::Id, Position)> = cur_loc
            .actors_byid
            .iter()
            .filter(|&(_, a)| !a.is_dead() && cur_loc.player().sees(a.pos.coord))
            .map(|(id, a)| (*id, a.pos))
            .collect();

        let player = cur_loc.player();

        actors.sort_by_key(|(id, _)| *id);
        // Collect before painting: overlapping heads must not depend on draw order.
        let mut heads = HashMap::<Coordinate, (bool, Option<Color>)>::new();
        for &(id, _) in &actors {
            let actor = &cur_loc.actors_byid[&id];
            let race = actor.race;

            let actor_coord = actor.pos.coord;
            let head = heads.entry(actor.head()).or_default();
            if actor.is_player() {
                head.0 = true;
            } else {
                head.1 = Some(race_to_palete(race)[0].into());
            }

            if let Some((vx, vy)) = self.coord_to_glyph_xy(actor_coord) {
                let base = self.base.at(vx, vy);
                debug_assert!(base.known);
                let fg_palete = race_to_palete(race);
                let fg = color_by_visibility(fg_palete, base.visible, base.light);
                let bg = color_by_visibility(color::CHAR_BG, base.visible, base.light);
                *self.actors.at_mut(vx, vy) = Some(DisplayCharacter {
                    fg,
                    bg,
                    bold: base.base.bold,
                    glyph: race_to_char(race),
                });
            }
        }
        for (coord, (player_head, enemy_color)) in heads {
            let Some((vx, vy)) = self.coord_to_glyph_xy(coord) else {
                continue;
            };
            let shared = player_head && enemy_color.is_some();
            let occupied_color = if shared {
                if presentation_elapsed.as_secs().is_multiple_of(2) {
                    color::ENEMY_HEAD_ACTOR_FG
                } else {
                    color::SELF_HEAD_ACTOR_FG
                }
            } else if player_head {
                color::SELF_HEAD_ACTOR_FG
            } else {
                color::ENEMY_HEAD_ACTOR_FG
            };
            let base = self.base.at(vx, vy);
            let cell = self.actors.at_mut(vx, vy);
            if let Some(body) = cell {
                body.fg = occupied_color.into();
            } else {
                let fg = if shared {
                    occupied_color.into()
                } else if player_head {
                    race_to_palete(player.race)[0].into()
                } else {
                    enemy_color.unwrap()
                };
                if base.known {
                    // Do not expose a concealed occupant through its facing cue.
                    if !cur_loc.at(coord).is_occupied() {
                        *cell = Some(DisplayCharacter {
                            fg,
                            bg: base.base.bg,
                            bold: true,
                            glyph: self.items.at(vx, vy).unwrap_or(base.base).glyph,
                        });
                    }
                } else {
                    // Preserve the existing blank-cell marker for unknown space.
                    *cell = Some(DisplayCharacter {
                        fg: base.base.fg,
                        bg: fg,
                        bold: true,
                        glyph: ' ',
                    });
                }
            }
        }
    }

    fn draw_effects_examine(&mut self, _cur_loc: &Location) {
        let coord_center = self.coord_center;
        let coord_head = self.coord_head;
        if let Some((vx, vy)) = self.coord_to_glyph_xy(coord_center) {
            let base = self.base.at(vx, vy);
            self.effects.at_mut(vx, vy).push(DisplayCharacter {
                fg: Color::from(color::CHAR_GRAY_FG),
                bg: base.base.bg,
                bold: false,
                glyph: '@',
            });
        }

        if let Some((vx, vy)) = self.coord_to_glyph_xy(coord_head) {
            let base = self.base.at(vx, vy);
            self.effects.at_mut(vx, vy).push(DisplayCharacter {
                fg: Color::from(color::CHAR_GRAY_FG),
                bg: base.base.bg,
                bold: true,
                glyph: self.dot,
            });
        }
    }

    fn draw_effects_target(&mut self, cur_loc: &Location) {
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
                            fg: color_by_visibility(color::CHAR_SELF_FG, base.visible, base.light),
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
                    },
                );
            }
        }
    }

    fn draw_effects(&mut self, cur_loc: &Location) {
        match self.mode {
            Mode::Normal | Mode::GoTo => {}
            Mode::Examine => self.draw_effects_examine(cur_loc),
            Mode::Target(_) => self.draw_effects_target(cur_loc),
            _ => panic!("wrong mode"),
        }
    }

    pub fn draw_into(&mut self, window: &Window) {
        window.clear();

        let (max_x, max_y) = (self.base.width, self.base.height);
        for vx in 0..max_x {
            for vy in 0..max_y {
                let ch = match self.effects.at(vx, vy).last() {
                    Some(ch) => ch,
                    _ => {
                        if let Some(ref ch) = *self.actors.at(vx, vy) {
                            ch
                        } else if let Some(ref ch) = *self.items.at(vx, vy) {
                            ch
                        } else {
                            &self.base.at(vx, vy).base
                        }
                    }
                };
                let mut style = color::style(ch.fg, ch.bg);
                if ch.bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                window.put(vy as i32, vx as i32, ch.glyph, style);
            }
        }
    }

    fn coord_to_glyph_xy(&self, coord: Coordinate) -> Option<(usize, usize)> {
        let (center_vx, center_vy) = self.coord_center.to_pixel_integer(SPACING);
        let (coord_vx, coord_vy) = coord.to_pixel_integer(SPACING);

        let (rel_vx, rel_vy) = (coord_vx - center_vx, coord_vy - center_vy);
        let (max_x, max_y) = (self.base.width as isize, self.base.height as isize);

        let mid_x = max_x / 2;
        let mid_y = max_y / 2;

        let vx = rel_vx as isize + mid_x;
        let vy = rel_vy as isize + mid_y;

        if vx >= 0 && vx < max_x && vy >= 0 && vy < max_y {
            Some((vx as usize, vy as usize))
        } else {
            None
        }
    }

    fn for_each_glyph<F>(&mut self, cur_loc: &Location, f: F)
    where
        F: Fn(&mut MapRenderer, &Location, usize, usize, Coordinate, bool),
    {
        let (vpx, vpy) = self.coord_center.to_pixel_integer(SPACING);
        let (max_x, max_y) = (self.base.width as i32, self.base.height as i32);

        let mid_x = max_x / 2;
        let mid_y = max_y / 2;

        for vx in 0i32..max_x {
            for vy in 0i32..max_y {
                let (rvx, rvy) = (vx - mid_x, vy - mid_y);

                let (cvx, cvy) = (rvx + vpx, rvy + vpy);

                let (c, off) = Coordinate::nearest_with_offset(SPACING, (cvx, cvy));

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
        Race::Rat => 'r',
        Race::Goblin => 'g',
        Race::Troll => 'T',
    }
}

pub fn color_by_visibility(color: [u8; 4], visible: bool, light: u32) -> Color {
    let fg = if !visible || light == 0 {
        if visible { color[2] } else { color[3] }
    } else if light < 3 {
        color[1]
    } else {
        color[0]
    };

    Color::from(fg)
}

// TODO: actor to palete, not race, and use is_player()
pub fn race_to_palete(race: Race) -> [u8; 4] {
    match race {
        Race::Human | Race::Elf | Race::Dwarf => color::CHAR_SELF_FG,
        _ => color::CHAR_ENEMY_FG,
    }
}

#[cfg(test)]
#[path = "map_tests.rs"]
mod tests;
