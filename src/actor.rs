use std::collections::{HashSet,HashMap};
use hex2d::{Coordinate, Angle, Position, ToCoordinate};
use game::{self, Action};
use hex2dext::algo;
use std::cmp;
use util;
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
        max_hp: hp, max_mp: 10, mp: 10, hp: hp }
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
    Quick,
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

    pub attack_cooldown : i32,
    pub item_letters: HashSet<char>,
    pub equipped : HashMap<Slot, (char, Box<Item>)>,
    pub items : HashMap<char, Box<Item>>,

    pub were_hit : bool,
    pub did_hit : bool,
}

impl State {
    pub fn new(behavior : Behavior, pos : Position) -> State {
        let stats = Stats::new(
            if behavior == Behavior::Player { 10 } else { 5 }
            );

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
            item_letters: HashSet::new(),
            attack_cooldown: 0,
            were_hit: false,
            did_hit: false,
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
        self.did_hit = false;
        self.were_hit = false;
    }

    pub fn post_tick(&mut self, gstate : &game::State) {
        self.postprocess_visibile(gstate);
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }
    }

    pub fn add_item(&mut self, item : Box<Item>) -> bool {
        for ch in range('a' as u8, 'z' as u8)
            .chain(range('A' as u8, 'Z' as u8)) {
            let ch = ch as char;
            if !self.item_letter_taken(ch) {
                assert!(!self.items.contains_key(&ch));
                self.item_letters.insert(ch);
                self.items.insert(ch, item);
                return true;
            }
        }
        false
    }

    pub fn item_letter_taken(&self, ch : char) -> bool {
        if self.item_letters.contains(&ch) {
            return true;
        }

        for (&_, &(ref item_ch, _)) in &self.equipped {
            if *item_ch == ch {
                return true;
            }
        }

        false
    }

    pub fn equip_switch(&mut self, ch : char) {
        if self.items.contains_key(&ch) {
            self.equip(ch);
        } else {
            self.unequip(ch);
        }
    }

    pub fn equip(&mut self, ch : char) {
        if let Some(item) = self.items.remove(&ch) {
            let slot = item.slot();
            self.unequip_slot(slot);
            self.equipped.insert(slot, (ch, item));
        }
    }

    pub fn unequip_slot(&mut self, slot : Slot) {
        if let Some((ch, item)) = self.equipped.remove(&slot) {
            self.items.insert(ch, item);
        }
    }

    pub fn unequip(&mut self, ch : char) {
        let mut found_slot = None;
        for (&slot, &(ref item_ch, _)) in &self.equipped {
            if ch == *item_ch {
                found_slot = Some(slot);
                break;
            }
        }
        if let Some(slot) = found_slot {
            self.unequip_slot(slot);
        }
    }

    pub fn attacks(&mut self, target : Option<&mut State>) {
        let (dmg, to_hit, cooldown) = self.attack();
        self.attack_cooldown = cooldown + 1;

        if let Some(target) = target {
            let (ac, ev) = target.defense();

            let apower = self.stats.dex + to_hit;
            let dpower = target.stats.dex + ev;

            if util::roll(apower, dpower) {
                target.were_hit = true;
                self.did_hit = true;
                target.stats.hp -= cmp::max(0, dmg - ac);
            }
        }
    }

    pub fn defense(&self) -> (i32, i32) {
        self.equipped.get(&Slot::Body).and_then(|&(_, ref i)| i.defense()).unwrap_or((0, 0))
    }

    pub fn attack(&self) -> (i32, i32, i32) {
        self.equipped.get(&Slot::RHand).and_then(|&(_, ref i)| i.attack()).unwrap_or(self.hand_attack())
    }

    pub fn hand_attack(&self) -> (i32, i32, i32) {
        match self.behavior {
            Behavior::Grue => (2, 1, 0),
            Behavior::Player => (1, 0, 0),
            Behavior::Pony => (1, -1, 0),
        }
    }
    pub fn can_attack(&self) -> bool {
        self.attack_cooldown == 0
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
