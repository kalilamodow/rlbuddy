use crate::rocket_league::get_rl_exe_path;
use num_enum::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs,
    io::{self, Seek},
    sync::LazyLock,
};

#[derive(Debug, Deserialize)]
struct PsynetPlaylist {
    #[serde(rename = "PlaylistID")]
    id: u8,
    #[serde(rename = "Title")]
    name: String,
}

// to load up-to-date info!
static PLAYLISTS_FROM_GAME: LazyLock<Option<Vec<PsynetPlaylist>>> = LazyLock::new(|| {
    let rl_exe_path = get_rl_exe_path()?;
    let cache_file = rl_exe_path
        .parent()?
        .join("../../TAGame/Cache/WebCache/")
        .join("L3YyL0NvbmZpZy9CYXR0bGVDYXJzLy0xODg3Njk0MDgzL1Byb2QvRXBpYy9JTlQv"); // cache file

    let mut cache_file = fs::File::open(cache_file).ok()?;
    cache_file.seek(io::SeekFrom::Start(55)).unwrap();
    let mut de = serde_json::Deserializer::from_reader(cache_file);
    let items: Vec<PsynetPlaylist> = serde_json::Map::deserialize(&mut de)
        .unwrap()
        .into_values()
        .filter(|obj| obj["Class"] == "PlaylistSettings_TA")
        .map(serde_json::from_value)
        .filter_map(Result::ok)
        .collect();

    Some(items)
});

// stuff that's worth hardcoding (comp playlists, stuff that the online config doesnt say)
#[derive(
    Debug, Copy, Clone, PartialOrd, Ord, FromPrimitive, PartialEq, Eq, Serialize, Deserialize, Hash,
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
    BasketballDoubles = 17,
    Rumble = 18,
    Workshop = 19,
    CustomTrainingEditor = 20,
    CustomTraining = 21,
    Breakout = 23,
    LocalMatch = 24,
    RankedBasketballDoubles = 27,
    RankedRumble = 28,
    RankedBreakout = 29,
    RankedSnowDay = 30,
    HauntedBall = 31,
    RumShot = 37,
    GodBall = 38,
    BoomerBall = 41,
    GodBallDoubles = 43,
    SpecialSnowDay = 44,
    TacticalRumble = 48,
    RumbleBM = 52,
    RankedQuads = 61,
    RankedHeatseekerDoubles = 63,
    GodBallSpooky = 64,
    GodBallHaunted = 65,
    GodBallRicochet = 66,
    RumShotDoubles = 70,
    OnlineFreeplay = 73,
    TerritoryDoubles = 74,
    GodballTerritory = 75,
    GodballTerritoryDoubles = 76,
    NonStandardSoccar = 77,
    NonStandardSoccarDoubles = 78,
    SnowdayTerritory = 79,
    #[num_enum(catch_all)]
    Other(u8),
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
            Self::RankedHeatseekerDoubles | Self::GodBall | Self::GodBallDoubles => "Heatseeker",
            Self::HauntedBall => "Ghost Hunt",
            Self::GodBallSpooky | Self::GodBallHaunted => "Haunted Heatseeker",
            Self::GodBallRicochet => "Heatseeker Ricochet",
            Self::RumShot | Self::RumShotDoubles => "Dropshot Rumble",
            Self::BoomerBall => "Boomer Ball",
            Self::SpecialSnowDay => "Winter Breakaway",
            Self::TacticalRumble => "Tactical Rumble",
            Self::RumbleBM => "Gotham City Rumble",
            Self::TerritoryDoubles => "Split Shot Doubles",
            Self::GodballTerritory => "Split Shot Heatseeker",
            Self::GodballTerritoryDoubles => "Split Shot Heatseeker Doubles",
            Self::OnlineFreeplay => "Online Free Play",
            Self::NonStandardSoccar => "Non-Standard Soccar",
            Self::NonStandardSoccarDoubles => "Non-Standard Soccar Doubles",
            Self::SnowdayTerritory => "Split Shot Snow Day",
            Self::PrivateMatch => "Private Match",
            Self::Season => "Season Match",
            Self::Exhibition => "Exhibition Match",
            Self::Training => "Training",
            Self::Workshop => "Workshop Map",
            Self::CustomTraining => "Custom Training",
            Self::CustomTrainingEditor => "Editing Custom Training",
            Self::LocalMatch => "Local Match",
            Self::Other(id) => PLAYLISTS_FROM_GAME
                .as_ref()
                .and_then(|ps| ps.iter().find(|p| p.id == id).map(|p| p.name.as_str()))
                .unwrap_or("Unknown"),
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
