//! CLI argument types

use clap::ValueEnum;

/// Bump type for version release
#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum BumpType {
    /// Major version bump (e.g. 1.0.0 -> 2.0.0)
    #[value(alias = "ma")]
    Major,
    /// Minor version bump (e.g. 1.0.0 -> 1.1.0)
    #[value(alias = "mi")]
    Minor,
    /// Patch version bump (e.g. 1.0.0 -> 1.0.1)
    #[value(alias = "pa")]
    Patch,
}

impl BumpType {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            BumpType::Major => "major",
            BumpType::Minor => "minor",
            BumpType::Patch => "patch",
        }
    }
}
