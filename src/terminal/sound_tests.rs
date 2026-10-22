use super::*;
use crate::game::sound::{Cell, ObservedSound};
use crate::terminal::fixtures;

fn observed(coord: Coordinate, hidden_source: Option<Coordinate>) -> ObservedSound {
    ObservedSound {
        cells: vec![Cell {
            coord,
            spent: 0,
            hops: 0,
        }],
        hidden_source,
    }
}

fn composed(renderer: &MapRenderer, coord: Coordinate) -> DisplayCharacter {
    let (x, y) = renderer.coord_to_glyph_xy(coord).unwrap();
    renderer
        .actors
        .at(x, y)
        .or(*renderer.items.at(x, y))
        .unwrap_or(renderer.base.at(x, y).base)
}

#[test]
fn real_source_only_sound_never_blinks_but_another_wave_can_cross_its_source() {
    let mut loc = fixtures::room();
    let origin = loc.player().coord();
    let other = loc.player().head();
    let id = loc.spawn(crate::game::Actor::new(
        crate::game::actor::Race::Human,
        Position::new(other, hex2d::XY),
    ));
    let mut renderer = MapRenderer::new('.');
    renderer.resize(21, 11);
    // Only the origin is visible: hearing remains, but no own-source animation.
    loc.actors_byid.get_mut(&0).unwrap().visible = [origin].into_iter().collect();
    loc.actors_byid.get_mut(&0).unwrap().noise_makes(1);
    loc.resolve_sounds();
    assert!(loc.player().heard.contains_key(&origin));
    assert_eq!(loc.sound_events.len(), 1);
    assert!(loc.sound_events[0].cells.is_empty());
    renderer
        .animations
        .enqueue_sounds(std::mem::take(&mut loc.sound_events), Duration::ZERO);
    for ms in [0, 100, 200, 300, 400, 500] {
        assert!(
            renderer
                .animations
                .sound_cells(Duration::from_millis(ms))
                .next()
                .is_none()
        );
    }
    loc.actors_byid.get_mut(&0).unwrap().visible.insert(other);
    loc.actors_byid.get_mut(&0).unwrap().noise_makes(1);
    loc.actors_byid.get_mut(&id).unwrap().noise_makes(1);
    loc.resolve_sounds();
    assert_eq!(loc.sound_events.len(), 2);
    assert!(loc.sound_events[0].cells.iter().all(|c| c.coord != origin));
    assert!(loc.sound_events[1].cells.iter().all(|c| c.coord != other));
    assert!(loc.sound_events[0].cells.iter().any(|c| c.coord == other));
    assert!(loc.sound_events[1].cells.iter().any(|c| c.coord == origin));
    renderer
        .animations
        .enqueue_sounds(std::mem::take(&mut loc.sound_events), Duration::ZERO);
    let cells: Vec<_> = renderer
        .animations
        .sound_cells(Duration::from_millis(300))
        .collect();
    assert!(cells.contains(&(origin, false)));
    assert!(cells.contains(&(other, false)));
    assert!(
        renderer
            .animations
            .sound_cells(Duration::from_millis(900))
            .next()
            .is_none()
    );
}

#[test]
fn waves_add_only_bold_preserve_facing_hit_and_natural_bold_and_restore_cleanly() {
    let mut loc = fixtures::room();
    let origin = loc.player().coord();
    let item_coord = origin + hex2d::XY;
    loc.items.insert(
        item_coord,
        Box::new(item::Item::new(item::Type::Sword, vec![])),
    );
    let wall = Coordinate::new(-1, 0);
    loc.map[wall] = tile::Tile::new(tile::Wall);
    let mut baseline = MapRenderer::new('.');
    let mut animated = MapRenderer::new('.');
    for r in [&mut baseline, &mut animated] {
        r.resize(21, 11);
    }
    animated.animations.enqueue_sounds(
        vec![
            observed(origin, None),
            observed(item_coord, None),
            observed(wall, None),
        ],
        Duration::ZERO,
    );
    for ms in [0, 100, 199, 200, 499, 500, 599, 600] {
        for r in [&mut baseline, &mut animated] {
            r.update(
                &loc,
                Mode::Normal,
                None,
                u32::from(ms != 0),
                Duration::from_millis(ms),
            );
        }
        for coord in [origin, item_coord, wall] {
            let mut expected = composed(&baseline, coord);
            if ms < 600 {
                expected.bold = true;
            }
            assert_eq!(
                composed(&animated, coord),
                expected,
                "time={ms} cell={coord:?}"
            );
        }
    }
    animated.animations.enqueue(
        vec![crate::game::combat_event::CombatEvent {
            source: item_coord,
            impact: origin,
        }],
        Duration::from_millis(600),
    );
    animated
        .animations
        .enqueue_sounds(vec![observed(origin, None)], Duration::from_millis(600));
    animated.update(&loc, Mode::Normal, None, 1, Duration::from_millis(600));
    assert_eq!(composed(&animated, origin).glyph, '-');
    assert!(composed(&animated, origin).bold);
}

#[test]
fn red_unseen_marker_uses_unknown_background_and_never_discovers_or_tracks_actor() {
    let mut loc = fixtures::room();
    let source = Coordinate::new(1, 0);
    let id = loc.spawn(crate::game::Actor::new(
        Race::Troll,
        Position::new(source, hex2d::XY),
    ));
    let player = loc.actors_byid.get_mut(&0).unwrap();
    player.visible.remove(&source);
    player.known.remove(&source);
    let mut renderer = MapRenderer::new('.');
    renderer.resize(21, 11);
    renderer.update(&loc, Mode::Normal, None, 0, Duration::ZERO);
    let unknown = composed(&renderer, source);
    renderer.animations.enqueue_sounds(
        vec![observed(loc.player().coord(), Some(source))],
        Duration::ZERO,
    );
    // Mutations after capture must reveal neither the new tile nor the moving actor.
    loc.map[source].feature = Some(tile::Statue);
    loc.actors_byid.get_mut(&id).unwrap().pos.coord = Coordinate::new(2, 0);
    renderer.update(&loc, Mode::Normal, None, 0, Duration::ZERO);
    assert_eq!(
        composed(&renderer, source),
        DisplayCharacter {
            glyph: '!',
            fg: color::RED_FG.into(),
            bg: unknown.bg,
            bold: true
        }
    );
    assert!(!loc.player().knows(source));
    // Current visibility suppresses the marker, then hiding restores the saved origin.
    loc.actors_byid.get_mut(&0).unwrap().visible.insert(source);
    loc.actors_byid.get_mut(&0).unwrap().known.insert(source);
    renderer.update(&loc, Mode::Normal, None, 0, Duration::from_millis(100));
    assert_ne!(composed(&renderer, source).glyph, '!');
    loc.actors_byid.get_mut(&0).unwrap().visible.remove(&source);
    loc.actors_byid.get_mut(&0).unwrap().known.remove(&source);
    renderer.update(
        &loc,
        Mode::Examine,
        Some(Position::new(source, hex2d::XY)),
        0,
        Duration::from_millis(200),
    );
    let (x, y) = renderer.coord_to_glyph_xy(source).unwrap();
    assert!(!renderer.effects.at(x, y).is_empty());
    renderer.update(&loc, Mode::Normal, None, 0, Duration::from_millis(500));
    assert_eq!(composed(&renderer, source), unknown);
}

#[test]
fn waves_require_display_visibility_and_do_not_invent_new_hidden_markers() {
    let mut loc = fixtures::room();
    let source = Coordinate::new(1, 0);
    let mut renderer = MapRenderer::new('.');
    renderer.resize(21, 11);
    renderer
        .animations
        .enqueue_sounds(vec![observed(source, None)], Duration::ZERO);
    loc.actors_byid.get_mut(&0).unwrap().visible.remove(&source);
    renderer.update(&loc, Mode::Normal, None, 0, Duration::ZERO);
    let hidden = composed(&renderer, source);
    assert_ne!(hidden.glyph, '!');
    renderer.animations.clear();
    renderer.update(&loc, Mode::Normal, None, 0, Duration::ZERO);
    assert_eq!(composed(&renderer, source), hidden);
}
