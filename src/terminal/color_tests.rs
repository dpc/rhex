use super::*;
use ratatui::style::Modifier;

/// Decode the fixed xterm palette independently of the production quantizer.
pub(crate) fn channels(index: u8) -> [u8; 3] {
    match index {
        16..=231 => {
            let levels = [0, 95, 135, 175, 215, 255];
            let n = usize::from(index - 16);
            [levels[n / 36], levels[n / 6 % 6], levels[n % 6]]
        }
        232..=255 => [8 + (index - 232) * 10; 3],
        _ => panic!("configurable ANSI color {index}"),
    }
}

#[test]
fn cube_colors_and_gray_ramp_round_trip_exactly() {
    for index in 16..=255 {
        let [r, g, b] = channels(index);
        assert_eq!(channels(Rgb::new(r, g, b).to_u8()), [r, g, b]);
    }
    assert_eq!(Rgb::new(0, 0, 0).to_u8(), 16);
    assert_eq!(Rgb::new(255, 255, 255).to_u8(), 231);
}

#[test]
fn every_channel_uses_nearest_nonuniform_cube_step_with_darker_ties() {
    for value in 0..=255u8 {
        let expected = match value {
            0..=47 => 0,
            48..=115 => 95,
            116..=155 => 135,
            156..=195 => 175,
            196..=235 => 215,
            _ => 255,
        };
        for (rgb, channel) in [
            (Rgb::new(value, 0, 255), 0),
            (Rgb::new(255, value, 0), 1),
            (Rgb::new(0, 255, value), 2),
        ] {
            assert_eq!(channels(rgb.to_u8())[channel], expected, "{value}");
        }
    }
}

#[test]
fn neutral_colors_use_nearest_gray_with_monotonic_brightness() {
    let mut previous = 0;
    for value in 0..=255u8 {
        let index = Rgb::new(value, value, value).to_u8();
        assert!([16, 59, 102, 145, 188, 231].contains(&index) || index >= 232);
        let [r, g, b] = channels(index);
        assert_eq!(r, g);
        assert_eq!(g, b);
        assert!(r >= previous);
        // The upper endpoint gap is 17; all other gray gaps are at most 10.
        assert!(r.abs_diff(value) <= 8);
        previous = r;
    }
    for (value, expected) in [
        (4, 16),
        (5, 232),
        (13, 232),
        (14, 233),
        (246, 255),
        (247, 231),
    ] {
        assert_eq!(Rgb::new(value, value, value).to_u8(), expected);
    }
}

#[test]
fn representative_shades_are_monotonic_and_saturate_without_overflow() {
    for shade in [
        EMPTY_DOT_FG,
        GROUND_FG,
        THICKET_FG,
        TRUNK_FG,
        EMPTY_BG,
        WATER_BG,
    ] {
        let mut previous = channels(shade.index(true, 0));
        for light in 1..=5 {
            let next = channels(shade.index(true, light));
            assert!(next.into_iter().zip(previous).all(|(a, b)| a >= b));
            previous = next;
        }
        assert_eq!(shade.index(true, 5), shade.index(true, u32::MAX));
        assert_eq!(shade.index(false, 0), shade.index(false, u32::MAX));
    }
    assert_eq!(EMPTY_DOT_FG.index(true, 0), 238);
    assert_eq!(EMPTY_DOT_FG.index(false, 5), 236);
    assert_eq!(FOREST_BG.index(false, 5), 16);
}

/// Measure decoded terminal colors independently of the selector's RGB helpers.
fn luma(index: u8) -> u32 {
    let [r, g, b] = channels(index).map(u32::from);
    2126 * r + 7152 * g + 722 * b
}

#[test]
fn arbitrary_palette_endpoints_get_ordered_shades_with_dark_neutral_fallback() {
    for index in 16..=255 {
        let [r, g, b] = channels(index);
        let base = Rgb::new(r, g, b);
        for (dark, lit, ascending) in [
            (Rgb::new(0, 0, 0), base, true),
            (base, Rgb::new(0, 0, 0), false),
        ] {
            let shade = Shade::new(lit, dark, 236);
            let colors = (0..=5)
                .map(|light| shade.index(true, light))
                .collect::<Vec<_>>();
            assert_eq!(channels(colors[0]), channels(dark.to_u8()));
            assert_eq!(channels(colors[5]), channels(lit.to_u8()));
            for pair in colors.windows(2) {
                assert!(
                    if ascending {
                        luma(pair[0]) <= luma(pair[1])
                    } else {
                        luma(pair[0]) >= luma(pair[1])
                    },
                    "{index}: {colors:?}"
                );
            }
            // Four interior neutral grays exist below any endpoint above gray38.
            if luma(index) > 380_000 {
                assert_eq!(
                    colors
                        .iter()
                        .copied()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len(),
                    6,
                    "{index}: {colors:?}"
                );
            }
            assert_eq!(shade.index(true, u32::MAX), colors[5]);
            assert_eq!(shade.index(false, u32::MAX), 236);
        }
    }
}

#[test]
fn constrained_and_fixed_colors_repeat_instead_of_inventing_brightness() {
    for (base, distinct) in [
        (Rgb::new(0, 0, 0), 1),
        (Rgb::new(0, 0, 95), 2),
        (Rgb::new(8, 8, 8), 2),
        (Rgb::new(18, 18, 18), 3),
        (Rgb::new(28, 28, 28), 4),
        (Rgb::new(38, 38, 38), 5),
        (Rgb::new(48, 48, 48), 6),
    ] {
        let shade = Shade::new(base, Rgb::new(0, 0, 0), 236);
        let colors = (0..=5)
            .map(|light| shade.index(true, light))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(colors.len(), distinct, "{base:?}: {colors:?}");
        assert!(colors.iter().all(|&c| luma(c) <= luma(base.to_u8())));
        let fixed = Shade::new(base, base, 236);
        for light in 0..=6 {
            assert_eq!(fixed.index(true, light), base.to_u8());
        }
    }
}

proptest::proptest! {
    #[test]
    fn arbitrary_rgb_endpoints_preserve_bounds_memory_and_available_steps(
        dark in proptest::array::uniform3(proptest::num::u8::ANY),
        lit in proptest::array::uniform3(proptest::num::u8::ANY),
    ) {
        let dark = Rgb::new(dark[0], dark[1], dark[2]);
        let lit = Rgb::new(lit[0], lit[1], lit[2]);
        let shade = Shade::new(lit, dark, 236);
        let first = dark.to_u8();
        let last = lit.to_u8();
        let low = luma(first).min(luma(last));
        let high = luma(first).max(luma(last));
        assert_eq!(shade.index(true, 0), first);
        assert_eq!(shade.index(true, 5), last);
        assert_eq!(shade.index(true, u32::MAX), last);
        let colors = (0..=5).map(|light| shade.index(true, light)).collect::<Vec<_>>();
        if low != high {
            for pair in colors.windows(2) {
                assert!(
                    if luma(first) < luma(last) {
                        luma(pair[0]) <= luma(pair[1])
                    } else {
                        luma(pair[0]) >= luma(pair[1])
                    },
                    "{dark:?}->{lit:?}: {colors:?}"
                );
            }
            if (232..=255).filter(|&c| low < luma(c) && luma(c) < high).count() >= 4 {
                assert_eq!(colors.iter().collect::<std::collections::BTreeSet<_>>().len(), 6);
            }
        }
        for light in [0, 1, 4, 5, 6, u32::MAX] {
            assert_eq!(shade.index(false, light), 236);
        }
        let inverse = shade.with_unlit(247);
        assert_eq!(inverse.index(true, 0), 247);
        for light in 1..=6 {
            assert_eq!(inverse.index(true, light), shade.index(true, light));
        }
        assert_eq!(inverse.index(false, 0), 236);
    }
}

#[test]
fn dungeon_foregrounds_keep_contrast_through_the_whole_ramp() {
    for foreground in [EMPTY_DOT_FG, EMPTY_FG, STONE_FG] {
        for light in 0..=5 {
            let fg = channels(foreground.index(true, light))[0];
            let bg = channels(EMPTY_BG.index(true, light))[0];
            assert!(fg.abs_diff(bg) >= 28, "light {light}: {fg} on {bg}");
        }
    }
}

#[test]
fn styles_preserve_indexed_foreground_and_background_without_attributes() {
    let style = style(196, 21);
    assert_eq!(style.fg, Some(TerminalColor::Indexed(196)));
    assert_eq!(style.bg, Some(TerminalColor::Indexed(21)));
    assert_eq!(style.add_modifier, Modifier::empty());
    assert_eq!(style.sub_modifier, Modifier::empty());
    assert_eq!(
        super::style(Rgb::new(255, 0, 0), Rgb::new(0, 0, 255)),
        style
    );
}
