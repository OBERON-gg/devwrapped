#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used)]

pub mod agents;
mod awards;
mod combine;
pub mod git;
mod languages;
mod lines;
mod range;
mod sample;
pub mod shell;
mod stats;
#[cfg(test)]
mod test_repo;

pub use awards::{AWARDS_SHOWN, Award, AwardMeasures, Holiday, award_measures, awards};
pub use combine::{Sources, combine};
pub use languages::language;
pub use range::Range;
pub use sample::sample;
pub use stats::{Activity, AgentStats, GitStats, ShellStats, Stats, Tally};
