use std::collections::HashMap;
use std::collections::hash_map::Entry;
use ncurses as nc;

pub const GRAY : [u8; 26] = [
    16, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243,
    244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 15
];
pub const BLACK : u8 = GRAY[0];
pub const WHITE : u8 = GRAY[25];
pub const YELLOW : u8 = 226;
pub const ORANGE : u8 = 3;

pub const BACKGROUND_BG : u8 = GRAY[1];
pub const MAP_BACKGROUND_BG : u8 = GRAY[1];

pub const VISIBLE_FG : u8 = WHITE;

pub const NOT_IN_LOS_FG : u8 = GRAY[16];
pub const NOT_IN_LOS_BG : u8 = GRAY[1];

// in light, shaded (barely visible), in LoS but not visible (dark), not in LoS
pub const EMPTY_FG : [u8; 4] = [GRAY[17], GRAY[10], NOT_IN_LOS_FG, NOT_IN_LOS_FG];
pub const EMPTY_BG : [u8; 4] = [GRAY[24], GRAY[16], GRAY[4], NOT_IN_LOS_BG];
pub const WATER_FG: [u8; 4] = EMPTY_FG;
pub const WATER_BG: [u8; 4] = [4, 74, 67, 67];
pub const STONE_FG : [u8; 4] = [BLACK, GRAY[1] , NOT_IN_LOS_FG, NOT_IN_LOS_FG];
pub const WALL_FG : [u8; 4] = STONE_FG;
pub const WALL_BG : [u8; 4] = EMPTY_BG;
pub const CHAR_SELF_FG : [u8; 4] = [19, 18, NOT_IN_LOS_FG, NOT_IN_LOS_FG];
pub const _CHAR_ALLY_FG : [u8; 4] = [28, 22, NOT_IN_LOS_FG, NOT_IN_LOS_FG];
pub const CHAR_ENEMY_FG : [u8; 4] = [124, 88, NOT_IN_LOS_FG, NOT_IN_LOS_FG];
pub const CHAR_GRAY_FG : u8= GRAY[17];
pub const CHAR_BG : [u8; 4] = EMPTY_BG;

pub const BLOCKED_BG :   u8 = 124;

pub const LABEL_FG: u8 = 94;
pub const GREEN_FG: u8 = 34;
pub const RED_FG:   u8 = 124;
pub const NOISE_BG : u8 = ORANGE;
pub const TARGET_SELF_FG : u8 = 33;
pub const TARGET_ENEMY_FG : u8 = 196;
pub const LIGHTSOURCE : u8 = YELLOW;
pub const LOG_1_FG : u8 = GRAY[25];
pub const LOG_2_FG : u8 = GRAY[21];
pub const LOG_3_FG : u8 = GRAY[17];
pub const LOG_4_FG : u8 = GRAY[13];
pub const LOG_5_FG : u8 = GRAY[9];

pub struct Allocator {
    map : HashMap<(u8, u8), i16>,
    cur : i16,
}

impl Allocator {
    pub fn new() -> Allocator {
        Allocator {
            cur: 1i16, /* 0 is reserved for defaults */
            map: HashMap::new(),
        }
    }

    pub fn get(&mut self, fg : u8, bg : u8) -> i16 {
        match self.map.entry((fg, bg)) {
            Entry::Occupied(i) => *i.get(),
            Entry::Vacant(i) => {
                assert!((self.cur as i32) < nc::COLOR_PAIRS, "curses run out of color pairs!");
                let ret = self.cur;
                i.insert(self.cur);
                nc::init_pair(ret, fg as i16, bg as i16);
                self.cur += 1;
                ret
            }
        }
    }
}
