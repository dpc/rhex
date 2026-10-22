pub use super::rgb::Rgb;
use super::shade::Shade;
use ratatui::style::{Color as TerminalColor, Style};

/// One of 256 colors
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Hash)]
pub struct Color(u8);

impl From<Rgb> for Color {
    fn from(rgb: Rgb) -> Self {
        Color(rgb.to_u8())
    }
}

impl From<u8> for Color {
    fn from(u: u8) -> Self {
        Color(u)
    }
}

pub const GRAY: [u8; 26] = [
    16, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249,
    250, 251, 252, 253, 254, 255, 231,
];

pub const BLACK: u8 = GRAY[0];
pub const WHITE: u8 = GRAY[25];

pub const YELLOW: Rgb = Rgb::new(255, 255, 0);

// Index 232 is near-black, brighter than unlit forest tiles. Use cube black
// rather than a grayscale step or the terminal's configurable default.
pub const BACKGROUND_BG: u8 = BLACK;
pub const MAP_BACKGROUND_BG: u8 = BACKGROUND_BG;

pub const VISIBLE_FG: u8 = WHITE;

pub const NOT_IN_LOS_FG: u8 = GRAY[16];
pub const NOT_IN_LOS_BG: u8 = BACKGROUND_BG;
pub const REMEMBERED_TERRAIN_FG: u8 = GRAY[5];

// Shared palette-aware ramps saturate at lantern strength. Terrain declares
// endpoint colors, not six hand-tuned indices. Bright backgrounds use inverse
// neutral glyphs for contrast; with_unlit handles their zero-light polarity.
pub const EMPTY_FG: Shade = Shade::new(
    Rgb::new(168, 168, 168),
    Rgb::new(108, 108, 108),
    REMEMBERED_TERRAIN_FG,
);
/// Unlit floor dots stay subdued below shaded dots.
pub const EMPTY_DOT_FG: Shade = Shade::new(
    Rgb::new(188, 188, 188),
    Rgb::new(68, 68, 68),
    REMEMBERED_TERRAIN_FG,
);
// Once lit, dungeon backgrounds must stay brighter than floor-dot grays.
// A straight ramp from the unlit background would erase dots at light=2.
pub const EMPTY_BG: Shade = Shade::new(
    Rgb::new(238, 238, 238),
    Rgb::new(128, 128, 128),
    NOT_IN_LOS_BG,
)
.with_unlit(235);
pub const WATER_FG: Shade = EMPTY_FG;
// Blue contributes little luminance. Add green at full light rather than using
// pure blue255, which would hide the player's unchanged blue facing marker.
pub const WATER_BG: Shade = Shade::new(Rgb::new(0, 95, 215), Rgb::new(0, 0, 0), NOT_IN_LOS_BG);
pub const STONE_FG: Shade = Shade::new(
    Rgb::new(0, 0, 0),
    Rgb::new(48, 48, 48),
    REMEMBERED_TERRAIN_FG,
)
.with_unlit(247);
pub const WALL_FG: Shade = STONE_FG;
pub const WALL_BG: Shade = EMPTY_BG;
/// Earth and foliage retain their color family while gaining intermediate shades.
pub const GROUND_FG: Shade = Shade::new(
    Rgb::new(175, 215, 95),
    Rgb::new(0, 55, 0),
    REMEMBERED_TERRAIN_FG,
);
// The lowest nonzero cube green is 95: black->green95 yields only two colors.
// The shared selector supplies neutral near-dark steps to bridge that gap.
// Stop at green175 rather than neon green255 to keep pale glyphs readable.
pub const FOREST_BG: Shade = Shade::new(Rgb::new(0, 175, 0), Rgb::new(0, 0, 0), NOT_IN_LOS_BG);
pub const THICKET_FG: Shade = Shade::new(
    Rgb::new(135, 255, 95),
    Rgb::new(0, 90, 0),
    REMEMBERED_TERRAIN_FG,
);
// A neutral dark floor avoids saturated red bark in the coarse bottom cube step.
pub const TRUNK_FG: Shade = Shade::new(
    Rgb::new(215, 175, 135),
    Rgb::new(78, 68, 58),
    REMEMBERED_TERRAIN_FG,
);
pub const CHAR_HUMANOID_FG: Shade =
    Shade::new(Rgb::new(0, 0, 175), Rgb::new(158, 158, 158), NOT_IN_LOS_FG);
pub const CHAR_ENEMY_FG: Shade =
    Shade::new(Rgb::new(175, 0, 0), Rgb::new(158, 158, 158), NOT_IN_LOS_FG);
pub const CHAR_GRAY_FG: u8 = GRAY[17];

pub const BLOCKED_BG: u8 = 124;

pub const LABEL_FG: u8 = 94;
pub const GREEN_FG: u8 = 34;
pub const RED_FG: u8 = 124;
pub const LIGHTSOURCE: Shade = Shade::new(YELLOW, YELLOW, REMEMBERED_TERRAIN_FG);
// Keep unrelated UI colors at their exact previous cube indices.
pub const LOG_1_FG: u8 = 231;
pub const LOG_2_FG: u8 = 188;
pub const LOG_3_FG: u8 = 145;
pub const LOG_4_FG: u8 = 102;
pub const LOG_5_FG: u8 = 59;

pub const SELF_HEAD_ACTOR_FG: Rgb = Rgb::new(0, 0, 255);
pub const ENEMY_HEAD_ACTOR_FG: Rgb = Rgb::new(255, 0, 0);

/// Select indexed terminal colors without allocating a bounded color-pair table.
pub fn style(fg: impl Into<Color>, bg: impl Into<Color>) -> Style {
    Style::default()
        .fg(TerminalColor::Indexed(fg.into().0))
        .bg(TerminalColor::Indexed(bg.into().0))
}

#[cfg(test)]
#[path = "color_tests.rs"]
pub(super) mod tests;
