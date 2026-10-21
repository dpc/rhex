use super::*;
use crate::game::{
    Action, Actor, Noise, fixtures,
    item::{Item, Type},
    tile,
};
use hex2d::{Position, XY};
use rand::{SeedableRng, rngs::SmallRng};

#[test]
fn visible_player_precedes_items_and_heard_targets() {
    let mut loc = fixtures::room();
    let coord = Coordinate::new(-1, 1);
    let id = loc.spawn(Actor::new(actor::Race::Rat, Position::new(coord, XY)));
    loc.items
        .insert(coord, Box::new(Item::new(Type::Knife, vec![])));
    let mut rng = SmallRng::seed_from_u64(0);
    assert_eq!(Simple.action(id, &loc, &mut rng), Action::Move(Forward));
    let actor = loc.actors_byid.get_mut(&id).unwrap();
    actor.visible.clear();
    actor.heard.clear();
    let head = actor.head();
    loc.items
        .insert(head, Box::new(Item::new(Type::Bow, vec![])));
    assert_eq!(Simple.action(id, &loc, &mut rng), Action::Pick);
    loc.items.remove(&head);
    assert_eq!(Simple.action(id, &loc, &mut rng), Action::Move(Back));
    loc.items.remove(&coord);
    loc.actors_byid
        .get_mut(&id)
        .unwrap()
        .heard
        .insert(head, Noise::Creature(actor::Race::Human));
    assert_eq!(Simple.action(id, &loc, &mut rng), Action::Move(Forward));
}

#[test]
fn cue_ties_use_coordinate_order_and_blocked_fallback_is_seeded() {
    let mut loc = fixtures::room();
    let id = loc.spawn(Actor::new(
        actor::Race::Rat,
        Position::new(Coordinate::new(3, -3), XY),
    ));
    let actor = loc.actors_byid.get_mut(&id).unwrap();
    actor.visible.clear();
    actor.heard.clear();
    let first = Coordinate::new(2, -3);
    let second = Coordinate::new(4, -3);
    actor
        .heard
        .insert(second, Noise::Creature(actor::Race::Human));
    actor
        .heard
        .insert(first, Noise::Creature(actor::Race::Human));
    let mut rng = SmallRng::seed_from_u64(42);
    assert_eq!(Simple.action(id, &loc, &mut rng), Action::Turn(Right));
    // Enclose the actor; fallback may legally choose a blocked move.
    let center = loc.actors_byid[&id].coord();
    for neighbor in center.neighbors() {
        loc.map[neighbor] = tile::Tile::new(tile::Wall);
    }
    let mut actions = Vec::new();
    for seed in 0..16 {
        let mut a = SmallRng::seed_from_u64(seed);
        let mut b = SmallRng::seed_from_u64(seed);
        let action = Simple.action(id, &loc, &mut a);
        assert_eq!(Simple.action(id, &loc, &mut b), action);
        assert!(matches!(
            action,
            Action::Wait | Action::Turn(Left | Right) | Action::Move(Forward)
        ));
        actions.push(action);
    }
    assert!(actions.windows(2).any(|a| a[0] != a[1]));
}
