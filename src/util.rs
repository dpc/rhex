use std::{cmp};
use rand::{self, Rng};

use hex2d::{Position, Direction, Coordinate};

pub fn random_pos(x : i32, y : i32) -> Position {

    let dir = Direction::from_int(rand::thread_rng().gen_range(0, 6));

    Position::new(Coordinate::new(x, y), dir)
}


/// Roll a against b
pub fn roll(a : i32, b : i32) -> bool {

    let base = cmp::max(a, b) / 3;

    let a = cmp::max(a - base, 0);
    let b = cmp::max(b - base, 0);

    let (a, b) = if (a, b) == (0, 0) {
        (1, 1)
    } else {
        (a, b)
    };

    rand::thread_rng().gen_range(0, a+b) < a
}
