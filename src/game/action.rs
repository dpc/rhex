use hex2d::{Angle, Coordinate};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Action {
    Wait,
    Turn(Angle),
    /// Step relative to facing without turning; Left/Right strafe front-left/right.
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
