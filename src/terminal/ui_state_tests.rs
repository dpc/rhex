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
                ui.engine.one_actor_tick();
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
    chars(&mut ui, "Iaz", now);
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
fn automove_cancel_consumes_key_and_goto_selects_only_stairs() {
    let now = Instant::now();
    let mut ui = fixture(now);
    chars(&mut ui, "W", now);
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
