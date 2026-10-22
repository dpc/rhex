use super::*;
use crate::game::item::{Item, Type};
use crate::terminal::fixtures;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

fn fixture(now: Instant) -> Ui {
    let mut ui = Ui::new(game::Engine::fixture(fixtures::room()), (24, 60), now);
    ui.spawned = true;
    ui.mode_switch_to(Mode::Normal);
    ui
}

fn key(ui: &mut Ui, code: KeyCode, now: Instant) {
    ui.handle_event(
        TerminalEvent::Key(KeyEvent::new(code, KeyModifiers::NONE)),
        now,
    );
}

fn chars(ui: &mut Ui, keys: &str, now: Instant) {
    for ch in keys.chars() {
        key(ui, KeyCode::Char(ch), now);
    }
}

fn player_mut(ui: &mut Ui) -> &mut Actor {
    let loc = ui.engine.current_location_mut();
    loc.actors_byid.get_mut(&loc.player_id()).unwrap()
}

fn give_bow(ui: &mut Ui) {
    player_mut(ui)
        .items_equipped
        .insert(Slot::RHand, ('b', Box::new(Item::new(Type::Bow, vec![]))));
}

fn frame(ui: &mut Ui, width: u16, height: u16) -> Buffer {
    let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
    ui.render_into(&mut buffer);
    buffer
}

#[test]
fn comma_production_dispatch_picks_then_toggles_without_moving() {
    for mode in [InputMode::Normal, InputMode::Vi] {
        let now = Instant::now();
        let mut ui = fixture(now);
        ui.input_mode = mode;
        let pos = ui.player().pos;
        let target = ui.player().head();
        ui.engine.current_location_mut().map[target].feature = Some(tile::Door(true));
        ui.engine
            .current_location_mut()
            .items
            .insert(target, Box::new(Item::new(Type::Cloak, vec![])));
        for (turn, description, door) in [
            (1, "open door", true),
            (2, "closed door", false),
            (3, "open door", true),
        ] {
            let time = now + Duration::from_secs(turn);
            chars(&mut ui, ",", time);
            assert_eq!(ui.game_action_queue.len(), 1);
            assert_eq!(ui.game_action_queue[0], game::Action::Interact);
            ui.advance((24, 60), time);
            assert!(ui.game_action_queue.is_empty());
            assert_eq!(ui.engine.turn(), turn);
            assert_eq!(ui.player().pos, pos);
            assert_eq!(ui.player().items_backpack.len(), 1);
            assert!(!ui.current_location().items.contains_key(&target));
            assert_eq!(
                ui.current_location().map[target].feature,
                Some(tile::Door(door))
            );
            let before = game::fixtures::outcome(ui.current_location());
            let buffer = frame(&mut ui, 60, 24);
            let front: String = (30..60).map(|x| buffer[(x, 8)].symbol()).collect();
            assert_eq!(front, format!("{:30}", format!("In front: {description}")));
            assert_eq!(before, game::fixtures::outcome(ui.current_location()));
        }
    }
}

#[test]
fn comma_does_not_escape_modal_input_or_select_inventory_items() {
    let now = Instant::now();
    for input_mode in [InputMode::Normal, InputMode::Vi] {
        for mode in [
            Mode::Inventory(InvMode::View),
            Mode::Inventory(InvMode::Equip),
            Mode::Inventory(InvMode::Drop_),
            Mode::Examine,
            Mode::Target(TargetMode::Ranged),
            Mode::GoTo,
            Mode::FullScreen(FSMode::Help),
            Mode::FullScreen(FSMode::Quit),
            Mode::FullScreen(FSMode::PickRace),
        ] {
            let mut ui = fixture(now);
            ui.input_mode = input_mode;
            give_bow(&mut ui);
            let target = ui.player().head();
            ui.engine.current_location_mut().map[target].feature = Some(tile::Door(false));
            ui.mode_switch_to(mode);
            // Start the scheduler's waiting turn without supplying an action.
            ui.advance((24, 60), now);
            let turn = ui.engine.turn();
            let before = game::fixtures::outcome(ui.current_location());
            chars(&mut ui, ",", now);
            assert!(ui.game_action_queue.is_empty(), "{input_mode:?} {mode:?}");
            ui.advance((24, 60), now);
            let _ = frame(&mut ui, 60, 24);
            assert_eq!(ui.engine.turn(), turn, "{input_mode:?} {mode:?}");
            assert_eq!(before, game::fixtures::outcome(ui.current_location()));
        }
    }
}

/// Build an unaware, visible enemy that cannot act during the tested turn.
fn awareness_fixture(now: Instant, sight: bool) -> (Ui, actor::Id) {
    let mut loc = game::fixtures::room();
    let id = loc.spawn(Actor::new(
        Race::Human,
        Position::new(Coordinate::new(1, 0), hex2d::XY),
    ));
    let enemy = loc.actors_byid.get_mut(&id).unwrap();
    assert!(!enemy.aware_of_player);
    enemy.action_cd = 100;
    if sight {
        enemy.pos.dir = hex2d::XY + Angle::Back;
    }
    let mut ui = Ui::new(game::Engine::fixture(loc), (24, 60), now);
    ui.spawned = true;
    ui.mode_switch_to(Mode::Normal);
    (ui, id)
}

#[test]
fn awareness_production_sight_and_sound_drain_once_independently_of_rendering() {
    fn run(draws: usize, sight: bool) -> String {
        let now = Instant::now();
        let (mut ui, id) = awareness_fixture(now, sight);
        ui.queue_action(if sight {
            game::Action::Wait
        } else {
            game::Action::Turn(Angle::Left)
        });
        ui.advance((24, 60), now);
        assert!(ui.current_location().actors_byid[&id].aware_of_player);
        assert!(ui.current_location().awareness_events.is_empty());
        assert_eq!(
            ui.map_renderer
                .animations
                .aware_actors(Duration::ZERO)
                .collect::<Vec<_>>(),
            [id]
        );
        let state = game::fixtures::outcome(ui.current_location());
        for _ in 0..draws {
            frame(&mut ui, 60, 24);
        }
        assert_eq!(state, game::fixtures::outcome(ui.current_location()));
        // Draining again later must not restart a consumed transition.
        ui.now = now + Duration::from_millis(200);
        ui.engine_change(id);
        assert!(
            ui.map_renderer
                .animations
                .aware_actors(Duration::from_millis(500))
                .next()
                .is_none()
        );
        ui.mode_switch_to(Mode::FullScreen(FSMode::Help));
        ui.advance((10, 20), now + Duration::from_millis(500));
        assert!(
            ui.map_renderer
                .animations
                .aware_actors(Duration::ZERO)
                .next()
                .is_none()
        );
        assert_eq!(state, game::fixtures::outcome(ui.current_location()));
        state
    }
    for sight in [false, true] {
        assert_eq!(run(0, sight), run(1, sight));
        assert_eq!(run(0, sight), run(7, sight));
    }
}

#[test]
fn awareness_descent_and_session_reset_discard_old_actor_id_markers() {
    for descend in [false, true] {
        let now = Instant::now();
        let (mut ui, id) = awareness_fixture(now, true);
        ui.queue_action(game::Action::Wait);
        ui.advance((24, 60), now);
        assert_eq!(
            ui.map_renderer
                .animations
                .aware_actors(Duration::ZERO)
                .collect::<Vec<_>>(),
            [id]
        );
        if descend {
            let origin = ui.player().coord();
            ui.engine.current_location_mut().map[origin].feature = Some(tile::Stairs);
            ui.queue_action(game::Action::Descend);
            ui.advance((24, 60), now + Duration::from_millis(100));
            assert_eq!(ui.current_location().level, 1);
            assert!(ui.current_location().awareness_events.is_empty());
        } else {
            let mut loc = fixtures::room();
            loc.actors_byid.clear();
            loc.actors_coord_to_id.clear();
            ui.engine = game::Engine::fixture(loc);
            ui.initial_spawn(Race::Human);
        }
        assert!(
            ui.map_renderer
                .animations
                .aware_actors(Duration::from_millis(100))
                .next()
                .is_none()
        );
    }
}

#[test]
fn real_same_turn_sounds_are_concurrent_and_expire_behind_menus_and_tiny_windows() {
    let now = Instant::now();
    let mut loc = fixtures::room();
    let origin = loc.player().coord();
    let sources = [Coordinate::new(1, 0), Coordinate::new(0, 1)];
    for source in sources {
        let id = loc.spawn(Actor::new(Race::Rat, Position::new(source, hex2d::XY)));
        loc.actors_byid.get_mut(&id).unwrap().visible.insert(origin);
        loc.actors_byid.get_mut(&0).unwrap().visible.remove(&source);
    }
    let mut ui = Ui::new(game::Engine::fixture(loc), (24, 60), now);
    ui.spawned = true;
    ui.mode_switch_to(Mode::Normal);
    ui.queue_action(game::Action::Wait);
    ui.advance((24, 60), now);
    for source in sources {
        assert!(ui.player().heard.contains_key(&source));
        assert!(
            ui.map_renderer
                .animations
                .sound_cells(Duration::ZERO)
                .any(|c| c == (source, true))
        );
    }
    assert!(ui.current_location().sound_events.is_empty());
    assert!(ui.pending_sounds.is_empty());
    assert_eq!(
        ui.log
            .borrow()
            .iter()
            .filter(|entry| entry.text.contains("You hear"))
            .count(),
        2
    );
    let state = game::fixtures::outcome(ui.current_location());
    game::sound::WORK.set((0, 0, 0));
    for _ in 0..7 {
        frame(&mut ui, 60, 24);
    }
    assert_eq!(game::sound::WORK.get(), (0, 0, 0));
    assert_eq!(state, game::fixtures::outcome(ui.current_location()));
    ui.mode_switch_to(Mode::FullScreen(FSMode::Help));
    ui.advance((10, 20), now + Duration::from_millis(500));
    assert!(
        ui.map_renderer
            .animations
            .sound_cells(Duration::ZERO)
            .next()
            .is_none()
    );
    assert!(
        ui.map_renderer
            .animations
            .sound_cells(Duration::from_millis(400))
            .next()
            .is_none()
    );
    assert_eq!(state, game::fixtures::outcome(ui.current_location()));
}

#[test]
fn actual_propagated_hearing_interrupts_automove_but_inaudible_noise_does_not() {
    let now = Instant::now();
    for blocked in [false, true] {
        let mut ui = fixture(now);
        let source = Coordinate::new(2, 0);
        let loc = ui.engine.current_location_mut();
        let id = loc.spawn(Actor::new(Race::Rat, Position::new(source, hex2d::XY)));
        if blocked {
            loc.map[Coordinate::new(0, 1)].feature = Some(tile::Statue);
        }
        let player = loc.actors_byid.get_mut(&0).unwrap();
        player.visible.clear();
        player.discovered.clear();
        player.discovered_areas.clear();
        ui.automoving = Some(AutoMoveType::Walk);
        ui.automoving_stopped_turn = u64::MAX;
        ui.engine
            .current_location_mut()
            .act(id, game::Action::Turn(Angle::Right));
        ui.engine_change(id);
        assert_eq!(ui.is_automoving(), blocked);
        assert_eq!(ui.player().heard.contains_key(&source), !blocked);
    }
}

#[test]
fn sound_playback_frequency_does_not_change_gameplay_or_rng() {
    fn run(draws: usize) -> String {
        let now = Instant::now();
        let mut ui = fixture(now);
        for (step, action) in [
            game::Action::Turn(Angle::Right),
            game::Action::Move(Angle::Forward),
            game::Action::Wait,
        ]
        .into_iter()
        .enumerate()
        {
            let time = now + Duration::from_millis(step as u64 * 1000);
            ui.queue_action(action);
            ui.advance((24, 60), time);
            let before = game::sound::WORK.get();
            for _ in 0..draws {
                frame(&mut ui, 60, 24);
            }
            assert_eq!(before, game::sound::WORK.get());
        }
        game::fixtures::outcome(ui.current_location())
    }
    assert_eq!(run(0), run(1));
    assert_eq!(run(0), run(7));
}

#[test]
fn descent_discards_staged_and_playing_sounds_before_first_new_level_action() {
    let now = Instant::now();
    let mut ui = fixture(now);
    let origin = ui.player().coord();
    ui.queue_action(game::Action::Turn(Angle::Right));
    ui.advance((24, 60), now);
    // Source exclusion can leave the early phases empty; check the whole wave.
    assert!((0..5).any(|tick| {
        ui.map_renderer
            .animations
            .sound_cells(Duration::from_millis(tick * 100))
            .next()
            .is_some()
    }));
    ui.engine.current_location_mut().map[origin].feature = Some(tile::Stairs);
    ui.queue_action(game::Action::Descend);
    ui.advance((24, 60), now + Duration::from_millis(100));
    assert_eq!(ui.current_location().level, 1);
    assert!(ui.pending_sounds.is_empty());
    assert!(ui.current_location().sound_events.is_empty());
    assert!(ui.player().heard.is_empty());
    assert!(ui.player().pending_noises.is_empty());
    assert!((0..5).all(|tick| {
        ui.map_renderer
            .animations
            .sound_cells(Duration::from_millis(tick * 100))
            .next()
            .is_none()
    }));
    ui.queue_action(game::Action::Turn(Angle::Right));
    ui.advance((24, 60), now + Duration::from_millis(200));
    assert!((2..7).any(|tick| {
        ui.map_renderer
            .animations
            .sound_cells(Duration::from_millis(tick * 100))
            .next()
            .is_some()
    }));
}

#[test]
#[ignore = "manual generated-turn/render measurement, never a timing gate"]
fn generated_sound_workload_measurement() {
    let now = Instant::now();
    let mut ui = Ui::new(game::Engine::seeded(42), (40, 120), now);
    ui.initial_spawn(Race::Human);
    ui.mode_switch_to(Mode::Normal);
    player_mut(&mut ui).hp = 100_000;
    game::sound::WORK.set((0, 0, 0));
    let started = Instant::now();
    for turn in 0..100 {
        ui.queue_action(game::Action::Turn(Angle::Right));
        ui.advance((40, 120), now + Duration::from_millis(turn * 1000));
    }
    eprintln!(
        "generated seed42 100 turns / {} actors: elapsed={:?}, (calls, settled, edges)={:?}",
        ui.current_location().actors_ids().len(),
        started.elapsed(),
        game::sound::WORK.get()
    );
    let before = game::sound::WORK.get();
    let started = Instant::now();
    for _ in 0..1000 {
        std::hint::black_box(frame(&mut ui, 120, 40));
    }
    eprintln!(
        "1000 composed 120x40 frames with active sound: elapsed={:?}",
        started.elapsed()
    );
    assert_eq!(before, game::sound::WORK.get());
}

#[test]
fn production_turn_consumes_all_hits_once_and_idle_time_finishes_them() {
    let now = Instant::now();
    let mut loc = game::fixtures::room();
    let origin = loc.player().coord();
    loc.actors_byid.get_mut(&0).unwrap().hp = 1000;
    loc.actors_byid.get_mut(&0).unwrap().stats.base.ev = 0;
    for dir in [hex2d::XY, hex2d::XZ] {
        let id = loc.spawn(Actor::new(Race::Troll, Position::new(origin + dir, -dir)));
        loc.actors_byid.get_mut(&id).unwrap().stats.melee_acc = 100_000;
    }
    let mut ui = Ui::new(game::Engine::fixture(loc), (24, 60), now);
    ui.spawned = true;
    ui.mode_switch_to(Mode::Normal);
    ui.queue_action(game::Action::Wait);
    ui.advance((24, 60), now);
    assert!(ui.current_location().combat_events.is_empty());
    let hits: usize = ui
        .current_location()
        .actors_byid
        .values()
        .flat_map(|a| &a.did_attack)
        .filter(|r| r.success)
        .count();
    assert_eq!(hits, 2);
    assert_eq!(
        ui.map_renderer
            .animations
            .glyphs(Duration::ZERO)
            .collect::<Vec<_>>(),
        [(origin, '-')]
    );
    let after_turn = game::fixtures::outcome(ui.current_location());
    frame(&mut ui, 60, 24);
    ui.frame_drawn(now);
    assert!(!ui.frame_due(now + Duration::from_millis(99)));
    assert!(ui.frame_due(now + Duration::from_millis(100)));
    for (ms, glyph) in [
        (199, '-'),
        (200, '*'),
        (399, '*'),
        (400, '/'),
        (600, '*'),
        (799, '*'),
    ] {
        ui.advance((24, 60), now + Duration::from_millis(ms));
        frame(&mut ui, 60, 24);
        ui.redraw();
        frame(&mut ui, 60, 24);
        assert_eq!(
            ui.map_renderer
                .animations
                .glyphs(Duration::from_millis(ms))
                .collect::<Vec<_>>(),
            [(origin, glyph)],
            "elapsed={ms}, turn={}, hp={}",
            ui.engine.turn(),
            ui.player().hp
        );
        assert_eq!(game::fixtures::outcome(ui.current_location()), after_turn);
    }
    ui.mode_switch_to(Mode::FullScreen(FSMode::Help));
    ui.advance((24, 60), now + Duration::from_millis(800));
    assert!(
        ui.map_renderer
            .animations
            .glyphs(Duration::from_millis(800))
            .next()
            .is_none()
    );
    assert!(ui.current_location().combat_events.is_empty());

    // A new location invalidates pending coordinates before its first draw.
    ui.map_renderer.animations.enqueue(
        vec![game::combat_event::CombatEvent {
            source: origin + hex2d::XY,
            impact: origin,
        }],
        Duration::from_millis(800),
    );
    ui.engine.current_location_mut().map[origin].feature = Some(tile::Stairs);
    ui.mode_switch_to(Mode::Normal);
    ui.queue_action(game::Action::Descend);
    ui.advance((24, 60), now + Duration::from_millis(800));
    assert_eq!(ui.current_location().level, 1);
    assert!(
        ui.map_renderer
            .animations
            .glyphs(Duration::from_millis(800))
            .next()
            .is_none()
    );
    let fresh = fixture(now);
    assert!(
        fresh
            .map_renderer
            .animations
            .glyphs(Duration::ZERO)
            .next()
            .is_none()
    );
}

#[test]
fn below_describes_ground_items_features_and_terrain_not_the_player() {
    let now = Instant::now();
    let origin = Coordinate::new(0, 0);
    for (terrain, feature, item, area, expected) in [
        (tile::Empty, None, false, false, "nothing"),
        (tile::Empty, None, false, true, "room"),
        (tile::Water, None, false, false, "deep water"),
        (tile::Ground, None, false, false, "forest ground"),
        (tile::ShallowWater, None, false, false, "shallow water"),
        (tile::TreeTrunk, None, false, false, "tree trunk"),
        (
            tile::Empty,
            Some(tile::Stairs),
            false,
            false,
            "to Dark Forest (>)",
        ),
        (
            tile::Empty,
            Some(tile::Door(true)),
            false,
            false,
            "open door",
        ),
        (tile::Empty, None, true, false, "knife"),
        // Match In front's item-before-feature convention.
        (tile::Empty, Some(tile::Stairs), true, false, "knife"),
    ] {
        let mut ui = fixture(now);
        let loc = ui.engine.current_location_mut();
        loc.map[origin] = tile::Tile::new(terrain);
        loc.map[origin].feature = feature;
        if area {
            loc.map[origin].area = Some(area::Area::new(origin, area::Room(3)));
        }
        if item {
            loc.items
                .insert(origin, Box::new(Item::new(Type::Knife, vec![])));
        }
        for mode in [Mode::Normal, Mode::Examine, Mode::Inventory(InvMode::View)] {
            ui.mode_switch_to(mode);
            ui.target_pos = Some(Position::new(Coordinate::new(2, -2), hex2d::XY));
            let buffer = frame(&mut ui, 60, 24);
            let row: String = (30..60).map(|x| buffer[(x, 9)].symbol()).collect();
            assert_eq!(row, format!("{:30}", format!("Below: {expected}")));
            let front: String = (30..60).map(|x| buffer[(x, 8)].symbol()).collect();
            assert_eq!(front, format!("{:30}", "In front: nothing"));
        }
    }
}

#[test]
fn below_updates_after_moving_onto_and_off_stairs() {
    let now = Instant::now();
    let mut ui = fixture(now);
    let stairs = ui.player().head();
    ui.engine.current_location_mut().map[stairs].feature = Some(tile::Stairs);
    let row =
        |buffer: &Buffer, y| -> String { (30..60).map(|x| buffer[(x, y)].symbol()).collect() };
    let before = frame(&mut ui, 60, 24);
    assert_eq!(
        row(&before, 8),
        format!("{:30}", "In front: to Dark Forest (>)")
    );
    assert_eq!(row(&before, 9), format!("{:30}", "Below: nothing"));
    for (step, expected) in [(1, "to Dark Forest (>)"), (2, "nothing")] {
        key(&mut ui, KeyCode::Char('k'), now);
        ui.advance((24, 60), now + Duration::from_secs(step));
        let buffer = frame(&mut ui, 60, 24);
        assert_eq!(
            row(&buffer, 9),
            format!("{:30}", format!("Below: {expected}"))
        );
    }
    assert_ne!(ui.player().pos.coord, stairs);
}

#[test]
fn long_ground_readouts_clip_without_overwriting_stats_or_log() {
    use crate::game::item::Feature;
    let now = Instant::now();
    for (width, height) in [(60, 24), (120, 36)] {
        let mut ui = fixture(now);
        ui.resize((height, width));
        for coord in [ui.player().pos.coord, ui.player().head()] {
            ui.engine.current_location_mut().items.insert(
                coord,
                Box::new(Item::new(
                    Type::Plate,
                    vec![Feature::Infravision, Feature::Light],
                )),
            );
        }
        ui.log("A quiet room.");
        let buffer = frame(&mut ui, width as u16, height as u16);
        let row = |y| -> String {
            ((width - 30) as u16..width as u16)
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        };
        assert_eq!(row(8), "In front: plate armor of infra");
        assert_eq!(row(9), "Below: plate armor of infravis");
        assert_eq!(row(10), format!("{:30}", "Turn: 0       Level: 0"));
        assert_eq!(row(13), format!("{:30}", "A quiet room."));
        ui.engine.current_location_mut().items.clear();
        let cleared = frame(&mut ui, width as u16, height as u16);
        let row: String = ((width - 30) as u16..width as u16)
            .map(|x| cleared[(x, 9)].symbol())
            .collect();
        assert_eq!(row, format!("{:30}", "Below: nothing"));
    }
}

#[test]
fn idle_shared_facing_uses_stable_time_across_redraws_modes_and_resize() {
    let now = Instant::now();
    let mut ui = fixture(now);
    let enemy_coord = Coordinate::new(2, -2);
    ui.engine.current_location_mut().spawn(Actor::new(
        actor::Race::Rat,
        Position::new(enemy_coord, hex2d::YX),
    ));
    ui.advance((24, 60), now);
    let turn = ui.engine.turn();
    // Follow the idle runner path without events or explicit redraw requests.
    for tick in 0..=20 {
        let instant = now + Duration::from_millis(tick * 100);
        ui.advance((24, 60), instant);
        assert!(ui.frame_due(instant));
        let buffer = frame(&mut ui, 60, 24);
        let body = buffer
            .content
            .iter()
            .position(|cell| cell.symbol() == "@")
            .unwrap();
        assert_eq!(
            buffer.content[body + 2].fg,
            ratatui::style::Color::Indexed(if tick / 10 % 2 == 0 { 196 } else { 21 })
        );
        ui.frame_drawn(instant);
        assert_eq!(ui.engine.turn(), turn);
    }
    for (ms, expected) in [(2999, 196), (3000, 21), (3999, 21), (4000, 196)] {
        let instant = now + Duration::from_millis(ms);
        ui.advance((24, 60), instant);
        assert!(ui.frame_due(instant));
        let buffer = frame(&mut ui, 60, 24);
        let body = buffer
            .content
            .iter()
            .position(|cell| cell.symbol() == "@")
            .unwrap();
        assert_eq!(
            buffer.content[body + 2].fg,
            ratatui::style::Color::Indexed(expected)
        );
        ui.frame_drawn(instant);
        // Reset the animation frame repeatedly, without resetting presentation time.
        ui.redraw();
        ui.mode_switch_to(Mode::Examine);
        frame(&mut ui, 60, 24);
        ui.mode_switch_to(Mode::Normal);
        ui.resize((23, 59));
        ui.resize((24, 60));
        assert!(ui.frame_due(instant));
        assert_eq!(frame(&mut ui, 60, 24), buffer);
        assert_eq!(ui.engine.turn(), turn);
    }
}

#[test]
fn stochastic_world_trace_is_independent_of_zero_one_or_many_renders() {
    use crate::game::fixtures::{occupancy, outcome};
    use rand::{SeedableRng, rngs::SmallRng};

    fn run(seed: u64, renders: u32, generated: bool) -> (Vec<String>, usize, i32) {
        let now = Instant::now();
        let engine = if generated {
            let mut engine = game::Engine::seeded(seed);
            engine.initial_spawn(actor::Race::Human);
            engine
        } else {
            let mut loc = game::fixtures::room();
            loc.rng = SmallRng::seed_from_u64(seed);
            let player = loc.actors_byid.get_mut(&0).unwrap();
            player.hp = 500;
            player.base_stats.max_hp = 500;
            player.base_stats.regeneration = 10;
            player.sp = 1;
            let head = player.head();
            let mut enemy = Actor::new(actor::Race::Troll, Position::new(head, hex2d::YX));
            enemy.hp = 500;
            enemy.base_stats.max_hp = 500;
            loc.spawn(enemy);
            game::Engine::fixture(loc)
        };
        let mut ui = Ui::new(engine, (24, 60), now);
        ui.spawned = true;
        ui.mode_switch_to(Mode::Normal);
        let mut trace = vec![outcome(ui.current_location())];
        let mut attacks = 0;
        let mut damage = 0;
        for turn in 0..20 {
            ui.engine.start_turn();
            if ui.engine.needs_player_input() {
                ui.engine.player_act(if turn % 3 == 0 {
                    game::Action::Wait
                } else {
                    game::Action::Move(Angle::Forward)
                });
            } else {
                ui.engine.player_skip_act();
            }
            ui.engine_change(ui.current_location().player_id());
            attacks += ui.player().did_attack.len();
            damage += ui
                .player()
                .did_attack
                .iter()
                .filter(|r| r.success)
                .map(|r| r.dmg)
                .sum::<i32>();
            loop {
                let before = outcome(ui.current_location());
                // Include base-layer rebuilds and animation-only effect frames.
                for index in 0..renders {
                    ui.anim_frame_count = index;
                    frame(&mut ui, 60, 24);
                }
                assert_eq!(outcome(ui.current_location()), before);
                if ui.engine.is_turn_done() {
                    break;
                }
                let actor_id = ui.engine.one_actor_tick();
                ui.engine_change(actor_id);
            }
            occupancy(ui.current_location());
            trace.push(outcome(ui.current_location()));
        }
        (trace, attacks, damage)
    }

    let mut damages = std::collections::BTreeSet::new();
    for seed in [0, 1, 7, 42] {
        for generated in [false, true] {
            let baseline = run(seed, 0, generated);
            assert_eq!(
                run(seed, 1, generated),
                baseline,
                "seed {seed}, generated={generated}"
            );
            assert_eq!(
                run(seed, 7, generated),
                baseline,
                "seed {seed}, generated={generated}"
            );
            if !generated {
                assert!(baseline.1 > 5, "must exercise repeated stochastic combat");
                assert!(baseline.2 > 0, "must exercise successful damage");
                damages.insert(baseline.2);
            }
        }
    }
    assert!(
        damages.len() > 1,
        "seeds must affect outcomes, not just rendering"
    );
}

#[test]
fn automove_stairs_occupancy_and_perception_interruptions() {
    let now = Instant::now();
    let mut ui = fixture(now);
    let head = ui.player().head();
    let goto = AutoMoveType::GoTo(GoToType::Stairs);
    assert!(matches!(ui.automove_action(goto), AutoMoveAction::Blocked));
    ui.engine.current_location_mut().map[head].feature = Some(tile::Stairs);
    assert!(matches!(
        ui.automove_action(goto),
        AutoMoveAction::Action(game::Action::Move(Angle::Forward))
    ));
    let id = ui
        .engine
        .current_location_mut()
        .spawn(Actor::new(actor::Race::Rat, Position::new(head, hex2d::XY)));
    assert!(matches!(ui.automove_action(goto), AutoMoveAction::Blocked));
    assert!(ui.should_stop_automoving());
    ui.engine.current_location_mut().remove(id);
    player_mut(&mut ui).discovered.clear();
    player_mut(&mut ui).discovered_areas.clear();
    player_mut(&mut ui).heard.clear();
    assert!(!ui.should_stop_automoving());
    player_mut(&mut ui).visible.remove(&head);
    player_mut(&mut ui).noise_hears(head, game::Noise::Creature(actor::Race::Rat));
    assert!(ui.should_stop_automoving());
    player_mut(&mut ui).heard.clear();
    player_mut(&mut ui).discovered.insert(head);
    assert!(ui.should_stop_automoving(), "new stairs interrupt travel");
    player_mut(&mut ui).discovered.clear();
    ui.engine.current_location_mut().map[head].feature = None;
    ui.engine
        .current_location_mut()
        .items
        .insert(head, Box::new(Item::new(Type::Knife, vec![])));
    player_mut(&mut ui).discovered.insert(head);
    assert!(ui.should_stop_automoving(), "new items interrupt travel");
    player_mut(&mut ui).discovered.clear();
    ui.engine.current_location_mut().map[head] = tile::Tile::new(tile::Wall);
    assert!(matches!(
        ui.automove_action(AutoMoveType::Walk),
        AutoMoveAction::Finish
    ));
}

#[test]
fn help_lists_only_supported_movement_keys_in_both_modes() {
    for input_mode in [InputMode::Normal, InputMode::Vi] {
        let now = Instant::now();
        let mut ui = fixture(now);
        ui.input_mode = input_mode;
        chars(&mut ui, "?", now);
        let buffer = frame(&mut ui, 120, 36);
        let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("Turn left/right: h/l or Left/Right arrows"));
        assert!(text.contains("Move forward/back: k/j or Up/Down arrows"));
        assert!(text.contains("Strafe/attack front-left/right: Shift + h/l"));
        assert_eq!(
            text.contains("Automove: shift + k"),
            input_mode == InputMode::Vi
        );
        for retired in ["awsdqe", "hjklui", "shift + w"] {
            assert!(!text.contains(retired));
        }
    }
}

#[test]
fn retired_movement_keys_do_not_act_or_move_target_cursors() {
    for input_mode in [InputMode::Normal, InputMode::Vi] {
        for mode in [
            Mode::Normal,
            Mode::Examine,
            Mode::Target(TargetMode::Ranged),
        ] {
            let now = Instant::now();
            let mut ui = fixture(now);
            ui.input_mode = input_mode;
            ui.advance((24, 60), now);
            ui.mode_switch_to(mode);
            ui.target_pos = Some(ui.player().pos);
            let target = ui.target_pos;
            let before = game::fixtures::outcome(ui.current_location());
            let turn = ui.engine.turn();
            // q remains a non-movement quit/cancel command in Vi and cursor modes.
            let keys = if mode == Mode::Normal && input_mode == InputMode::Normal {
                "asdwqeuiW"
            } else {
                "asdweuiW"
            };
            for ch in keys.chars() {
                key(&mut ui, KeyCode::Char(ch), now);
                ui.advance((24, 60), now);
                assert_eq!(ui.mode, mode, "{input_mode:?} {mode:?} {ch}");
                assert_eq!(ui.target_pos, target);
                assert!(ui.game_action_queue.is_empty());
                assert!(!ui.is_automoving());
                assert_eq!(ui.engine.turn(), turn);
                assert_eq!(game::fixtures::outcome(ui.current_location()), before);
            }
        }
    }
}

#[test]
fn retired_movement_letters_still_select_inventory_items() {
    for input_mode in [InputMode::Normal, InputMode::Vi] {
        for letter in "asdwqeuiW".chars() {
            let now = Instant::now();
            let mut ui = fixture(now);
            ui.input_mode = input_mode;
            player_mut(&mut ui)
                .items_backpack
                .insert(letter, Box::new(Item::new(Type::Knife, vec![])));
            ui.advance((24, 60), now);
            let turn = ui.engine.turn();
            chars(&mut ui, &format!("I{letter}"), now);
            assert_eq!(ui.mode, Mode::Inventory(InvMode::Actions(letter)));
            assert!(ui.game_action_queue.is_empty());
            assert_eq!(ui.engine.turn(), turn);
        }
    }
}

#[test]
fn unsupported_events_cannot_act_select_or_cancel() {
    let now = Instant::now();
    let mut ui = fixture(now);
    // Starting a turn itself increments Turn; first reach the input boundary.
    ui.advance((24, 60), now);
    assert!(ui.engine.needs_player_input());
    let turn = ui.engine.turn();
    let pose = ui.player().pos;
    let hp = ui.player().hp;
    let events = [
        TerminalEvent::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)),
        TerminalEvent::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::ALT)),
        TerminalEvent::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL)),
        TerminalEvent::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT)),
        TerminalEvent::Key(KeyEvent::new(KeyCode::Char('š'), KeyModifiers::NONE)),
        TerminalEvent::Key(KeyEvent::new(KeyCode::Char('Ł'), KeyModifiers::NONE)),
        TerminalEvent::Key(KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE)),
        TerminalEvent::Key(KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        )),
        TerminalEvent::FocusGained,
        TerminalEvent::Paste("a".into()),
    ];
    player_mut(&mut ui)
        .items_backpack
        .insert('a', Box::new(Item::new(Type::Knife, vec![])));
    for mode in [
        Mode::Normal,
        Mode::Inventory(InvMode::Equip),
        Mode::Inventory(InvMode::Drop_),
    ] {
        ui.mode_switch_to(mode);
        for event in &events {
            ui.handle_event(event.clone(), now);
            ui.advance((24, 60), now);
            assert_eq!(ui.mode, mode);
            assert!(ui.game_action_queue.is_empty());
            assert_eq!(ui.engine.turn(), turn);
            assert_eq!(ui.player().pos, pose);
            assert_eq!(ui.player().hp, hp);
            assert!(ui.player().items_backpack.contains_key(&'a'));
        }
    }
    ui.automoving = Some(AutoMoveType::Walk);
    for event in events {
        ui.handle_event(event, now);
        assert!(ui.is_automoving());
    }
}

#[test]
fn burst_keeps_exact_queue_and_modal_screens_do_not_pause_queued_actions() {
    let now = Instant::now();
    let mut ui = fixture(now);
    ui.advance((24, 60), now);
    chars(&mut ui, "hl?", now);
    key(&mut ui, KeyCode::F(2), now);
    assert_eq!(ui.mode, Mode::FullScreen(FSMode::Help));
    assert_eq!(ui.input_mode, InputMode::Vi);
    assert_eq!(ui.game_action_queue, [Turn(Left), Turn(Right)]);
    let original = ui.player().pos;
    ui.advance((24, 60), now);
    assert_eq!(ui.mode, Mode::FullScreen(FSMode::Help));
    assert_eq!(ui.player().pos.dir, hex2d::XZ);
    assert_eq!(ui.game_action_queue, [Turn(Right)]);
    ui.advance((24, 60), now);
    assert_eq!(ui.player().pos, original);
    assert!(ui.game_action_queue.is_empty());
    chars(&mut ui, " ", now);
    assert_eq!(ui.mode, Mode::Normal);
}

#[test]
fn inventory_only_queues_owned_letters_and_clears_removed_rows() {
    let now = Instant::now();
    let mut ui = fixture(now);
    player_mut(&mut ui)
        .items_backpack
        .insert('a', Box::new(Item::new(Type::Knife, vec![])));
    chars(&mut ui, "Iz", now);
    assert_eq!(ui.mode, Mode::Inventory(InvMode::View));
    assert!(ui.game_action_queue.is_empty());
    let before = frame(&mut ui, 60, 24);
    player_mut(&mut ui).items_backpack.clear();
    let after = frame(&mut ui, 60, 24);
    assert_ne!(before, after);
    assert_eq!(before[(1, 1)].symbol(), "a");
    assert_eq!(after[(1, 1)].symbol(), " ");
    for y in 3..24 {
        for x in 0..30 {
            assert_eq!(after[(x, y)].symbol(), " ");
        }
    }
    player_mut(&mut ui)
        .items_backpack
        .insert('a', Box::new(Item::new(Type::Knife, vec![])));
    key(&mut ui, KeyCode::Esc, now);
    chars(&mut ui, "Eza", now);
    assert_eq!(ui.game_action_queue, [Equip('a')]);
    key(&mut ui, KeyCode::Esc, now);
    chars(&mut ui, "Dza", now);
    assert_eq!(ui.game_action_queue, [Equip('a'), Drop_('a')]);
}

#[test]
fn inventory_letters_offer_only_valid_actions_without_spending_turns() {
    for input_mode in [InputMode::Normal, InputMode::Vi] {
        for (letter, type_, equipped, label, shortcut) in [
            ('a', Type::Knife, false, "Equip", 'e'),
            ('D', Type::Plate, false, "Equip", 'e'),
            ('q', Type::Knife, true, "Unequip", 'e'),
            ('h', Type::HealthPotion, false, "Drink", 'u'),
            ('e', Type::Junk, false, "Drop", 'd'),
        ] {
            let now = Instant::now();
            let mut ui = fixture(now);
            ui.input_mode = input_mode;
            let item = Box::new(Item::new(type_, vec![]));
            if equipped {
                player_mut(&mut ui).equip(item, letter);
                player_mut(&mut ui).action_cd = 0;
            } else {
                player_mut(&mut ui).items_backpack.insert(letter, item);
            }
            ui.advance((24, 60), now);
            let turn = ui.engine.turn();
            let before = game::fixtures::outcome(ui.current_location());
            chars(&mut ui, &format!("I{letter}"), now);
            assert_eq!(ui.mode, Mode::Inventory(InvMode::Actions(letter)));
            let rendered = frame(&mut ui, 60, 24);
            let text: String = rendered.content.iter().map(|cell| cell.symbol()).collect();
            assert!(text.contains(&format!("[{shortcut}] {label}")));
            assert!(text.contains("[d] Drop"));
            for unavailable in ["Equip", "Unequip", "Drink", "Use"] {
                if unavailable != label {
                    assert!(!text.contains(&format!("] {unavailable}")));
                }
            }
            chars(&mut ui, "z,k", now);
            key(&mut ui, KeyCode::Enter, now);
            ui.advance((24, 60), now);
            assert_eq!(ui.engine.turn(), turn);
            assert_eq!(before, game::fixtures::outcome(ui.current_location()));
            assert!(ui.game_action_queue.is_empty());
            key(&mut ui, KeyCode::Esc, now);
            assert_eq!(ui.mode, Mode::Inventory(InvMode::View));
            key(&mut ui, KeyCode::Esc, now);
            assert_eq!(ui.mode, Mode::Normal);
            assert!(ui.game_action_queue.is_empty());

            chars(&mut ui, &format!("I{letter}{shortcut}"), now);
            let action = if shortcut == 'd' {
                Drop_(letter)
            } else {
                Equip(letter)
            };
            assert_eq!(ui.game_action_queue, [action]);
            assert_eq!(ui.mode, Mode::Normal);
            ui.advance((24, 60), now + Duration::from_secs(1));
            assert_eq!(ui.engine.turn(), 1);
            if equipped {
                assert!(ui.player().items_backpack.contains_key(&letter));
                assert!(ui.player().items_equipped.is_empty());
            } else if type_ == Type::HealthPotion || type_ == Type::Junk {
                assert!(!ui.player().item_letter_taken(letter));
            } else {
                assert!(!ui.player().items_backpack.contains_key(&letter));
                assert!(
                    ui.player()
                        .items_equipped
                        .values()
                        .any(|(ch, _)| *ch == letter)
                );
            }
        }
    }
}

#[test]
fn inventory_action_menu_revalidates_removed_items() {
    let now = Instant::now();
    let mut ui = fixture(now);
    player_mut(&mut ui)
        .items_backpack
        .insert('a', Box::new(Item::new(Type::Knife, vec![])));
    chars(&mut ui, "Ia", now);
    player_mut(&mut ui).items_backpack.clear();
    chars(&mut ui, "ed", now);
    assert!(ui.game_action_queue.is_empty());
    key(&mut ui, KeyCode::Esc, now);
    assert_eq!(ui.mode, Mode::Inventory(InvMode::View));
}

#[test]
fn target_and_examine_change_only_selected_pose_and_queue_exact_target() {
    let now = Instant::now();
    let mut ui = fixture(now);
    let pose = ui.player().pos;
    chars(&mut ui, "f", now);
    assert_eq!(ui.mode, Mode::Normal);
    give_bow(&mut ui);
    chars(&mut ui, "f", now);
    key(&mut ui, KeyCode::Enter, now);
    assert!(ui.game_action_queue.is_empty());
    chars(&mut ui, "fkk", now);
    assert_eq!(ui.target_pos.unwrap().coord, Coordinate::new(2, -2));
    assert_eq!(ui.player().pos, pose);
    key(&mut ui, KeyCode::Enter, now);
    assert_eq!(ui.game_action_queue, [Ranged(Coordinate::new(2, -2))]);
    assert_eq!(ui.mode, Mode::Normal);
    assert_eq!(ui.target_pos, None);
    chars(&mut ui, "fk", now);
    key(&mut ui, KeyCode::Esc, now);
    assert_eq!(ui.target_pos, None);
    assert_eq!(ui.game_action_queue.len(), 1);
    chars(&mut ui, "xkh", now);
    assert_eq!(ui.mode, Mode::Examine);
    assert_eq!(
        ui.target_pos,
        Some(Position::new(Coordinate::new(1, -1), hex2d::XZ))
    );
    assert_eq!(ui.player().pos, pose);
    key(&mut ui, KeyCode::Esc, now);
    assert_eq!(ui.mode, Mode::Normal);
    assert_eq!(ui.target_pos, None);
}

#[test]
fn targeting_traverses_six_sectors_and_radius_boundaries() {
    let now = Instant::now();
    let mut ui = fixture(now);
    give_bow(&mut ui);
    chars(&mut ui, "fkk", now);
    // Radius-two ring, clockwise from east, including sector boundaries.
    for (x, y) in [
        (2, -2),
        (1, -2),
        (0, -2),
        (-1, -1),
        (-2, 0),
        (-2, 1),
        (-2, 2),
        (-1, 2),
        (0, 2),
        (1, 1),
        (2, 0),
        (2, -1),
    ] {
        assert_eq!(ui.target_pos.unwrap().coord, Coordinate::new(x, y));
        chars(&mut ui, "l", now);
    }
    assert_eq!(ui.target_pos.unwrap().coord, Coordinate::new(2, -2));
    chars(&mut ui, "j", now);
    for (x, y) in [(1, -1), (1, 0), (0, 1), (-1, 1), (-1, 0), (0, -1)] {
        assert_eq!(ui.target_pos.unwrap().coord, Coordinate::new(x, y));
        chars(&mut ui, "h", now);
    }
    chars(&mut ui, "j", now);
    assert_eq!(ui.target_pos.unwrap().coord, Coordinate::new(0, 0));
    chars(&mut ui, "h", now);
    assert_eq!(
        ui.target_pos.unwrap(),
        Position::new(Coordinate::new(0, 0), hex2d::XZ)
    );
    assert!(ui.game_action_queue.is_empty());
}

#[test]
fn intro_race_selection_and_quit_dispatch_without_terminal() {
    let now = Instant::now();
    for (letter, race) in [('a', Race::Human), ('b', Race::Elf), ('c', Race::Dwarf)] {
        let mut location = fixtures::room();
        // This constructor intentionally supplies an empty, handbuilt level.
        location.actors_byid.clear();
        location.actors_coord_to_id.clear();
        let mut ui = Ui::new(game::Engine::fixture(location), (24, 60), now);
        chars(&mut ui, " ", now);
        assert_eq!(ui.mode, Mode::FullScreen(FSMode::PickRace));
        chars(&mut ui, "z", now);
        assert!(!ui.spawned);
        assert!(ui.current_location().actors_byid.is_empty());
        key(&mut ui, KeyCode::Char(letter), now);
        assert!(ui.spawned);
        assert_eq!(ui.mode, Mode::Normal);
        assert_eq!(ui.player().race, race);
        assert_eq!(ui.current_location().actors_byid.len(), 1);
        key(&mut ui, KeyCode::Esc, now);
        chars(&mut ui, "n", now);
        assert!(!ui.exited());
        assert_eq!(ui.mode, Mode::Normal);
        key(&mut ui, KeyCode::Esc, now);
        chars(&mut ui, "y", now);
        assert!(ui.exited());
        assert!(ui.game_action_queue.is_empty());
        assert_eq!(ui.current_location().actors_byid.len(), 1);
    }
}

#[test]
fn automove_cancel_consumes_key_and_goto_selects_exits() {
    let now = Instant::now();
    let mut ui = fixture(now);
    key(&mut ui, KeyCode::F(2), now);
    chars(&mut ui, "K", now);
    assert!(ui.is_automoving());
    chars(&mut ui, "h", now);
    assert!(!ui.is_automoving());
    assert!(ui.game_action_queue.is_empty());
    chars(&mut ui, "l", now);
    assert_eq!(ui.game_action_queue, [Turn(Right)]);
    chars(&mut ui, "G>", now);
    assert!(matches!(
        ui.automoving,
        Some(AutoMoveType::GoTo(GoToType::Stairs))
    ));
    chars(&mut ui, "hGh", now);
    assert_eq!(ui.mode, Mode::Normal);
    assert!(!ui.is_automoving());
    assert_eq!(ui.game_action_queue, [Turn(Right)]);
}

#[test]
fn exit_keys_roundtrip_preserve_state_and_reset_location_presentation() {
    let now = Instant::now();
    let mut ui = fixture(now);
    let origin = ui.player().coord();
    ui.engine.current_location_mut().map[origin].feature = Some(tile::Stairs);
    player_mut(&mut ui).pick_item(Box::new(Item::new(Type::Bow, vec![])));
    for (step, key_char, expected, readout) in [
        (1, '>', 1, "Below: return to level 0 (<)"),
        (2, '<', 0, "Below: to Dark Forest (>)"),
        (3, '>', 1, "Below: return to level 0 (<)"),
    ] {
        ui.target_pos = Some(Position::new(Coordinate::new(50, 50), hex2d::XY));
        let time = now + Duration::from_secs(step);
        key(&mut ui, KeyCode::Char(key_char), time);
        ui.advance((24, 60), time);
        assert_eq!(ui.current_location().id, game::level::LevelId(expected));
        assert_eq!(ui.animation_level, ui.current_location().id);
        assert!(ui.target_pos.is_none());
        assert!(ui.automoving.is_none());
        assert!(ui.pending_sounds.is_empty());
        assert_eq!(ui.player().items_backpack[&'a'].description(), "bow");
        let before = game::fixtures::outcome(ui.current_location());
        for _ in 0..3 {
            let buffer = frame(&mut ui, 60, 24);
            let below: String = (30..60).map(|x| buffer[(x, 9)].symbol()).collect();
            assert_eq!(below, format!("{readout:30}"));
        }
        assert_eq!(before, game::fixtures::outcome(ui.current_location()));
    }
}

#[test]
fn goto_finds_return_and_forest_exits_but_never_clears_obstacles() {
    let now = Instant::now();
    for feature in [tile::Entrance, tile::ForestPath] {
        let mut ui = fixture(now);
        let head = ui.player().head();
        ui.engine.current_location_mut().map[head].feature = Some(feature);
        chars(&mut ui, "G<", now);
        assert!(matches!(ui.automoving, Some(AutoMoveType::GoTo(_))));
        assert!(matches!(
            ui.goto_action(GoToType::Stairs),
            AutoMoveAction::Action(Move(Forward))
        ));
        for c in ui.player().coord().neighbors() {
            ui.engine.current_location_mut().map[c] = tile::Tile::new(tile::Thicket);
        }
        assert!(matches!(
            ui.goto_action(GoToType::Stairs),
            AutoMoveAction::Blocked
        ));
    }
}

#[test]
fn below_minimum_pauses_queue_and_automove_and_recovers_each_mode() {
    let now = Instant::now();
    for mode in [
        Mode::Normal,
        Mode::FullScreen(FSMode::Help),
        Mode::Inventory(InvMode::View),
        Mode::Target(TargetMode::Ranged),
    ] {
        for size in [(59, 24), (60, 23), (1, 1), (0, 0)] {
            let mut ui = fixture(now);
            ui.mode_switch_to(mode);
            let target = matches!(mode, Mode::Target(_))
                .then_some(Position::new(Coordinate::new(2, -2), hex2d::XY));
            ui.target_pos = target;
            let _ = frame(&mut ui, 60, 24);
            ui.frame_drawn(now);
            ui.queue_action(Turn(Left));
            ui.automoving = Some(AutoMoveType::Walk);
            let pose = ui.player().pos;
            ui.handle_event(TerminalEvent::Resize(size.0, size.1), now);
            key(&mut ui, KeyCode::Char('h'), now);
            ui.advance(
                (size.1.into(), size.0.into()),
                now + Duration::from_secs(10),
            );
            assert_eq!(ui.engine.turn(), 0);
            assert_eq!(ui.player().pos, pose);
            assert_eq!(ui.game_action_queue, [Turn(Left)]);
            assert!(ui.is_automoving());
            assert_eq!(ui.mode, mode);
            assert_eq!(ui.target_pos, target);
            let _ = frame(&mut ui, size.0, size.1);
            for (width, height) in [(60, 24), (61, 25), (120, 36), (140, 40), (60, 24)] {
                ui.handle_event(TerminalEvent::Resize(width, height), now);
                let recovered = frame(&mut ui, width, height);
                let mut fresh = fixture(now);
                fresh.mode_switch_to(mode);
                fresh.target_pos = target;
                fresh.resize((height.into(), width.into()));
                assert_eq!(
                    recovered,
                    frame(&mut fresh, width, height),
                    "{mode:?} {width}x{height}"
                );
                assert_eq!(ui.target_pos, target);
            }
            key(&mut ui, KeyCode::Char('l'), now + Duration::from_secs(11));
            assert!(!ui.is_automoving());
            assert_eq!(ui.game_action_queue, [Turn(Left)]);
            ui.advance((24, 60), now + Duration::from_secs(11));
            assert!(ui.game_action_queue.is_empty());
            assert_eq!(ui.player().pos, Position::new(pose.coord, hex2d::XZ));
        }
    }
    let mut ui = fixture(now);
    ui.handle_event(TerminalEvent::Resize(59, 24), now);
    key(&mut ui, KeyCode::Esc, now);
    assert!(ui.exited());
}

#[test]
fn scheduling_uses_supplied_instants_and_rendering_does_not_advance_world() {
    let now = Instant::now();
    let mut ui = fixture(now);
    let mut terminal = Terminal::new(TestBackend::new(60, 24)).unwrap();
    assert!(ui.frame_due(now));
    terminal.draw(|f| ui.render_into(f.buffer_mut())).unwrap();
    ui.frame_drawn(now);
    assert!(!ui.frame_due(now + Duration::from_millis(99)));
    assert!(ui.frame_due(now + Duration::from_millis(100)));
    for tick in 1..=8 {
        terminal.draw(|f| ui.render_into(f.buffer_mut())).unwrap();
        ui.frame_drawn(now + Duration::from_millis(tick * 100));
    }
    assert_eq!(ui.engine.turn(), 0);
    assert_eq!(ui.player().pos.coord, Coordinate::new(0, 0));
    assert!(ui.game_action_queue.is_empty());
    chars(&mut ui, "x", now + Duration::from_secs(1));
    assert_eq!(ui.anim_frame_count, 0);
    assert!(ui.frame_due(now + Duration::from_secs(1)));
    ui.mode_switch_to(Mode::Normal);
    player_mut(&mut ui).discovered.clear();
    ui.automoving = Some(AutoMoveType::Walk);
    ui.advance((24, 60), now + Duration::from_secs(2));
    assert_eq!(ui.player().pos.coord, Coordinate::new(1, -1));
    ui.advance((24, 60), now + Duration::from_millis(2049));
    assert_eq!(ui.player().pos.coord, Coordinate::new(1, -1));
    ui.advance((24, 60), now + Duration::from_millis(2050));
    assert_eq!(ui.player().pos.coord, Coordinate::new(2, -2));
}

#[test]
fn dead_player_turns_are_paced_but_live_cooldown_is_not_idle() {
    let now = Instant::now();
    let mut ui = fixture(now);
    player_mut(&mut ui).hp = 0;
    ui.advance((24, 60), now);
    assert_eq!(ui.engine.turn(), 1);
    assert!(ui.engine.is_turn_done());
    let after = crate::game::fixtures::outcome(ui.current_location());
    ui.advance((24, 60), now + Duration::from_millis(49));
    assert_eq!(ui.engine.turn(), 1);
    assert_eq!(crate::game::fixtures::outcome(ui.current_location()), after);
    ui.advance((24, 60), now + Duration::from_millis(50));
    assert_eq!(ui.engine.turn(), 2);
    ui.advance((24, 60), now + Duration::from_millis(99));
    assert_eq!(ui.engine.turn(), 2);
    ui.advance((24, 60), now + Duration::from_millis(100));
    assert_eq!(ui.engine.turn(), 3);

    let mut ui = fixture(now);
    player_mut(&mut ui).action_cd = 2;
    ui.queue_action(Turn(Left));
    let pose = ui.player().pos;
    for (turn, cooldown) in [(1, 1), (2, 0)] {
        ui.advance((24, 60), now);
        assert_eq!(ui.engine.turn(), turn);
        assert_eq!(ui.player().action_cd, cooldown);
        assert_eq!(ui.player().pos, pose);
        assert_eq!(ui.game_action_queue, [Turn(Left)]);
    }
    ui.advance((24, 60), now);
    assert_eq!(ui.engine.turn(), 3);
    assert_eq!(ui.player().pos.dir, pose.dir + Left);
    assert!(ui.game_action_queue.is_empty());
}

#[test]
fn overflowing_inventory_and_multiline_log_stay_in_their_panels_and_clear() {
    use crate::game::item::Feature;
    let now = Instant::now();
    for (width, height) in [(60, 24), (120, 36)] {
        let mut ui = fixture(now);
        ui.resize((height, width));
        ui.mode_switch_to(Mode::Inventory(InvMode::View));
        let baseline = frame(&mut ui, width as u16, height as u16);
        for ch in 'a'..='z' {
            player_mut(&mut ui).items_backpack.insert(
                ch,
                Box::new(Item::new(
                    Type::Plate,
                    vec![Feature::Infravision, Feature::Regeneration, Feature::Light],
                )),
            );
        }
        // Newest first, enough wrapped lines to exceed the log region.
        for _ in 0..30 {
            ui.log("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\nsecond line");
        }
        let full = frame(&mut ui, width as u16, height as u16);
        let panel = (width - 30) as u16;
        let row = |buffer: &Buffer, y, from, to| -> String {
            (from..to).map(|x| buffer[(x, y)].symbol()).collect()
        };
        assert!(
            row(&full, 1, 0, panel).starts_with(" a - plate armor of infravisio"),
            "width {width}: {:?}",
            row(&full, 1, 0, panel)
        );
        if width == 60 {
            assert!(row(&full, 2, 0, panel).starts_with("n of regeneration of light"));
        }
        assert_eq!(
            row(&full, 13, panel, width as u16),
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123"
        );
        assert!(row(&full, 14, panel, width as u16).starts_with("456789"));
        assert!(row(&full, 15, panel, width as u16).starts_with("second line"));
        for y in 0..13 {
            for x in panel..width as u16 {
                assert_eq!(
                    full[(x, y)],
                    baseline[(x, y)],
                    "inventory/log spilled into stats"
                );
            }
        }
        // The lower left remains inventory, not the overflowing log's alphabet.
        assert!(!row(&full, 14, 0, panel).contains("second line"));
        ui.handle_event(TerminalEvent::Resize(59, 23), now);
        let _ = frame(&mut ui, 59, 23);
        ui.handle_event(TerminalEvent::Resize(width as u16, height as u16), now);
        assert_eq!(frame(&mut ui, width as u16, height as u16), full);
        player_mut(&mut ui).items_backpack.clear();
        ui.log.borrow_mut().clear();
        // Includes styles and formerly occupied cells, not just text.
        assert_eq!(frame(&mut ui, width as u16, height as u16), baseline);
    }
}
