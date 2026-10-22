use hex2d::Coordinate;

/// Immutable geometry of an observed, resolved hit, independent of actor lifetime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatEvent {
    /// Attacker position when the attack resolved.
    pub source: Coordinate,
    /// Struck position when the attack resolved.
    pub impact: Coordinate,
}
