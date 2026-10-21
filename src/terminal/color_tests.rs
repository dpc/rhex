use super::*;
use ratatui::style::Modifier;

#[test]
fn palette_examples_and_mix_endpoints() {
    for (index, rgb) in [
        (16, RGB(0, 0, 0)),
        (21, RGB(0, 0, 5)),
        (46, RGB(0, 5, 0)),
        (196, RGB(5, 0, 0)),
        (231, RGB(5, 5, 5)),
        (232, RGB(0, 0, 0)),
        (255, RGB(5, 5, 5)),
        (7, RGB(5, 5, 5)),
        (15, RGB(5, 5, 5)),
    ] {
        assert_eq!(Color::from(index).to_rgb(), rgb);
    }
    let red = Color::from(196);
    let blue = Color::from(21);
    assert_eq!(red.mix(blue, 0), red);
    assert_eq!(red.mix(blue, 5), blue);
    assert_eq!(red.mix(blue, 2).to_rgb(), RGB(3, 0, 2));
}

#[test]
fn styles_preserve_indexed_foreground_and_background_without_attributes() {
    let style = style(196, 21);
    assert_eq!(style.fg, Some(TerminalColor::Indexed(196)));
    assert_eq!(style.bg, Some(TerminalColor::Indexed(21)));
    assert_eq!(style.add_modifier, Modifier::empty());
    assert_eq!(style.sub_modifier, Modifier::empty());
}
