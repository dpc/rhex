use super::actor;

/// A visible enemy's transition from unaware to aware of the player.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AwarenessEvent {
    /// Stable identity lets presentation follow the enemy, never a replacement occupant.
    pub actor: actor::Id,
}
