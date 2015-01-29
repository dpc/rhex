use std::option::Option;
pub use self::Type::*;
pub use super::area;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Type {
    Wall,
    Tree,
    Empty,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Tile {
    pub type_ : Type,
    pub area: Option<area::Area>,
    pub light : i32,
}

impl Tile {
    pub fn new(t : Type) -> Tile {
        Tile { type_: t, area: None, light: 0 }
    }

    pub fn new_with_light(t : Type, light : i32) -> Tile {
        Tile { type_: t, area: None, light: light }
    }

    pub fn new_with_area(t : Type, area: area::Area) -> Tile {
        Tile { type_: t, area: Some(area), light: 0 }
    }

    pub fn is_passable(&self) -> bool {
        self.type_.is_passable()
    }

    pub fn opaqueness(&self) -> i32 {
        self.type_.opaqueness()
    }
}

impl Type {
    pub fn is_passable(&self) -> bool {
        match *self {
            Wall => false,
            Tree => false,
            Empty => true,
        }
    }

    pub fn opaqueness(&self) -> i32 {
        match *self {
            Wall => 1000,
            Tree => 4,
            Empty => 1,
        }
    }

    pub fn ascii_expand(&self) -> bool {
        match *self {
            Wall => true,
            Tree => true,
            Empty => false,
        }
    }
}
