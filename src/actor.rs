use std::collections::{HashMap, HashSet};
use std::ops::{Add, Sub};
use std::cmp;

use hex2d::{Coordinate, Angle, Position, ToCoordinate, Direction};
use hex2dext::algo;

use game::{self, Action};
use game::tile::{Feature};
use util;
use item::Item;

use self::Race::*;
use race::*;
use std::iter::Iterator;

use rand;
use rand::Rng;

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
    Elf,
    Dwarf,
    Rat,
    Goblin,
}

impl Race {
    pub fn description(&self) -> String {
        match *self {
            Race::Human => "Human",
            Race::Elf => "Elf",
            Race::Dwarf => "Dwarf",
            Race::Rat => "Rat",
            Race::Goblin => "Goblin",
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
    pub infravision : i32,
    pub vision : i32,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct EffectiveStats {
    pub base : Stats,
    pub melee_dmg: i32,
    pub melee_acc: i32,
    pub melee_str_req: i32,
}

impl Stats {
    pub fn new(race : Race) -> Stats {
        match race {
            Goblin => GOBLIN_STATS,
            Rat => RAT_STATS,
            Elf => ELF_STATS,
            Human => HUMAN_STATS,
            Dwarf => DWARF_STATS,
        }
    }

    pub fn to_effective(&self) -> EffectiveStats {
        let mut efs = EffectiveStats::default();
        efs.base = *self;
        efs
    }
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            int: 0, dex: 0, str_: 0,
            max_hp: 0, max_mp: 0, max_sp: 0,
            ac: 0, ev: 0,
            infravision: 0, vision: 0,
        }
    }
}

impl Default for EffectiveStats {
    fn default() -> Self {
        EffectiveStats {
            base: Default::default(),
            melee_dmg: 0,
            melee_acc: 0,
            melee_str_req: 0,
        }
    }
}

impl Add for Stats {
    type Output = Stats;

    fn add(self, s : Self) -> Self {
        Stats {
            int: self.int + s.int,
            dex: self.dex + s.dex,
            str_: self.str_ + s.str_,
            max_hp: self.max_hp + s.max_hp,
            max_mp: self.max_mp + s.max_mp,
            max_sp:  self.max_sp + s.max_sp,
            ac: self.ac + s.ac,
            ev: self.ev + s.ev,
            infravision: self.infravision + s.infravision,
            vision : self.vision + s.vision,
        }
    }
}

impl Add for EffectiveStats {
    type Output = EffectiveStats;

    fn add(self, s : Self) -> Self {
        EffectiveStats {
            base: self.base + s.base,
            melee_dmg: self.melee_dmg + s.melee_dmg,
            melee_acc: self.melee_acc + s.melee_acc,
            melee_str_req: self.melee_str_req + s.melee_str_req,
        }
    }
}

impl Sub for Stats {
    type Output = Stats;

    fn sub(self, s : Self) -> Self {
        Stats {
            int: self.int - s.int,
            dex: self.dex - s.dex,
            str_: self.str_ - s.str_,
            max_hp: self.max_hp - s.max_hp,
            max_mp: self.max_mp - s.max_mp,
            max_sp:  self.max_sp - s.max_sp,
            ac: self.ac - s.ac,
            ev: self.ev - s.ev,
            infravision: self.infravision - s.infravision,
            vision: self.vision - s.vision,
        }
    }
}

impl Sub for EffectiveStats {
    type Output = EffectiveStats;

    fn sub(self, s : Self) -> Self {
        EffectiveStats{
            base : self.base - s.base,
            melee_dmg: self.melee_dmg - s.melee_dmg,
            melee_acc: self.melee_acc - s.melee_acc,
            melee_str_req: self.melee_str_req- s.melee_str_req,
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
    pub saved_hp: i32,
    pub saved_mp: i32,
    pub saved_sp: i32,

    pub player : bool,
    pub pre_pos : Position,
    pub pos : Position,

    pub race : Race,
    pub base_stats : Stats,
    pub mod_stats : EffectiveStats,
    pub stats : EffectiveStats,

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

    pub heard: NoiseMap,
    pub noise_emision: i32,

    pub light_emision : u32,

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
            mod_stats: Default::default(), // from items etc.
            stats: Default::default(),     // effective stats
            in_los: Default::default(),
            temporary_los: Default::default(),
            visible: Default::default(),
            known: Default::default(),
            known_areas: Default::default(),
            heard: HashMap::new(),
            noise_emision: 0,
            discovered: Default::default(),
            discovered_areas: Default::default(),
            light_emision: 0,
            items_backpack: HashMap::new(),
            items_equipped: HashMap::new(),
            items_letters: Default::default(),
            action_cd: 0,
            was_attacked_by: Vec::new(),
            did_attack: Vec::new(),
            hp: stats.max_hp,
            mp: stats.max_mp,
            sp: stats.max_sp,
            saved_hp: stats.max_hp,
            saved_mp: stats.max_mp,
            saved_sp: stats.max_sp,
        }
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
        self.heard.contains_key(&coord)
    }

    pub fn coord(&self) -> Coordinate {
        self.pos.coord
    }

    /// "head" - The coordinate that is in front of an actor
    pub fn head(&self) -> Coordinate {
        self.pos.coord + self.pos.dir
    }

    pub fn pos_after_action(&self, action : Action) -> Vec<Position> {
        let pos = self.pos;
        match action {
            Action::Wait|Action::Pick|Action::Equip(_)|Action::Descend|Action::Fire(_) => vec!(pos),
            Action::Turn(a) => vec!(pos + a),
            Action::Move(a) => vec!(pos + (pos.dir + a).to_coordinate()),
            Action::Charge => if self.can_charge_sp() {
                vec!(pos + pos.dir.to_coordinate(),
                pos + pos.dir.to_coordinate() + pos.dir.to_coordinate())
            } else {
                vec!(pos + pos.dir.to_coordinate())
            },
            Action::Spin(a) => vec!(pos + (pos.dir + a).to_coordinate() +
                match a {
                    Angle::Right => Angle::Left,
                    Angle::Left => Angle::Right,
                    _ => return vec!(pos),
                }
            ),
        }
    }

    pub fn post_action(&mut self, action : Action) {
        match action {
            Action::Wait => {
                self.sp = cmp::min(self.stats.base.max_sp, self.sp + 1);
            },
            Action::Charge => {
                self.sp = cmp::max(0, self.sp - self.charge_sp_cost());
            },
            _ => {}
        }
    }

    fn los_to_visible(&self, gstate : &game::State, los : &Visibility ) -> Visibility {
        let mut visible : Visibility = Default::default();

        for &coord in los {
            if gstate.light_map[coord] > 0 {
                visible.insert(coord);
            } else if self.pos.coord.distance(coord) <= self.stats.base.infravision {
                visible.insert(coord);
            } else if coord == self.head() {
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

        for i in total_visible.iter().chain(visible.iter()) {
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

    // Save some stats for a reference
    pub fn save_stats(&mut self) {
        self.saved_hp = self.hp;
        self.saved_mp = self.mp;
        self.saved_sp = self.sp;
    }


    pub fn pre_tick(&mut self, _ : &game::State) {
        self.pre_pos = self.pos;
        self.did_attack = Vec::new();
        self.was_attacked_by = Vec::new();
        self.temporary_los = Default::default();

        if self.can_perform_action() {
            self.save_stats();
        }

        self.noise_emision = 0;
        self.heard = HashMap::new();
    }

    pub fn noise_makes(&mut self, noise : i32) {
        if self.noise_emision < noise {
            self.noise_emision = noise;
        }
    }

    pub fn noise_hears(&mut self, coord : Coordinate, type_ : Noise) {
        self.heard.insert(coord, type_);
    }

    pub fn post_tick(&mut self, gstate : &game::State) {
        self.postprocess_visibile(gstate);

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
        let vision = self.stats.base.vision;
        algo::los2::los(
            &|coord| gstate.at(coord).tile().opaqueness(),
            &mut |coord, _ | {
                let _ = self.temporary_los.insert(coord);
            },
            vision, pos.coord,
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
        self.stats = self.base_stats.to_effective() + self.mod_stats;

        // Add attributes to derived stats
        self.stats.melee_dmg += self.stats.base.str_;
        self.stats.melee_acc += self.stats.base.dex;
        self.stats.base.ac += self.stats.base.str_ / 2;
        self.stats.base.ev += self.stats.base.dex / 2;
        self.stats.base.max_sp += self.stats.base.str_ * 2;
        self.stats.base.max_mp += self.stats.base.int * 2;
    }

    pub fn attacks(&mut self, dir : Direction, target : &mut State) {
        let mut acc = self.stats.melee_acc;
        let mut dmg = self.stats.melee_dmg;

        let (ac, ev) = (target.stats.base.ac, target.stats.base.ev);

        let from_behind = match dir - target.pre_pos.dir {
            Angle::Forward|Angle::Left|Angle::Right => true,
            _ => false,
        };

        if from_behind {
            acc *= 2;
            dmg *= 2;
        }

        if !self.can_attack_sp() {
            acc /= 2;
            dmg /= 2;
            self.sp = 0;
        } else {
            self.sp -= self.melee_sp_cost();
        }

        let success = util::roll(acc, ev);

        let rand_ac = cmp::max(
            rand::thread_rng().gen_range(0, ac + 1),
            rand::thread_rng().gen_range(0, ac + 1),
            );

        let dmg = cmp::max(0, dmg - rand_ac);

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

    pub fn melee_sp_cost(&self) -> i32 {
        cmp::max(0, self.stats.melee_str_req - self.stats.base.str_)
    }

    pub fn charge_sp_cost(&self) -> i32 {
        cmp::max(0, 10 - self.stats.base.str_)
    }

    // Can attack considering only sp?
    pub fn can_attack_sp(&self) -> bool {
        self.sp > self.melee_sp_cost()
    }

    // Can attack considering only sp?
    pub fn can_charge_sp(&self) -> bool {
        self.sp >= self.charge_sp_cost()
    }

    pub fn can_attack(&self) -> bool {
        self.action_cd == 0 &&  self.can_attack_sp()
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


