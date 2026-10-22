use super::*;
use crate::game::{
    fixtures,
    item::{Item, Type},
    tile,
};
use hex2d::{Angle, Coordinate, Position, XY};

fn enemy(loc: &mut Location, coord: Coordinate) -> actor::Id {
    loc.spawn(Actor::new(actor::Race::Rat, Position::new(coord, XY)))
}

#[test]
fn player_first_then_each_spawn_id_once_with_cooldown_and_death() {
    let mut loc = fixtures::room();
    let first = enemy(&mut loc, Coordinate::new(-3, 0));
    let dead = enemy(&mut loc, Coordinate::new(-2, 0));
    let last = enemy(&mut loc, Coordinate::new(-1, 0));
    loc.actors_byid.get_mut(&first).unwrap().action_cd = 2;
    loc.actors_byid.get_mut(&dead).unwrap().hp = 0;
    loc.post_any_tick();
    let mut engine = Engine::fixture(loc);
    assert!(engine.is_turn_done());
    assert!(!engine.needs_player_input());
    engine.start_turn();
    assert_eq!(engine.turn(), 1);
    assert!(engine.needs_player_input());
    assert_eq!(engine.current_location().actors_byid[&first].action_cd, 2);
    engine.player_act(Action::Turn(Angle::Right));
    assert_eq!(engine.player().pos.dir, XY + Angle::Right);
    assert!(!engine.needs_player_input());
    assert_eq!(engine.one_actor_tick(), first);
    assert_eq!(engine.current_location().actors_byid[&first].action_cd, 1);
    assert_eq!(engine.one_actor_tick(), last);
    assert!(engine.is_turn_done());
    assert!(engine.ids_to_move.is_empty());
    fixtures::occupancy(engine.current_location());

    engine
        .current_location_mut()
        .actors_byid
        .get_mut(&0)
        .unwrap()
        .action_cd = 2;
    engine.start_turn();
    assert!(!engine.needs_player_input());
    engine.player_skip_act();
    assert_eq!(engine.player().action_cd, 1);
    while !engine.is_turn_done() {
        engine.one_actor_tick();
    }
    engine
        .current_location_mut()
        .actors_byid
        .get_mut(&0)
        .unwrap()
        .hp = 0;
    engine.start_turn();
    assert!(!engine.needs_player_input());
    engine.player_skip_act();
    while !engine.is_turn_done() {
        engine.one_actor_tick();
    }
    fixtures::occupancy(engine.current_location());
}

#[test]
fn actor_killed_after_schedule_does_not_act_and_loot_drops_once() {
    let mut loc = fixtures::room();
    let id = enemy(&mut loc, Coordinate::new(1, 0));
    loc.actors_byid
        .get_mut(&id)
        .unwrap()
        .pick_item(Box::new(Item::new(Type::Knife, vec![])));
    let mut engine = Engine::fixture(loc);
    engine.start_turn();
    engine.player_act(Action::Wait);
    engine
        .current_location_mut()
        .actors_byid
        .get_mut(&id)
        .unwrap()
        .hp = 0;
    assert_eq!(engine.one_actor_tick(), id);
    assert!(engine.is_turn_done());
    let loc = engine.current_location_mut();
    assert_eq!(loc.items.len(), 1);
    loc.post_any_tick();
    assert_eq!(loc.items.len(), 1);
    assert!(loc.actors_byid[&id].items_backpack.is_empty());
    fixtures::occupancy(loc);
}

#[test]
fn descent_transfers_one_player_and_discards_old_schedule() {
    let mut loc = fixtures::room();
    // Pending IDs intentionally exceed the next level's population.
    loc.actors_counter = 10_000;
    enemy(&mut loc, Coordinate::new(-2, 0));
    loc.map[Coordinate::new(0, 0)].feature = Some(tile::Stairs);
    loc.actors_byid
        .get_mut(&0)
        .unwrap()
        .pick_item(Box::new(Item::new(Type::Bow, vec![])));
    let hp = loc.player().hp;
    let mut engine = Engine::fixture(loc);
    engine.start_turn();
    engine.player_act(Action::Descend);
    assert!(engine.locations[0].sound_events.is_empty());
    assert!(engine.is_turn_done());
    assert!(engine.ids_to_move.is_empty());
    assert_eq!(engine.locations.len(), 2);
    assert_eq!(engine.location_cur, 1);
    assert!(
        !engine.locations[0]
            .actors_byid
            .values()
            .any(Actor::is_player)
    );
    assert_eq!(
        engine
            .current_location()
            .actors_byid
            .values()
            .filter(|a| a.is_player())
            .count(),
        1
    );
    assert_eq!(engine.player().hp, hp);
    assert_eq!(engine.player().items_backpack[&'a'].description(), "bow");
    assert!(!engine.player().descended());
    assert_eq!(engine.current_location().level, 1);
    engine.start_turn();
    engine.player_act(Action::Wait);
    while !engine.is_turn_done() {
        assert!(engine.one_actor_tick() < 10_000);
    }
    fixtures::occupancy(engine.current_location());
}

fn traverse(engine: &mut Engine) {
    assert!(engine.is_turn_done());
    engine.start_turn();
    engine.player_act(Action::Descend);
    assert!(engine.is_turn_done());
    assert!(engine.ids_to_move.is_empty());
    fixtures::occupancy(engine.current_location());
    assert_eq!(
        engine
            .locations
            .iter()
            .flat_map(|loc| loc.actors_byid.values())
            .filter(|actor| actor.is_player())
            .count(),
        1
    );
}

fn place_on_exit(engine: &mut Engine, coord: Coordinate) {
    let loc = engine.current_location_mut();
    assert!(!loc.actors_coord_to_id.contains_key(&coord));
    let id = loc.player_id();
    let actor = loc.actors_byid.get_mut(&id).unwrap();
    loc.actors_coord_to_id.remove(&actor.pos.coord);
    actor.pos.coord = coord;
    loc.actors_coord_to_id.insert(coord, id);
}

#[test]
fn reciprocal_links_preserve_three_levels_and_rng_across_repeated_roundtrips() {
    let mut loc = fixtures::room();
    let origin = Coordinate::new(0, 0);
    loc.map[origin].feature = Some(tile::Stairs);
    let enemy_id = enemy(&mut loc, Coordinate::new(-2, 0));
    let enemy = loc.actors_byid.get_mut(&enemy_id).unwrap();
    enemy.aware_of_player = true;
    enemy.action_cd = 17;
    let memory = Coordinate::new(-50, 25);
    let player = loc.actors_byid.get_mut(&0).unwrap();
    player.known.insert(memory);
    player.pick_item(Box::new(Item::new(Type::Bow, vec![])));
    let hp = player.hp;
    let mut engine = Engine::fixture(loc);
    traverse(&mut engine);
    assert_eq!(
        engine.current_location().biome,
        super::super::level::Biome::DarkForest
    );
    let old_rng: u64 = engine.locations[0].rng.clone().random();
    assert!(!engine.player().known.contains(&memory));
    let forest_id = engine.current_location().id;
    let forward = engine
        .current_location()
        .map
        .iter()
        .find(|(_, t)| t.feature == Some(tile::ForestPath))
        .map(|(&c, _)| c)
        .unwrap();
    let modified = Coordinate::new(-14, 0);
    engine.current_location_mut().map[modified] = tile::Tile::new(tile::Ground);
    engine
        .current_location_mut()
        .items
        .insert(modified, Box::new(Item::new(Type::Knife, vec![])));
    place_on_exit(&mut engine, forward);
    traverse(&mut engine);
    assert_eq!(
        engine.current_location().biome,
        super::super::level::Biome::Dungeon
    );
    assert_eq!(engine.current_location().id, LevelId(2));
    // Stable identity, not depth, controls the connection and current location.
    engine.current_location_mut().level = 91;
    assert_eq!(
        engine.current_location().exits[&origin],
        ExitLink {
            destination: forest_id,
            arrival: forward
        }
    );
    traverse(&mut engine);
    assert_eq!(engine.current_location().id, forest_id);
    assert_eq!(engine.player().coord(), forward);
    assert_eq!(engine.current_location().map[modified].type_, tile::Ground);
    assert_eq!(
        engine.current_location().items[&modified].description(),
        "knife"
    );
    place_on_exit(&mut engine, origin);
    traverse(&mut engine);
    assert_eq!(engine.location_cur, 0);
    assert_eq!(engine.player().coord(), origin);
    assert!(engine.player().known.contains(&memory));
    assert!(engine.current_location().actors_byid[&enemy_id].aware_of_player);
    assert_eq!(
        engine.current_location().actors_byid[&enemy_id].action_cd,
        17
    );
    assert_eq!(
        engine.current_location().rng.clone().random::<u64>(),
        old_rng
    );
    for _ in 0..8 {
        traverse(&mut engine);
        assert_eq!(engine.player().coord(), origin);
        assert_eq!(engine.player().hp, hp);
        assert_eq!(engine.player().items_backpack[&'a'].description(), "bow");
        assert!(engine.player().heard.is_empty());
        assert!(engine.player().pending_noises.is_empty());
        assert!(engine.current_location().combat_events.is_empty());
    }
    assert_eq!(engine.locations.len(), 3);
}

#[test]
fn occupied_return_searches_without_overwrite_and_full_arrival_refuses() {
    let mut loc = fixtures::room();
    let origin = Coordinate::new(0, 0);
    loc.map[origin].feature = Some(tile::Stairs);
    let mut engine = Engine::fixture(loc);
    traverse(&mut engine);
    let blocker = enemy(&mut engine.locations[0], origin);
    traverse(&mut engine);
    assert_eq!(engine.location_cur, 0);
    assert_ne!(engine.player().coord(), origin);
    assert_eq!(
        engine.current_location().actors_coord_to_id[&origin],
        blocker
    );
    assert_eq!(
        engine.current_location().actors_byid[&blocker].coord(),
        origin
    );

    // Build a full destination component, retaining the blocker and all actors.
    let mut engine = Engine::fixture(fixtures::room());
    engine.current_location_mut().map[origin].feature = Some(tile::Stairs);
    traverse(&mut engine);
    let inactive = &mut engine.locations[0];
    enemy(inactive, origin);
    for c in origin.neighbors() {
        inactive.map[c] = tile::Tile::new(tile::TreeTrunk);
    }
    let population = inactive.actors_byid.len();
    engine.start_turn();
    engine.player_act(Action::Descend);
    assert_eq!(engine.location_cur, 1);
    assert_eq!(engine.locations[0].actors_byid.len(), population);
    fixtures::occupancy(engine.current_location());
    while !engine.is_turn_done() {
        engine.one_actor_tick();
    }
}
