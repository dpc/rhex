//! Explicit, non-generated worlds shared by controller and rendering tests.

use crate::game::{Actor, Location, Map, actor::Race, tile};
use hex2d::{Coordinate, Position, XY};

/// An empty hex disk with a ready player facing east and explicit perception.
pub(super) fn room() -> Location {
    let origin = Coordinate::new(0, 0);
    let mut map = Map::default();
    let mut player = Actor::new(Race::Human, Position::new(origin, XY));
    player.set_player();
    for coord in origin.range_iter(8) {
        map[coord] = tile::Tile::new(tile::Empty);
        player.known.insert(coord);
        player.visible.insert(coord);
        player.in_los.insert(coord);
    }
    Location::fixture(map, player)
}
