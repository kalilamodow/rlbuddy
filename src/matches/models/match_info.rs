use std::{sync::Arc, time::SystemTime};

use crate::{
    matches::apis::{NameAPI, PlayerSkillInformation},
    rocket_league::{Platform, Playlist, Team},
    stats_api::{MatchState, PlayerData, TeamScores},
};

#[derive(Debug, Clone)]
pub struct MatchPlayer {
    pub left: bool,
    pub uncensored_name: Option<Arc<String>>,
    pub epic_name: Option<Arc<String>>,
    pub data: PlayerData,
    pub skill: Option<Arc<PlayerSkillInformation>>,
    pub is_local_player: bool,
}

impl MatchPlayer {
    pub fn uncensor_with(&mut self, api: &NameAPI) {
        self.uncensored_name = api.get(&self.data.platform_id);
    }

    pub fn display_name(&self) -> &str {
        // unwrap or else gives a error idk why
        match &self.uncensored_name {
            Some(name) => name,
            None => self.data.name.as_str(),
        }
    }

    pub fn trn_link(&self) -> Option<String> {
        let (prefix, id) = self.epic_name.as_ref().map_or(
            match self.data.platform {
                Platform::Bot => return None,
                Platform::Epic => ("epic", self.display_name()),
                Platform::Switch => ("switch", self.display_name()),
                Platform::PlayStation => ("psn", self.display_name()),
                Platform::Xbox => ("xbl", self.display_name()),
                Platform::Steam => ("steam", self.data.platform_id.split('|').nth(1).unwrap()),
            },
            |n| ("epic", n.as_str()),
        );

        Some(format!(
            "https://rocketleague.tracker.network/rocket-league/profile/{prefix}/{id}/overview"
        ))
    }
}

#[derive(Debug, Clone)]
pub struct MatchOverInfo {
    pub timestamp: SystemTime,
    pub winner: Option<Team>,
}

#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub players: Vec<MatchPlayer>,
    pub score: TeamScores,
    pub our_team: Team,
    pub finish: Option<MatchOverInfo>,
    pub started_at: SystemTime,
    pub max_active_players: usize,
    pub arena: Option<&'static str>,
    pub playlist: Option<Playlist>,
    pub state: MatchState,
}

impl Default for MatchInfo {
    fn default() -> Self {
        MatchInfo {
            players: Vec::new(),
            score: TeamScores { blue: 0, orange: 0 },
            our_team: Team::Blue,
            finish: None,
            started_at: SystemTime::now(),
            max_active_players: 0,
            arena: None,
            playlist: None,
            state: MatchState::Game,
        }
    }
}
