use std::collections::HashSet;
use hex2d::{Coordinate, Angle, Position, ToCoordinate};
use game;
use hex2dext::algo;

type Visibility = HashSet<Coordinate>;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Behavior {
    Player,
    Pony,
    Grue,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Stats {
    pub int : i32,
    pub dex : i32,
    pub str_ : i32,
    pub max_hp : i32,
    pub max_mp : i32,
    pub hp: i32,
    pub mp: i32,
}


impl Stats {
    pub fn new() -> Stats {
        Stats { int: 3, dex : 3, str_ : 3,
        max_hp: 3, max_mp: 3, mp: 3, hp: 3 }
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct State {
    pub pos : Position,

    pub behavior : Behavior,
    pub stats : Stats,

    /// Currently visible
    pub visible: Visibility,

    /// Known coordinates
    pub known: Visibility,
    /// Known areas
    pub known_areas: Visibility,

    /// Discovered in the last LoS
    pub discovered: Visibility,
    /// Just discovered areas
    pub discovered_areas: Visibility,

    pub light : u32,
}

fn calculate_los(pos : Position, gstate : &game::State) -> Visibility {
    let mut visibility = HashSet::new();
    algo::los::los(
        &|coord| gstate.tile_at(coord).map_or(10000, |tile| tile.opaqueness()),
        &mut |coord, _ | {
            if pos.coord.distance(coord) < 2 || gstate.light_map.contains_key(&coord) {
                let _ = visibility.insert(coord);
            }
        },
        10, pos.coord, &[pos.dir]
        );

    visibility
}

impl State {
    pub fn new(behavior : Behavior, pos : Position, gstate : &game::State) -> State {

        let visible = calculate_los(pos, gstate);

        let mut state = State {
            behavior : behavior,
            pos: pos,
            stats: Stats::new(),
            visible: visible,
            known: HashSet::new(),
            known_areas: HashSet::new(),
            discovered: HashSet::new(),
            discovered_areas: HashSet::new(),
            light: 0,
        };

        state.postprocess_visibile(gstate);

        state
    }

    pub fn new_nolosyet(behavior : Behavior, pos : Position) -> State {
        State {
            behavior : behavior,
            pos: pos,
            stats: Stats::new(),
            visible: HashSet::new(),
            known: HashSet::new(),
            known_areas: HashSet::new(),
            discovered: HashSet::new(),
            discovered_areas: HashSet::new(),
            light: 0,
        }
    }

    pub fn add_light(&self, light : u32) -> State {
        State {
            behavior: self.behavior,
            pos: self.pos,
            stats: Stats::new(),
            visible: self.visible.clone(),
            known: self.known.clone(),
            known_areas: self.known_areas.clone(),
            discovered: self.discovered.clone(),
            discovered_areas: self.discovered_areas.clone(),
            light: light,
        }
    }

    pub fn sees(&self, pos : Coordinate) -> bool {
        self.visible.contains(&pos)
    }

    pub fn knows(&self, pos : Coordinate) -> bool {
        self.known.contains(&pos)
    }

    pub fn act(&self, gstate : &game::State, action : game::Action) -> State {
        let pos = match action {
            game::Action::Wait => self.pos,
            game::Action::Turn(a) => self.pos + a,
            game::Action::Move(a) => self.pos + (self.pos.dir + a).to_coordinate(),
            game::Action::Spin(a) => self.pos + (self.pos.dir + a).to_coordinate() +
                                      match a {
                                          Angle::Right => Angle::Left,
                                          Angle::Left => Angle::Right,
                                          _ => return self.clone(),
                                      },
        };

        let tile_type =  gstate.tile_map_or(pos.coord, game::tile::Wall, |t| t.type_);
        if self.pos.coord == pos.coord || (tile_type.is_passable() && !gstate.actors.contains_key(&pos.coord)) {
            let visible = calculate_los(pos, gstate);

            let mut state = State {
                behavior: self.behavior,
                pos: pos,
                stats: self.stats,
                visible: visible,
                known: self.known.clone(),
                known_areas: self.known_areas.clone(),
                discovered: HashSet::new(),
                discovered_areas: HashSet::new(),
                light: self.light,
            };

            state.postprocess_visibile(gstate);

            state
        } else {
            self.clone()
        }
    }

    pub fn postprocess_visibile(&mut self, gstate : &game::State) {

        let mut discovered = HashSet::new();
        let mut discovered_areas = HashSet::new();

        for i in &self.visible {
            if !self.known.contains(i) {
                self.known.insert(*i);
                discovered.insert(*i);
            }
        }

        for &coord in &discovered {
            if let Some(area) = gstate.tile_at(coord).and_then(|t| t.area) {
                let area_center = area.center;

                if !self.known_areas.contains(&area_center) {
                    self.known_areas.insert(area_center);
                    discovered_areas.insert(area_center);
                }
            }
        }

        self.discovered_areas = discovered_areas;
        self.discovered = discovered;
    }

    pub fn is_player(&self) -> bool {
        self.behavior == Behavior::Player
    }

}
