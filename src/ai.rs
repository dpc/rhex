use rand::RngExt;

use crate::hex2dext::algo::bfs;

use crate::game;
use crate::game::actor;
use hex2d::Angle::{Back, Forward, Left, LeftBack, Right};
use hex2d::Coordinate;

#[cfg(test)]
#[path = "ai_tests.rs"]
mod tests;

pub trait Ai {
    fn action(
        &mut self,
        id: actor::Id,
        location: &game::Location,
        rng: &mut impl rand::Rng,
    ) -> game::Action;
}

pub struct Simple;

impl Ai for Simple {
    fn action(
        &mut self,
        id: actor::Id,
        location: &game::Location,
        rng: &mut impl rand::Rng,
    ) -> game::Action {
        grue(&location.actors_byid[&id], location, rng)
    }
}

fn roam(rng: &mut impl rand::Rng) -> game::Action {
    match rng.random_range(0..10) {
        0 => game::Action::Turn(Right),
        1 => game::Action::Turn(Left),
        2 => game::Action::Move(Forward),
        _ => game::Action::Wait,
    }
}

fn closest_reachable<F>(
    gstate: &game::Location,
    start: Coordinate,
    max_distance: i32,
    beast: bool,
    cond: F,
) -> Option<(Coordinate, Coordinate)>
where
    F: Fn(Coordinate) -> bool,
{
    let mut bfs = bfs::Traverser::new(
        |pos| {
            pos == start
                || (gstate.at(pos).tile().is_passable()
                    && (!beast || gstate.map[pos].feature != Some(game::tile::Door(false)))
                    && pos.distance(start) < max_distance
                    && !gstate.at(pos).is_occupied())
        },
        cond,
        start,
    );
    bfs.find()
        .map(|pos| (pos, bfs.backtrace_last(pos).unwrap()))
}

fn grue(astate: &actor::Actor, gstate: &game::Location, rng: &mut impl rand::Rng) -> game::Action {
    // Equally eligible cues use coordinate order, not hash-table iteration.
    let mut visible: Vec<_> = astate.visible.iter().copied().collect();
    visible.sort_by_key(|c| (c.x, c.y));
    for &visible_pos in &visible {
        if gstate
            .at(visible_pos)
            .actor_map_or(false, |a| a.is_player())
        {
            return go_to(visible_pos, astate, gstate, rng);
        }
    }

    if !astate.race.is_beast() && gstate.at(astate.head()).item_map_or(false, |_| true) {
        return game::Action::Pick;
    }

    if !astate.race.is_beast() && gstate.at(astate.pos.coord).item_map_or(false, |_| true) {
        return game::Action::Move(Back);
    }

    for &visible_coord in &visible {
        if !astate.race.is_beast() && gstate.at(visible_coord).item_map_or(false, |_| true) {
            return go_to(visible_coord, astate, gstate, rng);
        }
    }

    let mut heard: Vec<_> = astate.heard.keys().copied().collect();
    heard.sort_by_key(|c| (c.x, c.y));
    for coord in heard {
        if astate.pos.coord != coord {
            return go_to(coord, astate, gstate, rng);
        }
    }

    match rng.random_range(0..5) {
        0 => roam(rng),
        _ => game::Action::Wait,
    }
}

fn go_to(
    c: Coordinate,
    astate: &actor::Actor,
    gstate: &game::Location,
    rng: &mut impl rand::Rng,
) -> game::Action {
    let ndir = match astate.pos.coord.direction_to_cw(c) {
        None => return game::Action::Wait,
        Some(dir) => dir,
    };

    let n_pos = astate.pos + Coordinate::from(ndir);
    let can_step = |pos| {
        gstate.at(pos).tile().is_passable()
            && (!astate.race.is_beast() || gstate.map[pos].feature != Some(game::tile::Door(false)))
    };
    if can_step(n_pos.coord) {
        if ndir == astate.pos.dir {
            return game::Action::Move(Forward);
        } else {
            let rdir = ndir - astate.pos.dir;
            // Strafe into free cells, but face occupants before attacking.
            if matches!(rdir, Left | Right) && !gstate.at(n_pos.coord).is_occupied() {
                return game::Action::Move(rdir);
            }
            let rdir = match rdir {
                Left | LeftBack => Left,
                Back => {
                    if astate.pos.coord.x & 1 == 0 {
                        Left
                    } else {
                        Right
                    }
                }
                _ => Right,
            };
            return game::Action::Turn(rdir);
        }
    }
    // TODO: fallaback to A* instead of BFS
    let reachable = closest_reachable(
        gstate,
        astate.pos.coord,
        10,
        astate.race.is_beast(),
        |pos| pos == c,
    );

    // BFS may accept a blocked goal before checking traversability. Recursing
    // on that adjacent wall would retry the same step forever.
    if let Some((_, n)) = reachable.filter(|(_, n)| can_step(*n)) {
        go_to(n, astate, gstate, rng)
    } else {
        roam(rng)
    }
}

fn _pony_follow(
    astate: &actor::Actor,
    gstate: &game::Location,
    rng: &mut impl rand::Rng,
) -> game::Action {
    let start = astate.pos.coord;

    let player_pos = closest_reachable(gstate, start, 10, astate.race.is_beast(), |pos| {
        gstate.at(pos).actor_map_or(false, |a| a.is_player())
    });

    let player_pos = if let Some((dst, _)) = player_pos {
        let distance = dst.distance(start);
        if distance < 3 {
            closest_reachable(gstate, start, 10, astate.race.is_beast(), |pos| {
                pos.distance(dst) == 3 && gstate.at(pos).is_passable()
            })
        } else if distance < 5 {
            None
        } else {
            player_pos
        }
    } else {
        player_pos
    };

    if let Some((_, neigh)) = player_pos {
        go_to(neigh, astate, gstate, rng)
    } else {
        roam(rng)
    }
}
