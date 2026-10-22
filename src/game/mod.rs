use fnv::FnvHasher;
use simplemap::SimpleMap;
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasherDefault;

use hex2d::Coordinate;

use crate::game::item::Item;

pub mod actor;
pub mod area;
pub mod awareness_event;
pub use self::actor::Actor;
pub mod action;
pub use self::action::Action;
pub mod chest;
pub mod consts;
mod content;
pub mod engine;
pub mod item;
pub use self::engine::*;
pub mod tile;
pub use self::tile::Tile;
pub mod level;
pub mod location;
pub use self::location::Location;

#[cfg(test)]
pub(crate) mod fixtures;

#[cfg(test)]
mod content_tests;
#[cfg(test)]
mod forest_tests;
#[cfg(test)]
mod interaction_tests;
#[cfg(test)]
mod property_tests;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Noise {
    Creature(actor::Race),
}

impl Noise {
    pub fn description(&self) -> String {
        match *self {
            Noise::Creature(cr) => cr.description(),
        }
    }
}

pub type Visibility = HashSet<Coordinate, BuildHasherDefault<FnvHasher>>;
pub type NoiseMap = HashMap<Coordinate, Noise, BuildHasherDefault<FnvHasher>>;
pub type Map = SimpleMap<Coordinate, Tile, BuildHasherDefault<FnvHasher>>;
pub type Actors = HashMap<Coordinate, Actor, BuildHasherDefault<FnvHasher>>;
pub type Items = HashMap<Coordinate, Box<Item>, BuildHasherDefault<FnvHasher>>;
pub type LightMap = SimpleMap<Coordinate, u32, BuildHasherDefault<FnvHasher>>;
pub mod combat_event;
pub mod sound;
