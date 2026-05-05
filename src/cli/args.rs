use clap::ValueEnum;

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum BumpType {
    #[value(alias = "ma")]
    Major,
    #[value(alias = "mi")]
    Minor,
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
