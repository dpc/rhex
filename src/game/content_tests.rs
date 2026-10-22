//! Gameplay contracts for fixed containers, reusable tools, and populated levels.
use super::*;
use actor::{Race, Slot};
use chest::Chest;
use hex2d::{Angle, Coordinate, Position, XY};
use item::{Item, Type};
use rand::RngExt;

fn item(t: Type) -> Box<Item> {
    Box::new(Item::new(t, vec![]))
}

fn ready(loc: &mut Location, id: actor::Id) {
    while loc.actors_byid[&id].action_cd > 0 {
        loc.skip_act(id);
    }
}

#[test]
fn chest_opening_pick_priority_atomic_transfer_and_exhaustion() {
    for contents in [
        vec![],
        vec![Type::Whistle],
        vec![Type::ShortSword, Type::StaminaPotion],
    ] {
        let mut loc = fixtures::room();
        let head = loc.player().head();
        loc.chests
            .insert(head, Chest::new(contents.iter().copied().map(item)));
        let original = loc.chests[&head].clone();
        assert_eq!(original.description(), "closed chest");
        loc.act(0, Action::Pick);
        assert_eq!(loc.chests[&head], original);
        loc.items.insert(head, item(Type::Buckler));
        loc.act(0, Action::Interact);
        assert_eq!(loc.chests[&head], original);
        assert_eq!(loc.player().items_backpack[&'a'].description(), "buckler");
        loc.act(0, Action::Interact);
        assert!(loc.chests[&head].opened);
        assert_eq!(loc.chests[&head].contents, original.contents);
        assert_eq!(loc.player().action_cd, 2);
        assert_eq!(loc.sound_events.len(), 1);
        let expected = sound::propagate(
            &loc.map,
            &loc.items,
            &sound::Emission {
                source: loc.player().coord(),
                kind: Noise::Creature(Race::Human),
                level: 5,
            },
        );
        assert_eq!(
            loc.sound_events[0].cells,
            expected
                .into_iter()
                .filter(|c| c.coord != loc.player().coord() && loc.player().sees(c.coord))
                .collect::<Vec<_>>()
        );
        ready(&mut loc, 0);
        if !contents.is_empty() {
            for ch in (b'a'..=b'y').chain(b'A'..=b'Y') {
                loc.actors_byid
                    .get_mut(&0)
                    .unwrap()
                    .items_backpack
                    .insert(ch as char, item(Type::Junk));
            }
            let before = loc.chests[&head].clone();
            loc.act(0, Action::Interact);
            assert_eq!(loc.chests[&head], before);
            assert!(loc.messages.last().unwrap().contains("full"));
            assert!(!loc.items.contains_key(&head));
            loc.actors_byid.get_mut(&0).unwrap().items_backpack.clear();
        }
        for (index, t) in contents.iter().enumerate() {
            loc.act(0, Action::Interact);
            assert_eq!(
                loc.player().items_backpack[&((b'a' + index as u8) as char)],
                item(*t)
            );
            assert_eq!(loc.chests[&head].contents.len(), contents.len() - index - 1);
            assert!(loc.sound_events.is_empty());
            assert!(loc.player().items_equipped.is_empty());
        }
        for _ in 0..3 {
            loc.act(0, Action::Interact);
            assert_eq!(loc.chests[&head].description(), "empty chest");
            assert!(loc.sound_events.is_empty());
        }
    }
}

#[test]
fn whistle_is_reusable_and_heard_at_user_not_item_or_target() {
    let mut loc = fixtures::room();
    let listener = Coordinate::new(3, 0);
    let id = loc.spawn(Actor::new(Race::Wolf, Position::new(listener, XY)));
    loc.set_player_awareness(id, false);
    loc.actors_byid
        .get_mut(&0)
        .unwrap()
        .items_backpack
        .insert('a', item(Type::Whistle));
    for _ in 0..2 {
        loc.act(0, Action::Equip('a'));
        assert_eq!(loc.player().items_backpack[&'a'].description(), "whistle");
        assert_eq!(loc.player().action_cd, 2);
        assert_eq!(loc.sound_events.len(), 1);
        assert!(
            loc.actors_byid[&id]
                .heard
                .contains_key(&loc.player().coord())
        );
        assert!(loc.actors_byid[&id].aware_of_player);
        let expected = sound::propagate(
            &loc.map,
            &loc.items,
            &sound::Emission {
                source: loc.player().coord(),
                kind: Noise::Creature(Race::Human),
                level: 12,
            },
        );
        assert_eq!(
            loc.sound_events[0].cells,
            expected
                .into_iter()
                .filter(|c| c.coord != loc.player().coord() && loc.player().sees(c.coord))
                .collect::<Vec<_>>()
        );
        ready(&mut loc, 0);
    }
}

#[test]
fn stamina_drink_clamps_and_consumes_without_equipping() {
    for missing in [0, 2, 9] {
        let mut loc = fixtures::room();
        let player = loc.actors_byid.get_mut(&0).unwrap();
        player.sp = player.stats.base.max_sp - missing;
        player.items_backpack.insert('a', item(Type::StaminaPotion));
        let min_expected = (player.sp + 5).min(player.stats.base.max_sp);
        loc.act(0, Action::Equip('a'));
        assert!(!loc.player().item_letter_taken('a'));
        // Ordinary post-action recovery can add one point.
        assert!(
            (min_expected..=(min_expected + 1).min(loc.player().stats.base.max_sp))
                .contains(&loc.player().sp)
        );
        assert_eq!(loc.player().action_cd, 2);
        assert!(loc.player().items_equipped.is_empty());
        assert!(loc.sound_events.is_empty());
    }
}

#[test]
fn lantern_light_refreshes_both_observers_on_equip_swap_drop_and_death() {
    for ending in [Action::Equip('a'), Action::Equip('b'), Action::Drop_('a')] {
        let mut loc = fixtures::room();
        for c in Coordinate::new(0, 0).range_iter(5) {
            loc.map[c].light = 0;
        }
        loc.map.compact();
        let target = Coordinate::new(3, 0);
        let id = loc.spawn(Actor::new(
            Race::Human,
            Position::new(target, XY + Angle::Back),
        ));
        let player = loc.actors_byid.get_mut(&0).unwrap();
        player.items_backpack.insert('a', item(Type::HandLantern));
        player.items_backpack.insert('b', item(Type::Buckler));
        loc.recalculate_light_map();
        loc.refresh_sight();
        assert!(!loc.player().sees(target));
        assert!(!loc.actors_byid[&id].sees(Coordinate::new(0, 0)));
        loc.act(0, Action::Equip('a'));
        assert_eq!(loc.player().light_emision(), 5);
        assert!(loc.player().sees(target));
        assert!(loc.actors_byid[&id].sees(Coordinate::new(0, 0)));
        ready(&mut loc, 0);
        loc.act(0, ending);
        assert_eq!(loc.player().light_emision(), 0);
        assert!(!loc.player().sees(target));
        assert!(!loc.actors_byid[&id].sees(Coordinate::new(0, 0)));
        if ending == Action::Equip('b') {
            assert_eq!(
                loc.player().items_equipped[&Slot::LHand].1.description(),
                "buckler"
            );
            assert_eq!(
                loc.player().items_backpack[&'a'].description(),
                "hand lantern"
            );
        }
    }
    let mut loc = fixtures::room();
    for c in Coordinate::new(0, 0).range_iter(5) {
        loc.map[c].light = 0;
    }
    loc.map.compact();
    let target = Coordinate::new(3, 0);
    loc.recalculate_light_map();
    loc.refresh_sight();
    assert!(!loc.player().sees(target));
    let mut carrier = Actor::new(Race::Goblin, Position::new(target, XY));
    carrier.equip(item(Type::HandLantern), 'a');
    let id = loc.spawn(carrier);
    assert!(loc.player().sees(target));
    loc.actors_byid.get_mut(&id).unwrap().hp = 0;
    loc.post_any_tick();
    assert_eq!(loc.light_map[target], 0);
    assert!(!loc.player().sees(target));
    assert!(
        loc.items
            .values()
            .any(|i| i.description() == "hand lantern")
    );
}

#[test]
fn new_monsters_keep_roles_and_drop_actual_skeleton_gear() {
    let mut loc = fixtures::room();
    let id = loc.spawn(Actor::new(
        Race::Skeleton,
        Position::new(Coordinate::new(2, 0), XY),
    ));
    let skeleton = &loc.actors_byid[&id];
    // Base + one equipment contribution + the ordinary attribute-derived bonus.
    assert_eq!(skeleton.stats.base.ac, 2);
    assert_eq!(skeleton.stats.base.ev, 3);
    assert_eq!(skeleton.stats.melee_dmg, 5);
    loc.actors_byid.get_mut(&id).unwrap().hp = 0;
    loc.post_any_tick();
    let mut drops: Vec<_> = loc.items.values().map(|i| i.description()).collect();
    drops.sort();
    assert_eq!(drops, ["buckler", "short sword"]);
    for race in [Race::Wolf, Race::CaveBeetle] {
        for obstacle in [
            tile::Tile::new(tile::Thicket),
            *tile::Tile::new(tile::Empty).add_feature(tile::Door(false)),
        ] {
            let mut loc = fixtures::room();
            let c = Coordinate::new(2, 0);
            let id = loc.spawn(Actor::new(race, Position::new(c, XY)));
            let head = loc.actors_byid[&id].head();
            loc.map[head] = obstacle;
            loc.items.insert(head, item(Type::HealthPotion));
            for action in [
                Action::Pick,
                Action::Interact,
                Action::Move(Angle::Forward),
                Action::Charge,
            ] {
                loc.act(id, action);
                assert_eq!(loc.map[head], obstacle);
                assert_eq!(loc.actors_byid[&id].coord(), c);
                assert!(loc.actors_byid[&id].items_backpack.is_empty());
                assert!(loc.items.contains_key(&head));
            }
        }
    }
}

#[test]
fn seeded_content_is_bounded_reachable_and_never_overwrites_features() {
    use std::collections::{HashSet, VecDeque};
    let origin = Coordinate::new(0, 0);
    let mut seen_races = HashSet::new();
    let mut seen_items = HashSet::new();
    let mut chest_sizes = HashSet::new();
    for biome in [level::Biome::DarkForest, level::Biome::Dungeon] {
        for seed in 0..24 {
            let depth = (seed % 6) as u32;
            let loc = Location::generated(level::LevelId(0), depth, biome, seed);
            let same = Location::generated(level::LevelId(0), depth, biome, seed);
            assert_eq!(loc.chests, same.chests);
            assert_eq!(loc.items, same.items);
            assert_eq!(
                loc.rng.clone().random::<u64>(),
                same.rng.clone().random::<u64>()
            );
            let mut reached = HashSet::from([origin]);
            let mut queue = VecDeque::from([origin]);
            while let Some(c) = queue.pop_front() {
                for n in c.neighbors() {
                    if loc.map[n].is_passable() && reached.insert(n) {
                        queue.push_back(n);
                    }
                }
            }
            assert!(!loc.chests.is_empty());
            assert!(
                loc.chests.len()
                    <= if biome == level::Biome::DarkForest {
                        1
                    } else {
                        6
                    }
            );
            for (&c, chest) in &loc.chests {
                assert!(reached.contains(&c));
                assert!(loc.map[c].feature.is_none());
                assert!(!loc.items.contains_key(&c));
                assert!(!loc.actors_coord_to_id.contains_key(&c));
                assert!(c.distance(origin) > 3);
                assert!(!chest.opened);
                assert!(chest.contents.len() <= 2);
                chest_sizes.insert(chest.contents.len());
                seen_items.extend(chest.contents.iter().map(|i| i.description()));
            }
            if biome == level::Biome::DarkForest {
                assert_eq!(loc.items.len(), 2);
                for c in loc.items.keys() {
                    assert!(reached.contains(c));
                }
            }
            seen_items.extend(loc.items.values().map(|i| i.description()));
            seen_races.extend(loc.actors_byid.values().map(|a| a.race.description()));
        }
    }
    assert_eq!(chest_sizes, HashSet::from([0, 1, 2]));
    for race in ["wolf", "cave beetle", "skeleton"] {
        assert!(seen_races.contains(race));
    }
    for item in [
        "whistle",
        "hand lantern",
        "stamina potion",
        "short sword",
        "club",
        "buckler",
    ] {
        assert!(seen_items.contains(item), "{item}");
    }
    // A world without eligible sites terminates without retries or overwrites.
    let mut loc = fixtures::room();
    for c in origin.range_iter(5) {
        loc.map[c].feature = Some(tile::Statue);
    }
    loc.map.compact();
    super::content::populate(&mut loc);
    assert!(loc.chests.is_empty());
    assert!(loc.items.is_empty());
}

#[test]
fn dungeon_fallback_can_replace_the_only_eligible_loose_reward() {
    use rand::SeedableRng;
    let origin = Coordinate::new(0, 0);
    let site = Coordinate::new(4, 0);
    for seed in 0..16 {
        let mut loc = fixtures::room();
        for c in origin.range_iter(5) {
            loc.map[c] = tile::Tile::new(tile::Wall);
        }
        for x in 0..=4 {
            loc.map[Coordinate::new(x, 0)] = tile::Tile::new(tile::Empty);
        }
        loc.map.compact();
        loc.items.insert(site, item(Type::Whistle));
        loc.rng = rand::rngs::SmallRng::seed_from_u64(seed);
        super::content::populate(&mut loc);
        assert_eq!(loc.chests.len(), 1);
        assert!(loc.chests.contains_key(&site));
        assert!(loc.items.is_empty());
    }
}
