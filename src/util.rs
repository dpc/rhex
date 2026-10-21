use rand::{self, RngExt};
use std::cmp;

use hex2d::{Angle, Coordinate, Direction, Position};

pub fn random_pos(x: i32, y: i32) -> Position {
    let dir = Direction::from_int(rand::rng().random_range(0..6));

    Position::new(Coordinate::new(x, y), dir)
}

/// Roll a against b
pub fn roll(a: i32, b: i32) -> bool {
    let base = cmp::max(a, b) / 4;

    let a = cmp::max(a - base, 1);
    let b = cmp::max(b - base, 1);

    rand::rng().random_range(0..(a + b)) < a
}

pub fn circular_move(center: Position, cur: Position, angle: Angle) -> Position {
    let reldir = center.coord.direction_to_cw(cur.coord);

    let reldir = if let Some(reldir) = reldir {
        reldir
    } else if (angle == Angle::Left) || (angle == Angle::Right) {
        return cur + angle;
    } else {
        cur.dir
    };

    match angle {
        Angle::Forward => cur + Coordinate::from(reldir),
        Angle::Back => cur - Coordinate::from(reldir),
        Angle::Left => {
            let curdist = center.coord.distance(cur.coord);
            let c = cur.coord + (reldir + Angle::Left);
            if curdist != center.coord.distance(c) {
                let c = c - reldir;
                let newreldir = center.coord.direction_to_cw(c).unwrap_or(reldir);
                Position::new(c, newreldir)
            } else {
                Position::new(c, reldir)
            }
        }
        Angle::Right => {
            let curdist = center.coord.distance(cur.coord);
            let c = cur.coord + (reldir + Angle::Right);
            if curdist != center.coord.distance(c) {
                let c = c - reldir;
                let newreldir = center.coord.direction_to_cw(c).unwrap_or(reldir);
                Position::new(c, newreldir)
            } else {
                Position::new(c, reldir)
            }
        }
        _ => panic!(),
    }
}
