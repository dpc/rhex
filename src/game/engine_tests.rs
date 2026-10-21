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
