use hex2d as h2d;

use std::fmt;

pub use self::Type::*;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Type {
    Room(u32),
}

impl fmt::Display for Type {
    fn fmt(&self, fmt : &mut fmt::Formatter) -> Result<(), fmt::Error> {

        match *self {
            Type::Room(r) => {
                if r < 3 {
                    fmt.write_str("small room")
                } else if r < 5 {
                    fmt.write_str("room")
                } else {
                    fmt.write_str("big room")
                }
            }
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Area {
    pub center: h2d::Coordinate,
    pub type_ : Type
}

impl Area {
    pub fn new(center : h2d::Coordinate, type_ : Type) -> Area {
        Area { center: center, type_: type_ }
    }
}
