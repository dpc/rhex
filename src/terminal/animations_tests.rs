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
