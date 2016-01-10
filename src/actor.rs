use std::collections::{HashMap, HashSet};
use std::ops::{Add, Sub};
use std::cmp;

use hex2d::{Coordinate, Angle, Position, ToCoordinate, Direction};
use hex2dext::algo;

use game::{self, Action};
use game::tile::{Feature};
use util;
use item::Item;

pub type Visibility = HashSet<Coordinate>;
pub type NoiseMap = HashMap<Coordinate, Noise>;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Noise {
    Creature(Race),
}

impl Noise {
    pub fn description(&self) -> String {
        match *self {
            Noise::Creature(cr) => cr.description(),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Race {
    Human,
    Pony,
    Grue,
}

impl Race {
    pub fn description(&self) -> String {
        match *self {
            Race::Grue => "Grue",
            Race::Pony => "Pony",
            Race::Human => "Human",
        }.to_string()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Stats {
    pub int : i32,
    pub dex : i32,
    pub str_ : i32,
    pub max_hp : i32,
    pub max_mp : i32,
    pub max_sp : i32,
    pub ac: i32,
    pub ev: i32,
    pub melee_dmg: i32,
    pub melee_acc: i32,
    pub melee_cd: i32, // attack cooldown
}

impl Stats {
    pub fn new(race : Race) -> Stats {
        let mut s = Stats {
            int: 2, dex : 2, str_ : 2,
            max_hp: 10, max_mp: 5, max_sp: 5,
            ac: 0, ev: 0,
            melee_cd: 0, melee_dmg: 1, melee_acc: 1,
        };

        match race {
            Race::Grue => {
                s.int = 1; s.dex = 1; s.str_ = 1; s.max_hp = 5;
            },
            _ => {}
        }

        s
    }

    pub fn zero() -> Stats {
        Stats {
            int: 0, dex: 0, str_: 0,
            max_hp: 0, max_mp: 0, max_sp: 0,
            ac: 0, ev: 0,
            melee_cd: 0, melee_dmg: 0, melee_acc: 0,
        }
    }
}

impl Add for Stats {
    type Output = Stats;

    fn add(self, s : Stats) -> Stats {
        Stats {
            int: self.int + s.int,
            dex: self.dex + s.dex,
            str_: self.str_ + s.str_,
            max_hp: self.max_hp + s.max_hp,
            max_mp: self.max_mp + s.max_mp,
            max_sp:  self.max_sp + s.max_sp,
            ac: self.ac + s.ac,
            ev: self.ev + s.ev,
            melee_cd: self.melee_cd + s.melee_cd,
            melee_dmg: self.melee_dmg + s.melee_dmg,
            melee_acc: self.melee_acc + s.melee_acc,
        }
    }
}

impl Sub for Stats {
    type Output = Stats;

    fn sub (self, s : Stats) -> Stats {
        Stats {
            int: self.int - s.int,
            dex: self.dex - s.dex,
            str_: self.str_ - s.str_,
            max_hp: self.max_hp - s.max_hp,
            max_mp: self.max_mp - s.max_mp,
            max_sp:  self.max_sp - s.max_sp,
            ac: self.ac - s.ac,
            ev: self.ev - s.ev,
            melee_cd: self.melee_cd - s.melee_cd,
            melee_dmg: self.melee_dmg - s.melee_dmg,
            melee_acc: self.melee_acc - s.melee_acc,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Ord, PartialOrd)]
pub enum Slot {
    Head,
    Feet,
    LHand,
    RHand,
    Body,
    Cloak,
    Quick,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct AttackResult {
    pub success : bool,
    pub dmg : i32,
    pub who : String,
    pub behind : bool,
}

#[derive(Clone, Debug)]
pub struct State {

    pub hp: i32,
    pub mp: i32,
    pub sp: i32,
    pub prev_hp: i32,
    pub prev_mp: i32,
    pub prev_sp: i32,

    pub player : bool,
    pub pre_pos : Position,
    pub pos : Position,

    pub race : Race,
    pub base_stats : Stats,
    pub mod_stats : Stats,
    pub stats : Stats,

    /// LoS at the end of the tick
    pub in_los: Visibility,

    /// Additional LoS that actor could
    /// experience during the tick (eg. when
    /// more than one tile was traversed)
    temporary_los: Visibility,

    /// Currently visible: los + light
    pub visible: Visibility,

    /// Known coordinates
    pub known: Visibility,
    /// Known areas
    pub known_areas: Visibility,

    /// Discovered in the last LoS
    pub discovered: Visibility,
    /// Just discovered areas
    pub discovered_areas: Visibility,

    pub heared: NoiseMap,
    pub noise_emision: i32,

    pub light_emision : u32,

    pub melee_cd : i32,
    pub action_cd : i32,

    pub items_letters: HashSet<char>,
    pub items_equipped : HashMap<Slot, (char, Box<Item>)>,
    pub items_backpack : HashMap<char, Box<Item>>,

    pub was_attacked_by : Vec<AttackResult>,
    pub did_attack : Vec<AttackResult>,
}

impl State {
    pub fn new(race : Race, pos : Position) -> State {
        let stats = Stats::new(race);

        State {
            race: race,
            player: false,
            pos: pos, pre_pos: pos,
            base_stats: stats,        // base stats
            mod_stats: Stats::zero(), // from items etc.
            stats: Stats::zero(),     // effective stats
            in_los: Default::default(),
            temporary_los: Default::default(),
            visible: Default::default(),
            known: Default::default(),
            known_areas: Default::default(),
            heared: HashMap::new(),
            noise_emision: 0,
            discovered: Default::default(),
            discovered_areas: Default::default(),
            light_emision: 0,
            items_backpack: HashMap::new(),
            items_equipped: HashMap::new(),
            items_letters: Default::default(),
            melee_cd: 0,
            action_cd: 0,
            was_attacked_by: Vec::new(),
            did_attack: Vec::new(),
            hp: stats.max_hp,
            mp: stats.max_mp,
            sp: stats.max_sp,
            prev_hp: stats.max_hp,
            prev_mp: stats.max_mp,
            prev_sp: stats.max_sp,
        }
    }

    // TODO: Remove this, make more general etc.
    pub fn new_grue(level : i32, pos : Position) -> State {
        let mut ret = State::new(Race::Grue, pos);

        ret.base_stats.int = 1;
        ret.base_stats.dex = 1;
        ret.base_stats.str_ = 1;

        ret.base_stats.max_hp += level / 2;

        ret.base_stats.dex += level / 2;
        ret.base_stats.str_ += (2 + level) / 4;

        ret.base_stats.ev += (1 + level) / 5;
        ret.base_stats.ac += (2 + level) / 6;
        ret

    }

    pub fn sees(&self, pos : Coordinate) -> bool {
        self.visible.contains(&pos)
    }

    pub fn in_los(&self, pos : Coordinate) -> bool {
        self.in_los.contains(&pos)
    }

    pub fn knows(&self, pos : Coordinate) -> bool {
        self.known.contains(&pos)
    }

    pub fn hears(&self, coord : Coordinate) -> bool {
        self.heared.contains_key(&coord)
    }

    pub fn coord(&self) -> Coordinate {
        self.pos.coord
    }

    /// "head" - The coordinate that is in front of an actor
    pub fn head(&self) -> Coordinate {
        self.pos.coord + self.pos.dir.to_coordinate()
    }

    pub fn pos_after_action(&self, action : Action) -> Vec<Position> {
        let pos = self.pos;
        match action {
            Action::Wait|Action::Pick|Action::Equip(_)|Action::Descend|Action::Fire(_) => vec!(pos),
            Action::Turn(a) => vec!(pos + a),
            Action::Move(a) => vec!(pos + (pos.dir + a).to_coordinate()),
            Action::Charge => vec!(pos + pos.dir.to_coordinate(), pos + pos.dir.to_coordinate() + pos.dir.to_coordinate()),
            Action::Spin(a) => vec!(pos + (pos.dir + a).to_coordinate() +
                match a {
                    Angle::Right => Angle::Left,
                    Angle::Left => Angle::Right,
                    _ => return vec!(pos),
                }
            ),
        }
    }

    fn los_to_visible(&self, gstate : &game::State, los : &Visibility ) -> Visibility {
        let mut visible : Visibility = Default::default();

        for &coord in los {
            if gstate.light_map[coord] > 0 {
                visible.insert(coord);
            } else if self.pos.coord.distance(coord) < 2 {
                visible.insert(coord);
            } else if gstate.at(coord).tile().opaqueness() > 10 {
                if gstate.at(coord).light_as_seen_by(self) > 0 {
                    visible.insert(coord);
                }
            }
        }

        visible
    }

    fn postprocess_visibile(&mut self, gstate : &game::State) {
        let total_los = self.temporary_los.clone();
        let total_visible = self.los_to_visible(gstate, &total_los);

        self.temporary_los = Default::default();
        self.add_current_los_to_temporary_los(gstate);

        let visible = self.los_to_visible(gstate, &self.temporary_los);

        let mut discovered : HashSet<_> = Default::default();
        let mut discovered_areas : HashSet<_> = Default::default();

        for i in &total_visible {
            if !self.known.contains(i) {
                self.known.insert(*i);
                discovered.insert(*i);
            }
        }

        for &coord in &discovered {
            if let Some(area) = gstate.at(coord).tile().area {
                let area_center = area.center;

                if !self.known_areas.contains(&area_center) {
                    self.known_areas.insert(area_center);
                    discovered_areas.insert(area_center);
                }
            }
        }

        self.in_los = self.temporary_los.clone();
        self.visible = visible;
        self.discovered_areas = discovered_areas;
        self.discovered = discovered;
    }

    pub fn pre_tick(&mut self, _ : &game::State) {
        self.pre_pos = self.pos;
        self.prev_hp = self.hp;
        self.prev_mp = self.mp;
        self.prev_sp = self.sp;
        self.did_attack = Vec::new();
        self.was_attacked_by = Vec::new();
        self.temporary_los = Default::default();

        self.noise_emision = 0;
        self.heared = HashMap::new();
    }

    pub fn noise_makes(&mut self, noise : i32) {
        if self.noise_emision < noise {
            self.noise_emision = noise;
        }
    }

    pub fn noise_hears(&mut self, coord : Coordinate, type_ : Noise) {
        self.heared.insert(coord, type_);
    }

    pub fn post_tick(&mut self, gstate : &game::State) {
        self.postprocess_visibile(gstate);
        if self.melee_cd > 0 {
            self.melee_cd -= 1;
        }

        if self.action_cd > 0 {
            self.action_cd -= 1;
        }

        self.recalculate_stats();
    }

    pub fn add_item(&mut self, item : Box<Item>) -> bool {
        for ch in ('a' as u8..'z' as u8)
            .chain('A' as u8..'Z' as u8) {
            let ch = ch as char;
            if !self.item_letter_taken(ch) {
                assert!(!self.items_backpack.contains_key(&ch));
                self.items_letters.insert(ch);
                self.items_backpack.insert(ch, item);
                return true;
            }
        }
        false
    }

    pub fn item_letter_taken(&self, ch : char) -> bool {
        if self.items_letters.contains(&ch) {
            return true;
        }

        for (&_, &(ref item_ch, _)) in &self.items_equipped {
            if *item_ch == ch {
                return true;
            }
        }

        false
    }

    pub fn equip_switch(&mut self, ch : char) {
        if self.items_backpack.contains_key(&ch) {
            if let Some(item) = self.items_backpack.remove(&ch) {
                if item.is_usable() {
                    if !item.use_(self) {
                        self.items_backpack.insert(ch, item);
                    }
                    self.action_cd += 2;
                } else {
                    self.equip(item, ch);
                }
            }

        } else {
            self.unequip(ch);
        }
    }

    pub fn equip(&mut self, item : Box<Item>, ch : char) {
        if let Some(slot) = item.slot() {
            self.unequip_slot(slot);
            self.mod_stats = self.mod_stats + item.stats();
            self.items_equipped.insert(slot, (ch, item));
            self.action_cd += if slot == Slot::Body {
                4
            } else {
                2
            }
        } else {
            self.items_backpack.insert(ch, item);
        }
    }

    fn add_current_los_to_temporary_los(&mut self, gstate : &game::State) {
        let pos = self.pos;
        algo::los2::los(
            &|coord| gstate.at(coord).tile().opaqueness(),
            &mut |coord, _ | {
                let _ = self.temporary_los.insert(coord);
            },
            10, pos.coord,
            &[pos.dir]
            );
    }

    pub fn unequip_slot(&mut self, slot : Slot) {
        if let Some((ch, item)) = self.items_equipped.remove(&slot) {
            self.mod_stats = self.mod_stats - item.stats();
            self.items_backpack.insert(ch, item);
            self.action_cd += if slot == Slot::Body {
                4
            } else {
                2
            }
        }
    }

    pub fn unequip(&mut self, ch : char) {
        let mut found_slot = None;
        for (&slot, &(ref item_ch, _)) in &self.items_equipped {
            if ch == *item_ch {
                found_slot = Some(slot);
                break;
            }
        }
        if let Some(slot) = found_slot {
            self.unequip_slot(slot);
        }
    }

    pub fn recalculate_stats(&mut self) {
        self.stats = self.base_stats + self.mod_stats;

        // Add attributes to derived stats
        self.stats.melee_dmg += (self.stats.str_ + 1) / 2;
        self.stats.ac += self.stats.str_ / 2;
        self.stats.melee_acc += (self.stats.dex + 1) / 2;
        self.stats.ev += self.stats.dex / 2;
        self.stats.max_sp += self.stats.str_;
        self.stats.max_mp += self.stats.int;
    }

    pub fn attacks(&mut self, dir : Direction, target : &mut State) {
        self.melee_cd = self.stats.melee_cd + 1;

        let mut acc = self.stats.melee_acc;
        let mut dmg = self.stats.melee_dmg;

        let (ac, ev) = (target.stats.ac, target.stats.ev);

        let from_behind = match dir - target.pre_pos.dir {
            Angle::Forward|Angle::Left|Angle::Right => true,
            _ => false,
        };

        if from_behind {
            acc *= 2;
            dmg *= 2;
        }

        let success = util::roll(acc, ev);

        let dmg = cmp::max(0, dmg - ac);

        if success {
            target.hp -= dmg;
            target.noise_makes(7);
        }

        target.was_attacked_by.push(AttackResult {
            success: success,
            dmg: dmg,
            who: self.description(),
            behind: from_behind,
        });

        self.did_attack.push(AttackResult {
            success: success,
            dmg: dmg,
            who: target.description(),
            behind: from_behind,
        });
    }

    pub fn discovered_stairs(&self, gstate : &game::State) -> bool {
        self.discovered.iter().any(
            |c| gstate.at(*c).tile().feature == Some(Feature::Stairs)
            )
    }

    pub fn set_player(&mut self) {
        self.player = true;
    }

    pub fn can_attack(&self) -> bool {
        self.melee_cd == 0 && self.action_cd == 0
    }

    pub fn moved(&mut self, gstate : &game::State, new_pos : Position) {
        self.pos = new_pos;
        self.add_current_los_to_temporary_los(gstate);
        self.noise_makes(2);
    }

    pub fn changed_level(&mut self) {
        self.known = Default::default();
        self.known_areas = Default::default();
    }

    pub fn is_player(&self) -> bool {
        self.player
    }

    pub fn is_dead(&self) -> bool {
        self.hp <= 0
    }

    pub fn can_perform_action(&self) -> bool {
        !self.is_dead() && self.action_cd == 0
    }

    pub fn description(&self) -> String {
        self.race.description()

    }
}


