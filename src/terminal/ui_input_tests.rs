use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn left_and_right_keys_rotate_actor_in_opposite_directions() {
    use hex2d::{XY, XZ, YX, YZ, ZX, ZY};

    // Explicit clockwise neighbors include the YZ/YX modular wrap boundary.
    for (facing, left, right) in [
        (YZ, YX, XZ),
        (XZ, YZ, XY),
        (XY, XZ, ZY),
        (ZY, XY, ZX),
        (ZX, ZY, YX),
        (YX, ZX, YZ),
    ] {
        let pos = Position::new(Coordinate::new(3, -2), facing);
        let actor = Actor::new(Race::Human, pos);
        for mode in [InputMode::Normal, InputMode::Vi] {
            for (code, expected) in [
                (KeyCode::Char('h'), left),
                (KeyCode::Left, left),
                (KeyCode::Char('l'), right),
                (KeyCode::Right, right),
            ] {
                let key = input::game_key(KeyEvent::new(code, KeyModifiers::NONE)).unwrap();
                let Some(Game(action)) = Ui::key_to_action_normal(mode, key) else {
                    panic!("turn key did not produce a game action");
                };
                assert_eq!(
                    actor.pos_after_action(action),
                    vec![Position::new(pos.coord, expected)],
                    "{mode:?} {code:?} from {facing:?}"
                );
            }
        }
    }
}

#[test]
fn movement_and_commands_in_both_input_modes() {
    for mode in [InputMode::Normal, InputMode::Vi] {
        let mut bindings = vec![
            (KeyCode::Left, Turn(Left)),
            (KeyCode::Right, Turn(Right)),
            (KeyCode::Up, Move(Forward)),
            (KeyCode::Down, Move(Back)),
            (KeyCode::Char('h'), Turn(Left)),
            (KeyCode::Char('l'), Turn(Right)),
            (KeyCode::Char('k'), Move(Forward)),
            (KeyCode::Char('j'), Move(Back)),
            (KeyCode::Char('c'), Charge),
            (KeyCode::Char('.'), Wait),
            (KeyCode::Char(','), Pick),
            (KeyCode::Char('>'), Descend),
            (KeyCode::Char('<'), Descend),
        ];
        bindings.extend(match mode {
            InputMode::Normal => vec![
                (KeyCode::Char('a'), Turn(Left)),
                (KeyCode::Char('d'), Turn(Right)),
                (KeyCode::Char('w'), Move(Forward)),
                (KeyCode::Char('s'), Move(Back)),
                (KeyCode::Char('q'), Move(Left)),
                (KeyCode::Char('e'), Move(Right)),
            ],
            InputMode::Vi => vec![
                (KeyCode::Char('u'), Spin(Left)),
                (KeyCode::Char('i'), Spin(Right)),
                (KeyCode::Char('H'), Move(Left)),
                (KeyCode::Char('L'), Move(Right)),
            ],
        });
        for (code, expected) in bindings {
            let key = input::game_key(KeyEvent::new(code, KeyModifiers::NONE)).unwrap();
            let Some(Game(actual)) = Ui::key_to_action_normal(mode, key) else {
                panic!("{mode:?} {code:?} did not produce a game action");
            };
            assert_eq!(actual, expected, "{mode:?} {code:?}");
        }
    }
}

#[test]
fn ui_commands_are_not_game_actions() {
    for mode in [InputMode::Normal, InputMode::Vi] {
        for (key, expected) in [
            ('I', Mode::Inventory(InvMode::View)),
            ('E', Mode::Inventory(InvMode::Equip)),
            ('D', Mode::Inventory(InvMode::Drop_)),
            ('x', Mode::Examine),
            ('f', Mode::Target(TargetMode::Ranged)),
            ('?', Mode::FullScreen(FSMode::Help)),
            ('G', Mode::GoTo),
            ('\x1b', Mode::FullScreen(FSMode::Quit)),
        ] {
            let Some(ModeSwitch(actual)) = Ui::key_to_action_normal(mode, key as i32) else {
                panic!("{mode:?} {key:?} did not switch mode");
            };
            assert_eq!(actual, expected);
        }
        for (key, expected) in [
            ('o', AutoMoveType::Explore),
            (
                if mode == InputMode::Vi { 'K' } else { 'W' },
                AutoMoveType::Walk,
            ),
        ] {
            assert!(matches!(
                Ui::key_to_action_normal(mode, key as i32),
                Some(AutoMove(actual)) if actual == expected
            ));
        }
        assert!(Ui::key_to_action_normal(mode, input::F2).is_none());
    }
    assert!(matches!(
        Ui::key_to_action_normal(InputMode::Vi, 'q' as i32),
        Some(ModeSwitch(Mode::FullScreen(FSMode::Quit)))
    ));
    assert!(matches!(
        Ui::key_to_action_global(input::F2),
        Some(ToggleInputMode)
    ));
    let mut mode = InputMode::Normal;
    mode.toggle();
    assert_eq!(mode, InputMode::Vi);
    mode.toggle();
    assert_eq!(mode, InputMode::Normal);
}

#[test]
fn unmapped_keys_do_not_gain_vi_or_global_commands() {
    for (mode, keys) in [(InputMode::Normal, "uiHLK"), (InputMode::Vi, "adwseW")] {
        for key in keys.chars().chain(['z', ' ', '\n']) {
            assert!(
                Ui::key_to_action_normal(mode, key as i32).is_none(),
                "{mode:?} {key}"
            );
        }
    }
    for key in 0..=127 {
        assert!(Ui::key_to_action_global(key).is_none());
    }
}
