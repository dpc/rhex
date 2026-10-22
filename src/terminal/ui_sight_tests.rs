//! Composed HUD behavior and current-visibility boundaries.
use super::*;
use crate::game::item::{Feature, Item, Type};
use crate::terminal::fixtures;
use ratatui::layout::Rect;

fn fixture(loc: Location) -> Ui {
    let mut ui = Ui::new(game::Engine::fixture(loc), (24, 60), Instant::now());
    ui.spawned = true;
    ui.mode_switch_to(Mode::Normal);
    ui
}

fn frame(ui: &mut Ui) -> Buffer {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 60, 24));
    ui.render_into(&mut buffer);
    buffer
}

fn row(buffer: &Buffer, y: u16) -> String {
    (30..60).map(|x| buffer[(x, y)].symbol()).collect()
}

#[test]
fn chest_map_legend_and_readout_reveal_only_open_visible_contents() {
    use crate::game::chest::Chest;
    let mut loc = fixtures::room();
    let c = loc.player().head();
    loc.chests
        .insert(c, Chest::new([Box::new(Item::new(Type::Whistle, vec![]))]));
    let mut ui = fixture(loc);
    let entries = ui.map_renderer.sight_entries(ui.current_location());
    assert!(
        entries
            .iter()
            .any(|e| e.glyph == '%' && e.name == "closed chest")
    );
    assert!(!entries.iter().any(|e| e.name.contains("whistle")));
    assert_eq!(
        Ui::tile_surface_description(c, ui.current_location()),
        "closed chest"
    );
    ui.engine
        .current_location_mut()
        .act(0, game::Action::Interact);
    ui.update_changes();
    let entries = ui.map_renderer.sight_entries(ui.current_location());
    assert!(
        entries
            .iter()
            .any(|e| e.glyph == '[' && e.name == "open chest: whistle")
    );
    assert_eq!(
        Ui::tile_surface_description(c, ui.current_location()),
        "open chest: whistle"
    );
    let loc = ui.engine.current_location_mut();
    while loc.player().action_cd > 0 {
        loc.skip_act(0);
    }
    loc.act(0, game::Action::Interact);
    ui.update_changes();
    assert!(
        ui.map_renderer
            .sight_entries(ui.current_location())
            .iter()
            .any(|e| e.glyph == '_' && e.name == "empty chest")
    );
    ui.engine
        .current_location_mut()
        .actors_byid
        .get_mut(&0)
        .unwrap()
        .visible
        .remove(&c);
    assert!(
        !ui.map_renderer
            .sight_entries(ui.current_location())
            .iter()
            .any(|e| e.name.contains("chest"))
    );
    assert_eq!(
        Ui::tile_surface_description(c, ui.current_location()),
        "chest"
    );
    let _ = frame(&mut ui);
}

#[test]
fn resources_share_one_row_and_preserve_change_colors() {
    let mut ui = fixture(fixtures::room());
    let player = ui
        .engine
        .current_location_mut()
        .actors_byid
        .get_mut(&0)
        .unwrap();
    player.hp = 9;
    player.mp = 15;
    player.sp = 19;
    let buffer = frame(&mut ui);
    assert_eq!(
        row(&buffer, 3),
        format!("{:30}", "HP:9/15   MP:15/21  SP:19/21")
    );
    for (x, style) in [
        (33, ui.red_color),
        (43, ui.text_color),
        (53, ui.green_color),
    ] {
        assert_eq!(buffer[(x, 3)].fg, style.fg.unwrap());
    }
    assert!(row(&buffer, 4).starts_with("R:-"));
    assert!(row(&buffer, 10).starts_with("Turn:"));
    // Even out-of-range test values cannot wrap over equipment or each other.
    ui.engine
        .current_location_mut()
        .actors_byid
        .get_mut(&0)
        .unwrap()
        .hp = i32::MIN;
    let oversized = frame(&mut ui);
    assert_eq!(oversized[(39, 3)].symbol(), "…");
    assert_eq!(oversized[(40, 3)].symbol(), "M");
    assert_eq!(row(&oversized, 4), row(&buffer, 4));
}

#[test]
fn pairs_prioritize_creatures_then_items_and_report_distinct_overflow() {
    let mut loc = fixtures::room();
    // Common surroundings must not consume a slot or inflate the overflow.
    for (x, terrain) in [
        tile::Water,
        tile::ShallowWater,
        tile::TreeTrunk,
        tile::Thicket,
    ]
    .into_iter()
    .enumerate()
    {
        loc.map[Coordinate::new(x as i32 - 2, -2)] = tile::Tile::new(terrain);
    }
    loc.spawn(Actor::new(
        Race::Rat,
        Position::new(Coordinate::new(0, 2), hex2d::XY),
    ));
    loc.items.insert(
        Coordinate::new(0, 1),
        Box::new(Item::new(Type::Knife, vec![])),
    );
    loc.map[Coordinate::new(1, 0)].feature = Some(tile::Door(true));
    loc.map[Coordinate::new(-1, 0)].feature = Some(tile::Statue);
    let mut ui = fixture(loc);
    let before = game::fixtures::outcome(ui.current_location());
    let buffer = frame(&mut ui);
    assert_eq!(
        row(&buffer, 11),
        format!("{:15}{:15}", "r - rat", ") - knife")
    );
    assert_eq!(
        row(&buffer, 12),
        format!("{:15}{:15}", "& - statue", "_ - open door")
    );
    assert_eq!(game::fixtures::outcome(ui.current_location()), before);
    ui.engine.current_location_mut().map[Coordinate::new(-2, 0)].feature = Some(tile::Stairs);
    let crowded = frame(&mut ui);
    assert_eq!(row(&crowded, 11), row(&buffer, 11));
    assert_eq!(
        row(&crowded, 12),
        format!("{:15}{:15}", "& - statue", "+2 more")
    );
    ui.log("Log remains here.");
    assert_eq!(
        row(&frame(&mut ui), 13),
        format!("{:30}", "Log remains here.")
    );
}

#[test]
fn legend_symbols_share_fully_lit_map_styles_regardless_of_world_lighting() {
    for (terrain, feature, item, race, name) in [
        (
            tile::Empty,
            Some(tile::Door(false)),
            None,
            None,
            "closed door",
        ),
        (tile::Empty, Some(tile::Door(true)), None, None, "open door"),
        (tile::Empty, Some(tile::Stairs), None, None, "stairs down"),
        (
            tile::Ground,
            Some(tile::ForestPath),
            None,
            None,
            "forest path onward",
        ),
        (
            tile::Ground,
            Some(tile::Entrance),
            None,
            None,
            "return passage",
        ),
        (tile::Empty, Some(tile::Statue), None, None, "statue"),
        (tile::ShallowWater, None, Some(Type::Knife), None, "knife"),
        (tile::Ground, None, Some(Type::Whistle), None, "whistle"),
        (
            tile::Ground,
            None,
            Some(Type::HandLantern),
            None,
            "hand lantern",
        ),
        (tile::Ground, None, None, Some(Race::Rat), "rat"),
    ] {
        let mut canonical = None;
        let mut lit_map = None;
        for light in [4, 2, 1, 0] {
            let c = Coordinate::new(0, 2);
            let mut loc = fixtures::room();
            loc.map[c] = tile::Tile::new(terrain);
            loc.map[c].feature = feature;
            if let Some(item) = item {
                loc.items.insert(c, Box::new(Item::new(item, vec![])));
                loc.actors_byid.get_mut(&0).unwrap().discovered.insert(c);
            }
            if let Some(race) = race {
                loc.spawn(Actor::new(race, Position::new(c, hex2d::XY)));
            }
            for coord in c.range_iter(1) {
                loc.light_map[coord] = light;
            }
            let mut ui = fixture(loc);
            let buffer = frame(&mut ui);
            let (dx, dy) = c.to_pixel_integer(SPACING);
            let map_cell = &buffer[((15 + dx) as u16, (12 + dy) as u16)];
            let legend_cell = &buffer[(30, 11)];
            assert_eq!(legend_cell.symbol(), map_cell.symbol());
            if light == 4 {
                assert_eq!(legend_cell, map_cell, "{name}");
                assert_ne!(legend_cell.fg, legend_cell.bg, "{name}");
                canonical = Some(legend_cell.clone());
                lit_map = Some(map_cell.clone());
            } else {
                assert_eq!(
                    Some(legend_cell),
                    canonical.as_ref(),
                    "{name}, light {light}"
                );
                assert_ne!(Some(map_cell), lit_map.as_ref(), "{name}, light {light}");
            }
            assert_eq!(
                row(&buffer, 11).trim(),
                format!("{} - {name}", map_cell.symbol())
            );
        }
    }
}

#[test]
fn common_terrain_stays_on_the_map_but_not_in_the_legend() {
    for (terrain, glyph) in [
        (tile::Empty, UNICODE_DOT),
        (tile::Ground, UNICODE_DOT),
        (tile::Wall, '#'),
        (tile::Water, '~'),
        (tile::ShallowWater, ','),
        (tile::TreeTrunk, '#'),
        (tile::Thicket, '%'),
    ] {
        let c = Coordinate::new(0, 2);
        let mut loc = fixtures::room();
        loc.map[c] = tile::Tile::new(terrain);
        let mut ui = fixture(loc);
        let buffer = frame(&mut ui);
        let (dx, dy) = c.to_pixel_integer(SPACING);
        assert_eq!(
            buffer[((15 + dx) as u16, (12 + dy) as u16)].symbol(),
            glyph.to_string(),
            "{terrain:?}"
        );
        assert!(row(&buffer, 11).trim().is_empty(), "{terrain:?}");
        assert!(row(&buffer, 12).trim().is_empty(), "{terrain:?}");
        assert!(
            ui.map_renderer
                .sight_entries(ui.current_location())
                .is_empty(),
            "{terrain:?}"
        );
    }
}

#[test]
fn current_sight_excludes_hidden_remembered_unknown_and_dead_creatures() {
    let c = Coordinate::new(0, 2);
    for kind in 0..3 {
        let mut loc = fixtures::room();
        let id = match kind {
            0 => Some(loc.spawn(Actor::new(Race::Goblin, Position::new(c, hex2d::XY)))),
            1 => {
                loc.items
                    .insert(c, Box::new(Item::new(Type::Whistle, vec![])));
                None
            }
            _ => {
                loc.map[c].feature = Some(tile::Stairs);
                None
            }
        };
        let mut ui = fixture(loc);
        assert!(!row(&frame(&mut ui), 11).trim().is_empty());
        // In LOS and remembered is insufficient, including after player death.
        ui.engine
            .current_location_mut()
            .actors_byid
            .get_mut(&0)
            .unwrap()
            .visible
            .remove(&c);
        assert!(row(&frame(&mut ui), 11).trim().is_empty());
        ui.engine
            .current_location_mut()
            .actors_byid
            .get_mut(&0)
            .unwrap()
            .hp = 0;
        assert!(row(&frame(&mut ui), 11).trim().is_empty());
        let player = ui
            .engine
            .current_location_mut()
            .actors_byid
            .get_mut(&0)
            .unwrap();
        player.hp = 15;
        player.visible.insert(c);
        player.known.remove(&c);
        // Exercise legend directly: the map asserts visible implies known.
        assert!(
            ui.map_renderer
                .sight_entries(ui.current_location())
                .is_empty()
        );
        ui.engine
            .current_location_mut()
            .actors_byid
            .get_mut(&0)
            .unwrap()
            .known
            .insert(c);
        if let Some(id) = id {
            ui.engine
                .current_location_mut()
                .actors_byid
                .get_mut(&id)
                .unwrap()
                .hp = 0;
            assert!(row(&frame(&mut ui), 11).trim().is_empty());
        }
    }
}

#[test]
fn nearest_duplicate_wins_and_long_names_clip_without_spilling() {
    let mut loc = fixtures::room();
    let near = Coordinate::new(0, 1);
    let far = Coordinate::new(0, 2);
    for c in [far, near] {
        loc.items.insert(
            c,
            Box::new(Item::new(
                Type::Plate,
                vec![Feature::Infravision, Feature::Light],
            )),
        );
    }
    loc.light_map[near] = 4;
    loc.light_map[far] = 1;
    let mut ui = fixture(loc);
    let entries = ui.map_renderer.sight_entries(ui.current_location());
    assert_eq!(entries.len(), 1);
    let buffer = frame(&mut ui);
    assert_eq!(row(&buffer, 11), "[ - plate armor of infravisio…");
    assert!(row(&buffer, 12).trim().is_empty());
    assert_eq!(
        buffer[(30, 11)].fg,
        ratatui::style::Color::Indexed(color::WALL_FG[0])
    );
    // A dimmer duplicate retains the same legend style and distinct count.
    ui.engine
        .current_location_mut()
        .actors_byid
        .get_mut(&0)
        .unwrap()
        .visible
        .remove(&near);
    let buffer = frame(&mut ui);
    assert_eq!(
        buffer[(30, 11)].fg,
        ratatui::style::Color::Indexed(color::WALL_FG[0])
    );
    assert!(row(&buffer, 12).trim().is_empty());
    ui.engine.current_location_mut().items.clear();
    assert!(row(&frame(&mut ui), 11).trim().is_empty());
}

#[test]
fn topmost_occupants_hide_lower_layers_and_legend_is_independent_of_viewport() {
    let mut loc = fixtures::room();
    let c = Coordinate::new(0, 2);
    loc.map[c].feature = Some(tile::Stairs);
    loc.items
        .insert(c, Box::new(Item::new(Type::Knife, vec![])));
    let id = loc.spawn(Actor::new(Race::Rat, Position::new(c, hex2d::XY)));
    // This explicitly visible tile lies beyond the minimum-size map viewport.
    let far = Coordinate::new(8, 0);
    loc.map[far].feature = Some(tile::Entrance);
    let mut ui = fixture(loc);
    let names = |ui: &Ui| -> Vec<String> {
        ui.map_renderer
            .sight_entries(ui.current_location())
            .into_iter()
            .map(|entry| entry.name)
            .collect()
    };
    assert_eq!(names(&ui), ["rat", "return passage"]);
    let normal = frame(&mut ui);
    for mode in [Mode::Examine, Mode::Inventory(InvMode::View)] {
        ui.mode_switch_to(mode);
        let buffer = frame(&mut ui);
        for y in 11..13 {
            assert_eq!(row(&buffer, y), row(&normal, y));
        }
    }
    ui.engine.current_location_mut().remove(id);
    assert_eq!(names(&ui), ["knife", "return passage"]);
    ui.engine.current_location_mut().items.clear();
    assert_eq!(names(&ui), ["stairs down", "return passage"]);
}
