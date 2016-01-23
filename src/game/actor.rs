use std::collections::{HashMap, HashSet};
use std::ops::{Add, Sub};
use std::cmp;

use hex2d::{Coordinate, Angle, Position, ToCoordinate, Direction};
use hex2dext::algo;

use game::{self, Action, tile};
use game::tile::Feature;
use util;
use super::item::Item;

use self::Race::*;
use std::iter::Iterator;

use rand;
use rand::Rng;

use super::conts::*;
use super::{Visibility, NoiseMap};

use super::{Location, Noise};

pub type Id = u32;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Race {
    Human,
    Elf,
    Dwarf,
    Rat,
    Goblin,
    Troll,
}

impl Race {
    pub fn description(&self) -> String {
        match *self {
            Race::Human => "human",
            Race::Elf => "elf",
            Race::Dwarf => "dwarf",
            Race::Rat => "rat",
            Race::Goblin => "goblin",
            Race::Troll => "troll",
        }
        .to_string()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Stats {
    pub int: i32,
    pub dex: i32,
    pub str_: i32,
    pub max_hp: i32,
    pub max_mp: i32,
    pub max_sp: i32,
    pub ac: i32,
    pub ev: i32,
    pub infravision: i32,
    pub vision: i32,
    pub regeneration: u32,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct EffectiveStats {
    pub base: Stats,
    pub melee_dmg: i32,
    pub melee_acc: i32,
    pub melee_str_req: i32,
    pub light_emision: u32,
}

impl Stats {
    pub fn new(race: Race) -> Stats {
        match race {
            Goblin => GOBLIN_STATS,
            Rat => RAT_STATS,
            Elf => ELF_STATS,
            Human => HUMAN_STATS,
            Dwarf => DWARF_STATS,
            Troll => TROLL_STATS,
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
            int: 0,
            dex: 0,
            str_: 0,
            max_hp: 0,
            max_mp: 0,
            max_sp: 0,
            ac: 0,
            ev: 0,
            infravision: 0,
            vision: 0,
            regeneration: 0,
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
            light_emision: 0,
        }
    }
}

impl Add for Stats {
    type Output = Stats;

    fn add(self, s: Self) -> Self {
        Stats {
            int: self.int + s.int,
            dex: self.dex + s.dex,
            str_: self.str_ + s.str_,
            max_hp: self.max_hp + s.max_hp,
            max_mp: self.max_mp + s.max_mp,
            max_sp: self.max_sp + s.max_sp,
            ac: self.ac + s.ac,
            ev: self.ev + s.ev,
            infravision: self.infravision + s.infravision,
            vision: self.vision + s.vision,
            regeneration: self.regeneration + s.regeneration,
        }
    }
}

impl Add for EffectiveStats {
    type Output = EffectiveStats;

    fn add(self, s: Self) -> Self {
        EffectiveStats {
            base: self.base + s.base,
            melee_dmg: self.melee_dmg + s.melee_dmg,
            melee_acc: self.melee_acc + s.melee_acc,
            melee_str_req: self.melee_str_req + s.melee_str_req,
            light_emision: self.light_emision + s.light_emision,
        }
    }
}

impl Sub for Stats {
    type Output = Stats;

    fn sub(self, s: Self) -> Self {
        Stats {
            int: self.int - s.int,
            dex: self.dex - s.dex,
            str_: self.str_ - s.str_,
            max_hp: self.max_hp - s.max_hp,
            max_mp: self.max_mp - s.max_mp,
            max_sp: self.max_sp - s.max_sp,
            ac: self.ac - s.ac,
            ev: self.ev - s.ev,
            infravision: self.infravision - s.infravision,
            vision: self.vision - s.vision,
            regeneration: self.regeneration - self.regeneration,
        }
    }
}

impl Sub for EffectiveStats {
    type Output = EffectiveStats;

    fn sub(self, s: Self) -> Self {
        EffectiveStats {
            base: self.base - s.base,
            melee_dmg: self.melee_dmg - s.melee_dmg,
            melee_acc: self.melee_acc - s.melee_acc,
            melee_str_req: self.melee_str_req - s.melee_str_req,
            light_emision: self.light_emision - s.light_emision,
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
    pub success: bool,
    pub dmg: i32,
    pub who: String,
    pub behind: bool,
}

#[derive(Clone, Debug)]
pub struct Actor {
    pub hp: i32,
    pub mp: i32,
    pub sp: i32,
    pub saved_hp: i32,
    pub saved_mp: i32,
    pub saved_sp: i32,

    pub player: bool,
    pub pre_pos: Option<Position>,
    pub pos: Position,
    pub acted: bool,
    descended: bool,

    pub race: Race,
    pub base_stats: Stats,
    pub mod_stats: EffectiveStats,
    pub stats: EffectiveStats,

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

    pub action_cd: i32,

    pub items_letters: HashSet<char>,
    pub items_equipped: HashMap<Slot, (char, Box<Item>)>,
    pub items_backpack: HashMap<char, Box<Item>>,

    pub was_attacked_by: Vec<AttackResult>,
    pub did_attack: Vec<AttackResult>,
}

impl Actor {
    pub fn new(race: Race, pos: Position) -> Self {
        let stats = Stats::new(race);

        Actor {
            race: race,
            player: false,
            pos: pos,
            pre_pos: None,
            base_stats: stats, // base stats
            mod_stats: Default::default(), // from items etc.
            stats: Default::default(), // effective stats
            in_los: Default::default(),
            temporary_los: Default::default(),
            visible: Default::default(),
            known: Default::default(),
            known_areas: Default::default(),
            heard: Default::default(),
            noise_emision: 0,
            discovered: Default::default(),
            discovered_areas: Default::default(),
            items_backpack: Default::default(),
            items_equipped: Default::default(),
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
            acted: false,
            descended: false,
        }
    }

    pub fn sees(&self, pos: Coordinate) -> bool {
        self.visible.contains(&pos)
    }

    pub fn in_los(&self, pos: Coordinate) -> bool {
        self.in_los.contains(&pos)
    }

    pub fn knows(&self, pos: Coordinate) -> bool {
        self.known.contains(&pos)
    }

    pub fn hears(&self, coord: Coordinate) -> bool {
        self.heard.contains_key(&coord)
    }

    pub fn coord(&self) -> Coordinate {
        self.pos.coord
    }

    /// "head" - The coordinate that is in front of an actor
    pub fn head(&self) -> Coordinate {
        self.pos.coord + self.pos.dir
    }

    pub fn pos_after_action(&self, action: Action) -> Vec<Position> {
        let pos = self.pos;
        match action {
            Action::Wait |
            Action::Pick |
            Action::Equip(_) |
            Action::Descend |
            Action::Ranged(_) |
            Action::Drop_(_) => vec![pos],
            Action::Turn(a) => vec![pos + a],
            Action::Move(a) => vec![pos + (pos.dir + a).to_coordinate()],
            Action::Charge => {
                if self.can_charge_sp() {
                    vec![pos + pos.dir.to_coordinate(),
                         pos + pos.dir.to_coordinate() + pos.dir.to_coordinate()]
                } else {
                    vec![pos + pos.dir.to_coordinate()]
                }
            }
            Action::Spin(a) => {
                vec![pos + (pos.dir + a).to_coordinate() +
                     match a {
                         Angle::Right => Angle::Left,
                         Angle::Left => Angle::Right,
                         _ => return vec![pos],
                     }]
            }
        }
    }

    pub fn attack_ranged(&mut self,
                         loc : &mut Location,
                         target_coord : Coordinate) {

        let target_id = match loc.actors_coord_to_id.get(&target_coord) {
            None => return,
            Some(id) => id
        };

        let mut target= match loc.actors_byid.remove(&target_id) {
            None => return,
            Some(target) => target,
        };
        let mut acc = self.stats.melee_acc;
        let mut dmg = self.stats.melee_dmg;

        let (ac, ev) = (target.stats.base.ac, target.stats.base.ev);

        let from_behind = match self.pos.dir - target.pos.dir {
            Angle::Forward | Angle::Left | Angle::Right => true,
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

        let rand_ac = cmp::max(rand::thread_rng().gen_range(0, ac + 1),
        rand::thread_rng().gen_range(0, ac + 1));

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

        loc.actors_byid.insert(*target_id, target);
    }

    pub fn try_attack_ranged(&mut self,
                         loc : &mut Location,
                         target_coord : Coordinate) {

        if self.can_attack_ranged() {
            self.attack_ranged(loc, target_coord);
        }
    }

    pub fn post_action(&mut self, action: Action) {
        match action {
            Action::Wait => {
                self.sp = cmp::min(self.stats.base.max_sp, self.sp + 1);
            }
            Action::Charge => {
                self.sp = cmp::max(0, self.sp - self.charge_sp_cost());
                self.acted = true;
            }
            _ => {
                self.acted = true;
            }
        }
    }

    pub fn acted_last_tick(&self) -> bool {
        self.acted
    }

    fn los_to_visible(&self, loc: &game::Location, los: &Visibility) -> Visibility {
        let mut visible: Visibility = Default::default();

        for &coord in los {
            if loc.light_map[coord] > 0 {
                visible.insert(coord);
            } else if self.pos.coord.distance(coord) <= self.stats.base.infravision {
                visible.insert(coord);
            } else if coord == self.head() {
                visible.insert(coord);
            } else if loc.at(coord).tile().opaqueness() > 10 {
                if loc.at(coord).light_as_seen_by(self) > 0 {
                    visible.insert(coord);
                }
            }
        }

        visible
    }

    fn postprocess_visibile(&mut self, loc: &game::Location) {
        let total_los = self.temporary_los.clone();
        let total_visible = self.los_to_visible(loc, &total_los);

        self.temporary_los = Default::default();
        self.add_current_los_to_temporary_los(loc);

        let visible = self.los_to_visible(loc, &self.temporary_los);

        for &i in total_visible.iter().chain(visible.iter()) {
            if !self.known.contains(&i) {
                self.known.insert(i);
                self.discovered.insert(i);
            }
        }

        for &coord in self.discovered.iter() {
            if let Some(area) = loc.at(coord).tile().area {
                let area_center = area.center;

                if !self.known_areas.contains(&area_center) {
                    self.known_areas.insert(area_center);
                    self.discovered_areas.insert(area_center);
                }
            }
        }

        self.in_los = self.temporary_los.clone();
        self.visible = visible;
    }

    // Could this actor have seen action/movement
    // of another actor (given by id)
    pub fn could_have_seen(&self, actor: &Actor) -> bool {
        self.sees(actor.pos.coord) || self.sees(actor.pre_pos.unwrap().coord)
    }

    // Save some stats for a reference
    pub fn save_stats(&mut self) {
        self.saved_hp = self.hp;
        self.saved_mp = self.mp;
        self.saved_sp = self.sp;
    }


    pub fn noise_makes(&mut self, noise: i32) {
        if self.noise_emision < noise {
            self.noise_emision = noise;
        }
    }

    pub fn noise_hears(&mut self, coord: Coordinate, type_: Noise) {
        self.heard.insert(coord, type_);
    }

    pub fn pre_any_tick(&mut self) {
        self.pre_pos = Some(self.pos);
        self.did_attack = Vec::new();
        self.was_attacked_by = Vec::new();
        self.temporary_los = Default::default();

        self.discovered = Default::default();
        self.discovered_areas = Default::default();

        self.noise_emision = 0;
        self.heard = Default::default();

        self.acted = false;
        self.descended = false;
    }

    pub fn pre_own_tick(&mut self) {
        if self.action_cd > 0 {
            self.action_cd -= 1;
        }
        if self.can_perform_action() {
            self.save_stats();
        }
    }

    pub fn post_own_tick(&mut self, loc: &Location) {
        if !self.is_dead() {
            if self.sp < self.stats.base.max_sp {
                if rand::thread_rng().gen_weighted_bool(20) {
                    self.sp += 1
                }
            }

            if self.hp < self.stats.base.max_hp {
                if rand::thread_rng().gen_range(0, 50) < self.stats.base.regeneration {
                    self.hp += 1
                }
            }

            if self.pre_pos != Some(self.pos) {
                self.postprocess_visibile(loc);
            }
        }
    }

    pub fn post_any_tick(&mut self, _loc: &Location) {
        self.recalculate_stats();
    }

    pub fn post_spawn(&mut self, loc: &Location) {
        // TODO: save & restore, using global location uuids ?
        self.known = Default::default();
        self.known_areas = Default::default();
        let pos = self.pos;
        self.moved(loc, pos)
    }

    pub fn pick_item(&mut self, item: Box<Item>) -> Option<Box<Item>> {
        for ch in ('a' as u8..'z' as u8).chain('A' as u8..'Z' as u8) {
            let ch = ch as char;
            if !self.item_letter_taken(ch) {
                assert!(!self.items_backpack.contains_key(&ch));
                self.items_backpack.insert(ch, item);
                return None;
            }
        }
        Some(item)
    }

    pub fn item_letter_taken(&self, ch: char) -> bool {
        if self.items_backpack.contains_key(&ch) {
            return true;
        }

        for (&_, &(ref item_ch, _)) in &self.items_equipped {
            if *item_ch == ch {
                return true;
            }
        }

        false
    }

    pub fn equip_switch(&mut self, ch: char) {
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

    pub fn equip_drop(&mut self, ch: char) -> Option<Box<Item>> {
        self.unequip(ch);
        self.items_backpack.remove(&ch)
    }

    pub fn equip(&mut self, item: Box<Item>, ch: char) {
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

    fn add_current_los_to_temporary_los(&mut self, loc: &Location) {
        let pos = self.pos;
        let vision = self.stats.base.vision;
        algo::los2::los(&|coord| loc.at(coord).tile().opaqueness(),
                        &mut |coord, _| {
                            let _ = self.temporary_los.insert(coord);
                        },
                        vision,
                        pos.coord,
                        &[pos.dir]);
    }

    pub fn unequip_slot(&mut self, slot: Slot) {
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

    pub fn unequip(&mut self, ch: char) {
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

    pub fn attacks(&mut self, dir: Direction, target: &mut Actor) {
        let mut acc = self.stats.melee_acc;
        let mut dmg = self.stats.melee_dmg;

        let (ac, ev) = (target.stats.base.ac, target.stats.base.ev);

        let from_behind = match dir - target.pos.dir {
            Angle::Forward | Angle::Left | Angle::Right => true,
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

        let rand_ac = cmp::max(rand::thread_rng().gen_range(0, ac + 1),
                               rand::thread_rng().gen_range(0, ac + 1));

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

    pub fn discovered_stairs(&self, loc: &Location) -> bool {
        self.discovered.iter().any(|c| loc.at(*c).tile().feature == Some(Feature::Stairs))
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
        self.sp >= self.melee_sp_cost()
    }

    // Can attack considering only sp?
    pub fn can_charge_sp(&self) -> bool {
        self.sp >= self.charge_sp_cost()
    }

    pub fn can_act(&self) -> bool {
        self.action_cd == 0 && !self.is_dead()
    }

    pub fn can_attack(&self) -> bool {
        self.can_act()
    }

    pub fn can_attack_ranged(&self) -> bool {
       self.can_act() && self.is_holding_ranged_weapon()
    }

    pub fn is_holding_ranged_weapon(&self) -> bool {
        if let Some(&(_, ref box_item)) = self.items_equipped.get(&Slot::RHand) {
            box_item.is_ranged_weapon()
        } else {
            false
        }
    }

    pub fn moved(&mut self, loc: &Location, new_pos: Position) {
        self.pos = new_pos;
        self.add_current_los_to_temporary_los(loc);
        self.noise_makes(2);
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

    pub fn descend(&mut self) {
        self.descended = true;
    }

    pub fn descended(&self) -> bool {
        self.descended
    }

    pub fn light_emision(&self) -> u32 {
        self.stats.light_emision
    }

    pub fn act(&mut self, loc : &mut Location, action: Action) {
        let new_pos = self.pos_after_action(action);

        for &new_pos in &new_pos {
            let old_pos = self.pos;

            if old_pos == new_pos {
                // no movement
                match action {
                    Action::Pick => {
                        let head = self.head();
                        let item = loc.at_mut(head).pick_item();

                        match item {
                            Some(item) => {
                                if let Some(item) = self.pick_item(item) {
                                    loc.at_mut(head).drop_item(item);
                                }
                            }
                            None => {}
                        }
                    }
                    Action::Equip(ch) => {
                        self.equip_switch(ch);
                    }
                    Action::Drop_(ch) => {
                        if let Some(item) = self.equip_drop(ch) {
                            loc.at_mut(self.pos.coord).drop_item(item);
                        }
                    }
                    Action::Descend => {
                        if loc.at(self.coord()).tile().feature == Some(tile::Feature::Stairs) {
                            self.descend();
                        }
                    }
                    Action::Ranged(target_coord) => {
                        self.try_attack_ranged(loc, target_coord);
                    },
                    _ => {}
                }
            } else if action.could_be_attack() && old_pos.coord != new_pos.coord &&
                loc.actors_coord_to_id.contains_key(&new_pos.coord) {
                    // we've tried to move into self; attack?
                    if !self.can_attack() {
                        break;
                    }
                    let dir = match action {
                        Action::Move(dir) => old_pos.dir + dir,
                        _ => old_pos.dir,
                    };

                    let target_id = loc.actors_coord_to_id[&new_pos.coord];

                    let mut target = loc.actors_byid.remove(&target_id).unwrap();
                    self.attacks(dir, &mut target);
                    loc.actors_byid.insert(target_id, target);
                    // Can't attack twice
                    break;
                } else if loc.at(new_pos.coord).tile().feature == Some(tile::Door(false)) {
                    // walked into door: open it
                    loc.map[new_pos.coord].add_feature(tile::Door(true));
                    // Can't charge through the doors
                    break;
                } else if old_pos.coord == new_pos.coord && old_pos.dir != new_pos.dir {
                    // we've rotated
                    self.moved(loc, new_pos);
                } else if old_pos.coord != new_pos.coord && loc.at(new_pos.coord).is_passable() &&
                    !loc.actors_coord_to_id.contains_key(&new_pos.coord) {
                        // for the rest of this turn this self can be found through both new
                        // and old coor
                        debug_assert!(!loc.actors_coord_to_id.contains_key(&new_pos.coord));
                        let id = loc.actors_coord_to_id.remove(&self.pos.coord).unwrap();
                        // we've moved
                        self.moved(loc, new_pos);
                        loc.actors_coord_to_id.insert(new_pos.coord, id);
                    } else {
                        // we hit the wall or something
                    }
        }
    }
}
