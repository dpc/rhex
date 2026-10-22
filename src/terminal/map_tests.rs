use super::*;
use crate::game::{Actor, item::Item};
use crate::terminal::{fixtures, ui::TargetMode};
use hex2d::XY;
use ratatui::{buffer::Buffer, layout::Rect, style::Color as TerminalColor};

fn player_mut(loc: &mut Location) -> &mut Actor {
    loc.actors_byid.get_mut(&loc.player_id()).unwrap()
}

fn scene() -> Location {
    let mut loc = fixtures::room();
    for coord in Coordinate::new(0, 0).range_iter(8) {
        loc.light_map[coord] = FULL_LIGHT;
    }
    loc
}

#[test]
fn anonymous_light_sources_render_only_the_underlying_surface() {
    for terrain in [tile::Ground, tile::Empty] {
        for feature in [None, Some(tile::Statue)] {
            let mut loc = scene();
            let coord = Coordinate::new(2, 0);
            loc.map[coord] = tile::Tile::new(terrain);
            loc.map[coord].feature = feature;
            let mut renderer = MapRenderer::new(UNICODE_DOT);
            renderer.resize(20, 12);
            // Hold incident illumination constant: an emitter must not add a
            // marker or recolor grass, floor dots, or unrelated features.
            let ordinary = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
            loc.map[coord].light = 5;
            let emitting = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
            assert_eq!(ordinary, emitting);
        }
    }
}

#[test]
fn torch_is_visible_remembered_and_not_revealed_on_unknown_tiles() {
    use crate::game::Action;
    use hex2d::Angle;

    let mut loc = crate::game::fixtures::room();
    let coord = Coordinate::new(2, 0);
    loc.map[coord].add_feature(tile::Torch).add_light(6);
    loc.post_any_tick();
    assert!(loc.player().sees(coord));
    let mut renderer = MapRenderer::new(UNICODE_DOT);
    renderer.resize(20, 12);
    let visible = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    let (x, y) = renderer.coord_to_glyph_xy(coord).unwrap();
    let xy = (x as u16, y as u16);
    assert_eq!(visible[xy].symbol(), "i");
    assert!(
        renderer
            .sight_entries(&loc)
            .iter()
            .any(|e| e.name == "torch" && e.glyph == 'i')
    );

    loc.act(0, Action::Turn(Angle::Back));
    assert!(!loc.player().sees(coord));
    let hidden = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(
        &hidden,
        xy,
        "i",
        color::REMEMBERED_TERRAIN_FG,
        color::EMPTY_BG.index(false, 0),
        false,
    );
    assert!(
        !renderer
            .sight_entries(&loc)
            .iter()
            .any(|e| e.name == "torch")
    );

    player_mut(&mut loc).known.remove(&coord);
    let unknown = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    assert_eq!(unknown[xy].symbol(), " ");
}

fn render(
    renderer: &mut MapRenderer,
    loc: &Location,
    mode: Mode,
    target: Option<Position>,
    frame: u32,
    width: u16,
    height: u16,
) -> Buffer {
    renderer.update(
        loc,
        mode,
        target,
        frame,
        Duration::from_millis(u64::from(frame) * 100),
    );
    let window = Window::new(width.into(), height.into(), 0, 0);
    renderer.draw_into(&window);
    let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
    window.render(&mut buffer);
    buffer
}

fn cell(buffer: &Buffer, xy: (u16, u16), glyph: &str, fg: u8, bg: u8, bold: bool) {
    let cell = &buffer[xy];
    assert_eq!(cell.symbol(), glyph, "{xy:?}");
    assert_eq!(cell.fg, TerminalColor::Indexed(fg), "{xy:?}");
    assert_eq!(cell.bg, TerminalColor::Indexed(bg), "{xy:?}");
    assert_eq!(
        cell.modifier,
        if bold {
            Modifier::BOLD
        } else {
            Modifier::empty()
        },
        "{xy:?}"
    );
}

#[test]
fn player_body_and_facing_stay_blue_at_every_light_level() {
    for (terrain, backgrounds) in [
        (tile::Empty, color::EMPTY_BG),
        (tile::Ground, color::FOREST_BG),
        (tile::Water, color::WATER_BG),
        (tile::ShallowWater, color::WATER_BG),
    ] {
        for (light, dot_gray) in [
            (0, 238),
            (1, 59),
            (2, 243),
            (3, 245),
            (4, 248),
            (5, 250),
            (6, 250),
        ] {
            let mut loc = scene();
            for coord in loc.player().coord().range_iter(8) {
                loc.map[coord] = tile::Tile::new(terrain);
                loc.light_map[coord] = light;
            }
            let mut renderer = MapRenderer::new(UNICODE_DOT);
            renderer.resize(20, 12);
            let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
            cell(
                &buffer,
                (10, 6),
                "@",
                21,
                backgrounds.index(true, light),
                false,
            );
            cell(
                &buffer,
                (12, 6),
                match terrain {
                    tile::Water => "~",
                    tile::ShallowWater => ",",
                    _ => "•",
                },
                21,
                backgrounds.index(true, light),
                true,
            );
            // Check actual facing-cell colors, not just the palette's indices.
            let facing = &buffer[(12, 6)];
            let TerminalColor::Indexed(bg) = facing.bg else {
                panic!("facing background must be indexed");
            };
            let [r, g, b] = color::tests::channels(bg).map(i32::from);
            assert!(
                r * r + g * g + (255 - b) * (255 - b) >= 40 * 40,
                "{terrain:?}: blue facing marker disappears at light {light}"
            );
            if !matches!(terrain, tile::Water | tile::ShallowWater) {
                cell(
                    &buffer,
                    (8, 4),
                    "•",
                    dot_gray,
                    backgrounds.index(true, light),
                    false,
                );
                player_mut(&mut loc).visible.remove(&Coordinate::new(0, 2));
                let remembered = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
                cell(
                    &remembered,
                    (8, 4),
                    "•",
                    color::REMEMBERED_TERRAIN_FG,
                    backgrounds.index(false, light),
                    false,
                );
                player_mut(&mut loc).visible.insert(Coordinate::new(0, 2));
            }

            // Fixed blue is player-specific, not a recolor of humanoids.
            let coord = Coordinate::new(0, 2);
            for race in [Race::Human, Race::Elf, Race::Dwarf] {
                loc.actors_byid
                    .insert(1, Actor::new(race, Position::new(coord, XY)));
                loc.actors_coord_to_id.insert(coord, 1);
                let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
                cell(
                    &buffer,
                    (8, 4),
                    "@",
                    color::CHAR_HUMANOID_FG.index(true, light),
                    backgrounds.index(true, light),
                    false,
                );
            }
        }
    }
}

#[test]
fn enemy_bodies_and_facing_stay_red_at_every_light_level() {
    for (race, glyph) in [
        (Race::Rat, "r"),
        (Race::Goblin, "g"),
        (Race::Troll, "T"),
        (Race::Wolf, "w"),
        (Race::CaveBeetle, "b"),
        (Race::Skeleton, "s"),
    ] {
        for (terrain, backgrounds, head_glyph) in [
            (tile::Empty, color::EMPTY_BG, "•"),
            (tile::Ground, color::FOREST_BG, "•"),
            (tile::Water, color::WATER_BG, "~"),
            (tile::ShallowWater, color::WATER_BG, ","),
        ] {
            for light in [0, 1, 2, 3, 4, 5, 6, u32::MAX / 2] {
                let mut loc = scene();
                for coord in loc.player().coord().range_iter(8) {
                    loc.map[coord] = tile::Tile::new(terrain);
                    loc.light_map[coord] = light;
                }
                let coord = Coordinate::new(0, 2);
                let actor = Actor::new(race, Position::new(coord, XY));
                let head = actor.head();
                loc.actors_byid.insert(1, actor);
                loc.actors_coord_to_id.insert(coord, 1);
                let mut renderer = MapRenderer::new(UNICODE_DOT);
                renderer.resize(20, 12);
                let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
                let (x, y) = renderer.coord_to_glyph_xy(coord).unwrap();
                cell(
                    &buffer,
                    (x as u16, y as u16),
                    glyph,
                    124,
                    backgrounds.index(true, light),
                    false,
                );
                let (x, y) = renderer.coord_to_glyph_xy(head).unwrap();
                cell(
                    &buffer,
                    (x as u16, y as u16),
                    head_glyph,
                    124,
                    backgrounds.index(true, light),
                    true,
                );
                cell(
                    &buffer,
                    (10, 6),
                    "@",
                    21,
                    backgrounds.index(true, light),
                    false,
                );
                let entry = renderer
                    .sight_entries(&loc)
                    .into_iter()
                    .find(|entry| entry.name == race.description())
                    .unwrap();
                assert_eq!(entry.style.fg, Some(TerminalColor::Indexed(124)));

                // Fixed red must not leak bodies or facing cues outside sight,
                // whether the terrain is remembered or completely unknown.
                for known in [true, false] {
                    player_mut(&mut loc).visible.remove(&coord);
                    if !known {
                        player_mut(&mut loc).known.remove(&coord);
                    }
                    let hidden = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
                    let actor = loc.actors_byid.remove(&1).unwrap();
                    loc.actors_coord_to_id.remove(&coord);
                    let absent = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
                    assert_eq!(hidden, absent, "{race:?}, {terrain:?}, light {light}");
                    loc.actors_byid.insert(1, actor);
                    loc.actors_coord_to_id.insert(coord, 1);
                }
            }
        }
    }
}

#[test]
fn empty_tile_chests_are_shaded_but_the_legend_is_fully_lit() {
    use crate::game::chest::Chest;

    for (opened, has_item, glyph) in [(false, true, "◼"), (true, true, "◻"), (true, false, "_")]
    {
        assert_eq!(ratatui::text::Line::from(glyph).width(), 1);
        for (light, fg) in [(0, 242), (1, 243), (2, 102), (3, 246), (4, 247), (5, 248)] {
            let mut loc = scene();
            let coord = Coordinate::new(0, 2);
            loc.light_map[coord] = light;
            let mut chest =
                Chest::new(has_item.then(|| Box::new(Item::new(item::Type::Whistle, vec![]))));
            chest.opened = opened;
            let name = chest.description();
            loc.chests.insert(coord, chest);
            let mut renderer = MapRenderer::new(UNICODE_DOT);
            renderer.resize(20, 12);
            let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
            cell(
                &buffer,
                (8, 4),
                glyph,
                fg,
                color::EMPTY_BG.index(true, light),
                false,
            );
            let entry = renderer
                .sight_entries(&loc)
                .into_iter()
                .find(|entry| entry.name == name)
                .unwrap();
            assert_eq!(entry.style.fg, Some(TerminalColor::Indexed(248)));
        }
    }
}

#[test]
fn chest_memory_survives_turning_without_observing_hidden_mutations() {
    use crate::game::{Action, chest::Chest};
    use hex2d::Angle;

    for (opened, has_item, glyph) in [(false, true, "◼"), (true, true, "◻"), (true, false, "_")]
    {
        let mut loc = crate::game::fixtures::room();
        let coord = Coordinate::new(2, 0);
        let mut chest =
            Chest::new(has_item.then(|| Box::new(Item::new(item::Type::Whistle, vec![]))));
        chest.opened = opened;
        loc.chests.insert(coord, chest);
        loc.post_any_tick();
        assert!(loc.player().sees(coord));
        let mut renderer = MapRenderer::new(UNICODE_DOT);
        renderer.resize(20, 12);
        let visible = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
        let (x, y) = renderer.coord_to_glyph_xy(coord).unwrap();
        let xy = (x as u16, y as u16);
        assert_eq!(visible[xy].symbol(), glyph);

        loc.act(0, Action::Turn(Angle::Back));
        assert!(!loc.player().sees(coord));
        let hidden = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
        cell(
            &hidden,
            xy,
            glyph,
            color::REMEMBERED_TERRAIN_FG,
            color::EMPTY_BG.index(false, 0),
            false,
        );
        assert!(
            !renderer
                .sight_entries(&loc)
                .iter()
                .any(|e| e.name.contains("chest"))
        );
        while loc.player().action_cd > 0 {
            loc.skip_act(0);
        }
        loc.act(0, Action::Turn(Angle::Back));
        let returned = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
        assert_eq!(returned[xy].symbol(), glyph);
        while loc.player().action_cd > 0 {
            loc.skip_act(0);
        }
        loc.act(0, Action::Turn(Angle::Back));

        // Neither changing contents nor removing the hidden container changes memory.
        let chest = loc.chests.get_mut(&coord).unwrap();
        chest.opened = true;
        chest.contents.clear();
        loc.post_any_tick();
        let mutated = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
        assert_eq!(hidden, mutated);
        loc.chests.remove(&coord);
        loc.post_any_tick();
        let removed = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
        assert_eq!(hidden, removed);

        // A new hidden container on known terrain must not appear either.
        let other = Coordinate::new(3, 0);
        loc.chests.insert(other, Chest::new([]));
        loc.post_any_tick();
        assert!(!loc.remembered_chests.contains_key(&other));
        let added = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
        assert_eq!(hidden, added);

        // Looking back observes the removal and the newly closed chest.
        while loc.player().action_cd > 0 {
            loc.skip_act(0);
        }
        loc.act(0, Action::Turn(Angle::Back));
        assert!(loc.player().sees(coord));
        let returned = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
        assert_eq!(returned[xy].symbol(), "•");
        let (x, y) = renderer.coord_to_glyph_xy(other).unwrap();
        assert_eq!(returned[(x as u16, y as u16)].symbol(), "◼");
        assert!(!loc.remembered_chests.contains_key(&coord));
    }
}

#[test]
fn actor_bodies_inherit_terrain_background_through_facing_and_overlays() {
    for terrain in [tile::Empty, tile::Ground, tile::ShallowWater] {
        for light in [1, 4] {
            let mut loc = scene();
            let origin = loc.player().coord();
            let enemy_coord = loc.player().head();
            loc.actors_byid.insert(
                1,
                Actor::new(Race::CaveBeetle, Position::new(enemy_coord, hex2d::YX)),
            );
            loc.actors_coord_to_id.insert(enemy_coord, 1);
            for coord in origin.range_iter(8) {
                loc.map[coord] = tile::Tile::new(terrain);
                loc.light_map[coord] = light;
            }
            let mut renderer = MapRenderer::new(UNICODE_DOT);
            renderer.resize(20, 12);
            let palette = match terrain {
                tile::Empty => color::EMPTY_BG,
                tile::Ground => color::FOREST_BG,
                tile::ShallowWater => color::WATER_BG,
                _ => unreachable!(),
            };
            let bg = palette.index(true, light);
            for frame in [0, 1, 6] {
                let buffer = render(&mut renderer, &loc, Mode::Normal, None, frame, 20, 12);
                // Both actors face occupied cells: facing changes foreground only.
                cell(&buffer, (10, 6), "@", 196, bg, false);
                cell(&buffer, (12, 6), "b", 21, bg, false);
            }
            let entry = renderer
                .sight_entries(&loc)
                .into_iter()
                .find(|entry| entry.name == "cave beetle")
                .unwrap();
            // The legend retains the terrain category, without map shading.
            assert_eq!(
                entry.style.bg,
                Some(TerminalColor::Indexed(palette.index(true, FULL_LIGHT)))
            );
            let target = Some(Position::new(enemy_coord, XY));
            let buffer = render(
                &mut renderer,
                &loc,
                Mode::Target(TargetMode::Ranged),
                target,
                0,
                20,
                12,
            );
            assert_eq!(buffer[(12, 6)].symbol(), "*");
            assert_eq!(buffer[(12, 6)].bg, TerminalColor::Indexed(bg));
            let buffer = render(&mut renderer, &loc, Mode::Examine, target, 0, 20, 12);
            cell(&buffer, (10, 6), "@", color::CHAR_GRAY_FG, bg, false);
        }
    }
}

#[test]
fn heard_only_enemy_and_previously_seen_movement_leave_no_trail() {
    let mut loc = scene();
    let coord = Coordinate::new(0, 2);
    let prev = Coordinate::new(-1, 2);
    let mut enemy = Actor::new(actor::Race::Rat, Position::new(coord, XY));
    enemy.pre_pos = Some(Position::new(prev, XY));
    enemy.recalculate_stats();
    loc.actors_byid.insert(1, enemy);
    loc.actors_coord_to_id.insert(coord, 1);
    player_mut(&mut loc).visible.remove(&coord);
    player_mut(&mut loc).visible.remove(&prev);
    player_mut(&mut loc).noise_hears(coord, crate::game::Noise::Creature(actor::Race::Rat));
    let mut renderer = MapRenderer::new(UNICODE_DOT);
    renderer.resize(20, 12);
    let heard = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    let actor = loc.actors_byid.remove(&1).unwrap();
    loc.actors_coord_to_id.remove(&coord);
    let absent = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    assert_eq!(
        heard, absent,
        "sound alone must not reveal body or movement"
    );
    loc.actors_byid.insert(1, actor);
    loc.actors_coord_to_id.insert(coord, 1);
    player_mut(&mut loc).visible.insert(prev);
    let moving = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    assert_eq!(moving[(7, 5)].symbol(), "•");
    assert_ne!(moving[(8, 4)].symbol(), "r");
    let faded = render(&mut renderer, &loc, Mode::Normal, None, 6, 20, 12);
    assert_ne!(faded[(7, 5)].symbol(), "/");
}

#[test]
fn death_lights_remembered_terrain_without_revealing_unknown_cells() {
    let mut loc = scene();
    let remembered = Coordinate::new(0, 2);
    let unknown = Coordinate::new(-2, 2);
    loc.map[remembered] = tile::Tile::new(tile::Wall);
    loc.map[unknown] = tile::Tile::new(tile::Wall);
    player_mut(&mut loc).visible.remove(&remembered);
    player_mut(&mut loc).visible.remove(&unknown);
    player_mut(&mut loc).known.remove(&unknown);
    player_mut(&mut loc).hp = 0;
    let mut renderer = MapRenderer::new(UNICODE_DOT);
    renderer.resize(20, 12);
    let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(
        &buffer,
        (8, 4),
        "#",
        color::WALL_FG.index(true, FULL_LIGHT),
        color::WALL_BG.index(true, FULL_LIGHT),
        true,
    );
    cell(
        &buffer,
        (6, 6),
        " ",
        color::MAP_BACKGROUND_BG,
        color::MAP_BACKGROUND_BG,
        false,
    );
}

#[test]
fn dark_forest_remembered_and_unknown_cells_share_true_black() {
    let mut loc = scene();
    let coord = Coordinate::new(0, 2);
    loc.map[coord] = tile::Tile::new(tile::Ground);
    loc.light_map[coord] = 0;
    let mut renderer = MapRenderer::new(UNICODE_DOT);
    renderer.resize(20, 12);

    let visible = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(&visible, (8, 4), "•", 238, 16, false);
    player_mut(&mut loc).visible.remove(&coord);
    let remembered = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(&remembered, (8, 4), "•", 236, 16, false);
    player_mut(&mut loc).known.remove(&coord);
    let unknown = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(&unknown, (8, 4), " ", 16, 16, false);
}

#[test]
fn terrain_has_distinct_rendered_light_steps_but_memory_ignores_live_light() {
    use std::fmt::Write as _;

    use super::color::tests::channels;

    // Integer Rec.709 luma measures the rendered palette, not pre-quantized RGB.
    let luma = |rgb: [u8; 3]| {
        2126 * u32::from(rgb[0]) + 7152 * u32::from(rgb[1]) + 722 * u32::from(rgb[2])
    };
    let mut svg = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1000\" height=\"910\">\
         <rect width=\"100%\" height=\"100%\" fill=\"#181818\"/>\
         <g font-family=\"monospace\" font-size=\"18\" fill=\"white\">\
         <text x=\"20\" y=\"25\">Rendered terrain: light 0 / 1 / 2 / 3 / 4 / 5</text>",
    );
    // Literal indexed colors keep this independent of the production palette.
    // Each visible style covers every light unit from darkness to lantern strength.
    for (row, (terrain, feature, glyph, fg, bg, bold)) in [
        (
            tile::Empty,
            None,
            "•",
            [238, 59, 243, 245, 248, 250],
            [235, 246, 145, 251, 188, 255],
            false,
        ),
        (
            tile::Wall,
            None,
            "#",
            [247, 235, 234, 233, 232, 16],
            [235, 246, 145, 251, 188, 255],
            true,
        ),
        (
            tile::Water,
            None,
            "~",
            [242, 243, 102, 246, 247, 248],
            [16, 17, 23, 24, 25, 26],
            false,
        ),
        (
            tile::ShallowWater,
            None,
            ",",
            [242, 243, 102, 246, 247, 248],
            [16, 17, 23, 24, 25, 26],
            false,
        ),
        (
            tile::Ground,
            None,
            "•",
            [238, 59, 243, 245, 248, 250],
            [16, 234, 236, 22, 28, 34],
            false,
        ),
        (
            tile::Thicket,
            None,
            "γ",
            [22, 28, 70, 71, 77, 119],
            [16, 234, 236, 22, 28, 34],
            false,
        ),
        (
            tile::TreeTrunk,
            None,
            "#",
            [59, 95, 101, 137, 138, 180],
            [16, 234, 236, 22, 28, 34],
            true,
        ),
        (
            tile::Ground,
            Some(tile::ForestPath),
            ">",
            [22, 58, 64, 65, 107, 149],
            [16, 234, 236, 22, 28, 34],
            false,
        ),
        (
            tile::Ground,
            Some(tile::Entrance),
            "<",
            [22, 58, 64, 65, 107, 149],
            [16, 234, 236, 22, 28, 34],
            false,
        ),
        (
            tile::Empty,
            Some(tile::Door(false)),
            "×",
            [247, 235, 234, 233, 232, 16],
            [235, 246, 145, 251, 188, 255],
            false,
        ),
        (
            tile::Empty,
            Some(tile::Door(true)),
            "_",
            [247, 235, 234, 233, 232, 16],
            [235, 246, 145, 251, 188, 255],
            false,
        ),
        (
            tile::Empty,
            Some(tile::Statue),
            "&",
            [247, 235, 234, 233, 232, 16],
            [235, 246, 145, 251, 188, 255],
            false,
        ),
        (
            tile::Empty,
            Some(tile::Stairs),
            ">",
            [247, 235, 234, 233, 232, 16],
            [235, 246, 145, 251, 188, 255],
            false,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut loc = scene();
        let coord = Coordinate::new(0, 2);
        loc.map[coord] = tile::Tile::new(terrain);
        loc.map[coord].feature = feature;
        let mut renderer = MapRenderer::new(UNICODE_DOT);
        renderer.resize(20, 12);
        let mut rendered_backgrounds = std::collections::BTreeSet::new();
        let mut rendered_foregrounds = std::collections::BTreeSet::new();
        let mut previous_luma = None;
        writeln!(
            svg,
            "<text x=\"10\" y=\"{}\">{terrain:?} {feature:?}</text>",
            55 + row * 65
        )
        .unwrap();
        for (band, light) in (0..=5).enumerate() {
            // Opaque terrain takes its apparent light from neighboring cells.
            for lit_coord in Coordinate::new(0, 0).range_iter(8) {
                loc.light_map[lit_coord] = light;
            }
            let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
            cell(&buffer, (8, 4), glyph, fg[band], bg[band], bold);
            let rendered = &buffer[(8, 4)];
            let TerminalColor::Indexed(background) = rendered.bg else {
                panic!("terrain must use indexed backgrounds");
            };
            let TerminalColor::Indexed(foreground) = rendered.fg else {
                panic!("terrain must use indexed foregrounds");
            };
            let rgb = channels(background);
            let fg_rgb = channels(foreground);
            assert!(
                rendered_foregrounds.insert(fg_rgb),
                "{terrain:?}: collapsed foreground at light {light}"
            );
            assert!(
                rendered_backgrounds.insert(rgb),
                "{terrain:?}: light {light}"
            );
            let brightness = luma(rgb);
            if let Some(previous) = previous_luma {
                assert!(brightness > previous, "{terrain:?}: light {light}");
            }
            previous_luma = Some(brightness);
            assert!(
                brightness.abs_diff(luma(fg_rgb)) >= 28 * 10000,
                "{terrain:?}: unreadable glyph at light {light}"
            );
            let [r, g, b] = rgb;
            let [fr, fg, fb] = fg_rgb;
            let x = 310 + band * 110;
            let y = 35 + row * 65;
            let symbol = rendered.symbol().replace('&', "&amp;").replace('<', "&lt;");
            writeln!(
                svg,
                "<rect x=\"{x}\" y=\"{y}\" width=\"105\" height=\"42\" fill=\"rgb({r},{g},{b})\"/>\
                 <text x=\"{}\" y=\"{}\" fill=\"rgb({fr},{fg},{fb})\" font-weight=\"{}\">{symbol} {background}</text>",
                x + 10,
                y + 27,
                if bold { "bold" } else { "normal" }
            )
            .unwrap();
        }
        assert_eq!(rendered_backgrounds.len(), 6);
        for lit_coord in Coordinate::new(0, 0).range_iter(8) {
            loc.light_map[lit_coord] = 100;
        }
        let saturated = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
        cell(&saturated, (8, 4), glyph, fg[5], bg[5], bold);
        player_mut(&mut loc).visible.remove(&coord);
        // Current lighting must not brighten terrain outside sight.
        for light in [0, 1, 3, 5, u32::MAX / 2] {
            for lit_coord in Coordinate::new(0, 0).range_iter(8) {
                loc.light_map[lit_coord] = light;
            }
            let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
            cell(&buffer, (8, 4), glyph, 236, 16, bold);
        }
    }
    svg.push_str("</g></svg>");
    // Optional visual evidence uses the same rendered cells as the assertions.
    if let Some(path) = std::env::var_os("RHEX_PALETTE_SVG") {
        std::fs::write(path, svg).unwrap();
    }
}

#[test]
fn shoreline_glue_matches_ground_light_in_both_directions() {
    let left = Coordinate::new(0, 2);
    let right = left + Direction::XY;
    for water in [tile::Water, tile::ShallowWater] {
        for (ground, wet) in [(left, right), (right, left)] {
            for (ground_light, water_light, expected_bg) in
                [(4, 0, 28), (3, 1, 22), (1, 4, 234), (0, 4, 16)]
            {
                let mut loc = scene();
                loc.map[ground] = tile::Tile::new(tile::Ground);
                loc.map[wet] = tile::Tile::new(water);
                loc.light_map[ground] = ground_light;
                loc.light_map[wet] = water_light;
                let mut renderer = MapRenderer::new(UNICODE_DOT);
                renderer.resize(20, 12);
                let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
                assert_eq!(buffer[(9, 4)].symbol(), " ");
                assert_eq!(buffer[(9, 4)].bg, TerminalColor::Indexed(expected_bg));

                // Hidden ground must not lend its current lighting to visible water.
                player_mut(&mut loc).visible.remove(&ground);
                let before = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
                loc.light_map[ground] = 6;
                let after = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
                assert_eq!(before[(9, 4)], after[(9, 4)]);

                player_mut(&mut loc).visible.remove(&wet);
                let remembered = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
                cell(&remembered, (9, 4), " ", 236, 16, false);
                player_mut(&mut loc).known.remove(&ground);
                player_mut(&mut loc).known.remove(&wet);
                let unknown = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
                cell(&unknown, (9, 4), " ", 16, 16, false);
            }
        }
    }
}

#[test]
fn adjacent_water_keeps_tile_glyphs_and_continuous_background_without_glue_glyphs() {
    let left = Coordinate::new(0, 2);
    let right = left + Direction::XY;
    for (left_water, left_glyph) in [(tile::Water, "~"), (tile::ShallowWater, ",")] {
        for (right_water, right_glyph) in [(tile::Water, "~"), (tile::ShallowWater, ",")] {
            for (light, fg, bg) in [(0, 242, 16), (1, 243, 17), (3, 246, 24), (5, 248, 26)] {
                for (left_seen, right_seen) in
                    [(true, true), (true, false), (false, true), (false, false)]
                {
                    let mut loc = scene();
                    for (coord, water, seen) in [
                        (left, left_water, left_seen),
                        (right, right_water, right_seen),
                    ] {
                        loc.map[coord] = tile::Tile::new(water);
                        loc.light_map[coord] = light;
                        if !seen {
                            player_mut(&mut loc).visible.remove(&coord);
                        }
                    }
                    let mut renderer = MapRenderer::new(UNICODE_DOT);
                    renderer.resize(20, 12);
                    let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
                    for (xy, glyph, seen) in [
                        ((8, 4), left_glyph, left_seen),
                        ((9, 4), " ", left_seen || right_seen),
                        ((10, 4), right_glyph, right_seen),
                    ] {
                        let (fg, bg) = if seen { (fg, bg) } else { (236, 16) };
                        cell(&buffer, xy, glyph, fg, bg, false);
                    }

                    // A hidden neighbor cannot change the water glue's lighting.
                    for (coord, seen) in [(left, left_seen), (right, right_seen)] {
                        if !seen {
                            loc.light_map[coord] = 6;
                        }
                    }
                    let relit = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
                    assert_eq!(buffer[(9, 4)], relit[(9, 4)]);

                    if !left_seen && !right_seen {
                        for coord in [left, right] {
                            player_mut(&mut loc).known.remove(&coord);
                        }
                        let unknown = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
                        for xy in [(8, 4), (9, 4), (10, 4)] {
                            cell(&unknown, xy, " ", 16, 16, false);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn projected_terrain_visibility_light_and_glue_have_exact_cells() {
    let mut loc = scene();
    let unknown = Coordinate::new(-2, 2);
    let remembered = Coordinate::new(0, 2);
    let water = Coordinate::new(2, 0);
    let door = Coordinate::new(0, -2);
    player_mut(&mut loc).known.remove(&unknown);
    player_mut(&mut loc).visible.remove(&unknown);
    player_mut(&mut loc).visible.remove(&remembered);
    loc.map[unknown] = tile::Tile::new(tile::Wall);
    loc.map[remembered] = tile::Tile::new(tile::Wall);
    loc.map[water] = tile::Tile::new(tile::Water);
    loc.map[door].feature = Some(tile::Door(false));
    // Explicit projected offsets: (x-y, -x-y), not the renderer's helper.
    for (width, height, cx, cy) in [(20, 12, 10, 6), (21, 13, 10, 6)] {
        let mut renderer = MapRenderer::new(UNICODE_DOT);
        renderer.resize(width as usize, height as usize);
        let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, width, height);
        cell(
            &buffer,
            (cx - 4, cy),
            " ",
            color::MAP_BACKGROUND_BG,
            color::MAP_BACKGROUND_BG,
            false,
        );
        cell(
            &buffer,
            (cx - 2, cy - 2),
            "#",
            color::WALL_FG.index(false, 0),
            color::WALL_BG.index(false, 0),
            true,
        );
        cell(
            &buffer,
            (cx + 2, cy - 2),
            "~",
            color::WATER_FG.index(true, FULL_LIGHT),
            color::WATER_BG.index(true, FULL_LIGHT),
            false,
        );
        cell(
            &buffer,
            (cx + 2, cy + 2),
            "×",
            color::STONE_FG.index(true, FULL_LIGHT),
            color::WALL_BG.index(true, FULL_LIGHT),
            false,
        );
        cell(
            &buffer,
            (cx - 1, cy),
            " ",
            color::STONE_FG.index(true, FULL_LIGHT),
            color::EMPTY_BG.index(true, FULL_LIGHT),
            false,
        );
        cell(
            &buffer,
            (cx, cy),
            "@",
            21,
            color::EMPTY_BG.index(true, FULL_LIGHT),
            false,
        );
        cell(
            &buffer,
            (cx + 2, cy),
            "•",
            21,
            color::EMPTY_BG.index(true, FULL_LIGHT),
            true,
        );
    }
    let mut renderer = MapRenderer::new(UNICODE_DOT);
    renderer.resize(20, 12);
    for light in 0..=5 {
        loc.light_map[water] = light;
        let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
        cell(
            &buffer,
            (12, 4),
            "~",
            color::WATER_FG.index(true, light),
            color::WATER_BG.index(true, light),
            false,
        );
    }
    loc.map[door].feature = Some(tile::Door(true));
    let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(
        &buffer,
        (12, 8),
        "_",
        color::STONE_FG.index(true, FULL_LIGHT),
        color::EMPTY_BG.index(true, FULL_LIGHT),
        false,
    );
}

#[test]
fn concealed_entities_do_not_change_ordinary_output_but_visible_heads_can_mark_unknown() {
    let mut loc = scene();
    let hidden = Coordinate::new(-3, 3);
    player_mut(&mut loc).known.remove(&hidden);
    player_mut(&mut loc).visible.remove(&hidden);
    let mut renderer = MapRenderer::new(UNICODE_DOT);
    renderer.resize(20, 12);
    let before = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    // Keep the same glue expansion as empty terrain so only concealed cell
    // identity changes, not a legitimately known neighboring glue cell.
    loc.map[hidden].feature = Some(tile::Stairs);
    loc.items
        .insert(hidden, Box::new(Item::new(item::Type::Bow, vec![])));
    loc.actors_byid
        .insert(1, Actor::new(Race::Rat, Position::new(hidden, XY)));
    loc.actors_coord_to_id.insert(hidden, 1);
    let after = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    assert_eq!(before, after);

    // Remembered items and unseen bodies remain absent.
    player_mut(&mut loc).known.insert(hidden);
    let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(
        &buffer,
        (4, 6),
        ">",
        color::STONE_FG.index(false, 0),
        color::EMPTY_BG.index(false, 0),
        false,
    );

    // Intentional exception: the player's facing marker can mark unknown space.
    let head = Coordinate::new(1, -1);
    player_mut(&mut loc).known.remove(&head);
    player_mut(&mut loc).visible.remove(&head);
    let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(&buffer, (12, 6), " ", color::MAP_BACKGROUND_BG, 21, true);
}

#[test]
fn layers_target_blockage_and_attribute_reset_are_exact() {
    let mut loc = scene();
    let coord = Coordinate::new(2, -2);
    let mut renderer = MapRenderer::new(UNICODE_DOT);
    renderer.resize(20, 12);
    loc.items
        .insert(coord, Box::new(Item::new(item::Type::Knife, vec![])));
    player_mut(&mut loc).discovered.insert(coord);
    let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(
        &buffer,
        (14, 6),
        ")",
        color::WALL_FG.index(true, FULL_LIGHT),
        color::EMPTY_BG.index(true, FULL_LIGHT),
        true,
    );
    loc.actors_byid
        .insert(1, Actor::new(Race::Rat, Position::new(coord, XY)));
    loc.actors_coord_to_id.insert(coord, 1);
    let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(
        &buffer,
        (14, 6),
        "r",
        color::CHAR_ENEMY_FG.index(true, FULL_LIGHT),
        color::EMPTY_BG.index(true, FULL_LIGHT),
        false,
    );
    let target = Some(Position::new(Coordinate::new(4, -4), XY));
    loc.map[Coordinate::new(3, -3)] = tile::Tile::new(tile::Wall);
    player_mut(&mut loc).known.remove(&Coordinate::new(4, -4));
    player_mut(&mut loc).visible.remove(&Coordinate::new(4, -4));
    let buffer = render(
        &mut renderer,
        &loc,
        Mode::Target(TargetMode::Ranged),
        target,
        0,
        20,
        12,
    );
    cell(
        &buffer,
        (14, 6),
        "*",
        color::CHAR_HUMANOID_FG.index(true, FULL_LIGHT),
        color::EMPTY_BG.index(true, FULL_LIGHT),
        false,
    );
    cell(
        &buffer,
        (16, 6),
        "#",
        color::WALL_FG.index(true, FULL_LIGHT),
        color::BLOCKED_BG,
        true,
    );
    cell(
        &buffer,
        (18, 6),
        " ",
        color::MAP_BACKGROUND_BG,
        color::BLOCKED_BG,
        true,
    );

    loc.actors_byid.remove(&1);
    loc.actors_coord_to_id.remove(&coord);
    loc.items.clear();
    loc.map[Coordinate::new(3, -3)] = tile::Tile::new(tile::Empty);
    let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(
        &buffer,
        (14, 6),
        "•",
        color::EMPTY_DOT_FG.index(true, FULL_LIGHT),
        color::EMPTY_BG.index(true, FULL_LIGHT),
        false,
    );
    cell(
        &buffer,
        (16, 6),
        "•",
        color::EMPTY_DOT_FG.index(true, FULL_LIGHT),
        color::EMPTY_BG.index(true, FULL_LIGHT),
        false,
    );
}

#[test]
fn occupied_heads_use_distinct_colors_then_reset_when_the_head_turns_away() {
    let mut loc = scene();
    let coord = loc.player().head();
    loc.actors_byid
        .insert(1, Actor::new(Race::Rat, Position::new(coord, XY)));
    loc.actors_coord_to_id.insert(coord, 1);
    let mut renderer = MapRenderer::new(UNICODE_DOT);
    renderer.resize(20, 12);
    let occupied = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(
        &occupied,
        (12, 6),
        "r",
        21,
        color::EMPTY_BG.index(true, FULL_LIGHT),
        false,
    );
    // A different enemy's head now overlaps the same rat.
    player_mut(&mut loc).pos.dir = hex2d::YX;
    let other = Coordinate::new(2, -2);
    loc.actors_byid
        .insert(2, Actor::new(Race::Rat, Position::new(other, hex2d::YX)));
    loc.actors_coord_to_id.insert(other, 2);
    let occupied = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(
        &occupied,
        (12, 6),
        "r",
        196,
        color::EMPTY_BG.index(true, FULL_LIGHT),
        false,
    );
    loc.actors_byid.get_mut(&2).unwrap().pos.dir = XY;
    let clear = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(
        &clear,
        (12, 6),
        "r",
        color::CHAR_ENEMY_FG.index(true, FULL_LIGHT),
        color::EMPTY_BG.index(true, FULL_LIGHT),
        false,
    );
    // An enemy pointing at the player changes only the player's foreground too.
    loc.actors_byid.get_mut(&1).unwrap().pos.dir = hex2d::YX;
    let player_head = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(
        &player_head,
        (10, 6),
        "@",
        196,
        color::EMPTY_BG.index(true, FULL_LIGHT),
        false,
    );
}

#[test]
fn movement_and_turns_leave_no_trails_and_mode_changes_rebuild_camera_cache() {
    let mut loc = scene();
    let coord = Coordinate::new(2, -2);
    let mut rat = Actor::new(Race::Rat, Position::new(coord, XY));
    rat.pre_pos = Some(Position::new(Coordinate::new(1, -1), XY));
    loc.actors_byid.insert(1, rat);
    loc.actors_coord_to_id.insert(coord, 1);
    let mut renderer = MapRenderer::new(UNICODE_DOT);
    renderer.resize(20, 12);
    for frame in [0, 1, 5, 6] {
        let buffer = render(&mut renderer, &loc, Mode::Normal, None, frame, 20, 12);
        assert_eq!(buffer[(12, 6)].symbol(), "•");
        assert_eq!(buffer[(14, 6)].symbol(), "r");
    }
    let target = Some(Position::new(coord, XY));
    let buffer = render(&mut renderer, &loc, Mode::Examine, target, 0, 20, 12);
    cell(
        &buffer,
        (10, 6),
        "@",
        color::CHAR_GRAY_FG,
        color::EMPTY_BG.index(true, FULL_LIGHT),
        false,
    );
    assert_eq!(buffer[(6, 6)].symbol(), "@"); // Player moves on screen, not in world.
    let buffer = render(
        &mut renderer,
        &loc,
        Mode::Target(TargetMode::Ranged),
        target,
        0,
        20,
        12,
    );
    assert_eq!(buffer[(10, 6)].symbol(), "*"); // Target stays centered on player.
    assert_eq!(buffer[(14, 6)].symbol(), "*");
    let rat = loc.actors_byid.get_mut(&1).unwrap();
    rat.pre_pos = Some(rat.pos);
    rat.pos.dir = hex2d::XZ;
    let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    assert_eq!(buffer[(16, 6)].symbol(), "•");
    let buffer = render(&mut renderer, &loc, Mode::Normal, None, 6, 20, 12);
    assert_eq!(buffer[(16, 6)].symbol(), "•");
    assert_eq!(loc.player().pos.coord, Coordinate::new(0, 0));
}

#[test]
fn shared_heads_blink_on_elapsed_seconds_independent_of_order_and_occupancy() {
    for occupied in [false, true] {
        for reverse in [false, true] {
            let mut loc = scene();
            let head = loc.player().head();
            let enemy_coord = Coordinate::new(2, -2);
            let mut additions = vec![
                (
                    1,
                    Actor::new(Race::Rat, Position::new(enemy_coord, hex2d::YX)),
                ),
                (
                    2,
                    Actor::new(
                        Race::Goblin,
                        Position::new(Coordinate::new(1, 0), hex2d::YZ),
                    ),
                ),
            ];
            if occupied {
                additions.push((3, Actor::new(Race::Rat, Position::new(head, XY))));
            }
            if reverse {
                additions.reverse();
            }
            for (id, actor) in additions {
                loc.actors_coord_to_id.insert(actor.pos.coord, id);
                loc.actors_byid.insert(id, actor);
            }
            let mut renderer = MapRenderer::new(UNICODE_DOT);
            renderer.resize(20, 12);
            for (ms, fg) in [(0, 196), (999, 196), (1000, 21), (1999, 21), (2000, 196)] {
                // Exercise both terrain rebuilds and animation-only draws.
                for frame in [0, 7] {
                    renderer.update(&loc, Mode::Normal, None, frame, Duration::from_millis(ms));
                    let window = Window::new(20, 12, 0, 0);
                    renderer.draw_into(&window);
                    let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 12));
                    window.render(&mut buffer);
                    cell(
                        &buffer,
                        (12, 6),
                        if occupied { "r" } else { "•" },
                        fg,
                        color::EMPTY_BG.index(true, FULL_LIGHT),
                        !occupied,
                    );
                }
            }
            // Losing all visible enemy contributors restores the player's cue.
            player_mut(&mut loc).visible.remove(&enemy_coord);
            let other = loc.actors_byid[&2].pos.coord;
            player_mut(&mut loc).visible.remove(&other);
            let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
            cell(
                &buffer,
                (12, 6),
                if occupied { "r" } else { "•" },
                21,
                color::EMPTY_BG.index(true, FULL_LIGHT),
                !occupied,
            );

            player_mut(&mut loc).visible.insert(enemy_coord);
            loc.actors_byid.get_mut(&1).unwrap().hp = 0;
            let dead = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
            assert_eq!(dead[(12, 6)], buffer[(12, 6)]);
            loc.actors_byid.get_mut(&1).unwrap().hp = 1;
            loc.actors_byid.get_mut(&1).unwrap().pos.dir = XY;
            let turned = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
            assert_eq!(turned[(12, 6)], buffer[(12, 6)]);
        }
    }
}

#[test]
fn player_movement_and_turns_never_replace_terrain_with_trail_glyphs() {
    for previous in [
        Position::new(Coordinate::new(-1, 1), XY),
        Position::new(Coordinate::new(0, 0), hex2d::XZ),
    ] {
        let mut loc = scene();
        player_mut(&mut loc).pre_pos = Some(previous);
        let mut renderer = MapRenderer::new(UNICODE_DOT);
        renderer.resize(20, 12);
        for frame in [0, 1, 5, 6] {
            let buffer = render(&mut renderer, &loc, Mode::Normal, None, frame, 20, 12);
            assert!(
                buffer
                    .content
                    .iter()
                    .all(|cell| !matches!(cell.symbol(), "-" | "/" | "\\"))
            );
            assert_eq!(loc.player().pre_pos, Some(previous));
        }
    }
}
