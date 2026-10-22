//! Bounded tactical traces with conservation, geometry and perception oracles.
use super::*;
use hex2d::{Angle, Position, XY};
use proptest::prelude::*;
use rand::{SeedableRng, rngs::SmallRng};
use std::collections::BTreeMap;

fn actions() -> impl Strategy<Value = Vec<Action>> {
    prop::collection::vec(
        prop::sample::select(vec![
            Action::Wait,
            Action::Move(Angle::Forward),
            Action::Move(Angle::Back),
            Action::Move(Angle::Left),
            Action::Move(Angle::Right),
            Action::Move(Angle::LeftBack),
            Action::Move(Angle::RightBack),
            Action::Turn(Angle::Left),
            Action::Turn(Angle::Right),
            Action::Charge,
            Action::Spin(Angle::Left),
            Action::Spin(Angle::Right),
            Action::Pick,
            Action::Equip('a'),
            Action::Equip('b'),
            Action::Equip('c'),
            Action::Drop_('a'),
            Action::Drop_('b'),
            Action::Drop_('c'),
            Action::Ranged(Coordinate::new(1, -1)),
        ]),
        1..=64,
    )
}

fn inventory(loc: &Location) -> BTreeMap<String, usize> {
    let mut items = BTreeMap::new();
    for item in loc
        .items
        .values()
        .chain(loc.actors_byid.values().flat_map(|actor| {
            actor
                .items_backpack
                .values()
                .chain(actor.items_equipped.values().map(|(_, item)| item))
        }))
    {
        *items.entry(item.description()).or_default() += 1;
    }
    items
}

fn check(seed: u64, trace: &[Action]) {
    let mut loc = fixtures::room();
    loc.rng = SmallRng::seed_from_u64(seed);
    // Unique non-consumables let the multiset detect duplication AND lost items.
    for (coord, type_) in [
        (Coordinate::new(1, -1), item::Type::Knife),
        (Coordinate::new(-1, 1), item::Type::Sword),
    ] {
        loc.items.insert(coord, Box::new(Item::new(type_, vec![])));
    }
    loc.actors_byid
        .get_mut(&0)
        .unwrap()
        .pick_item(Box::new(Item::new(item::Type::Bow, vec![])));
    let mut enemy = Actor::new(
        actor::Race::Troll,
        Position::new(Coordinate::new(1, -1), XY),
    );
    enemy.pick_item(Box::new(Item::new(item::Type::Junk, vec![])));
    loc.spawn(enemy);
    loc.map[Coordinate::new(0, -1)].feature = Some(tile::Door(false));
    for (coord, terrain) in [
        (Coordinate::new(2, 0), tile::Thicket),
        (Coordinate::new(-2, 0), tile::TreeTrunk),
        (Coordinate::new(0, 2), tile::ShallowWater),
        (Coordinate::new(0, -2), tile::Water),
        (Coordinate::new(2, -2), tile::Ground),
    ] {
        loc.map[coord] = Tile::new(terrain);
    }
    let original_items = inventory(&loc);

    for (index, action) in trace.iter().enumerate() {
        // These are individual ready-player tactical actions, not an engine
        // scheduler model. NPCs are stationary targets; no artificial HP bounds.
        loc.actors_byid.get_mut(&0).unwrap().action_cd = 0;
        let old = loc.player().pos;
        let known = loc.player().known.clone();
        let hp: i32 = loc
            .actors_byid
            .values()
            .filter(|a| !a.is_player())
            .map(|a| a.hp)
            .sum();
        loc.act(0, *action);
        fixtures::occupancy(&loc);
        assert_eq!(
            inventory(&loc),
            original_items,
            "prefix {:?}",
            &trace[..=index]
        );
        for actor in loc.actors_byid.values() {
            let mut letters = std::collections::HashSet::new();
            for ch in actor
                .items_backpack
                .keys()
                .chain(actor.items_equipped.values().map(|(ch, _)| ch))
            {
                assert!(letters.insert(*ch), "duplicate item letter at step {index}");
            }
            for (slot, (_, item)) in &actor.items_equipped {
                assert_eq!(item.slot(), Some(*slot));
            }
            if !actor.is_dead() {
                assert!(loc.map[actor.coord()].is_passable());
            }
        }
        let player = loc.player();
        assert!(
            known.is_subset(&player.known),
            "knowledge lost at step {index}"
        );
        assert!(player.visible.is_subset(&player.known));
        assert!(player.discovered.is_disjoint(&known));
        assert!(player.discovered.is_subset(&player.known));
        let damage: i32 = player
            .did_attack
            .iter()
            .filter(|hit| hit.success)
            .map(|hit| hit.dmg)
            .sum();
        let new_hp: i32 = loc
            .actors_byid
            .values()
            .filter(|a| !a.is_player())
            .map(|a| a.hp)
            .sum();
        assert_eq!(hp - new_hp, damage, "unaccounted damage at step {index}");
        match action {
            Action::Turn(angle) => {
                assert_eq!(player.coord(), old.coord);
                assert_eq!(player.pos.dir, old.dir + *angle);
            }
            Action::Move(angle) => {
                assert_eq!(player.pos.dir, old.dir);
                assert!(
                    player.coord() == old.coord || player.coord() == old.coord + (old.dir + *angle)
                );
            }
            Action::Wait
            | Action::Pick
            | Action::Equip(_)
            | Action::Drop_(_)
            | Action::Ranged(_) => {
                assert_eq!(player.pos, old);
            }
            _ => {}
        }
    }
}

proptest! {
    #![proptest_config(crate::property_support::config(
        "tactical", include_str!("regressions/tactical.txt"), 64))]
    #[test]
    fn tactical_sequences_preserve_world_invariants(seed in any::<u64>(), trace in actions()) {
        check(seed, &trace);
    }
}

#[test]
fn pickup_equip_drop_and_turn_trace_conserves_ownership() {
    check(
        7,
        &[
            Action::Pick,
            Action::Equip('b'),
            Action::Drop_('b'),
            Action::Turn(Angle::Left),
            Action::Move(Angle::Forward),
            Action::Turn(Angle::Right),
            Action::Charge,
        ],
    );
}

#[test]
fn dropping_a_backpack_item_transfers_instead_of_cloning_it() {
    check(0, &[Action::Drop_('a')]);
}

#[test]
fn committed_tactical_regression_replays_without_random_cases() {
    let mut config =
        crate::property_support::config("tactical", include_str!("regressions/tactical.txt"), 0);
    config.cases = 0;
    let calls = std::cell::Cell::new(0);
    proptest::test_runner::TestRunner::new(config)
        .run(&(any::<u64>(), actions()), |(seed, trace)| {
            calls.set(calls.get() + 1);
            check(seed, &trace);
            Ok(())
        })
        .unwrap();
    assert!(calls.get() > 0, "committed corpus did not execute");
}
