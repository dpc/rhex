use hex2d::{Angle, Coordinate};

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

