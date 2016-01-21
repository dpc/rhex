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
    Fire(Coordinate),
    Pick,
    Descend,
}

impl Action {
    pub fn could_be_attack(&self) -> bool {
        match *self {
            Action::Charge => true,
            Action::Move(angle) => {
                match angle {
                    Left | Right | Forward => true,
                    _ => false,
                }
            }
            _ => false,
        }
    }
}
