use std::option::Option;
pub use self::Type::*;
pub use self::Feature::*;
pub use super::area;

use std::fmt;
use rand::{Rng, self};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Type {
    Wall,
    Empty,
    Water,
}

impl Type {
    pub fn description(&self) -> &str {
        match *self {
            Wall => "wall",
            Empty => "nothing",
            Water => "water",
        }
    }
}


impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}


#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Feature {
    Door(bool),
    Stairs,
    Statue,
}

impl Feature {
    pub fn description(&self) -> &str {
        match *self {
            Door(true) => "open door",
            Door(false) => "closed door",
            Stairs => "stairs down",
            Statue => "statue",
        }
    }
}


impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Tile {
    pub type_: Type,
    pub feature: Option<Feature>,
    pub area: Option<area::Area>,
    pub light: i32,
}

impl Tile {
    pub fn new(t: Type) -> Tile {
        Tile {
            type_: t,
            feature: None,
            area: None,
            light: 0,
        }
    }

    pub fn add_feature(&mut self, f: Feature) -> &mut Tile {
        self.feature = Some(f);
        self
    }

    pub fn add_light(&mut self, light: i32) -> &mut Tile {
        self.light = light;
        self
    }

    pub fn add_area(&mut self, area: area::Area) -> &mut Tile {
        self.area = Some(area);
        self
    }

    pub fn is_passable(&self) -> bool {
        match self.feature {
            Some(Statue) => return false,
            _ => {}
        }

        self.type_.is_passable()
    }

    pub fn opaqueness(&self) -> i32 {
        match self.feature {
            Some(Statue) => return 3,
            Some(Door(false)) => return 1000,
            _ => {}
        }

        self.type_.opaqueness()
    }

    pub fn ascii_expand(&self) -> i32 {
        match self.feature {
            Some(Door(open)) => {
                return if open {
                    1
                } else {
                    0
                }
            }
            Some(Statue) => return 8,
            _ => {}
        }

        self.type_.base_ascii_expand()
    }

    pub fn can_dig_through(&self) -> bool {
        self.type_.can_dig_through()
    }

    pub fn dig(&mut self) {
        let r = rand::thread_rng().gen_range(0..10);
        match self.type_ {
            Wall => if r < 5 { self.type_ = Empty },
            _ => {}
        }
    }
}

impl Type {
    pub fn is_passable(&self) -> bool {
        match *self {
            Wall => false,
            Empty => true,
            Water => false,
        }
    }

    pub fn can_dig_through(&self) -> bool {
        *self == Wall
    }

    pub fn opaqueness(&self) -> i32 {
        match *self {
            Wall => 1000,
            Empty | Water => 1,
        }
    }

    pub fn base_ascii_expand(&self) -> i32 {
        match *self {
            Water => 7,
            Wall => 9,
            Empty => 10,
        }
    }
}

impl Default for Tile {
    fn default() -> Tile {
        Tile {
            type_: Wall,
            feature: None,
            area: None,
            light: 0,
        }
    }
}
