use super::*;
use crate::game::sound::{Cell, ObservedSound};
use crate::game::{Actor, awareness_event::AwarenessEvent, combat_event::CombatEvent, fixtures};

fn composed(renderer: &MapRenderer, coord: Coordinate) -> DisplayCharacter {
    let (x, y) = renderer.coord_to_glyph_xy(coord).unwrap();
    renderer
        .actors
        .at(x, y)
        .or(*renderer.items.at(x, y))
        .unwrap_or(renderer.base.at(x, y).base)
}

fn setup() -> (Location, actor::Id, MapRenderer) {
    let mut loc = fixtures::room();
    let id = loc.spawn(Actor::new(
        Race::Human,
        Position::new(Coordinate::new(1, 0), hex2d::XY + hex2d::Angle::Back),
    ));
    assert_eq!(loc.awareness_events, [AwarenessEvent { actor: id }]);
    let mut renderer = MapRenderer::new('.');
    renderer.resize(21, 11);
    renderer
        .animations
        .enqueue_awareness(std::mem::take(&mut loc.awareness_events), Duration::ZERO);
    (loc, id, renderer)
}

#[test]
fn awareness_exact_yellow_style_over_hit_sound_and_facing_restores_at_500ms() {
    let (loc, id, mut renderer) = setup();
    let coord = loc.actors_byid[&id].coord();
    let mut baseline = MapRenderer::new('.');
    baseline.resize(21, 11);
    renderer.animations.enqueue(
        vec![CombatEvent {
            source: loc.player().coord(),
            impact: coord,
        }],
        Duration::ZERO,
    );
    renderer.animations.enqueue_sounds(
        vec![ObservedSound {
            cells: vec![Cell {
                coord,
                spent: 0,
                hops: 0,
            }],
            hidden_source: Some(coord), // Red is suppressed on this visible cell.
        }],
        Duration::ZERO,
    );
    for ms in [0, 199, 200, 399, 400, 499, 500, 599, 600, 1000] {
        let now = Duration::from_millis(ms);
        renderer.update(&loc, Mode::Normal, None, 0, now);
        baseline.update(&loc, Mode::Normal, None, 0, now);
        let mut expected = composed(&baseline, coord);
        if ms < 500 {
            expected.glyph = '!';
            expected.fg = color::YELLOW.into();
            expected.bold = true;
            assert_eq!(expected.fg, color::Color::from(226u8));
        } else if ms < 600 {
            expected.bold = true;
        }
        assert_eq!(composed(&renderer, coord), expected, "{ms}ms");
    }
}

#[test]
fn awareness_follows_identity_not_old_cell_and_hides_with_visibility_or_death() {
    let (mut loc, id, mut renderer) = setup();
    let old = loc.actors_byid[&id].coord();
    let new = Coordinate::new(2, 0);
    loc.actors_coord_to_id.remove(&old);
    loc.actors_byid.get_mut(&id).unwrap().pos.coord = new;
    loc.actors_coord_to_id.insert(new, id);
    let other = loc.spawn(Actor::new(Race::Rat, Position::new(old, hex2d::XY)));
    renderer.update(&loc, Mode::Normal, None, 0, Duration::from_millis(100));
    assert_eq!(composed(&renderer, new).glyph, '!');
    assert_ne!(composed(&renderer, old).glyph, '!');

    loc.actors_byid.get_mut(&0).unwrap().visible.remove(&new);
    renderer.update(&loc, Mode::Normal, None, 0, Duration::from_millis(200));
    assert_ne!(composed(&renderer, new).glyph, '!');
    loc.actors_byid.get_mut(&0).unwrap().visible.insert(new);
    renderer.update(&loc, Mode::Normal, None, 0, Duration::from_millis(300));
    assert_eq!(composed(&renderer, new).glyph, '!');
    loc.actors_byid.get_mut(&id).unwrap().hp = 0;
    renderer.update(&loc, Mode::Normal, None, 0, Duration::from_millis(301));
    assert_ne!(composed(&renderer, new).glyph, '!');
    loc.remove(id);
    loc.remove(other);
    let replacement = loc.spawn(Actor::new(Race::Rat, Position::new(new, hex2d::XY)));
    assert_ne!(id, replacement);
    renderer.update(&loc, Mode::Normal, None, 0, Duration::from_millis(400));
    assert_ne!(composed(&renderer, new).glyph, '!');
}

#[test]
fn awareness_target_and_examine_effects_remain_above_marker_and_resize_keeps_time() {
    let (loc, id, mut renderer) = setup();
    let coord = loc.actors_byid[&id].coord();
    for mode in [
        Mode::Target(super::super::ui::TargetMode::Ranged),
        Mode::Examine,
    ] {
        renderer.update(
            &loc,
            mode,
            Some(Position::new(coord, hex2d::XY)),
            0,
            Duration::ZERO,
        );
        let (x, y) = renderer.coord_to_glyph_xy(coord).unwrap();
        assert_eq!(composed(&renderer, coord).glyph, '!');
        assert!(!renderer.effects.at(x, y).is_empty());
        let window = Window::new(21, 11, 0, 0);
        renderer.draw_into(&window);
        let mut buffer = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 21, 11));
        window.render(&mut buffer);
        assert_eq!(
            buffer[(x as u16, y as u16)].symbol(),
            renderer.effects.at(x, y).last().unwrap().glyph.to_string()
        );
    }
    renderer.resize(31, 15);
    renderer.update(&loc, Mode::Normal, None, 0, Duration::from_millis(499));
    assert_eq!(composed(&renderer, coord).glyph, '!');
    renderer.update(&loc, Mode::Normal, None, 0, Duration::from_millis(500));
    assert_ne!(composed(&renderer, coord).glyph, '!');
}
