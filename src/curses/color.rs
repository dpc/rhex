use std::collections::HashMap;
use std::collections::hash_map::Entry;
use ncurses as nc;


/// One of 256 colors
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Hash)]
pub struct Color(u8);

impl Color {
    /// Round to nearest RGB
    pub fn to_rgb(&self) -> RGB {
        match self.0 {
            7 => Color(255).to_rgb(),
            15 => RGB(5, 5, 5),
            16...231 => {
                let c = self.0 - 16;
                let b = c % 6;
                let c = (c - b) / 6;
                let g = c % 6;
                let r = (c - g) / 6;
                RGB::new(r, g, b)
            },
            232...255 => {
                let c = (self.0 - 232) / 4;
                RGB::new(c, c, c)
            }
            _ => panic!("Unimplemented color {}", self.0),
        }
    }

    pub fn mix(&self, color : Color, s : u8) -> Color {
        assert!(s < 6);
        let s_rgb = self.to_rgb();
        let c_rgb = color.to_rgb();
        RGB::new(
            (s_rgb.0 * (5 - s) + c_rgb.0 * s) / 5,
            (s_rgb.1 * (5 - s) + c_rgb.1 * s) / 5,
            (s_rgb.2 * (5 - s) + c_rgb.2 * s) / 5,
            ).into()
    }
}

impl From<RGB> for Color {
    fn from(rgb : RGB) -> Self {
        Color(rgb.to_u8())
    }
}

impl From<u8> for Color {
    fn from(u : u8) -> Self {
        Color(u)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Hash)]
pub struct RGB(u8, u8, u8);

impl RGB {
    pub fn new(r : u8, g : u8, b : u8) -> Self {
        assert!(r < 6);
        assert!(g < 6);
        assert!(b < 6);

        RGB(r, g, b)
    }

    pub fn to_u8(&self) -> u8 {
        16 + self.0 * 36 + self.1 * 6 + self.2
    }
}

#[test]
fn to_rgb_and_back() {
    for r in 0u8..6u8 {
        for g in 0u8..6u8 {
            for b in 0u8..6u8 {
                let rgb = RGB(r, g, b);
                let color : Color = rgb.into();
                assert_eq!(rgb, color.to_rgb());
            }
        }
    }
}

pub const GRAY: [u8; 26] = [16, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244,
                            245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 231];

pub const RGB_GRAY : [RGB; 6] = [RGB(0, 0, 0), RGB(1, 1, 1), RGB(2, 2, 2), RGB(3, 3, 3), RGB(4, 4, 4), RGB(5, 5, 5)];

pub const BLACK: u8 = GRAY[0];
pub const WHITE: u8 = GRAY[25];

pub const YELLOW: RGB = RGB(5, 5, 0);
pub const ORANGE: RGB = RGB(5, 3, 0);

pub const BACKGROUND_BG : u8 = GRAY[1];
pub const MAP_BACKGROUND_BG : u8 = BACKGROUND_BG;

pub const VISIBLE_FG: u8 = WHITE;

pub const NOT_IN_LOS_FG: u8 = GRAY[16];
pub const NOT_IN_LOS_BG: u8 = GRAY[1];

// in light, shaded (barely visible), in LoS but not visible (dark), not in LoS
pub const EMPTY_FG: [u8; 4] = [GRAY[17], GRAY[10], NOT_IN_LOS_FG, NOT_IN_LOS_FG];
pub const EMPTY_BG: [u8; 4] = [GRAY[24], GRAY[16], GRAY[4], NOT_IN_LOS_BG];
pub const WATER_FG: [u8; 4] = EMPTY_FG;
pub const WATER_BG: [u8; 4] = [4, 74, 67, NOT_IN_LOS_BG];
pub const STONE_FG: [u8; 4] = [BLACK, GRAY[1], NOT_IN_LOS_FG, NOT_IN_LOS_FG];
pub const WALL_FG: [u8; 4] = STONE_FG;
pub const WALL_BG: [u8; 4] = EMPTY_BG;
pub const CHAR_SELF_FG: [u8; 4] = [19, 18, NOT_IN_LOS_FG, NOT_IN_LOS_FG];
pub const _CHAR_ALLY_FG: [u8; 4] = [28, 22, NOT_IN_LOS_FG, NOT_IN_LOS_FG];
pub const CHAR_ENEMY_FG: [u8; 4] = [124, 88, NOT_IN_LOS_FG, NOT_IN_LOS_FG];
pub const CHAR_GRAY_FG: u8 = GRAY[17];
pub const CHAR_BG: [u8; 4] = EMPTY_BG;

pub const BLOCKED_BG: u8 = 124;

pub const LABEL_FG: u8 = 94;
pub const GREEN_FG: u8 = 34;
pub const RED_FG: u8 = 124;
pub const NOISE_BG: RGB = ORANGE;
pub const TARGET_SELF_FG: u8 = 33;
pub const TARGET_ENEMY_FG: u8 = 196;
pub const LIGHTSOURCE: RGB = YELLOW;
pub const LOG_1_FG: RGB = RGB_GRAY[5];
pub const LOG_2_FG: RGB = RGB_GRAY[4];
pub const LOG_3_FG: RGB = RGB_GRAY[3];
pub const LOG_4_FG: RGB = RGB_GRAY[2];
pub const LOG_5_FG: RGB = RGB_GRAY[1];

pub const SELF_HEAD_ACTOR_BG : RGB = RGB(0, 0, 5);
pub const ENEMY_HEAD_ACTOR_BG : RGB = RGB(5, 0, 0);
pub const ACTOR_HEAD_ACTOR_FG : RGB = RGB(5, 5, 5);

pub struct Allocator {
    map: HashMap<(Color, Color), i16>,
    cur: i16,
}

impl Allocator {
    pub fn new() -> Allocator {
        Allocator {
            cur: 1i16, // 0 is reserved for defaults
            map: HashMap::new(),
        }
    }

    pub fn get<C1, C2>(&mut self, fg: C1, bg: C2) -> i16
    where C1 : Into<Color>+Clone,
          C2 : Into<Color>+Clone {
        match self.map.entry((fg.clone().into(), bg.clone().into())) {
            Entry::Occupied(i) => *i.get(),
            Entry::Vacant(i) => {
                assert!((self.cur as i32) < nc::COLOR_PAIRS,
                        "curses run out of color pairs!");
                let ret = self.cur;
                i.insert(self.cur);
                nc::init_pair(ret, fg.into().0 as i16, bg.into().0 as i16);
                self.cur += 1;
                ret
            }
        }
    }
}
