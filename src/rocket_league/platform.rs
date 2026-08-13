use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Epic,
    Steam,
    Xbox,
    PlayStation,
    Switch,
    Bot,
}

#[derive(Debug)]
pub struct UnknownPlatform;

impl FromStr for Platform {
    type Err = UnknownPlatform;
    fn from_str(s: &str) -> Result<Platform, Self::Err> {
        match s {
            "Epic" => Ok(Platform::Epic),
            "Steam" => Ok(Platform::Steam),
            "XboxOne" => Ok(Platform::Xbox),
            "PS4" => Ok(Platform::PlayStation),
            "Switch" => Ok(Platform::Switch),
            "Unknown" => Ok(Platform::Bot),
            _ => Err(UnknownPlatform),
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Platform::Epic => "Epic",
                Platform::Steam => "Steam",
                Platform::PlayStation => "PlayStation",
                Platform::Xbox => "Xbox",
                Platform::Switch => "Switch",
                Platform::Bot => "Bot",
            }
        )
    }
}
