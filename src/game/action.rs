use hex2d::{Angle, Coordinate};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Action {
    Wait,
    Turn(Angle),
    /// Step without turning; Left/Right strafe and bump terrain, but never attack actors.
    Move(Angle),
    Charge,
    Equip(char),
    Drop_(char),
    Ranged(Coordinate),
    Pick,
    /// Pick up the front item, otherwise toggle the unoccupied door in front.
    Interact,
    Descend,
}
