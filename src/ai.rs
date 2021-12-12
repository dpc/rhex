use rand;
use rand::Rng;

use hex2dext::algo::bfs;

use hex2d::Coordinate;
use hex2d::Angle::{Left, Right, Forward, Back, LeftBack};
use game;
use game::actor;

pub trait Ai {
    fn action(&mut self, id: actor::Id, engine: &game::Engine) -> game::Action;
}

pub struct Simple;

impl Ai for Simple {
    fn action(&mut self, id: actor::Id, engine: &game::Engine) -> game::Action {
        grue(&engine.current_location().actors_byid[&id],
             engine.current_location())
    }
}

fn roam() -> game::Action {
    match rand::thread_rng().gen_range(0..10) {
        0 => game::Action::Turn(Right),
        1 => game::Action::Turn(Left),
        2 => game::Action::Move(Forward),
        _ => game::Action::Wait,
    }
}

fn closest_reachable<F>(gstate: &game::Location,
                        start: Coordinate,
                        max_distance: i32,
                        cond: F)
                        -> Option<(Coordinate, Coordinate)>
    where F: Fn(Coordinate) -> bool
{
    let mut bfs = bfs::Traverser::new(|pos| {
                                          pos == start ||
                                          (gstate.at(pos).tile().is_passable() &&
                                           pos.distance(start) < max_distance &&
                                           !gstate.at(pos).is_occupied())
                                      },
                                      cond,
                                      start);
    bfs.find().map(|pos| (pos, bfs.backtrace_last(pos).unwrap()))
}

fn grue(astate: &actor::Actor, gstate: &game::Location) -> game::Action {

    for &visible_pos in &astate.visible {
        if gstate.at(visible_pos).actor_map_or(false, |a| a.is_player()) {
            return go_to(visible_pos, astate, gstate);
        }
    }

    if gstate.at(astate.head()).item_map_or(false, |_| true) {
        return game::Action::Pick;
    }

    if gstate.at(astate.pos.coord).item_map_or(false, |_| true) {
        return game::Action::Move(Back);
    }

    for &visible_coord in &astate.visible {
        if gstate.at(visible_coord).item_map_or(false, |_| true) {
            return go_to(visible_coord, astate, gstate);
        }
    }

    for (&coord, _) in &astate.heard {
        if astate.pos.coord != coord {
            return go_to(coord, astate, gstate);
        }
    }

    match rand::thread_rng().gen_range(0..5) {
        0 => roam(),
        _ => game::Action::Wait,
    }
}

fn go_to(c: Coordinate, astate: &actor::Actor, gstate: &game::Location) -> game::Action {
    let ndir = match astate.pos.coord.direction_to_cw(c) {
        None => return game::Action::Wait,
        Some(dir) => dir,
    };

    let n_pos = astate.pos + Coordinate::from(ndir);
    if gstate.at(n_pos.coord).tile().type_.is_passable() {
        if ndir == astate.pos.dir {
            return game::Action::Move(Forward);
        } else {
            let rdir = ndir - astate.pos.dir;
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
    let reachable = closest_reachable(gstate, astate.pos.coord, 10, |pos| pos == c);

    if let Some((_, n)) = reachable {
        go_to(n, astate, gstate)
    } else {
        roam()
    }
}

fn _pony_follow(astate: &actor::Actor, gstate: &game::Location) -> game::Action {
    let start = astate.pos.coord;

    let player_pos = closest_reachable(gstate,
                                       start,
                                       10,
                                       |pos| gstate.at(pos).actor_map_or(false, |a| a.is_player()));

    let player_pos = if let Some((dst, _)) = player_pos {
        let distance = dst.distance(start);
        if distance < 3 {
            closest_reachable(gstate,
                              start,
                              10,
                              |pos| pos.distance(dst) == 3 && gstate.at(pos).is_passable())
        } else if distance < 5 {
            None
        } else {
            player_pos
        }
    } else {
        player_pos
    };

    if let Some((_, neigh)) = player_pos {
        go_to(neigh, astate, gstate)
    } else {
        roam()
    }
}
