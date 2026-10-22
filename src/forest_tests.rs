use super::*;
use crate::game::{
    Location,
    level::{Biome, LevelId},
};
use proptest::prelude::*;
use std::collections::HashSet;

fn check(seed: u64) {
    let (map, actors, _) = generate(1, seed);
    let (repeat, _, _) = generate(1, seed);
    let origin = Coordinate::new(0, 0);
    let cells: Vec<_> = origin.range_iter(RADIUS).collect();
    assert_eq!(map.iter().count(), 817);
    for &c in &cells {
        assert_eq!(map[c], repeat[c]);
    }
    assert!(map[origin].is_passable());
    assert!(!actors.contains_key(&origin));
    assert!(actors.keys().all(|c| map[*c].is_passable()));
    let count = |terrain| cells.iter().filter(|&&c| map[c].type_ == terrain).count();
    assert!(count(tile::TreeTrunk) > 60, "seed {seed}");
    assert!(count(tile::Thicket) > 250, "seed {seed}");
    assert!(count(tile::Ground) > 90, "seed {seed}");
    assert!(count(tile::Water) > 8, "seed {seed}");
    assert!(count(tile::ShallowWater) > 45, "seed {seed}");
    assert!(cells.iter().filter(|&&c| map[c].light > 0).count() == 5);
    assert!(
        cells
            .iter()
            .filter(|&&c| map[c].type_ == tile::TreeTrunk)
            .all(|c| c
                .neighbors()
                .iter()
                .any(|&n| map[n].type_ == tile::TreeTrunk))
    );

    let mut reached = HashSet::from([origin]);
    let mut queue = VecDeque::from([origin]);
    while let Some(c) = queue.pop_front() {
        for n in c.neighbors() {
            if map[n].is_passable() && reached.insert(n) {
                queue.push_back(n);
            }
        }
    }
    for &c in &cells {
        if map[c].light > 0 || map[c].feature == Some(tile::ForestPath) {
            assert!(
                reached.contains(&c),
                "seed {seed}, unreachable clearing/exit {c:?}"
            );
        }
    }
    let mut loc = Location::generated(LevelId(37), 1, Biome::DarkForest, seed);
    loc.recalculate_light_map();
    assert!(cells.iter().filter(|&&c| loc.light_map[c] == 0).count() > 600);
    for &c in &cells {
        if map[c].light > 0 {
            assert_eq!(loc.light_map[c], 5);
            assert!(c.neighbors().iter().all(|&n| loc.light_map[n] > 0));
        }
    }
}

#[test]
fn forest_corpus_is_dense_dark_clustered_and_connected() {
    for seed in 0..128 {
        check(seed);
    }
}

proptest! {
    #![proptest_config(crate::property_support::config(
        "forest", include_str!("regressions/generator.txt"), 32))]
    #[test]
    fn bounded_forests(seed in any::<u64>()) {
        check(seed);
    }
}

/// Opt-in map artifact for visual inspection; no golden regeneration.
#[test]
#[ignore]
fn print_forest_atlas() {
    for seed in [0, 7, 42] {
        let (map, _, _) = generate(1, seed);
        println!("seed {seed}: # trunk, % thicket, . ground, , shallow, ~ deep, * clearing");
        for y in -RADIUS..=RADIUS {
            let mut line = " ".repeat((y + RADIUS) as usize);
            for x in -RADIUS..=RADIUS {
                let c = Coordinate::new(x, y);
                let t = map[c];
                let glyph = if t.feature.is_some() {
                    '>'
                } else if t.light > 0 {
                    '*'
                } else {
                    match t.type_ {
                        tile::Ground => '.',
                        tile::Thicket => '%',
                        tile::TreeTrunk => '#',
                        tile::ShallowWater => ',',
                        tile::Water => '~',
                        _ => ' ',
                    }
                };
                line.push(glyph);
                line.push(' ');
            }
            println!("{line}");
        }
    }
}
