//! Per-particle schema: the columns every backend exposes for every particle.

use std::fmt;

/// A queryable per-particle quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ParticleField {
    Id,
    /// AoSoA tile holding the particle (equals `id` for non-tiled layouts).
    Tile,
    /// SIMD lane within the tile.
    Lane,
    X,
    Y,
    Z,
    Vx,
    Vy,
    Vz,
    Speed,
    /// Cylindrical radius `sqrt(x² + y²)`.
    Radius,
    Mass,
    Kinetic,
    Potential,
    Energy,
    /// Angular momentum about the z axis.
    Lz,
    /// Normalized activity state in `0..=1`.
    Activity,
}

/// Number of [`ParticleField`]s; the width of a [`ParticleRow`].
pub const FIELD_COUNT: usize = 17;

/// One particle's values, indexed by [`ParticleField::index`].
pub type ParticleRow = [f64; FIELD_COUNT];

impl ParticleField {
    pub const ALL: [Self; FIELD_COUNT] = [
        Self::Id,
        Self::Tile,
        Self::Lane,
        Self::X,
        Self::Y,
        Self::Z,
        Self::Vx,
        Self::Vy,
        Self::Vz,
        Self::Speed,
        Self::Radius,
        Self::Mass,
        Self::Kinetic,
        Self::Potential,
        Self::Energy,
        Self::Lz,
        Self::Activity,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// Canonical query-language name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::Tile => "tile",
            Self::Lane => "lane",
            Self::X => "x",
            Self::Y => "y",
            Self::Z => "z",
            Self::Vx => "vx",
            Self::Vy => "vy",
            Self::Vz => "vz",
            Self::Speed => "speed",
            Self::Radius => "r",
            Self::Mass => "mass",
            Self::Kinetic => "ke",
            Self::Potential => "pe",
            Self::Energy => "energy",
            Self::Lz => "lz",
            Self::Activity => "activity",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Id => "particle index",
            Self::Tile => "AoSoA tile index",
            Self::Lane => "SIMD lane in tile",
            Self::X => "position x",
            Self::Y => "position y",
            Self::Z => "position z",
            Self::Vx => "velocity x",
            Self::Vy => "velocity y",
            Self::Vz => "velocity z",
            Self::Speed => "|v|",
            Self::Radius => "cylindrical radius",
            Self::Mass => "mass",
            Self::Kinetic => "kinetic energy ½m|v|²",
            Self::Potential => "potential energy mΦ",
            Self::Energy => "total energy ke + pe",
            Self::Lz => "angular momentum (z)",
            Self::Activity => "activity state 0..1",
        }
    }

    /// Accepts canonical names and common aliases, case-insensitively.
    pub fn parse(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();
        let field = match lower.as_str() {
            "radius" => Self::Radius,
            "m" => Self::Mass,
            "kinetic" => Self::Kinetic,
            "potential" => Self::Potential,
            "e" => Self::Energy,
            "act" => Self::Activity,
            other => *Self::ALL.iter().find(|f| f.name() == other)?,
        };
        Some(field)
    }

    /// Whether values are whole numbers (affects formatting only).
    pub fn is_integer(self) -> bool {
        matches!(self, Self::Id | Self::Tile | Self::Lane)
    }
}

impl fmt::Display for ParticleField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Read access to every particle of a live state, implemented by backends.
///
/// Implementations must present one consistent state for the lifetime of the
/// borrow so every view computed from it describes the same tick.
pub trait ParticleSource {
    fn particle_count(&self) -> u64;

    fn tick(&self) -> u64;

    fn sim_time(&self) -> f64;

    /// Writes all fields of particle `index` (`< particle_count`) into `row`.
    fn read(&self, index: u64, row: &mut ParticleRow);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_match_order() {
        for (i, f) in ParticleField::ALL.iter().enumerate() {
            assert_eq!(f.index(), i);
        }
    }

    #[test]
    fn names_round_trip_and_aliases_resolve() {
        for f in ParticleField::ALL {
            assert_eq!(ParticleField::parse(f.name()), Some(f));
        }
        assert_eq!(ParticleField::parse("RADIUS"), Some(ParticleField::Radius));
        assert_eq!(ParticleField::parse("act"), Some(ParticleField::Activity));
        assert_eq!(ParticleField::parse("nope"), None);
    }
}
