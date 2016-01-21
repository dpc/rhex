use super::actor::{self, Actor, Slot};
use rand::{self, Rng, Rand};
use rand::distributions::IndependentSample;

use core::cmp;
use std::fmt::{self, Write};

use self::Category::*;
use self::Type::*;
use self::Feature::*;

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

impl Type {
    pub fn description(&self) -> &str {
        match *self {
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
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Feature {
    Infravision,
    Light,
    Regeneration,
}

impl Feature {
    pub fn description(&self) -> &str {
        match *self {
            Infravision => "infravision",
            Light => "light",
            Regeneration => "regeneration",
        }
    }


    pub fn stats(&self) -> actor::EffectiveStats {
        let mut s: actor::EffectiveStats = Default::default();

        match *self {
            Infravision => s.base.infravision += 1,
            Light => s.light_emision += 1,
            Regeneration => s.base.regeneration += 1,
        }

        s
    }
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Rand for Feature {
    fn rand<R: Rng>(rng: &mut R) -> Self {
        match rng.gen_range(0, 3) {
            0 => Infravision,
            1 => Light,
            2 => Regeneration,
            _ => panic!(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Item {
    type_: Type,
    features: Vec<Feature>,
}

impl Item {
    pub fn new(t: Type, features: Vec<Feature>) -> Item {
        Item {
            type_: t,
            features: features,
        }
    }

    pub fn description(&self) -> String {
        let mut s = String::new();

        write!(s, "{}", *self).unwrap();

        s
    }

    pub fn category(&self) -> Category {
        match self.type_ {
            Knife | Sword | Axe => Weapon,
            Leather | Plate | Helmet | Boots | Buckler | Cloak => Armor,
            HealthPotion => Consumable,
            Junk => Misc,
        }
    }

    pub fn slot(&self) -> Option<Slot> {
        match self.type_ {
            Axe | Sword | Knife => Some(Slot::RHand),
            Leather | Plate => Some(Slot::Body),
            Helmet => Some(Slot::Head),
            Boots => Some(Slot::Feet),
            Buckler => Some(Slot::LHand),
            Cloak => Some(Slot::Cloak),
            _ => None,
        }
    }

    pub fn stats(&self) -> actor::EffectiveStats {
        let mut s: actor::EffectiveStats = Default::default();

        match self.type_ {
            Plate => {
                s.base.ac = 4;
                s.base.ev = -2;
            }
            Leather => {
                s.base.ac = 1;
            }
            Helmet => {
                s.base.ac = 1;
                s.base.vision = -2;
                s.base.infravision = -1;
            }
            Boots => s.base.ev = 1,
            Buckler => {
                s.base.ev = 1;
                s.base.ac = 1
            }
            Cloak => {
                s.base.ev = 1;
            }
            Knife => {
                s.melee_dmg = 1;
                s.melee_str_req = 2;
            }
            Sword => {
                s.melee_dmg += 3;
                s.melee_str_req = 4;
            }
            Axe => {
                s.melee_dmg += 4;
                s.melee_str_req = 5;
            }
            _ => {}
        }

        for feature in &self.features {
            s = s + feature.stats()
        }
        s
    }

    pub fn is_usable(&self) -> bool {
        self.category() == Consumable
    }

    /// Use item
    ///
    /// Returns: true if the item was consumed in the process.
    pub fn use_(&self, astate: &mut Actor) -> bool {
        match self.type_ {
            HealthPotion => {
                astate.hp += 5;
                astate.hp = cmp::min(astate.hp, astate.stats.base.max_hp);
                true
            }
            _ => false,
        }
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "{}", self.type_));

        for feature in &self.features {
            try!(write!(f, " of {}", feature));
        }

        Ok(())
    }
}


pub fn random(level: i32) -> Box<Item> {

    let a = -(level / 2);
    let b = level + 1;
    let mut rng = rand::thread_rng();
    let lvrange = rand::distributions::Range::new(a, b);
    let r = lvrange.ind_sample(&mut rng) + lvrange.ind_sample(&mut rng) +
            lvrange.ind_sample(&mut rng) + lvrange.ind_sample(&mut rng);

    let mut features = vec![];
    let mut chance = level;
    const PER_LOOP: i32 = 30;
    let looprange = rand::distributions::Range::new(0, PER_LOOP);
    while looprange.ind_sample(&mut rng) < chance {
        features.push(rng.gen::<Feature>());
        chance = cmp::max(0, chance - PER_LOOP);
    }

    Box::new(Item::new(match r {
                           0 => HealthPotion,
                           1 => Knife,
                           2 => Cloak,
                           3 => Sword,
                           4 => Helmet,
                           5 => Leather,
                           6 => Boots,
                           7 => Axe,
                           8 => Buckler,
                           9 => Plate,
                           _ => Junk,
                       },
                       features))
}
