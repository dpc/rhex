use super::*;
use ratatui::style::Modifier;

#[test]
fn palette_examples() {
    for (index, rgb) in [
        (16, Rgb(0, 0, 0)),
        (21, Rgb(0, 0, 5)),
        (46, Rgb(0, 5, 0)),
        (196, Rgb(5, 0, 0)),
        (231, Rgb(5, 5, 5)),
        (232, Rgb(0, 0, 0)),
        (255, Rgb(5, 5, 5)),
        (7, Rgb(5, 5, 5)),
        (15, Rgb(5, 5, 5)),
    ] {
        assert_eq!(Color::from(index).to_rgb(), rgb);
    }
}

#[test]
fn player_grays_brighten_with_light() {
    let [lit, shaded, dark, unseen] = CHAR_SELF_FG;
    assert!(
        CHAR_SELF_FG
            .into_iter()
            .all(|index| (232..=255).contains(&index))
    );
    // Indexed grays increase in brightness; the RGB helper rounds these away.
    assert!(dark < shaded && shaded < lit);
    assert_eq!(unseen, dark);
}

#[test]
fn styles_preserve_indexed_foreground_and_background_without_attributes() {
    let style = style(196, 21);
    assert_eq!(style.fg, Some(TerminalColor::Indexed(196)));
    assert_eq!(style.bg, Some(TerminalColor::Indexed(21)));
    assert_eq!(style.add_modifier, Modifier::empty());
    assert_eq!(style.sub_modifier, Modifier::empty());
}
