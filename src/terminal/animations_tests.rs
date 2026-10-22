use super::*;

fn at(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

#[test]
fn six_axes_ranged_geometry_and_exact_phase_boundaries() {
    // Explicit screen-space directions, independent of actor facing.
    for (dx, dy, glyph) in [
        (1, -1, '-'),
        (-1, 1, '-'),
        (1, 0, '/'),
        (-1, 0, '/'),
        (0, 1, '\\'),
        (0, -1, '\\'),
    ] {
        for range in [1, 4] {
            let impact = Coordinate::new(0, 0);
            let mut animations = Animations::default();
            animations.enqueue(
                vec![CombatEvent {
                    source: Coordinate::new(dx * range, dy * range),
                    impact,
                }],
                at(100),
            );
            for (time, expected) in [
                (99, None),
                (100, Some(glyph)),
                (299, Some(glyph)),
                (300, Some('*')),
                (499, Some('*')),
                (500, None),
            ] {
                assert_eq!(
                    animations.glyphs(at(time)).next(),
                    expected.map(|g| (impact, g))
                );
            }
            animations.advance(at(500));
            assert!(animations.active.is_empty());
        }
    }
}

#[test]
fn same_cell_hits_queue_in_order_other_cells_play_concurrently_and_reset() {
    let origin = Coordinate::new(0, 0);
    let first = CombatEvent {
        source: Coordinate::new(1, -1),
        impact: origin,
    };
    let second = CombatEvent {
        source: Coordinate::new(1, 0),
        impact: origin,
    };
    let other = CombatEvent {
        source: origin,
        impact: Coordinate::new(0, 1),
    };
    let mut animations = Animations::default();
    animations.enqueue(vec![first, second, other], at(0));
    assert_eq!(
        animations.glyphs(at(0)).collect::<Vec<_>>(),
        [(origin, '-'), (other.impact, '\\')]
    );
    animations.enqueue(vec![first], at(100));
    for (time, glyph) in [(200, '*'), (400, '/'), (600, '*'), (800, '-'), (1000, '*')] {
        assert_eq!(animations.glyphs(at(time)).next(), Some((origin, glyph)));
    }
    animations.advance(at(1200));
    assert!(animations.active.is_empty());
    animations.enqueue(vec![first], at(1200));
    animations.clear();
    assert!(animations.glyphs(at(1200)).next().is_none());
}

fn sound(source: Coordinate) -> crate::game::sound::ObservedSound {
    crate::game::sound::ObservedSound {
        cells: (0..=3)
            .map(|hops| crate::game::sound::Cell {
                coord: source + Coordinate::new(i32::from(hops), 0),
                spent: 4 * hops,
                hops,
            })
            .collect(),
        hidden_source: Some(source),
    }
}

#[test]
fn waves_start_concurrently_union_overlaps_and_expire_at_phase_boundaries() {
    let a = Coordinate::new(0, 0);
    let b = Coordinate::new(0, 4);
    let mut animations = Animations::default();
    animations.enqueue_sounds(vec![sound(a), sound(b), sound(a)], at(0));
    for ms in [
        0, 99, 100, 199, 200, 299, 300, 399, 499, 500, 599, 600, 699, 700, 799, 800, 899, 900,
    ] {
        let cells: Vec<_> = animations.sound_cells(at(ms)).collect();
        for source in [a, b] {
            assert_eq!(cells.contains(&(source, true)), ms < 500);
            for phase in 0..=3 {
                let coord = source + Coordinate::new(phase, 0);
                assert_eq!(
                    cells.contains(&(coord, false)),
                    (phase as u64 * 100..phase as u64 * 100 + 600).contains(&ms)
                );
            }
        }
        let unique: std::collections::HashSet<_> = cells.iter().collect();
        assert_eq!(unique.len(), cells.len());
    }
    animations.advance(at(900));
    assert!(animations.sounds.is_empty());
}

#[test]
fn staggered_waves_keep_independent_arrivals_and_trailing_highlights() {
    let source = Coordinate::new(0, 0);
    let mut animations = Animations::default();
    animations.enqueue_sounds(vec![sound(source)], at(0));
    // Preserve existing 100ms start quantization for off-boundary enqueues.
    animations.enqueue_sounds(vec![sound(source)], at(450));
    for ms in [
        450, 499, 500, 599, 600, 699, 700, 899, 900, 999, 1000, 1199, 1200, 1299, 1300,
    ] {
        let cells: Vec<_> = animations.sound_cells(at(ms)).collect();
        assert_eq!(cells.contains(&(source, true)), ms < 900);
        for phase in 0..=3 {
            let arrival = phase as u64 * 100;
            let expected = (arrival..arrival + 600).contains(&ms)
                || (arrival + 400..arrival + 1000).contains(&ms);
            assert_eq!(
                cells
                    .iter()
                    .filter(|&&cell| cell == (source + Coordinate::new(phase, 0), false))
                    .count(),
                usize::from(expected),
                "phase {phase} at {ms}ms"
            );
        }
    }
    animations.advance(at(1300));
    assert!(animations.sounds.is_empty());
}

#[test]
fn rapid_batches_keep_every_source_in_bounded_time_masks_without_fifo() {
    let mut animations = Animations::default();
    for ms in 0..1000 {
        let source = Coordinate::new(ms as i32 % 10, 0);
        animations.enqueue_sounds(vec![sound(source)], at(ms));
        assert!(animations.sounds.len() <= 9);
        // Every source from the last bucket remains, including after >8 batches.
        if ms >= 10 {
            for x in 0..10 {
                assert!(
                    animations
                        .sound_cells(at(ms))
                        .any(|c| c == (Coordinate::new(x, 0), true))
                );
            }
        }
    }
    assert!(animations.sound_cells(at(1799)).next().is_some());
    animations.advance(at(1800));
    assert!(animations.sounds.is_empty());
    animations.enqueue_sounds(vec![sound(Coordinate::new(0, 0))], at(1900));
    animations.clear();
    assert!(animations.sounds.is_empty());
}
