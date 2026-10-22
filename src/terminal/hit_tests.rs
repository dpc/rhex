use super::*;
use crate::game::Actor;
use crate::game::combat_event::CombatEvent;
use crate::terminal::fixtures;

#[test]
fn six_direction_hits_render_strike_then_star_then_normal_cells() {
    use ratatui::{buffer::Buffer, layout::Rect};
    for (source, strike) in [
        (Coordinate::new(1, -1), "-"),
        (Coordinate::new(-1, 1), "-"),
        (Coordinate::new(1, 0), "/"),
        (Coordinate::new(-1, 0), "/"),
        (Coordinate::new(0, 1), "\\"),
        (Coordinate::new(0, -1), "\\"),
    ] {
        let loc = fixtures::room();
        let impact = loc.player().coord();
        let mut renderer = MapRenderer::new('.');
        renderer.resize(21, 11);
        renderer
            .animations
            .enqueue(vec![CombatEvent { source, impact }], Duration::ZERO);
        for (ms, glyph) in [
            (0, strike),
            (199, strike),
            (200, "*"),
            (399, "*"),
            (400, "@"),
        ] {
            renderer.update(&loc, Mode::Normal, None, 0, Duration::from_millis(ms));
            let window = Window::new(21, 11, 0, 0);
            renderer.draw_into(&window);
            let mut buffer = Buffer::empty(Rect::new(0, 0, 21, 11));
            window.render(&mut buffer);
            assert_eq!(buffer[(10, 5)].symbol(), glyph);
        }
    }
}

#[test]
fn hit_overlay_preserves_styles_visibility_targeting_and_immutable_impact() {
    let mut loc = fixtures::room();
    let origin = loc.player().coord();
    let impact = origin + hex2d::XY;
    let id = loc.spawn(Actor::new(Race::Troll, Position::new(impact, hex2d::XY)));
    let mut renderer = MapRenderer::new('.');
    renderer.resize(21, 11);
    renderer.update(&loc, Mode::Normal, None, 0, Duration::ZERO);
    let xy = renderer.coord_to_glyph_xy(impact).unwrap();
    let normal = renderer.actors.at(xy.0, xy.1).unwrap();
    renderer.animations.enqueue(
        vec![CombatEvent {
            source: origin,
            impact,
        }],
        Duration::ZERO,
    );
    renderer.update(&loc, Mode::Normal, None, 1, Duration::ZERO);
    let strike = renderer.actors.at(xy.0, xy.1).unwrap();
    assert_eq!(strike.glyph, '-');
    assert_eq!(strike.fg, normal.fg);
    assert_eq!(strike.bg, normal.bg);
    assert_eq!(strike.bold, normal.bold);

    // Neither disappearance nor later movement changes the saved impact coordinate.
    loc.remove(id);
    renderer.update(&loc, Mode::Normal, None, 0, Duration::from_millis(200));
    assert_eq!(renderer.actors.at(xy.0, xy.1).unwrap().glyph, '*');
    loc.actors_byid
        .get_mut(&loc.player_id())
        .unwrap()
        .visible
        .remove(&impact);
    renderer.update(&loc, Mode::Normal, None, 0, Duration::from_millis(201));
    assert!(
        renderer
            .actors
            .at(xy.0, xy.1)
            .is_none_or(|ch| ch.glyph != '*')
    );
    loc.actors_byid
        .get_mut(&loc.player_id())
        .unwrap()
        .visible
        .insert(impact);
    renderer.update(
        &loc,
        Mode::Target(super::super::ui::TargetMode::Ranged),
        Some(Position::new(impact, hex2d::XY)),
        0,
        Duration::from_millis(202),
    );
    assert_eq!(renderer.effects.at(xy.0, xy.1).last().unwrap().glyph, '*');
    renderer.update(&loc, Mode::Normal, None, 0, Duration::from_millis(400));
    assert!(
        renderer
            .animations
            .glyphs(Duration::from_millis(400))
            .next()
            .is_none()
    );
    assert_ne!(renderer.actors.at(xy.0, xy.1).unwrap().glyph, '*');
}
