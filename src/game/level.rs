use hex2d::Coordinate;

/// Stable identity of a retained location, independent of its difficulty.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct LevelId(pub usize);

/// Environment used to generate and describe a location.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Biome {
    Dungeon,
    DarkForest,
}

impl Biome {
    /// Player-facing environment name.
    pub fn description(self) -> &'static str {
        match self {
            Self::Dungeon => "Dungeon",
            Self::DarkForest => "Dark Forest",
        }
    }

    /// Default progression alternates enclosed dungeons and outdoor forests.
    pub fn next(self) -> Self {
        match self {
            Self::Dungeon => Self::DarkForest,
            Self::DarkForest => Self::Dungeon,
        }
    }
}

/// One half of a reciprocal connection, keyed by its source tile.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ExitLink {
    /// Retained destination location.
    pub destination: LevelId,
    /// Destination's matching exit, preferred for arrival.
    pub arrival: Coordinate,
}
