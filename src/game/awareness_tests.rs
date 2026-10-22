use super::*;
use crate::game::{actor::Race, fixtures};
use hex2d::{Angle, Position, XY};
use rand::RngExt;

fn enemy(loc: &mut Location, coord: Coordinate, facing_player: bool) -> actor::Id {
    loc.spawn(Actor::new(
        Race::Human,
        Position::new(coord, if facing_player { XY + Angle::Back } else { XY }),
    ))
}

fn events(loc: &mut Location) -> Vec<actor::Id> {
    std::mem::take(&mut loc.awareness_events)
        .into_iter()
        .map(|event| event.actor)
        .collect()
}

#[test]
fn awareness_sight_at_spawn_and_action_boundary_is_once_and_resettable() {
    let mut loc = fixtures::room();
    let id = enemy(&mut loc, Coordinate::new(1, 0), true);
    assert!(loc.actors_byid[&id].aware_of_player);
    assert_eq!(events(&mut loc), [id]);
    assert!(!loc.player().aware_of_player);

    // Sight and actual player sound in the same boundary cannot duplicate acquisition.
    loc.act(0, Action::Turn(Angle::Right));
    assert!(events(&mut loc).is_empty());
    loc.actors_byid.get_mut(&id).unwrap().pre_own_tick();
    assert!(loc.actors_byid[&id].aware_of_player);
    loc.set_player_awareness(id, false);
    assert!(!loc.actors_byid[&id].aware_of_player);
    assert!(events(&mut loc).is_empty());
    loc.act(0, Action::Turn(Angle::Left));
    assert_eq!(events(&mut loc), [id]);
    loc.act(0, Action::Wait);
    assert!(events(&mut loc).is_empty());
}

#[test]
fn awareness_player_entry_is_seen_before_enemy_turn_without_mutating_ai_cues() {
    let mut loc = fixtures::room();
    let coord = Coordinate::new(2, 0);
    let id = enemy(&mut loc, coord, true);
    loc.set_player_awareness(id, false);
    events(&mut loc);
    let enemy = loc.actors_byid.get_mut(&id).unwrap();
    enemy.visible.clear(); // A stale AI cache must not defer actual perception.
    enemy.action_cd = 100;
    let rng = loc.rng.clone().random::<u64>();
    loc.post_any_tick();
    assert_eq!(events(&mut loc), [id]);
    assert!(loc.actors_byid[&id].visible.is_empty());
    assert_eq!(loc.actors_byid[&id].action_cd, 100);
    assert_eq!(loc.rng.clone().random::<u64>(), rng);
}

#[test]
fn awareness_initial_player_spawn_observes_existing_enemies() {
    let mut loc = fixtures::room();
    let player = loc.remove(0).unwrap();
    loc.player_id = None;
    let id = enemy(&mut loc, Coordinate::new(1, 0), true);
    assert!(!loc.actors_byid[&id].aware_of_player);
    loc.spawn_player(player);
    assert!(loc.actors_byid[&id].aware_of_player);
    assert_eq!(events(&mut loc), [id]);
}

#[test]
fn awareness_sound_uses_emitter_identity_not_race_or_ephemeral_heard_cues() {
    let mut loc = fixtures::room();
    let id = enemy(&mut loc, Coordinate::new(1, 0), false);
    let other = enemy(&mut loc, Coordinate::new(0, 1), false);
    assert!(!loc.actors_byid[&id].aware_of_player);
    events(&mut loc);
    loc.actors_byid.get_mut(&other).unwrap().noise_makes(9);
    loc.resolve_sounds();
    assert!(!loc.actors_byid[&id].heard.is_empty());
    assert!(!loc.actors_byid[&id].aware_of_player);
    assert!(events(&mut loc).is_empty());
    loc.actors_byid.get_mut(&0).unwrap().noise_makes(9);
    loc.resolve_sounds();
    assert!(loc.actors_byid[&id].aware_of_player);
    let observed = events(&mut loc);
    assert_eq!(observed.iter().filter(|&&event| event == id).count(), 1);
    loc.resolve_sounds();
    assert!(events(&mut loc).is_empty());
    loc.skip_act(id);
    assert!(loc.actors_byid[&id].heard.is_empty());
    assert!(loc.actors_byid[&id].aware_of_player);
    assert!(events(&mut loc).is_empty());
}

#[test]
fn awareness_hidden_capture_never_replays_and_unconsumed_events_expire() {
    let mut loc = fixtures::room();
    let coord = Coordinate::new(1, 0);
    let id = enemy(&mut loc, coord, false);
    loc.actors_byid.get_mut(&0).unwrap().visible.remove(&coord);
    loc.actors_byid.get_mut(&0).unwrap().noise_makes(9);
    loc.resolve_sounds();
    assert!(loc.actors_byid[&id].aware_of_player);
    assert!(events(&mut loc).is_empty());
    loc.actors_byid.get_mut(&0).unwrap().visible.insert(coord);
    loc.post_any_tick();
    assert!(events(&mut loc).is_empty());
    loc.set_player_awareness(id, false);
    loc.actors_byid.get_mut(&0).unwrap().noise_makes(9);
    loc.resolve_sounds();
    assert_eq!(loc.awareness_events.len(), 1);
    loc.skip_act(0);
    assert!(events(&mut loc).is_empty());
}

#[test]
fn awareness_requires_living_participants_and_actual_unoccluded_sight_or_sound() {
    let mut loc = fixtures::room();
    let id = enemy(&mut loc, Coordinate::new(2, 0), false);
    loc.actors_byid.get_mut(&id).unwrap().pos.dir = XY + Angle::Back;
    loc.map[Coordinate::new(1, 0)] = tile::Tile::new(tile::Wall);
    loc.post_any_tick();
    assert!(!loc.actors_byid[&id].aware_of_player);
    loc.map[Coordinate::new(1, 0)] = tile::Tile::new(tile::Empty);
    for coord in loc.map.iter().map(|(coord, _)| *coord).collect::<Vec<_>>() {
        loc.map[coord].light = 0;
    }
    loc.actors_byid.get_mut(&id).unwrap().base_stats.infravision = 0;
    loc.post_any_tick();
    assert!(!loc.actors_byid[&id].aware_of_player);
    loc.actors_byid.get_mut(&id).unwrap().hp = 0;
    loc.actors_byid.get_mut(&0).unwrap().noise_makes(9);
    loc.post_any_tick();
    loc.set_player_awareness(id, true);
    assert!(!loc.actors_byid[&id].aware_of_player);
    assert!(events(&mut loc).is_empty());

    let living = enemy(&mut loc, Coordinate::new(1, 0), false);
    loc.actors_byid.get_mut(&0).unwrap().hp = 0;
    loc.actors_byid.get_mut(&0).unwrap().noise_makes(9);
    loc.post_any_tick();
    assert!(!loc.actors_byid[&living].aware_of_player);
    assert!(!loc.player().aware_of_player);
    assert!(events(&mut loc).is_empty());
}
