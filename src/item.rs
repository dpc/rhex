use std::fmt;

pub trait Item : Send+Sync+fmt::Debug {
    fn description(&self) -> &str;
    fn type_(&self) -> Type;
    fn clone_item<'a>(&self) -> Box<Item + Send + Sync + 'a> where Self: 'a;
}

impl<'a> Clone for Box<Item+Send+Sync+'a> {
    fn clone(&self) -> Box<Item+Send+Sync+'a> {
        self.clone_item()
    }
}

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

    fn type_(&self) -> Type {
        Type::Weapon
    }

    fn clone_item<'a>(&self) -> Box<Item + Send + Sync + 'a> where Self: 'a {
        Box::new(self.clone())
    }
}

pub enum Type {
    Weapon,
}

