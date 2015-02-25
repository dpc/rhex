use std::fmt;
use actor::Slot;

pub trait Item : Send+Sync+fmt::Debug {
    fn description(&self) -> &str;
    fn type_(&self) -> Type;
    fn slot(&self) -> Slot;
    fn clone_item<'a>(&self) -> Box<Item + 'a> where Self: 'a;
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
}

pub use self::weapon::Weapon;
pub use self::armor::Armor;

pub mod weapon {
    use super::Item;
    use super::Type as ItemType;
    use actor::Slot;

    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub struct Weapon {
        pub base_dmg : i32,
    }

    impl Weapon {
        pub fn new() -> Weapon {
            Weapon { base_dmg: 1 }
        }
    }

    impl Item for Weapon {
        fn description(&self) -> &str {
            "sword"
        }

        fn type_(&self) -> ItemType {
            ItemType::Weapon
        }

        fn clone_item<'a>(&self) -> Box<Item + 'a> where Self: 'a {
            Box::new(self.clone())
        }

        fn slot(&self) -> Slot {
            Slot::RHand
        }
    }
}

pub mod armor {
    use super::Item;
    use super::Type as ItemType;
    use actor::Slot;

    pub use self::Type::*;

    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum Type {
        Leather,
        Plate,
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
    }

    impl Item for Armor {
        fn description(&self) -> &str {
            match self.type_ {
                Type::Plate => "plate armor",
                Type::Leather => "leather armor",
            }
        }

        fn type_(&self) -> ItemType {
            ItemType::Armor
        }

        fn clone_item<'a>(&self) -> Box<Item + 'a> where Self: 'a {
            Box::new(self.clone())
        }

        fn slot(&self) -> Slot {
            Slot::Body
        }
    }
}
