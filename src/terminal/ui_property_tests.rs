//! A deliberately small model of event dispatch, not a second UI implementation.
use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use proptest::prelude::*;

/// The model covers normal/help/quit/inventory/ranged modes and radial targeting.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Screen {
    Normal,
    Help,
    Quit,
    Inventory,
    Target,
}

/// Named operations keep shrunk traces readable.
#[derive(Clone, Copy, Debug)]
enum Event {
    Forward,
    Wait,
    Fire,
    Confirm,
    Cancel,
    Inventory,
    Help,
    Toggle,
    Unsupported,
    Small,
    Restore,
}

fn events() -> impl Strategy<Value = Vec<Event>> {
    proptest::collection::vec(
        prop::sample::select(vec![
            Event::Forward,
            Event::Wait,
            Event::Fire,
            Event::Confirm,
            Event::Cancel,
            Event::Inventory,
            Event::Help,
            Event::Toggle,
            Event::Unsupported,
            Event::Small,
            Event::Restore,
        ]),
        0..=80,
    )
}

fn check(events: &[Event]) {
    let now = Instant::now();
    let mut loc = crate::terminal::fixtures::room();
    loc.actors_byid.get_mut(&0).unwrap().equip(
        Box::new(game::item::Item::new(game::item::Type::Bow, vec![])),
        'b',
    );
    loc.actors_byid.get_mut(&0).unwrap().action_cd = 0;
    let mut ui = Ui::new(game::Engine::fixture(loc), (24, 60), now);
    ui.spawned = true;
    ui.mode_switch_to(Mode::Normal);
    let mut screen = Screen::Normal;
    let mut radius: Option<i32> = None;
    let mut paused = false;
    let mut vi = false;
    let mut exit = false;
    let mut queue = VecDeque::new();

    for (index, event) in events.iter().enumerate() {
        let code = match event {
            Event::Forward => KeyCode::Char('k'),
            Event::Wait => KeyCode::Char('.'),
            Event::Fire => KeyCode::Char('f'),
            Event::Confirm => KeyCode::Enter,
            Event::Cancel => KeyCode::Esc,
            Event::Inventory => KeyCode::Char('I'),
            Event::Help => KeyCode::Char('?'),
            Event::Toggle => KeyCode::F(2),
            Event::Unsupported => KeyCode::Char('λ'),
            Event::Small | Event::Restore => KeyCode::Null,
        };
        match event {
            Event::Small | Event::Restore => {
                paused = matches!(event, Event::Small);
                ui.handle_event(TerminalEvent::Resize(if paused { 59 } else { 60 }, 24), now);
            }
            _ => {
                ui.handle_event(
                    TerminalEvent::Key(KeyEvent::new(code, KeyModifiers::NONE)),
                    now,
                );
                if matches!(event, Event::Unsupported) {
                    // Rejected by terminal decoding in every mode.
                } else if paused {
                    exit |= matches!(event, Event::Cancel);
                } else if matches!(event, Event::Toggle) {
                    vi = !vi;
                } else {
                    match screen {
                        Screen::Normal => match event {
                            Event::Forward => queue.push_back(Move(Forward)),
                            Event::Wait => queue.push_back(Wait),
                            Event::Fire => {
                                screen = Screen::Target;
                                radius = None;
                            }
                            Event::Inventory => screen = Screen::Inventory,
                            Event::Help => screen = Screen::Help,
                            Event::Cancel => screen = Screen::Quit,
                            _ => {}
                        },
                        Screen::Help | Screen::Quit => screen = Screen::Normal,
                        Screen::Inventory => {
                            if matches!(event, Event::Cancel) {
                                screen = Screen::Normal;
                            }
                        }
                        Screen::Target => match event {
                            Event::Forward => radius = Some(radius.unwrap_or(0) + 1),
                            Event::Fire | Event::Confirm => {
                                if let Some(r) = radius {
                                    queue.push_back(Ranged(Coordinate::new(r, -r)));
                                }
                                radius = None;
                                screen = Screen::Normal;
                            }
                            Event::Cancel => {
                                radius = None;
                                screen = Screen::Normal;
                            }
                            _ => {}
                        },
                    }
                }
            }
        }
        let expected_mode = match screen {
            Screen::Normal => Mode::Normal,
            Screen::Help => Mode::FullScreen(FSMode::Help),
            Screen::Quit => Mode::FullScreen(FSMode::Quit),
            Screen::Inventory => Mode::Inventory(InvMode::View),
            Screen::Target => Mode::Target(TargetMode::Ranged),
        };
        assert_eq!(ui.mode, expected_mode, "prefix {:?}", &events[..=index]);
        assert_eq!(
            ui.target_pos.map(|p| p.coord),
            radius.map(|r| Coordinate::new(r, -r))
        );
        assert_eq!(
            ui.game_action_queue,
            queue,
            "prefix {:?}",
            &events[..=index]
        );
        assert_eq!(ui.too_small(), paused);
        assert_eq!(ui.in_vi_input_mode(), vi);
        assert_eq!(ui.exit, exit);
        if paused {
            let before = game::fixtures::outcome(ui.current_location());
            let turn = ui.engine.turn();
            ui.advance((24, 59), now + Duration::from_secs(10));
            assert_eq!(game::fixtures::outcome(ui.current_location()), before);
            assert_eq!(ui.engine.turn(), turn);
            assert_eq!(ui.game_action_queue, queue);
        }
    }
}

proptest! {
    #![proptest_config(crate::property_support::config(
        "ui", include_str!("regressions/ui.txt"), 64))]
    #[test]
    fn event_sequences_match_small_model(trace in events()) {
        check(&trace);
    }
}

#[test]
fn queued_move_survives_modal_pause_and_target_confirmation() {
    check(&[
        Event::Forward,
        Event::Fire,
        Event::Forward,
        Event::Small,
        Event::Confirm,
        Event::Restore,
        Event::Confirm,
    ]);
}

#[test]
fn two_moves_append_instead_of_replacing_the_queue() {
    check(&[Event::Forward, Event::Forward]);
}

#[test]
fn committed_ui_regression_replays_without_random_cases() {
    let mut config = crate::property_support::config("ui", include_str!("regressions/ui.txt"), 0);
    config.cases = 0;
    let calls = std::cell::Cell::new(0);
    proptest::test_runner::TestRunner::new(config)
        .run(&events(), |trace| {
            calls.set(calls.get() + 1);
            check(&trace);
            Ok(())
        })
        .unwrap();
    assert!(calls.get() > 0, "committed corpus did not execute");
}
