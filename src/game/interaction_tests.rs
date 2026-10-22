//! Front-cell interaction through the normal location and engine boundaries.

use super::*;
use actor::Slot;
use hex2d::{Angle, Coordinate, Position, XY};
use item::{Item, Type};

fn item() -> Box<Item> {
    Box::new(Item::new(Type::Cloak, vec![]))
}

#[test]
fn interaction_toggles_doors_without_movement_attack_or_equipment_noise() {
    for &dir in hex2d::Direction::all() {
        let mut loc = fixtures::room();
        loc.actors_byid.get_mut(&0).unwrap().pos.dir = dir;
        let pos = loc.player().pos;
        let target = loc.player().head();
        loc.map[target].feature = Some(tile::Door(false));
        let player = loc.actors_byid.get_mut(&0).unwrap();
        player.items_equipped.insert(
            Slot::RHand,
            ('a', Box::new(Item::new(Type::Pickaxe, vec![]))),
        );
        player
            .items_equipped
            .insert(Slot::Body, ('b', Box::new(Item::new(Type::Plate, vec![]))));
        player.recalculate_stats();
        let sp = loc.player().sp;
        for open in [true, false] {
            assert_eq!(loc.player().pos_after_action(Action::Interact), vec![pos]);
            loc.act(0, Action::Interact);
            assert_eq!(loc.map[target].feature, Some(tile::Door(open)));
            assert_eq!(loc.player().pos, pos);
            assert_eq!(loc.player().sp, sp);
            assert!(loc.player().acted);
            assert!(loc.combat_events.is_empty());
            assert_eq!(loc.sound_events.len(), 1);
            let sound = &loc.sound_events[0];
            assert!(sound.hidden_source.is_none());
            assert!(loc.player().heard.contains_key(&pos.coord));
            // Exactly the existing level-3 door field, using post-toggle terrain.
            let expected = sound::propagate(
                &loc.map,
                &loc.items,
                &sound::Emission {
                    source: pos.coord,
                    kind: Noise::Creature(actor::Race::Human),
                    level: 3,
                },
            );
            assert_eq!(
                sound.cells,
                expected
                    .iter()
                    .filter(|cell| cell.coord != pos.coord && loc.player().sees(cell.coord))
                    .copied()
                    .collect::<Vec<_>>()
            );
            fixtures::occupancy(&loc);
        }
    }
}

#[test]
fn interaction_item_priority_full_inventory_and_pick_compatibility() {
    for action in [Action::Interact, Action::Pick] {
        for open in [false, true] {
            for full in [false, true] {
                let mut loc = fixtures::room();
                let target = loc.player().head();
                let pos = loc.player().pos;
                loc.map[target].feature = Some(tile::Door(open));
                loc.items.insert(target, item());
                loc.items.insert(pos.coord, item());
                if full {
                    for ch in (b'a'..=b'y').chain(b'A'..=b'Y') {
                        loc.actors_byid
                            .get_mut(&0)
                            .unwrap()
                            .items_backpack
                            .insert(ch as char, item());
                    }
                }
                loc.act(0, action);
                assert_eq!(loc.player().pos, pos);
                assert_eq!(loc.map[target].feature, Some(tile::Door(open)));
                assert_eq!(loc.items.contains_key(&target), full);
                assert!(loc.items.contains_key(&pos.coord));
                assert_eq!(loc.items.len(), if full { 2 } else { 1 });
                assert_eq!(loc.player().items_backpack.len(), if full { 50 } else { 1 });
                assert!(loc.sound_events.is_empty());
                assert!(loc.player().acted);
            }
        }
        let mut loc = fixtures::room();
        let target = loc.player().head();
        loc.map[target].feature = Some(tile::Door(false));
        loc.act(0, action);
        assert_eq!(
            loc.map[target].feature,
            Some(tile::Door(action == Action::Interact))
        );
    }
}

#[test]
fn interaction_refuses_occupied_doors_and_obeys_readiness() {
    let mut loc = fixtures::room();
    let target = loc.player().head();
    loc.map[target].feature = Some(tile::Door(true));
    let id = loc.spawn(Actor::new(actor::Race::Rat, Position::new(target, XY)));
    let hp = loc.actors_byid[&id].hp;
    loc.act(0, Action::Interact);
    assert_eq!(loc.map[target].feature, Some(tile::Door(true)));
    assert_eq!(loc.actors_byid[&id].hp, hp);
    assert!(loc.sound_events.is_empty());
    assert!(loc.combat_events.is_empty());
    assert!(loc.player().acted);
    fixtures::occupancy(&loc);

    for dead in [false, true] {
        for with_item in [false, true] {
            let mut loc = fixtures::room();
            let target = loc.player().head();
            loc.map[target].feature = Some(tile::Door(false));
            if with_item {
                loc.items.insert(target, item());
            }
            let player = loc.actors_byid.get_mut(&0).unwrap();
            if dead {
                player.hp = 0;
            } else {
                player.action_cd = 2;
            }
            loc.act(0, Action::Interact);
            assert_eq!(loc.map[target].feature, Some(tile::Door(false)));
            assert_eq!(loc.items.contains_key(&target), with_item);
            assert!(loc.player().items_backpack.is_empty());
            assert!(!loc.player().acted);
            assert!(loc.sound_events.is_empty());
        }
    }
}

#[test]
fn interaction_refreshes_light_sight_and_sound_in_both_directions() {
    let origin = Coordinate::new(0, 0);
    let mut loc = fixtures::room();
    for c in origin.range_iter(5) {
        loc.map[c].light = 0;
    }
    for y in -4..=4 {
        loc.map[Coordinate::new(1, y)] = Tile::new(tile::Wall);
    }
    let target = loc.player().head();
    let behind = target.scale(2);
    loc.map[target] = Tile::new(tile::Empty);
    loc.map[target].feature = Some(tile::Door(false));
    loc.map[origin].light = 5;
    loc.recalculate_light_map();
    loc.act(0, Action::Turn(Angle::Forward));
    assert_eq!(loc.light_map[behind], 0);
    for open in [true, false, true] {
        loc.act(0, Action::Interact);
        assert_eq!(loc.player().in_los(behind), open);
        assert_eq!(loc.player().sees(behind), open);
        assert_eq!(loc.light_map[behind] > 0, open);
        assert!(loc.player().knows(behind));
        assert_eq!(loc.sound_events.len(), 1);
        assert_eq!(
            loc.sound_events[0]
                .cells
                .iter()
                .any(|cell| cell.coord == behind),
            open
        );
    }
}

#[test]
fn interaction_refreshes_stationary_observers_without_forgetting_awareness() {
    for acting_player in [true, false] {
        let mut loc = fixtures::room();
        let origin = loc.player().coord();
        let door = loc.player().head();
        for y in -4..=4 {
            loc.map[Coordinate::new(1, y)] = Tile::new(tile::Wall);
        }
        loc.map[door] = Tile::new(tile::Empty);
        loc.map[door].feature = Some(tile::Door(true));
        let observer_coord = door.scale(2);
        let enemy = loc.spawn(Actor::new(
            actor::Race::Human,
            Position::new(observer_coord, -XY),
        ));
        loc.refresh_sight();
        assert!(loc.player().sees(observer_coord));
        assert!(loc.actors_byid[&enemy].sees(origin));
        assert!(loc.actors_byid[&enemy].aware_of_player);
        let (acting, observing, seen_coord) = if acting_player {
            (0, enemy, origin)
        } else {
            (enemy, 0, observer_coord)
        };
        // An observer can remain idle/cooling down after the geometry changes.
        loc.actors_byid.get_mut(&observing).unwrap().action_cd = 10;
        let pos = loc.actors_byid[&observing].pos;
        for open in [false, true, false] {
            loc.act(acting, Action::Interact);
            let observer = &loc.actors_byid[&observing];
            assert_eq!(observer.pos, pos);
            assert_eq!(observer.action_cd, 10);
            assert_eq!(observer.in_los(seen_coord), open);
            assert_eq!(observer.sees(seen_coord), open);
            assert!(observer.knows(seen_coord));
            assert!(loc.actors_byid[&enemy].aware_of_player);
            assert_eq!(loc.map[door].feature, Some(tile::Door(open)));
        }
    }
}

#[test]
fn interaction_uses_one_engine_turn_even_when_nothing_can_be_done() {
    for feature in [None, Some(tile::Door(false)), Some(tile::Stairs)] {
        let mut loc = fixtures::room();
        let pos = loc.player().pos;
        let target = loc.player().head();
        loc.map[target].feature = feature;
        let id = loc.spawn(Actor::new(
            actor::Race::Rat,
            Position::new(Coordinate::new(-3, 0), XY),
        ));
        loc.actors_byid.get_mut(&id).unwrap().action_cd = 10;
        let mut engine = Engine::fixture(loc);
        let level = engine.current_location().level;
        engine.start_turn();
        engine.player_act(Action::Interact);
        assert_eq!(engine.turn(), 1);
        assert!(!engine.needs_player_input());
        assert!(!engine.is_turn_done());
        assert_eq!(engine.one_actor_tick(), id);
        assert!(engine.is_turn_done());
        assert_eq!(engine.current_location().actors_byid[&id].action_cd, 9);
        assert_eq!(engine.current_location().level, level);
        assert_eq!(engine.player().pos, pos);
        assert!(!engine.player().descended());
    }
}

#[test]
fn interaction_does_not_clear_thickets_or_use_underfoot_features() {
    let mut loc = fixtures::room();
    let pos = loc.player().pos;
    let target = loc.player().head();
    loc.map[pos.coord].feature = Some(tile::Stairs);
    loc.items.insert(pos.coord, item());
    loc.map[target] = Tile::new(tile::Thicket);
    loc.act(0, Action::Interact);
    assert_eq!(loc.map[target].type_, tile::Thicket);
    assert!(loc.items.contains_key(&pos.coord));
    assert!(!loc.player().descended());
    assert!(loc.sound_events.is_empty());
    loc.act(0, Action::Move(Angle::Forward));
    assert_eq!(loc.map[target].type_, tile::Ground);

    loc.map[target].feature = Some(tile::Door(false));
    loc.act(0, Action::Move(Angle::Forward));
    assert_eq!(loc.map[target].feature, Some(tile::Door(true)));
    assert_eq!(loc.player().pos, pos);
    loc.act(0, Action::Move(Angle::Forward));
    assert_eq!(loc.player().coord(), target);
}
