use hex2d::IntegerSpacing;

pub static SPACING: IntegerSpacing<i32> = IntegerSpacing::PointyTop(2, 1);

pub const WALL_CH : &'static str = "#";
pub const DOOR_OPEN_CH : &'static str = "_";
pub const DOOR_CLOSED_CH : &'static str = "×";
pub const STATUE_CH : &'static str = "&";
pub const STAIRS_DOWN_CH : &'static str = ">";
pub const WATER_CH : &'static str = "~";
pub const NOTHING_CH : &'static str = "~";
pub const NORMAL_DOT : &'static str = ".";
pub const UNICODE_DOT : &'static str = "·";

pub const KEY_ESC : i32 = 0x1b;
pub const KEY_ENTER: i32 = '\n' as i32;
pub const KEY_LOWX : i32 = 'x' as i32;
pub const KEY_LOWA : i32 = 'a' as i32;
pub const KEY_LOWB : i32 = 'b' as i32;
pub const KEY_LOWC : i32 = 'c' as i32;
pub const KEY_LOWY : i32 = 'y' as i32;
pub const KEY_LOWH : i32 = 'h' as i32;
pub const KEY_LOWL : i32 = 'l' as i32;
pub const KEY_LOWK : i32 = 'k' as i32;
pub const KEY_LOWU : i32 = 'u' as i32;
pub const KEY_LOWI : i32 = 'i' as i32;
pub const KEY_LOWO : i32 = 'o' as i32;
pub const KEY_LOWQ : i32 = 'q' as i32;
pub const KEY_LOWJ : i32 = 'j' as i32;
pub const KEY_LOWF : i32 = 'f' as i32;
pub const KEY_CAPY : i32 = 'Y' as i32;
pub const KEY_CAPH : i32 = 'H' as i32;
pub const KEY_CAPL : i32 = 'L' as i32;
pub const KEY_CAPE : i32 = 'E' as i32;
pub const KEY_CAPD : i32 = 'D' as i32;
pub const KEY_CAPI : i32 = 'I' as i32;
pub const KEY_CAPK : i32 = 'K' as i32;
pub const KEY_CAPJ : i32 = 'J' as i32;
pub const KEY_DOT  : i32 = '.' as i32;
pub const KEY_COMMA   : i32 = ',' as i32;
pub const KEY_HELP    : i32 = '?' as i32;
pub const KEY_GOTO : i32 = 'G' as i32;
pub const KEY_DESCEND : i32 = '>' as i32;


