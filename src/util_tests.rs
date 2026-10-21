use super::*;

#[test]
fn off_center_movement_uses_radius_not_cursor_facing() {
    let center = Position::new(Coordinate::new(3, -2), hex2d::YZ);
    let cursor = Position::new(Coordinate::new(5, -2), hex2d::ZX);
    for (angle, expected) in [
        (Angle::Left, Coordinate::new(4, -1)),
        (Angle::Right, Coordinate::new(5, -3)),
        (Angle::Forward, Coordinate::new(6, -2)),
        (Angle::Back, Coordinate::new(4, -2)),
    ] {
        assert_eq!(
            circular_move(center, cursor, angle).coord,
            expected,
            "{angle:?}"
        );
    }
}

#[test]
fn circular_movement_at_center_turns_or_steps_along_facing() {
    for &dir in Direction::all() {
        let center = Position::new(Coordinate::new(3, -2), dir);
        for angle in [Angle::Left, Angle::Right] {
            let turned = circular_move(center, center, angle);
            assert_eq!(turned.coord, center.coord);
            assert_eq!(turned.dir, dir + angle);
        }
        assert_eq!(
            circular_move(center, center, Angle::Forward).coord,
            center.coord + dir
        );
        assert_eq!(
            circular_move(center, center, Angle::Back).coord,
            center.coord - dir
        );
    }
}

#[test]
fn circling_a_hex_ring_visits_every_cell_and_returns_to_start() {
    let center = Position::new(Coordinate::new(-3, 2), Direction::from_int(0));
    for radius in [1, 2, 3] {
        for angle in [Angle::Left, Angle::Right] {
            let start = Position::new(
                center.coord + Coordinate::new(radius, 0),
                Direction::from_int(0),
            );
            let mut current = start;
            let mut visited = std::collections::HashSet::new();
            for _ in 0..6 * radius {
                assert!(visited.insert(current.coord));
                let next = circular_move(center, current, angle);
                assert_eq!(center.coord.distance(next.coord), radius);
                assert_eq!(current.coord.distance(next.coord), 1);
                current = next;
            }
            assert_eq!(current.coord, start.coord);
        }
    }
}
