use std::collections::{HashSet,HashMap};
use hex2d::{Coordinate, Angle, Position, ToCoordinate, Direction};
use game::{self, Action};
use game::tile::{Feature};
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
    pub base_ac: i32,
    pub base_ev: i32,
}

impl Stats {
    pub fn new(hp : i32) -> Stats {
        Stats {
            int: 3, dex : 3, str_ : 3,
            max_hp: hp, hp: hp,
            max_mp: 10, mp: 10,
            base_ac: 0, base_ev: 0,
        }
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

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct AttackResult {
    pub success : bool,
    pub dmg : i32,
    pub who : String,
    pub behind : bool,
}

#[derive(Clone, Debug)]
pub struct State {

    pub pre_pos : Position,
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

    pub heared: Visibility,
    pub noise_emision: i32,

    pub light_emision : u32,

    pub attack_cooldown : i32,
    pub action_cooldown : i32,
    pub items_letters: HashSet<char>,
    pub items_equipped : HashMap<Slot, (char, Box<Item>)>,
    pub items_backpack : HashMap<char, Box<Item>>,

    pub was_attacked_by : Vec<AttackResult>,
    pub did_attack : Vec<AttackResult>,
}

impl State {
    pub fn new(behavior : Behavior, pos : Position) -> State {
        let stats = Stats::new(
            if behavior == Behavior::Player { 10 } else { 5 }
            );

        State {
            behavior : behavior,
            pos: pos, pre_pos: pos,
            stats: stats,
            prev_stats: stats,
            visible: HashSet::new(),
            known: HashSet::new(),
            known_areas: HashSet::new(),
            heared: HashSet::new(),
            noise_emision: 0,
            discovered: HashSet::new(),
            discovered_areas: HashSet::new(),
            light_emision: 0,
            items_backpack: HashMap::new(),
            items_equipped: HashMap::new(),
            items_letters: HashSet::new(),
            attack_cooldown: 0,
            action_cooldown: 0,
            was_attacked_by: Vec::new(),
            did_attack: Vec::new(),
        }
    }

    pub fn new_grue(level : i32, pos : Position) -> State {
        let mut ret = State::new(Behavior::Grue, pos);

        ret.stats.hp += level / 2;
        ret.stats.max_hp += level / 2;

        ret.stats.dex += level / 6;
        ret.stats.str_ += (2 + level) / 5;

        ret.stats.base_ev += (2 + level) / 3;
        ret.stats.base_ac += (2 + level) / 4;
        ret

    }


    pub fn add_light(&mut self, light : u32) {
        self.light_emision = light;
    }

    pub fn sees(&self, pos : Coordinate) -> bool {
        self.visible.contains(&pos)
    }

    pub fn knows(&self, pos : Coordinate) -> bool {
        self.known.contains(&pos)
    }

    pub fn hears(&self, coord : Coordinate) -> bool {
        self.heared.contains(&coord)
    }

    pub fn pos_after_action(&self, action : Action) -> Position {
        let pos = self.pos;
        match action {
            Action::Wait|Action::Pick|Action::Equip(_)|Action::Descend => pos,
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
        self.pre_pos = self.pos;
        self.prev_stats = self.stats;
        self.did_attack = Vec::new();
        self.was_attacked_by = Vec::new();
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }

        if self.action_cooldown > 0 {
            self.action_cooldown -= 1;
        }

        self.noise_emision = 0;
        self.heared = HashSet::new();
    }

    pub fn noise_makes(&mut self, noise : i32) {
        if self.noise_emision < noise {
            self.noise_emision = noise;
        }
    }

    pub fn noise_hears(&mut self, coord : Coordinate) {
        self.heared.insert(coord);
    }

    pub fn post_tick(&mut self, gstate : &game::State) {
        self.postprocess_visibile(gstate);
    }

    pub fn add_item(&mut self, item : Box<Item>) -> bool {
        for ch in range('a' as u8, 'z' as u8)
            .chain(range('A' as u8, 'Z' as u8)) {
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
            self.equip(ch);
        } else {
            self.unequip(ch);
        }
    }

    pub fn equip(&mut self, ch : char) {
        if let Some(item) = self.items_backpack.remove(&ch) {
            let slot = item.slot();
            self.unequip_slot(slot);
            self.items_equipped.insert(slot, (ch, item));
            self.action_cooldown += if slot == Slot::Body {
                4
            } else {
                1
            }
        }
    }

    pub fn unequip_slot(&mut self, slot : Slot) {
        if let Some((ch, item)) = self.items_equipped.remove(&slot) {
            self.items_backpack.insert(ch, item);
            self.action_cooldown += if slot == Slot::Body {
                4
            } else {
                1
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

    pub fn attacks(&mut self, dir : Direction, target : Option<&mut State>) {
        let (mut dmg, mut acc, cooldown) = self.attack();
        self.attack_cooldown = cooldown;

        if let Some(target) = target {
            let (ac, ev) = target.defense();

            let from_behind = match dir - target.pre_pos.dir {
                Angle::Forward|Angle::Left|Angle::Right => true,
                _ => false,
            };

            if from_behind {
                acc += acc;
                dmg += dmg;
            }

            let apower = self.stats.dex + acc;
            let dpower = target.stats.dex + ev;

            let success = util::roll(apower, dpower);

            let dmg = cmp::max(0, dmg - ac);
            if success {
                target.stats.hp -= dmg;
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
    }

    pub fn defense(&self) -> (i32, i32) {
        let (ac, ev) = self.items_equipped.get(&Slot::Body).and_then(|&(_, ref i)| i.defense()).unwrap_or((0, 0));

        (ac + self.stats.base_ac + self.stats.str_ / 3, ev + self.stats.dex / 2 + self.stats.base_ev)
    }

    pub fn attack(&self) -> (i32, i32, i32) {
        let (dmg, acc, cooldown) = self.items_equipped.get(&Slot::RHand).and_then(|&(_, ref i)| i.attack()).unwrap_or(self.hand_attack());

        (dmg + (1 + self.stats.str_) / 2, acc + (1 + self.stats.dex) / 2, cooldown)
    }

    pub fn discovered_stairs(&self, gstate : &game::State) -> bool {
        self.discovered.iter().any(
            |c| gstate.at(*c).tile_map_or(
                false, |t| t.feature == Some(Feature::Stairs)
                )
            )
    }

    pub fn hand_attack(&self) -> (i32, i32, i32) {
        match self.behavior {
            Behavior::Grue => (2, 1, 0),
            Behavior::Player => (1, 0, 0),
            Behavior::Pony => (1, -1, 0),
        }
    }
    pub fn can_attack(&self) -> bool {
        self.attack_cooldown == 0 && self.action_cooldown == 0
    }

    pub fn moved(&mut self, new_pos : Position) {
        self.pos = new_pos;
        self.noise_makes(2);
    }

    pub fn changed_level(&mut self) {
        self.known = HashSet::new();
        self.known_areas = HashSet::new();
    }

    pub fn is_player(&self) -> bool {
        self.behavior == Behavior::Player
    }

    pub fn is_dead(&self) -> bool {
        self.stats.hp <= 0
    }

    pub fn can_perform_action(&self) -> bool {
        !self.is_dead() && self.action_cooldown == 0
    }

    pub fn description(&self) -> String {
        match self.behavior {
            Behavior::Grue => "Grue",
            Behavior::Pony => "Pony",
            Behavior::Player => "Human",
        }.to_string()
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
        10, pos.coord,
        //&[pos.dir, pos.dir + Angle::Left, pos.dir + Angle::Right]
        &[pos.dir]
        );

    visibility
}
