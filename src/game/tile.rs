pub use self::Feature::*;
pub use self::Type::*;
pub use super::area;
use std::option::Option;

use rand::{self, RngExt};
use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Type {
    Wall,
    Empty,
    /// Deep, transparent water that cannot be walked through.
    Water,
    Ground,
    Thicket,
    TreeTrunk,
    ShallowWater,
}

impl Type {
    pub fn description(&self) -> &str {
        match *self {
            Wall => "wall",
            Empty => "nothing",
            Water => "deep water",
            Ground => "forest ground",
            Thicket => "thicket",
            TreeTrunk => "tree trunk",
            ShallowWater => "shallow water",
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
    ForestPath,
    Entrance,
    Statue,
    /// A permanent, walkable wall-side light fixture.
    Torch,
}

impl Feature {
    pub fn description(&self) -> &str {
        match *self {
            Door(true) => "open door",
            Door(false) => "closed door",
            Stairs => "stairs down",
            ForestPath => "forest path onward",
            Entrance => "return passage",
            Statue => "statue",
            Torch => "torch",
        }
    }

    /// Whether the feature can connect to another retained level.
    pub fn is_exit(self) -> bool {
        matches!(self, Stairs | ForestPath | Entrance)
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
    /// Walls block transmission; movement restrictions do not define acoustics.
    pub fn transmits_sound(&self) -> bool {
        !matches!(self.type_, Wall | TreeTrunk)
    }

    /// Quarter-level absorption, also applied to the six adjacent cells.
    pub fn sound_dampening(&self) -> u16 {
        let terrain = match self.type_ {
            Wall | TreeTrunk => 1,
            Thicket => 2,
            _ => 0,
        };
        terrain.max(match self.feature {
            Some(Statue) => 4,
            Some(Door(false)) => 2,
            _ => 0,
        })
    }

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
        if let Some(Statue) = self.feature {
            return false;
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
            Some(Door(open)) => return if open { 1 } else { 0 },
            Some(Statue) => return 8,
            _ => {}
        }

        self.type_.base_ascii_expand()
    }

    pub fn can_dig_through(&self) -> bool {
        self.type_.can_dig_through()
    }

    pub fn dig(&mut self, rng: &mut impl rand::Rng) {
        let r = rng.random_range(0..10);
        if self.type_ == Wall && r < 5 {
            self.type_ = Empty
        }
    }
}

impl Type {
    pub fn is_passable(&self) -> bool {
        match *self {
            Wall | Water | Thicket | TreeTrunk => false,
            Empty | Ground | ShallowWater => true,
        }
    }

    pub fn can_dig_through(&self) -> bool {
        *self == Wall
    }

    pub fn opaqueness(&self) -> i32 {
        match *self {
            Wall | TreeTrunk | Thicket => 1000,
            Empty | Water | Ground | ShallowWater => 1,
        }
    }

    pub fn base_ascii_expand(&self) -> i32 {
        match *self {
            Water | ShallowWater => 7,
            Wall | TreeTrunk | Thicket => 9,
            Empty | Ground => 10,
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
