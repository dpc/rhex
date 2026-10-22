use super::*;
use crate::game::{Action, Actor, actor::Race, fixtures, item, tile};
use hex2d::{Angle, Position, XY};
use proptest::prelude::*;

fn emission(level: i32) -> Emission {
    Emission {
        source: Coordinate::new(0, 0),
        kind: Noise::Creature(Race::Human),
        level,
    }
}

fn floor(radius: i32) -> Map {
    let mut map = Map::default();
    for c in Coordinate::new(0, 0).range_iter(radius) {
        map[c] = tile::Tile::new(tile::Empty);
    }
    map
}

#[test]
fn inclusive_radii_and_fixed_work_bound_ignore_remote_map_size() {
    for level in [2, 3, 7, 9, 16] {
        let mut map = floor(18);
        WORK.set((0, 0, 0));
        let cells = propagate(&map, &Items::default(), &emission(level));
        let work = WORK.get();
        assert_eq!(work.1, cells.len());
        assert!(work.2 <= 6 * cells.len());
        assert_eq!(cells.len(), (1 + 3 * level * (level + 1)) as usize);
        for cell in &cells {
            let distance = cell.coord.distance(Coordinate::new(0, 0));
            assert_eq!(cell.spent, (4 * distance) as u16);
            assert_eq!(cell.hops, distance as u16);
        }
        for c in Coordinate::new(1000, 1000).range_iter(100) {
            map[c] = tile::Tile::new(tile::Empty);
        }
        WORK.set((0, 0, 0));
        assert_eq!(cells, propagate(&map, &Items::default(), &emission(level)));
        assert_eq!(work, WORK.get());
    }
    for level in [-1, 0] {
        assert!(propagate(&floor(3), &Items::default(), &emission(level)).is_empty());
    }
    assert!(propagate(&Map::default(), &Items::default(), &emission(2)).is_empty());
}

#[test]
fn acoustic_fields_rotate_with_the_hex_map_and_items() {
    let mut map = floor(7);
    map[Coordinate::new(1, 0)].feature = Some(tile::Statue);
    map[Coordinate::new(-1, 1)].type_ = tile::Wall;
    map[Coordinate::new(2, 1)].feature = Some(tile::Door(false));
    map.compact();
    let mut items = Items::default();
    items.insert(
        Coordinate::new(0, -1),
        Box::new(item::Item::new(item::Type::Cloak, vec![])),
    );
    let cells = propagate(&map, &items, &emission(7));
    for angle in [
        Angle::Forward,
        Angle::Left,
        Angle::LeftBack,
        Angle::Back,
        Angle::RightBack,
        Angle::Right,
    ] {
        let mut rotated = Map::default();
        for (&c, &tile) in map.iter() {
            rotated[c.rotate_around_zero(angle)] = tile;
        }
        let rotated_items = items
            .iter()
            .map(|(&c, item)| (c.rotate_around_zero(angle), item.clone()))
            .collect();
        let actual = propagate(&rotated, &rotated_items, &emission(7));
        let mut expected: Vec<_> = cells
            .iter()
            .map(|cell| Cell {
                coord: cell.coord.rotate_around_zero(angle),
                ..*cell
            })
            .collect();
        expected.sort_unstable_by_key(|c| (c.hops, c.coord.x, c.coord.y));
        assert_eq!(actual, expected);
    }
}

#[test]
fn unsupported_loudness_is_an_explicit_bound() {
    let result =
        std::panic::catch_unwind(|| propagate(&floor(18), &Items::default(), &emission(i32::MAX)));
    if cfg!(debug_assertions) {
        assert!(result.is_err());
    } else {
        assert_eq!(result.unwrap().len(), 817);
    }
}

#[test]
#[ignore = "manual throughput measurement, never a timing gate"]
fn sound_workload_measurement() {
    use std::time::Instant;
    let map = floor(18);
    let items = Items::default();
    for level in [2, 9, 16] {
        WORK.set((0, 0, 0));
        let start = Instant::now();
        for _ in 0..2000 {
            std::hint::black_box(propagate(&map, &items, &emission(level)));
        }
        eprintln!(
            "open disk level={level}: elapsed={:?}, (calls, settled, edges)={:?}",
            start.elapsed(),
            WORK.get()
        );
    }
    let mut loc = fixtures::room();
    for (i, c) in Coordinate::new(0, 0)
        .ring_iter(3, hex2d::Spin::CW(XY))
        .enumerate()
    {
        if i == 12 {
            break;
        }
        loc.spawn(Actor::new(Race::Human, Position::new(c, XY)));
    }
    WORK.set((0, 0, 0));
    let start = Instant::now();
    for _ in 0..100 {
        for id in loc.actors_ids() {
            loc.actors_byid.get_mut(&id).unwrap().noise_makes(7);
        }
        loc.pre_any_tick();
        loc.post_any_tick();
    }
    eprintln!(
        "crowded room 100 action boundaries / 1300 emissions: elapsed={:?}, work={:?}",
        start.elapsed(),
        WORK.get()
    );
}

#[test]
fn adjacent_terrain_and_item_halos_use_max_not_sum() {
    let mut map = floor(5);
    let receiver = Coordinate::new(1, 0);
    let damper = Coordinate::new(1, 1);
    map[damper].feature = Some(tile::Statue);
    let mut items = Items::default();
    items.insert(damper, Box::new(item::Item::new(item::Type::Cloak, vec![])));
    let cells = propagate(&map, &items, &emission(2));
    assert_eq!(cells.iter().find(|c| c.coord == receiver).unwrap().spent, 8);
    assert!(!cells.iter().any(|c| c.coord == Coordinate::new(2, 0)));
    map[damper].feature = None;
    let cells = propagate(&map, &items, &emission(2));
    assert_eq!(cells.iter().find(|c| c.coord == receiver).unwrap().spent, 6);
    items.insert(
        damper + hex2d::XY,
        Box::new(item::Item::new(item::Type::Leather, vec![])),
    );
    assert_eq!(
        cells.iter().find(|c| c.coord == receiver),
        propagate(&map, &items, &emission(2))
            .iter()
            .find(|c| c.coord == receiver)
    );
}

#[test]
fn walls_block_doors_attenuate_water_transmits_and_corridors_remain_audible() {
    let mut map = Map::default();
    for x in 0..6 {
        map[Coordinate::new(x, 0)] = tile::Tile::new(tile::Empty);
    }
    let items = Items::default();
    let cells = propagate(&map, &items, &emission(2));
    assert!(cells.iter().any(|c| c.coord == Coordinate::new(1, 0)));
    assert!(!cells.iter().any(|c| c.coord == Coordinate::new(2, 0)));
    map[Coordinate::new(2, 0)].feature = Some(tile::Door(false));
    assert!(
        !propagate(&map, &items, &emission(3))
            .iter()
            .any(|c| c.coord == Coordinate::new(3, 0))
    );
    assert!(
        propagate(&map, &items, &emission(7))
            .iter()
            .any(|c| c.coord == Coordinate::new(3, 0))
    );
    map[Coordinate::new(2, 0)] = tile::Tile::new(tile::Wall);
    assert!(
        !propagate(&map, &items, &emission(16))
            .iter()
            .any(|c| c.coord == Coordinate::new(3, 0))
    );
    map[Coordinate::new(2, 0)] = tile::Tile::new(tile::Water);
    assert!(
        propagate(&map, &items, &emission(7))
            .iter()
            .any(|c| c.coord == Coordinate::new(3, 0))
    );
}

#[test]
fn least_attenuated_path_can_detour_and_insertion_order_does_not_matter() {
    let mut map = floor(10);
    for x in 1..=5 {
        map[Coordinate::new(x, 0)].feature = Some(tile::Statue);
    }
    let cells = propagate(&map, &Items::default(), &emission(12));
    let target = cells
        .iter()
        .find(|c| c.coord == Coordinate::new(7, 0))
        .unwrap();
    assert!(target.hops > 7);
    assert!(target.spent < 7 * 8);
    map.compact();
    let mut reversed = Map::default();
    let mut entries: Vec<_> = map.iter().map(|(&c, &t)| (c, t)).collect();
    entries.sort_by_key(|(c, _)| (c.x, c.y));
    for (c, t) in entries.into_iter().rev() {
        reversed[c] = t;
    }
    assert_eq!(
        cells,
        propagate(&reversed, &Items::default(), &emission(12))
    );
}

#[test]
fn charge_keeps_both_origins_and_door_uses_post_action_geometry() {
    let mut loc = fixtures::room();
    let origin = loc.player().coord();
    let first = loc.player().head();
    loc.act(0, Action::Charge);
    assert_eq!(loc.sound_events.len(), 2);
    assert!(loc.player().heard.contains_key(&first));
    assert!(loc.player().heard.contains_key(&(first + XY)));
    // Earlier footsteps can be behind the player after a charge. They still
    // produce waves, but must not pretend to be unseen external sources.
    assert!(loc.sound_events.iter().all(|s| s.hidden_source.is_none()));
    let mut loc = fixtures::room();
    loc.map[first].feature = Some(tile::Door(false));
    loc.act(0, Action::Move(Angle::Forward));
    assert_eq!(loc.map[first].feature, Some(tile::Door(true)));
    let sound = &loc.sound_events[0];
    assert_eq!(
        sound.cells.iter().find(|c| c.hops == 0).unwrap().coord,
        origin
    );
    assert_eq!(
        sound.cells.iter().find(|c| c.coord == first).unwrap().spent,
        4
    );
    assert!(sound.cells.iter().any(|c| c.hops == 3));
}

#[test]
fn action_capture_keeps_existing_levels_even_while_actor_is_removed() {
    let mut loc = fixtures::room();
    let mut player = loc.actors_byid.remove(&0).unwrap();
    player.act(&mut loc, Action::Turn(Angle::Right));
    assert_eq!(player.pending_noises[0].level, 2);
    player.pending_noises.clear();
    player.open_door(&loc);
    assert_eq!(player.pending_noises[0].level, 3);
    player.pending_noises.clear();
    player.equip(Box::new(item::Item::new(item::Type::Pickaxe, vec![])), 'p');
    player.recalculate_stats();
    let head = player.head();
    loc.map[head] = tile::Tile::new(tile::Wall);
    player.dig(Angle::Forward, &mut loc);
    assert_eq!(player.pending_noises[0].level, 9);
}

#[test]
fn hearing_reaches_ai_before_its_choice_and_not_retroactively() {
    use crate::ai::Ai;
    use rand::{SeedableRng, rngs::SmallRng};
    let mut loc = fixtures::room();
    let source = loc.player().coord();
    let listener = Coordinate::new(-2, 2);
    let id = loc.spawn(Actor::new(Race::Rat, Position::new(listener, XY)));
    loc.actors_byid.get_mut(&id).unwrap().visible.clear();
    loc.act(0, Action::Turn(Angle::Right));
    assert!(loc.actors_byid[&id].heard.contains_key(&source));
    let action = crate::ai::Simple.action(id, &loc, &mut SmallRng::seed_from_u64(1));
    assert_eq!(action, Action::Move(Angle::Forward));
    loc.act(id, action);
    assert!(!loc.actors_byid[&id].heard.contains_key(&source));
    // A listener which enters an old sound's range does not hear it later.
    loc.skip_act(0);
    assert!(!loc.player().heard.contains_key(&source));
}

#[test]
fn discrete_emissions_preserve_origins_death_and_do_not_replay() {
    let mut loc = fixtures::room();
    assert!(loc.player().pending_noises.is_empty());
    assert!(loc.player().heard.is_empty());
    let source = Coordinate::new(1, 0);
    let id = loc.spawn(Actor::new(Race::Human, Position::new(source, XY)));
    assert!(loc.sound_events.is_empty());
    let actor = loc.actors_byid.get_mut(&id).unwrap();
    actor.noise_makes(7);
    actor.pos.coord = Coordinate::new(2, 0);
    actor.noise_makes(9);
    actor.hp = 0;
    loc.post_any_tick();
    assert!(loc.player().heard.contains_key(&source));
    assert!(loc.player().heard.contains_key(&Coordinate::new(2, 0)));
    assert_eq!(loc.sound_events.len(), 2);
    assert!(loc.actors_byid[&id].pending_noises.is_empty());
    loc.actors_byid.get_mut(&0).unwrap().heard.clear();
    loc.skip_act(0);
    assert!(loc.sound_events.is_empty());
    assert!(loc.player().heard.is_empty());
}

#[test]
fn real_move_turn_wait_and_charge_use_action_scoped_snapshots() {
    for action in [
        Action::Move(Angle::Forward),
        Action::Turn(Angle::Left),
        Action::Charge,
    ] {
        let mut loc = fixtures::room();
        loc.act(0, action);
        assert!(!loc.sound_events.is_empty(), "{action:?}");
        assert!(loc.player().pending_noises.is_empty());
        let heard = loc.player().heard.clone();
        assert!(!heard.is_empty());
        loc.skip_act(0);
        assert!(loc.sound_events.is_empty());
        assert!(loc.player().heard.is_empty());
    }
    let mut loc = fixtures::room();
    loc.act(0, Action::Wait);
    assert!(loc.sound_events.is_empty());
}

#[test]
fn unseen_audible_origin_is_saved_without_hidden_wave_cells() {
    let mut loc = fixtures::room();
    let source = Coordinate::new(1, 0);
    let id = loc.spawn(Actor::new(Race::Human, Position::new(source, XY)));
    loc.actors_byid.get_mut(&0).unwrap().visible.clear();
    loc.actors_byid.get_mut(&0).unwrap().known.clear();
    loc.actors_byid.get_mut(&id).unwrap().noise_makes(7);
    loc.post_any_tick();
    assert_eq!(loc.sound_events.len(), 1);
    assert_eq!(loc.sound_events[0].hidden_source, Some(source));
    assert!(loc.sound_events[0].cells.is_empty());
    assert!(!loc.player().knows(source));
}

// Independent repeated edge relaxation oracle: no heap or finalized visitation.
fn relaxed(map: &Map, items: &Items, level: i32) -> HashMap<Coordinate, u16> {
    let mut costs = HashMap::from([(Coordinate::new(0, 0), 0)]);
    for _ in 0..level {
        for (c, cost) in costs.clone() {
            for &d in Direction::all() {
                let next = c + d;
                if !map[next].transmits_sound() {
                    continue;
                }
                let mut halo = 0;
                for near in next.range_iter(1) {
                    halo = halo
                        .max(map[near].sound_dampening())
                        .max(items.get(&near).map_or(0, |i| i.sound_dampening()));
                }
                let new = cost + 4 + halo;
                if new <= 4 * level as u16 {
                    let old = costs.entry(next).or_insert(new);
                    *old = (*old).min(new);
                }
            }
        }
    }
    costs
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn weighted_paths_match_relaxation_and_obey_monotonicity(
        materials in prop::collection::vec(0u8..5, 37),
        level in 1i32..10,
    ) {
        let mut map = floor(6);
        for (c, material) in Coordinate::new(0,0).range_iter(3).zip(materials) {
            if c == Coordinate::new(0,0) { continue; }
            match material {
                1 => map[c].feature = Some(tile::Statue),
                2 => map[c].feature = Some(tile::Door(false)),
                3 => map[c].type_ = tile::Wall,
                _ => {},
            }
        }
        let items = Items::default();
        let cells = propagate(&map, &items, &emission(level));
        let costs: HashMap<_,_> = cells.iter().map(|c| (c.coord,c.spent)).collect();
        prop_assert_eq!(&costs, &relaxed(&map, &items, level));
        let louder = propagate(&map, &items, &emission(level+1));
        for cell in cells {
            prop_assert!(louder.iter().any(|c| c.coord == cell.coord && c.spent == cell.spent));
            prop_assert!(cell.coord.distance(Coordinate::new(0,0)) <= level);
        }
        map[Coordinate::new(1,0)].feature = Some(tile::Statue);
        for cell in propagate(&map, &items, &emission(level)) {
            prop_assert!(costs.get(&cell.coord).is_some_and(|&old| old <= cell.spent));
        }
    }
}
