use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::collections::VecDeque;
use std::{self, cmp, env};
use std::io::Write;
use core::str::StrExt;
use ncurses as nc;

use num::integer::Integer;

use util::circular_move;

use super::{Action, AutoMoveType};
use game;
use game::area;
use actor::{self, Race, Slot};
use ui;
use item;

use hex2d::{Angle, IntegerSpacing, Coordinate, ToCoordinate, Position};

use game::tile;

use std::fmt;
use std::fmt::Write as FmtWrite;



impl CursesUI {
    
}

impl ui::UiFrontend for CursesUI {



}

