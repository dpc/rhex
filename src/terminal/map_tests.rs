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
        loc.light_map[coord] = 4;
    }
    loc
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
    renderer.update(loc, mode, target, frame);
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
fn heard_only_enemy_is_not_drawn_but_previously_seen_movement_leaves_a_trail() {
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
    assert_eq!(moving[(7, 5)].symbol(), "/");
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
        color::WALL_FG[0],
        color::WALL_BG[0],
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
            color::WALL_FG[3],
            color::WALL_BG[3],
            true,
        );
        cell(
            &buffer,
            (cx + 2, cy - 2),
            "~",
            color::WATER_FG[0],
            color::WATER_BG[0],
            false,
        );
        cell(
            &buffer,
            (cx + 2, cy + 2),
            "×",
            color::STONE_FG[0],
            color::WALL_BG[0],
            false,
        );
        cell(
            &buffer,
            (cx - 1, cy),
            " ",
            color::STONE_FG[0],
            color::EMPTY_BG[0],
            false,
        );
        cell(
            &buffer,
            (cx, cy),
            "@",
            color::CHAR_SELF_FG[0],
            color::CHAR_BG[0],
            false,
        );
        cell(
            &buffer,
            (cx + 2, cy),
            "•",
            color::CHAR_SELF_FG[0],
            color::EMPTY_BG[0],
            true,
        );
    }
    let mut renderer = MapRenderer::new(UNICODE_DOT);
    renderer.resize(20, 12);
    for (light, band) in [(0, 2), (1, 1), (2, 1), (3, 0)] {
        loc.light_map[water] = light;
        let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
        cell(
            &buffer,
            (12, 4),
            "~",
            color::WATER_FG[band],
            color::WATER_BG[band],
            false,
        );
    }
    loc.map[door].feature = Some(tile::Door(true));
    let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(
        &buffer,
        (12, 8),
        "_",
        color::STONE_FG[0],
        color::EMPTY_BG[0],
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
        color::STONE_FG[3],
        color::EMPTY_BG[3],
        false,
    );

    // Intentional exception: the player's facing marker can mark unknown space.
    let head = Coordinate::new(1, -1);
    player_mut(&mut loc).known.remove(&head);
    player_mut(&mut loc).visible.remove(&head);
    let buffer = render(&mut renderer, &loc, Mode::Normal, None, 0, 20, 12);
    cell(
        &buffer,
        (12, 6),
        " ",
        color::MAP_BACKGROUND_BG,
        color::CHAR_SELF_FG[0],
        true,
    );
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
        color::WALL_FG[0],
        color::EMPTY_BG[0],
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
        color::CHAR_ENEMY_FG[0],
        color::CHAR_BG[0],
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
        color::CHAR_SELF_FG[0],
        color::EMPTY_BG[0],
        false,
    );
    cell(
        &buffer,
        (16, 6),
        "#",
        color::WALL_FG[0],
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
        color::EMPTY_FG[0],
        color::EMPTY_BG[0],
        false,
    );
    cell(
        &buffer,
        (16, 6),
        "•",
        color::EMPTY_FG[0],
        color::EMPTY_BG[0],
        false,
    );
}

#[test]
fn movement_and_turn_trails_expire_and_mode_changes_rebuild_camera_cache() {
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
        assert_eq!(buffer[(12, 6)].symbol(), if frame < 6 { "-" } else { "•" });
        assert_eq!(buffer[(14, 6)].symbol(), "r");
    }
    let target = Some(Position::new(coord, XY));
    let buffer = render(&mut renderer, &loc, Mode::Examine, target, 0, 20, 12);
    cell(
        &buffer,
        (10, 6),
        "@",
        color::CHAR_GRAY_FG,
        color::EMPTY_BG[0],
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
    assert_eq!(buffer[(16, 6)].symbol(), "\\");
    let buffer = render(&mut renderer, &loc, Mode::Normal, None, 6, 20, 12);
    assert_eq!(buffer[(16, 6)].symbol(), "•");
    assert_eq!(loc.player().pos.coord, Coordinate::new(0, 0));
}
