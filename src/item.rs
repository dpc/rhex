use actor::{self, Slot};
use rand::{self, Rng};

use core::cmp;

use self::Category::*;
use self::Type::*;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Category {
    Weapon,
    Armor,
    Misc,
    Consumable,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Type {
    Knife,
    Sword,
    Axe,
    HealthPotion,
    Junk,
    Leather,
    Plate,
    Helmet,
    Boots,
    Buckler,
    Cloak,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Item  {
    type_ : Type,

}

impl Item {

    pub fn new(t : Type) -> Item {
        Item { type_: t }
    }

    pub fn description(&self) -> &str {
        match self.type_ {
            Junk => "junk",
            Knife => "knife",
            Sword => "sword",
            Axe => "axe",
            HealthPotion => "health potion",
            Plate => "plate armor",
            Leather => "leather armor",
            Helmet => "helmet",
            Boots => "boots",
            Buckler => "buckler",
            Cloak => "cloak",
        }
    }

    pub fn category(&self) -> Category {
        match self.type_ {
            Knife|Sword|Axe => Weapon,
            Leather|Plate|Helmet|Boots|Buckler|Cloak => Armor,
            HealthPotion => Consumable,
            Junk => Misc,
        }
    }

    pub fn slot(&self) -> Option<Slot> {
        match self.type_ {
            Axe|Sword|Knife => Some(Slot::RHand),
            Leather|Plate => Some(Slot::Body),
            Helmet => Some(Slot::Head),
            Boots => Some(Slot::Feet),
            Buckler => Some(Slot::LHand),
            Cloak => Some(Slot::Cloak),
            _ => None,
        }
    }

    pub fn stats(&self) -> actor::EffectiveStats {
        let mut s : actor::EffectiveStats = Default::default();

        match self.type_ {
            Plate => {
                s.base.ac = 4;
                s.base.ev = -2;
            },
            Leather => {
                s.base.ac = 1;
            },
            Helmet => {
                s.base.ac = 1;
                s.base.vision = -2;
                s.base.infravision = -1;
            }
            Boots => s.base.ev = 1,
            Buckler => { s.base.ev = 1; s.base.ac = 1 },
            Cloak => { s.base.ev = 1; },
            Knife => {
                s.melee_dmg = 1;
                s.melee_str_req = 2;
            },
            Sword => {
                s.melee_dmg += 3;
                s.melee_str_req = 4;
            },
            Axe => {
                s.melee_dmg += 4;
                s.melee_str_req = 5;
            },
            _ => {},
        }

        s
    }

    pub fn is_usable(&self) -> bool {
        self.category() == Consumable
    }

    /// Use item
    ///
    /// Returns: true if the item was consumed in the process.
    pub fn use_(&self, astate : &mut actor::State) -> bool {
        match self.type_ {
            HealthPotion => {
                astate.hp += 5;
                astate.hp = cmp::min(astate.hp, astate.stats.base.max_hp);
                true
            },
            _ => {
                false
            }
        }
    }
}

pub fn random(level : i32) -> Box<Item> {

    let a = -(level / 2);
    let b = level + 1;
    let r = rand::thread_rng().gen_range(a, b) +
        rand::thread_rng().gen_range(a, b) +
        rand::thread_rng().gen_range(a, b) +
        rand::thread_rng().gen_range(a, b);

    Box::new(match r {
        0 => Item::new(HealthPotion),
        1 => Item::new(Knife),
        2 => Item::new(Cloak),
        3 => Item::new(Sword),
        4 => Item::new(Helmet),
        5 => Item::new(Leather),
        6 => Item::new(Boots),
        7 => Item::new(Axe),
        8 => Item::new(Buckler),
        9 => Item::new(Plate),
        _ => Item::new(Junk),
    })
}
