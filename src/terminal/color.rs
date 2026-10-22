use ratatui::style::{Color as TerminalColor, Style};

/// One of 256 colors
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Hash)]
pub struct Color(u8);

#[cfg(test)]
impl Color {
    /// Round to nearest Rgb
    pub fn to_rgb(self) -> Rgb {
        match self.0 {
            7 => Color(255).to_rgb(),
            15 => Rgb(5, 5, 5),
            16..=231 => {
                let c = self.0 - 16;
                let b = c % 6;
                let c = (c - b) / 6;
                let g = c % 6;
                let r = (c - g) / 6;
                Rgb::new(r, g, b)
            }
            232..=255 => {
                let c = (self.0 - 232) / 4;
                Rgb::new(c, c, c)
            }
            _ => panic!("Unimplemented color {}", self.0),
        }
    }
}

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

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Hash)]
pub struct Rgb(u8, u8, u8);

impl Rgb {
    #[cfg(test)]
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        assert!(r < 6);
        assert!(g < 6);
        assert!(b < 6);

        Rgb(r, g, b)
    }

    pub fn to_u8(self) -> u8 {
        16 + self.0 * 36 + self.1 * 6 + self.2
    }
}

#[test]
fn to_rgb_and_back() {
    for r in 0u8..6u8 {
        for g in 0u8..6u8 {
            for b in 0u8..6u8 {
                let rgb = Rgb(r, g, b);
                let color: Color = rgb.into();
                assert_eq!(rgb, color.to_rgb());
            }
        }
    }
}

pub const GRAY: [u8; 26] = [
    16, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249,
    250, 251, 252, 253, 254, 255, 231,
];

pub const RGB_GRAY: [Rgb; 6] = [
    Rgb(0, 0, 0),
    Rgb(1, 1, 1),
    Rgb(2, 2, 2),
    Rgb(3, 3, 3),
    Rgb(4, 4, 4),
    Rgb(5, 5, 5),
];

pub const BLACK: u8 = GRAY[0];
pub const WHITE: u8 = GRAY[25];

pub const YELLOW: Rgb = Rgb(5, 5, 0);

pub const BACKGROUND_BG: u8 = GRAY[1];
pub const MAP_BACKGROUND_BG: u8 = BACKGROUND_BG;

pub const VISIBLE_FG: u8 = WHITE;

pub const NOT_IN_LOS_FG: u8 = GRAY[16];
pub const NOT_IN_LOS_BG: u8 = GRAY[1];
pub const REMEMBERED_TERRAIN_FG: u8 = GRAY[10];

// in light, shaded (barely visible), in LoS but not visible (dark), not in LoS
pub const EMPTY_FG: [u8; 4] = [GRAY[17], GRAY[10], NOT_IN_LOS_FG, REMEMBERED_TERRAIN_FG];
pub const EMPTY_BG: [u8; 4] = [GRAY[24], GRAY[16], GRAY[4], NOT_IN_LOS_BG];
pub const WATER_FG: [u8; 4] = EMPTY_FG;
pub const WATER_BG: [u8; 4] = [4, 74, 67, NOT_IN_LOS_BG];
pub const STONE_FG: [u8; 4] = [BLACK, GRAY[1], NOT_IN_LOS_FG, REMEMBERED_TERRAIN_FG];
pub const WALL_FG: [u8; 4] = STONE_FG;
pub const WALL_BG: [u8; 4] = EMPTY_BG;
/// Earth and foliage palettes, with the shared subdued remembered state.
pub const GROUND_FG: [u8; 4] = [65, 58, 22, REMEMBERED_TERRAIN_FG];
pub const FOREST_BG: [u8; 4] = [22, 16, 16, NOT_IN_LOS_BG];
pub const THICKET_FG: [u8; 4] = [28, 22, 22, REMEMBERED_TERRAIN_FG];
pub const TRUNK_FG: [u8; 4] = [94, 58, 52, REMEMBERED_TERRAIN_FG];
pub const CHAR_SELF_FG: [u8; 4] = [19, 18, NOT_IN_LOS_FG, NOT_IN_LOS_FG];
pub const _CHAR_ALLY_FG: [u8; 4] = [28, 22, NOT_IN_LOS_FG, NOT_IN_LOS_FG];
pub const CHAR_ENEMY_FG: [u8; 4] = [124, 88, NOT_IN_LOS_FG, NOT_IN_LOS_FG];
pub const CHAR_GRAY_FG: u8 = GRAY[17];
pub const CHAR_BG: [u8; 4] = EMPTY_BG;

pub const BLOCKED_BG: u8 = 124;

pub const LABEL_FG: u8 = 94;
pub const GREEN_FG: u8 = 34;
pub const RED_FG: u8 = 124;
pub const LIGHTSOURCE: [Rgb; 4] = [YELLOW, YELLOW, YELLOW, YELLOW];
pub const LOG_1_FG: Rgb = RGB_GRAY[5];
pub const LOG_2_FG: Rgb = RGB_GRAY[4];
pub const LOG_3_FG: Rgb = RGB_GRAY[3];
pub const LOG_4_FG: Rgb = RGB_GRAY[2];
pub const LOG_5_FG: Rgb = RGB_GRAY[1];

pub const SELF_HEAD_ACTOR_FG: Rgb = Rgb(0, 0, 5);
pub const ENEMY_HEAD_ACTOR_FG: Rgb = Rgb(5, 0, 0);

/// Select indexed terminal colors without allocating a bounded color-pair table.
pub fn style(fg: impl Into<Color>, bg: impl Into<Color>) -> Style {
    Style::default()
        .fg(TerminalColor::Indexed(fg.into().0))
        .bg(TerminalColor::Indexed(bg.into().0))
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod tests;
