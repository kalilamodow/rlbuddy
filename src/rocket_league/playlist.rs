use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::fmt;

// combination of data scraped from PsyNet and https://bakkesplugins.com/wiki/bakkesmod-sdk/code-snippets/playlist-id
#[derive(
    Debug,
    Copy,
    Clone,
    PartialOrd,
    Ord,
    TryFromPrimitive,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Hash,
)]
#[repr(u8)]
pub enum Playlist {
    Casual = 0,
    Duel = 1,
    Doubles = 2,
    Standard = 3,
    Quads = 4,
    PrivateMatch = 6,
    Season = 7,
    Exhibition = 8,
    Training = 9,
    RankedSoloDuel = 10,
    RankedTeamDoubles = 11,
    RankedStandard = 13,
    SnowDayPromotion = 15,
    Experimental = 16,
    BasketballDoubles = 17,
    Rumble = 18,
    Workshop = 19,
    CustomTrainingEditor = 20,
    CustomTraining = 21,
    Tournament = 22,
    Breakout = 23,
    LocalMatch = 24,
    FaceIt = 26,
    RankedBasketballDoubles = 27,
    RankedRumble = 28,
    RankedBreakout = 29,
    RankedSnowDay = 30,
    HauntedBall = 31,
    BeachBall = 32,
    Rugby = 33,
    AutoTournament = 34,
    RocketLabs = 35,
    RumShot = 37,
    GodBall = 38,
    BoomerBall = 41,
    GodBallDoubles = 43,
    SpecialSnowDay = 44,
    Football = 46,
    Cubic = 47,
    TacticalRumble = 48,
    SpringLoaded = 49,
    SpeedDemon = 50,
    RumbleBM = 52,
    Knockout = 54,
    ThirdWheel = 55,
    RankedQuads = 61,
    MagnusFutball = 62,
    RankedHeatseekerDoubles = 63,
    GodBallSpooky = 64,
    GodBallHaunted = 65,
    GodBallRicochet = 66,
    CubicSpooky = 67,
    GForceFrenzy = 68,
    RumShotDoubles = 70,
    Territory = 72,
    OnlineFreeplay = 73,
    TerritoryDoubles = 74,
    GodballTerritory = 75,
    GodballTerritoryDoubles = 76,
    NonStandardSoccar = 77,
    NonStandardSoccarDoubles = 78,
    SnowdayTerritory = 79,
    RunItBack = 80,
    CarWars = 81,
    PizzaParty = 82,
    PushThePuck = 83,
    Possession = 84,
    FCShowdown = 86,
    Sacrifice = 87,
    JumpJam = 88,
    SonicRush = 89,
    UpToNoGood = 90,
    ProjectAIM = 91,
}

impl Playlist {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Casual => "Casual",
            Self::Standard | Self::RankedStandard => "Standard",
            Self::Doubles | Self::RankedTeamDoubles => "Doubles",
            Self::Duel | Self::RankedSoloDuel => "Duel",
            Self::Quads | Self::RankedQuads => "Quads",
            Self::Breakout | Self::RankedBreakout => "Dropshot",
            Self::Rumble | Self::RankedRumble => "Rumble",
            Self::RankedSnowDay | Self::SnowDayPromotion => "Snow Day",
            Self::BasketballDoubles | Self::RankedBasketballDoubles => "Hoops",
            Self::Experimental | Self::RocketLabs => "Rocket Labs",
            Self::Tournament | Self::AutoTournament => "Tournament Match",
            Self::FaceIt => "External Match",
            Self::RankedHeatseekerDoubles | Self::GodBall | Self::GodBallDoubles => "Heatseeker",
            Self::HauntedBall => "Ghost Hunt",
            Self::BeachBall => "Beach Ball",
            Self::Rugby => "Spike Rush",
            Self::GodBallSpooky | Self::GodBallHaunted => "Haunted Heatseeker",
            Self::GodBallRicochet => "Heatseeker Ricochet",
            Self::RumShot | Self::RumShotDoubles => "Dropshot Rumble",
            Self::BoomerBall => "Boomer Ball",
            Self::SpecialSnowDay => "Winter Breakaway",
            Self::Football => "Gridiron",
            Self::Cubic => "Super Cube",
            Self::CubicSpooky => "Spooky Cube",
            Self::TacticalRumble => "Tactical Rumble",
            Self::SpringLoaded => "Spring Loaded",
            Self::SpeedDemon => "Speed Demon",
            Self::RumbleBM => "Gotham City Rumble",
            Self::Knockout => "Knockout",
            Self::ThirdWheel => "confidential_thirdwheel_test",
            Self::MagnusFutball => "Nike FC Showdown",
            Self::GForceFrenzy => "G-Force Frenzy",
            Self::Territory => "Split Shot",
            Self::TerritoryDoubles => "Split Shot Doubles",
            Self::GodballTerritory => "Split Shot Heatseeker",
            Self::GodballTerritoryDoubles => "Split Shot Heatseeker Doubles",
            Self::OnlineFreeplay => "Online Free Play",
            Self::NonStandardSoccar => "Non-Standard Soccar",
            Self::NonStandardSoccarDoubles => "Non-Standard Soccar Doubles",
            Self::SnowdayTerritory => "Split Shot Snow Day",
            Self::RunItBack => "Run It Back",
            Self::CarWars => "Spike Drop",
            Self::PizzaParty => "Pizza Party",
            Self::PushThePuck => "Push The Puck",
            Self::Possession => "Possession Rumble",
            Self::FCShowdown => "FIFA Soccar Strike",
            Self::Sacrifice => "Demolition Duel",
            Self::JumpJam => "Jump Jam",
            Self::SonicRush => "Sonic Spin",
            Self::UpToNoGood => "Up To No Good",
            Self::ProjectAIM => "FREE AERIALS *not clickbait*",
            Self::PrivateMatch => "Private Match",
            Self::Season => "Season Match",
            Self::Exhibition => "Exhibition Match",
            Self::Training => "Training",
            Self::Workshop => "Workshop Map",
            Self::CustomTraining => "Custom Training",
            Self::CustomTrainingEditor => "Editing Custom Training",
            Self::LocalMatch => "Local Match",
        }
    }

    pub fn is_singleplayer(self) -> bool {
        matches!(
            self,
            Self::Training | Self::Workshop | Self::CustomTraining | Self::CustomTrainingEditor
        )
    }

    pub fn infer_from_player_count(match_player_count: usize) -> Option<Self> {
        Some(match match_player_count {
            1 => Self::Training,
            2 => Self::Duel,
            3 | 4 => Self::RankedTeamDoubles,
            5 | 6 => Self::RankedStandard,
            7 | 8 => Self::RankedQuads,
            _ => return None,
        })
    }

    pub fn in_ranked(self) -> Option<Self> {
        Some(match self {
            Self::Duel | Self::RankedSoloDuel => Self::RankedSoloDuel,
            Self::Doubles | Self::RankedTeamDoubles => Self::RankedTeamDoubles,
            Self::Standard | Self::RankedStandard => Self::RankedStandard,
            Self::Quads | Self::RankedQuads => Self::RankedQuads,
            Self::SnowDayPromotion
            | Self::SnowdayTerritory
            | Self::SpecialSnowDay
            | Self::RankedSnowDay => Self::RankedSnowDay,
            Self::BasketballDoubles | Self::RankedBasketballDoubles => {
                Self::RankedBasketballDoubles
            }
            Self::Rumble | Self::RumbleBM | Self::TacticalRumble | Self::RankedRumble => {
                Self::RankedRumble
            }
            Self::Breakout | Self::RankedBreakout => Self::RankedBreakout,
            Self::GodBall
            | Self::GodBallHaunted
            | Self::GodballTerritory
            | Self::GodballTerritoryDoubles
            | Self::GodBallDoubles
            | Self::GodBallRicochet
            | Self::GodBallSpooky
            | Self::RankedHeatseekerDoubles => Self::RankedHeatseekerDoubles,
            _ => return None,
        })
    }
}

impl fmt::Display for Playlist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
