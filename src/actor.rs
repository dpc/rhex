use std::collections::HashSet;
use hex2d::{Coordinate, Direction, Angle};
use game;
use hex2dext::algo;

type Visibility = HashSet<Coordinate>;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Behavior {
    Player,
    Pony,
    Grue,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct State {
    pub pos : Coordinate,
    pub dir : Direction,

    pub behavior : Behavior,

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

fn calculate_los(pos : Coordinate, dir : Direction, gstate : &game::State) -> Visibility {
    let mut visibility = HashSet::new();
    algo::los::los(
        &|coord| gstate.tile_at(coord).map_or(10000, |tile| tile.opaqueness()),
        &mut |coord, _ | {
            if pos.distance(coord) < 2 || gstate.light_map.contains_key(&coord) {
                let _ = visibility.insert(coord);
            }
        },
        10, pos, &[dir]
        );

    visibility
}

impl State {
    pub fn new(behavior : Behavior, pos : Coordinate, dir : Direction, gstate : &game::State) -> State {

        let visible = calculate_los(pos, dir, gstate);

        let mut state = State {
            behavior : behavior,
            pos: pos, dir: dir,
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

    pub fn new_nolosyet(behavior : Behavior, pos : Coordinate, dir : Direction) -> State {
        State {
            behavior : behavior,
            pos: pos, dir: dir,
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
            dir: self.dir,
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
        let (pos, dir) = match action {
            game::Action::Wait => (self.pos, self.dir),
            game::Action::Turn(a) => (self.pos, self.dir + a),
            game::Action::Move(a) => (self.pos + (self.dir + a), self.dir),
            game::Action::Spin(a) => (self.pos + (self.dir + a),
                                      match a {
                                          Angle::Right => self.dir + Angle::Left,
                                          Angle::Left => self.dir + Angle::Right,
                                          _ => return self.clone(),
                                      }),
        };

        let tile_type =  gstate.tile_map_or(pos, game::tile::Wall, |t| t.type_);
        if self.pos == pos || (tile_type.is_passable() && !gstate.actors.contains_key(&pos)) {
            let visible = calculate_los(pos, dir, gstate);

            let mut state = State {
                behavior: self.behavior,
                pos: pos,
                dir: dir,
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

        for i in self.visible.iter() {
            if !self.known.contains(i) {
                self.known.insert(*i);
                discovered.insert(*i);
            }
        }

        for &coord in discovered.iter() {
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
