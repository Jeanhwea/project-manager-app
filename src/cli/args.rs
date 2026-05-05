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
