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
fn torch_surface_readout_respects_discovery() {
    let mut loc = fixtures::room();
    let c = loc.player().head();
    loc.map[c].add_feature(tile::Torch).add_light(6);
    assert_eq!(Ui::below_description(c, loc.player(), &loc), "torch");
    loc.actors_byid.get_mut(&0).unwrap().visible.remove(&c);
    assert_eq!(Ui::below_description(c, loc.player(), &loc), "Out of sight");
    loc.actors_byid.get_mut(&0).unwrap().known.remove(&c);
    assert_eq!(Ui::below_description(c, loc.player(), &loc), "Unknown");
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
            .any(|e| e.glyph == '◼' && e.name == "closed chest")
    );
    assert!(!entries.iter().any(|e| e.name.contains("whistle")));
    assert_eq!(
        Ui::tile_surface_description(c, ui.current_location()),
        "closed chest"
    );
    let buffer = frame(&mut ui);
    assert!((0..24).any(|y| row(&buffer, y).starts_with("◼ - closed chest")));
    ui.engine
        .current_location_mut()
        .act(0, game::Action::Interact);
    ui.update_changes();
    let entries = ui.map_renderer.sight_entries(ui.current_location());
    assert!(
        entries
            .iter()
            .any(|e| e.glyph == '◻' && e.name == "open chest: whistle")
    );
    assert_eq!(
        Ui::tile_surface_description(c, ui.current_location()),
        "open chest: whistle"
    );
    let buffer = frame(&mut ui);
    assert!((0..24).any(|y| row(&buffer, y).starts_with("◻ - open chest: whistle")));
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
fn resource_ratios_distinguish_decreases_from_critical_levels() {
    let mut ui = fixture(fixtures::room());
    // Each case applies to every resource, including recovery after a low frame.
    for (current, previous, maximum, numerator, suffix) in [
        (9, 10, 15, ui.red_color, ui.text_color),
        (2, 3, 15, ui.red_color, ui.red_color),
        (2, 2, 15, ui.red_color, ui.red_color),
        (2, 2, 15, ui.red_color, ui.red_color),
        (2, 1, 15, ui.red_color, ui.red_color),
        (3, 2, 15, ui.green_color, ui.green_color),
        (3, 3, 15, ui.text_color, ui.text_color),
        (3, 4, 15, ui.red_color, ui.text_color),
        (4, 4, 21, ui.red_color, ui.red_color),
        (5, 5, 21, ui.text_color, ui.text_color),
        (0, 0, 0, ui.text_color, ui.text_color),
        (0, 1, 0, ui.red_color, ui.text_color),
        (1, 0, 0, ui.green_color, ui.green_color),
    ] {
        let player = ui
            .engine
            .current_location_mut()
            .actors_byid
            .get_mut(&0)
            .unwrap();
        player.hp = current;
        player.mp = current;
        player.sp = current;
        player.saved_hp = previous;
        player.saved_mp = previous;
        player.saved_sp = previous;
        player.stats.base.max_hp = maximum;
        player.stats.base.max_mp = maximum;
        player.stats.base.max_sp = maximum;
        let buffer = frame(&mut ui);
        for (x, label) in [(30, "HP"), (40, "MP"), (50, "SP")] {
            let ratio = format!("{current}/{maximum}");
            let text: String = (x..x + 3 + ratio.len() as u16)
                .map(|x| buffer[(x, 3)].symbol())
                .collect();
            assert_eq!(text, format!("{label}:{ratio}"));
            for offset in 0..3 {
                assert_eq!(buffer[(x + offset, 3)].fg, ui.label_color.fg.unwrap());
            }
            for offset in 0..ratio.len() {
                let style = if offset < current.to_string().len() {
                    numerator
                } else {
                    suffix
                };
                assert_eq!(
                    buffer[(x + 3 + offset as u16, 3)].fg,
                    style.fg.unwrap(),
                    "{label}:{ratio}, previous={previous}, offset={offset}"
                );
            }
        }
    }
}

#[test]
fn resource_threshold_handles_extreme_values_without_overflow() {
    let mut ui = fixture(fixtures::room());
    for (current, previous, maximum, expected) in [
        (i32::MAX, i32::MAX, i32::MAX, ui.text_color),
        (429_496_729, 429_496_729, i32::MAX, ui.red_color),
        (429_496_730, 429_496_730, i32::MAX, ui.text_color),
        (i32::MIN, i32::MIN, i32::MAX, ui.red_color),
        (i32::MIN, i32::MIN, 0, ui.text_color),
        (-123_456, 0, 0, ui.red_color),
    ] {
        let player = ui
            .engine
            .current_location_mut()
            .actors_byid
            .get_mut(&0)
            .unwrap();
        player.hp = current;
        player.saved_hp = previous;
        player.stats.base.max_hp = maximum;
        let buffer = frame(&mut ui);
        assert_eq!(buffer[(33, 3)].fg, expected.fg.unwrap());
        assert_eq!(buffer[(39, 3)].symbol(), "…");
        assert_eq!(buffer[(40, 3)].symbol(), "M");
    }
}

#[test]
fn pairs_prioritize_creatures_then_items_without_premature_overflow() {
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
    assert_eq!(row(&crowded, 12), row(&buffer, 12));
    assert_eq!(row(&crowded, 13).trim(), "> - stairs down");
    ui.log("Log remains here.");
    assert_eq!(
        row(&frame(&mut ui), 16),
        format!("{:30}", "Log remains here.")
    );
}

#[test]
fn five_sight_rows_keep_long_entries_overflow_and_log_separate() {
    for (width, height) in [(60, 24), (120, 36)] {
        let mut loc = fixtures::room();
        loc.spawn(Actor::new(
            Race::CaveBeetle,
            Position::new(Coordinate::new(0, 1), hex2d::XY),
        ));
        for (x, item) in [Type::Plate, Type::Knife, Type::Whistle, Type::HandLantern]
            .into_iter()
            .enumerate()
        {
            loc.items.insert(
                Coordinate::new(x as i32 - 2, 2),
                Box::new(Item::new(item, vec![Feature::Light])),
            );
        }
        let mut ui = fixture(loc);
        ui.resize((height, width));
        ui.log("Log stays below sight.");
        let render = |ui: &mut Ui| {
            let mut buffer = Buffer::empty(Rect::new(0, 0, width as u16, height as u16));
            ui.render_into(&mut buffer);
            buffer
        };
        let text = |buffer: &Buffer, y| -> String {
            ((width - 30) as u16..width as u16)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        };
        let full = render(&mut ui);
        assert_eq!(ui.windows.map.size(), (height, width - 30));
        assert_eq!(text(&full, 11), "b - cave beetle");
        for y in 12..16 {
            assert!(text(&full, y).contains(" of light"));
        }
        assert_eq!(text(&full, 16), "Log stays below sight.");
        ui.engine.current_location_mut().map[Coordinate::new(0, 3)].feature = Some(tile::Stairs);
        let crowded = render(&mut ui);
        for y in 0..15 {
            assert_eq!(text(&crowded, y), text(&full, y));
        }
        assert_eq!(text(&crowded, 15), "+2 more");
        assert_eq!(text(&crowded, 16), text(&full, 16));
        ui.handle_event(TerminalEvent::Resize(60, 23), Instant::now());
        let mut short = Buffer::empty(Rect::new(0, 0, 60, 23));
        ui.render_into(&mut short);
        let short_text: String = short.content.iter().map(|cell| cell.symbol()).collect();
        assert!(!short_text.contains("cave beetle"));
        ui.handle_event(
            TerminalEvent::Resize(width as u16, height as u16),
            Instant::now(),
        );
        assert_eq!(render(&mut ui), crowded);
        ui.engine.current_location_mut().items.clear();
        ui.engine.current_location_mut().map[Coordinate::new(0, 3)].feature = None;
        let cleared = render(&mut ui);
        for y in 12..16 {
            assert!(text(&cleared, y).is_empty());
        }
        assert_eq!(text(&cleared, 16), text(&full, 16));
    }
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
        for light in [5, 4, 3, 2, 1, 0] {
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
            if light == 5 {
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
                if light == 0 {
                    assert_ne!(Some(map_cell), lit_map.as_ref(), "{name}, light {light}");
                }
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
        (tile::Thicket, 'γ'),
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
        ratatui::style::Color::Indexed(color::WALL_FG.index(true, 5))
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
        ratatui::style::Color::Indexed(color::WALL_FG.index(true, 5))
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
