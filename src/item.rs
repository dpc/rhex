use std::fmt;
use actor::{self, Slot};
use rand::{self, Rng};

pub use self::weapon::Weapon;
pub use self::armor::Armor;
pub use self::misc::Misc;
pub use self::consumable::Consumable;

pub trait Item : Send+Sync+fmt::Debug {
    fn description(&self) -> &str;
    fn type_(&self) -> Type;
    fn slot(&self) -> Option<Slot>;
    fn clone_item<'a>(&self) -> Box<Item + 'a> where Self: 'a;
    fn stats(&self) -> actor::Stats;

    fn is_usable(&self) -> bool {
        self.type_() == Type::Consumable
    }

    /// Use item
    ///
    /// Returns: true if the item was consumed in the process.
    fn use_(&self, &mut actor::State) -> bool {
        false
    }
}

impl<'a> Clone for Box<Item+'a> {
    fn clone(&self) -> Box<Item+'a> {
        self.clone_item()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Type {
    Weapon,
    Armor,
    Misc,
    Consumable,
}

pub fn random(level : i32) -> Box<Item> {

    let a = -level;
    let b = level + 1;
    let r = rand::thread_rng().gen_range(a, b) +
        rand::thread_rng().gen_range(a, b) +
        rand::thread_rng().gen_range(a, b) +
        rand::thread_rng().gen_range(-2, 3) +
        level / 2;


    match r {
        0 => Consumable::new(consumable::HealthPotion).to_item(),
        1 => Weapon::new(weapon::Knife).to_item(),
        2 => Armor::new(armor::Cloak).to_item(),
        3 => Weapon::new(weapon::Sword).to_item(),
        4 => Armor::new(armor::Helmet).to_item(),
        5 => Armor::new(armor::Leather).to_item(),
        6 => Armor::new(armor::Boots).to_item(),
        7 => Weapon::new(weapon::Axe).to_item(),
        8 => Armor::new(armor::Buckler).to_item(),
        9 => Armor::new(armor::Plate).to_item(),
        _ => Misc::new(misc::Junk).to_item(),
    }
}
pub mod weapon {
    use super::Item;
    use super::Type as ItemType;
    use actor::{self, Slot};
    pub use self::Type::*;

    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum Type {
        Knife,
        Sword,
        Axe,
    }

    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub struct Weapon {
        type_ : Type,
    }

    impl Weapon {
        pub fn new(type_ : Type) -> Weapon {
            Weapon {
                type_: type_,
            }
        }

        pub fn to_item(self) -> Box<Item> {
            Box::new(self)
        }
    }

    impl Item for Weapon {
        fn description(&self) -> &str {
            match self.type_ {
                Knife => "knife",
                Sword => "sword",
                Axe => "axe",
            }
        }

        fn type_(&self) -> ItemType {
            ItemType::Weapon
        }

        fn clone_item<'a>(&self) -> Box<Item + 'a> where Self: 'a {
            Box::new(self.clone())
        }

        fn slot(&self) -> Option<Slot> {
            Some(Slot::RHand)
        }

        fn stats(&self) -> actor::Stats {
            let mut stats = actor::Stats::zero();

            match self.type_ {
                Knife => {
                    stats.melee_dmg += 1;
                },
                Sword => {
                    stats.melee_dmg += 3;
                    stats.melee_cd += 1;
                },
                Axe => {
                    stats.melee_dmg += 4;
                    stats.melee_cd += 2;
                },
            }

            stats
        }
    }
}

pub mod armor {
    use super::Item;
    use super::Type as ItemType;
    use actor::{self, Slot};

    pub use self::Type::*;

    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum Type {
        Leather,
        Plate,
        Helmet,
        Boots,
        Buckler,
        Cloak,
    }

    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub struct Armor {
        pub type_ : Type,
    }

    impl Armor {
        pub fn new(type_ : Type) -> Armor {
            Armor{
                type_: type_,
            }
        }

        pub fn to_item(self) -> Box<Item> {
            Box::new(self)
        }
    }

    impl Item for Armor {
        fn description(&self) -> &str {
            match self.type_ {
                Plate => "plate armor",
                Leather => "leather armor",
                Helmet => "helmet",
                Boots => "boots",
                Buckler => "buckler",
                Cloak => "cloak",
            }
        }

        fn type_(&self) -> ItemType {
            ItemType::Armor
        }

        fn clone_item<'a>(&self) -> Box<Item + 'a> where Self: 'a {
            Box::new(self.clone())
        }

        fn slot(&self) -> Option<Slot> {
            match self.type_ {
                Leather|Plate => Some(Slot::Body),
                Helmet => Some(Slot::Head),
                Boots => Some(Slot::Feet),
                Buckler => Some(Slot::LHand),
                Cloak => Some(Slot::Cloak),
            }
        }

        fn stats(&self) -> actor::Stats {
            let mut s = actor::Stats::zero();

            match self.type_ {
                Plate => {
                    s.ac += 4;
                    s.ev -= 2;
                },
                Leather => {
                    s.ac += 1;
                },
                Helmet => s.ac += 1,
                Boots => s.ev += 1,
                Buckler => { s.ev += 1; s.ac += 1 }
                Cloak => { s.ev += 1; }
            }

            s
        }
    }
}

pub mod misc {
    use super::Item;
    use super::Type as ItemType;
    use actor::{self, Slot};

    pub use self::Type::*;

    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum Type {
        Junk,
    }


    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub struct Misc {
        type_ : Type,
    }

    impl Misc {
        pub fn new(type_ : Type) -> Misc {
            Misc {
                type_: type_,
            }
        }

        pub fn to_item(self) -> Box<Item> {
            Box::new(self)
        }
    }

    impl Item for Misc {
        fn description(&self) -> &str {
            match self.type_ {
                Junk => "junk",
            }
        }

        fn type_(&self) -> ItemType {
            ItemType::Misc
        }

        fn clone_item<'a>(&self) -> Box<Item + 'a> where Self: 'a {
            Box::new(self.clone())
        }

        fn slot(&self) -> Option<Slot> {
            None
        }

        fn stats(&self) -> actor::Stats {
            actor::Stats::zero()
        }
    }
}

pub mod consumable {
    use std::cmp;
    use super::Item;
    use super::Type as ItemType;
    use actor::{self, Slot};

    pub use self::Type::*;

    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum Type {
        HealthPotion,
    }


    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub struct Consumable {
        type_ : Type,
    }

    impl Consumable {
        pub fn new(type_ : Type) -> Consumable {
            Consumable {
                type_: type_,
            }
        }

        pub fn to_item(self) -> Box<Item> {
            Box::new(self)
        }
    }

    impl Item for Consumable {
        fn description(&self) -> &str {
            match self.type_ {
                HealthPotion => "health potion",
            }
        }

        fn type_(&self) -> ItemType {
            ItemType::Consumable
        }

        fn clone_item<'a>(&self) -> Box<Item + 'a> where Self: 'a {
            Box::new(self.clone())
        }

        fn slot(&self) -> Option<Slot> {
            None
        }

        fn stats(&self) -> actor::Stats {
            actor::Stats::zero()
        }

        fn use_(&self, astate : &mut actor::State) -> bool {
            astate.hp += 5;
            astate.hp = cmp::min(astate.hp, astate.stats.max_hp);
            true
        }
    }
}
