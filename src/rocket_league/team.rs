use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Team {
    Blue,
    Orange,
}

impl From<u8> for Team {
    fn from(value: u8) -> Self {
        match value {
            0 => Team::Blue,
            1 => Team::Orange,
            _ => unreachable!("invalid team {}", value),
        }
    }
}

impl fmt::Display for Team {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Team::Blue => "Blue",
                Team::Orange => "Orange",
            }
        )
    }
}
