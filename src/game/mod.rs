use std::collections::{HashMap, HashSet};
use std::hash::BuildHasherDefault;
use simplemap::SimpleMap;
use fnv::FnvHasher;

use hex2d::Coordinate;

use game::item::Item;

pub mod area;
pub mod actor;
pub use self::actor::Actor;
pub mod action;
pub use self::action::Action;
pub mod consts;
pub mod item;
pub mod engine;
pub use self::engine::*;
pub mod tile;
pub use self::tile::Tile;
pub mod location;
pub use self::location::Location;


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
