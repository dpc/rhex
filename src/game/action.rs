use hex2d::{Angle, Coordinate, Left, Right, Forward};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Action {
    Wait,
    Turn(Angle),
    Move(Angle),
    Charge,
    Spin(Angle),
    Equip(char),
    Drop_(char),
    Ranged(Coordinate),
    Pick,
    Descend,
}

