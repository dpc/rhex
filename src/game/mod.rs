use std::collections::{HashMap, HashSet};
use std::collections::hash_state::{DefaultState};
use simplemap::SimpleMap;
use std::sync::{Arc};
use fnv::FnvHasher;

use hex2dext::algo::bfs;
use hex2d::{Coordinate, Direction, Angle, Position};
use hex2d::Angle::{Left, Right, Forward};

use generate;
use hex2dext::algo;
use game::item::Item;
use util::random_pos;

use self::tile::{Feature};

pub mod area;
pub mod actor;
pub use self::actor::Actor;
pub mod action;
pub use self::action::Action;
pub mod conts;
pub mod item;
pub mod engine;
pub use self::engine::*;
pub mod tile;
pub use self::tile::{Tile};
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


pub type Visibility = HashSet<Coordinate, DefaultState<FnvHasher>>;
pub type NoiseMap = HashMap<Coordinate, Noise, DefaultState<FnvHasher>>;
pub type Map = SimpleMap<Coordinate, Tile, DefaultState<FnvHasher>>;
pub type Actors = HashMap<Coordinate, Actor, DefaultState<FnvHasher>>;
pub type Items = HashMap<Coordinate, Box<Item>, DefaultState<FnvHasher>>;
pub type LightMap = SimpleMap<Coordinate, u32, DefaultState<FnvHasher>>;

