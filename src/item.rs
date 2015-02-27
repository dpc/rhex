use std::fmt;
use actor::Slot;

pub trait Item : Send+Sync+fmt::Debug {
    fn description(&self) -> &str;
    fn type_(&self) -> Type;
    fn slot(&self) -> Slot;
    fn clone_item<'a>(&self) -> Box<Item + 'a> where Self: 'a;
    fn attack(&self) -> Option<(i32, i32, i32)>;
    fn defense(&self) -> Option<(i32, i32)>;
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

        fn slot(&self) -> Slot {
            Slot::RHand
        }

        fn defense(&self) -> Option<(i32, i32)> {
            None
        }

        fn attack(&self) -> Option<(i32, i32, i32)> {
            Some(match self.type_ {
                Knife => (2, 0, 0),
                Sword => (4, 0, 1),
                Axe => (6, -1, 2),
            })
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

        fn defense(&self) -> Option<(i32, i32)> {
            Some(match self.type_ {
               Plate => (3, -1),
               Leather => (1, 0),
            })
        }

        fn attack(&self) -> Option<(i32, i32, i32)> {
            None
        }
    }
}
