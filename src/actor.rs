use std::collections::{HashSet,HashMap};
use hex2d::{Coordinate, Angle, Position, ToCoordinate};
use game::{self, Action};
use hex2dext::algo;

use item::Item;

type Visibility = HashSet<Coordinate>;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Behavior {
    Player,
    Pony,
    Grue,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Stats {
    pub int : i32,
    pub dex : i32,
    pub str_ : i32,
    pub max_hp : i32,
    pub max_mp : i32,
    pub hp: i32,
    pub mp: i32,
}

impl Stats {
    pub fn new(hp : i32) -> Stats {
        Stats { int: 3, dex : 3, str_ : 3,
        max_hp: 3, max_mp: 3, mp: 3, hp: hp }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Slot {
    Head,
    Feet,
    LHand,
    RHand,
    Body,
    Cloak,
}

#[derive(Clone, Debug)]
pub struct State {
    pub pos : Position,

    pub behavior : Behavior,
    pub stats : Stats,
    pub prev_stats : Stats,

    /// Currently visible
    pub visible: Visibility,

    /// Known coordinates
    pub known: Visibility,
    /// Known areas
    pub known_areas: Visibility,

    /// Discovered in the last LoS
    pub discovered: Visibility,
    /// Just discovered areas
    pub discovered_areas: Visibility,

    pub light : u32,

    pub equipped : HashMap<Slot, (char, Box<Item>)>,
    pub items : HashMap<char, Box<Item>>,
}

impl State {
    pub fn new(behavior : Behavior, pos : Position) -> State {
        let stats = Stats::new(if behavior == Behavior::Player { 3 } else { 1 });
        State {
            behavior : behavior,
            pos: pos,
            stats: stats,
            prev_stats: stats,
            visible: HashSet::new(),
            known: HashSet::new(),
            known_areas: HashSet::new(),
            discovered: HashSet::new(),
            discovered_areas: HashSet::new(),
            light: 0,
            items: HashMap::new(),
            equipped: HashMap::new(),
        }
    }

    pub fn add_light(&mut self, light : u32) {
        self.light = light;
    }

    pub fn sees(&self, pos : Coordinate) -> bool {
        self.visible.contains(&pos)
    }

    pub fn knows(&self, pos : Coordinate) -> bool {
        self.known.contains(&pos)
    }

    pub fn pos_after_action(&self, action : Action) -> Position {
        let pos = self.pos;
        match action {
            Action::Wait|Action::Pick|Action::Equip(_) => pos,
            Action::Turn(a) => pos + a,
            Action::Move(a) => pos + (pos.dir + a).to_coordinate(),
            Action::Spin(a) => pos + (pos.dir + a).to_coordinate() +
                match a {
                    Angle::Right => Angle::Left,
                    Angle::Left => Angle::Right,
                    _ => return pos,
                },
        }
    }

    fn postprocess_visibile(&mut self, gstate : &game::State) {

        let visible = calculate_los(self.pos, gstate);

        let mut discovered = HashSet::new();
        let mut discovered_areas = HashSet::new();

        for i in &visible {
            if !self.known.contains(i) {
                self.known.insert(*i);
                discovered.insert(*i);
            }
        }

        for &coord in &discovered {
            if let Some(area) = gstate.at(coord).tile().and_then(|t| t.area) {
                let area_center = area.center;

                if !self.known_areas.contains(&area_center) {
                    self.known_areas.insert(area_center);
                    discovered_areas.insert(area_center);
                }
            }
        }

        self.visible = visible;
        self.discovered_areas = discovered_areas;
        self.discovered = discovered;
    }

    pub fn pre_tick(&mut self, _ : &game::State) {
        self.prev_stats = self.stats;
    }

    pub fn post_tick(&mut self, gstate : &game::State) {
        self.postprocess_visibile(gstate);
    }

    pub fn add_item(&mut self, item : Box<Item>) -> bool {
        for ch in range('a' as u8, 'z' as u8).chain(range('A' as u8, 'Z' as u8)) {
            let ch = ch as char;
            if !self.items.contains_key(&ch) {
                self.items.insert(ch, item);
                return true;
            }
        }
        false
    }

    pub fn equip(&mut self, ch : char, slot : Slot) {
        if let Some(item) = self.items.remove(&ch) {
            self.unequip(slot);
            self.equipped.insert(slot, (ch, item));
        }
    }

    pub fn unequip(&mut self, slot : Slot) {
        if let Some((ch, item)) = self.equipped.remove(&slot) {
            self.items.insert(ch, item);
        }
    }

    pub fn hit(&self) -> State {
        let mut state = self.clone();

        state.stats.hp -= 1;

        return state;
    }

    pub fn change_position(&mut self, new_pos : Position) {
        self.pos = new_pos;
    }

    pub fn is_player(&self) -> bool {
        self.behavior == Behavior::Player
    }

    pub fn is_dead(&self) -> bool {
        self.stats.hp <= 0
    }
}

fn calculate_los(pos : Position, gstate : &game::State) -> Visibility {
    let mut visibility = HashSet::new();
    algo::los::los(
        &|coord| gstate.at(coord).tile_map_or(10000, |tile| tile.opaqueness()),
        &mut |coord, _ | {
            if pos.coord.distance(coord) < 2 || gstate.light_map.contains_key(&coord) {
                let _ = visibility.insert(coord);
            }
        },
        10, pos.coord, &[pos.dir, pos.dir + Angle::Left, pos.dir + Angle::Right]
        );

    visibility
}
