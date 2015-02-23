use rand;
use rand::Rng;
use std::sync::{mpsc};

use hex2dext::algo::bfs;

use hex2d::{self, Coordinate, ToCoordinate};
use hex2d as h2d;
use game;
use actor;
use error::Error;

fn roam() -> game::Action {
    match rand::thread_rng().gen_range(0, 10) {
        0 => game::Action::Turn(h2d::Angle::Right),
        1 => game::Action::Turn(h2d::Angle::Left),
        2 => game::Action::Move(h2d::Angle::Forward),
        _ => game::Action::Wait,
    }
}

fn closest_reachable<F>(gstate : &game::State, start : Coordinate, max_distance : i32, cond : F) -> Option<(Coordinate, Coordinate)>
    where F : Fn(Coordinate) -> bool
{
    let mut bfs = bfs::Traverser::new(
        |pos| pos == start || (gstate.at(pos).tile_map_or(false, |t| t.is_passable())
                               && pos.distance(start) < max_distance && !gstate.at(pos).is_occupied()),
                               cond,
                               start
                               );
    bfs.find().map(|pos| (pos, bfs.backtrace_last(pos).unwrap()))
}

fn grue(astate : &actor::State, gstate : &game::State) -> game::Action {

    for &visible_pos in &astate.visible {
        if gstate.at(visible_pos).actor_map_or(false, |a| a.is_player()) {
            return go_to(visible_pos, astate, gstate);
        }
    }

    roam()
}

fn go_to(c: Coordinate, astate : &actor::State, gstate : &game::State) -> game::Action {
    let ndir = astate.pos.coord.direction_to_cw(c).expect("bfs gave me trash");

    let n_pos = astate.pos + ndir.to_coordinate();
    if gstate.at(n_pos.coord).tile_map_or(false, |t| t.type_.is_passable()) {
        if ndir == astate.pos.dir {
            return game::Action::Move(hex2d::Angle::Forward)
        } else {
            return game::Action::Turn(ndir - astate.pos.dir)
        }
    }
    //TODO: fallaback to A* instead of BFS
    let reachable = closest_reachable(gstate, astate.pos.coord, 10,
                                   |pos| pos == c
                                  );

    if let Some((_, n)) = reachable {
        go_to(n, astate, gstate)
    } else {
        roam()
    }
}

fn pony_follow(astate : &actor::State, gstate : &game::State) -> game::Action {

        let start = astate.pos.coord;

        let player_pos = closest_reachable(gstate, start, 10,
            |pos| gstate.at(pos).actor_map_or(false, |a| a.is_player())
            );

        let player_pos = if let Some((dst, _)) = player_pos {
            let distance = dst.distance(start);
            if distance < 3 {
                closest_reachable(gstate, start, 10, |pos| pos.distance(dst) == 3 && gstate.at(pos).is_passable())
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

pub fn run(
    req : mpsc::Receiver<game::controller::Request>,
    rep : mpsc::Sender<game::controller::Reply>
    ) -> Result<(), Error<game::controller::Reply>>
{

    loop {
        let (ref astate, ref gstate) = try!(req.recv());

        let action = match astate.behavior {
            actor::Behavior::Grue => grue(&astate, &gstate),
            actor::Behavior::Pony => pony_follow(&astate, &gstate),
            _ => panic!(),
        };

        try!(rep.send((astate.clone(), action)));
    }
}
