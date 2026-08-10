use std::{sync::Arc, time::SystemTime};

use crate::{
    matches::apis::{EpicIdAPI, NameAPI, PlayerSkillInformation, RankAPI},
    rocket_league::{Platform, Playlist, Team},
    stats_api::{MatchState, MatchUpdate, PlayerData, TeamScores},
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
    pub arena: &'static str,
    pub playlist: Playlist,
    pub state: MatchState,
}

impl MatchInfo {
    pub fn new(mut update: MatchUpdate, local_player_id: &Option<String>) -> Self {
        normalize_bot_player_ids(&mut update.players);
        Self {
            score: update.score,
            our_team: update
                .players
                .iter()
                .find(|p| Some(&p.platform_id) == local_player_id.as_ref())
                .map(|p| p.team)
                .unwrap_or(Team::Blue),
            finish: None,
            started_at: SystemTime::now(),
            max_active_players: update.players.len(),
            arena: update.arena,
            playlist: update.playlist,
            state: update.state,
            players: update
                .players
                .into_iter()
                .map(|p| MatchPlayer {
                    is_local_player: Some(&p.platform_id) == local_player_id.as_ref(),
                    left: false,
                    uncensored_name: None,
                    epic_name: None,
                    skill: None,
                    data: p,
                })
                .collect(),
        }
    }

    pub fn update(
        &mut self,
        mut updated: MatchUpdate,
        local_player_id: &Option<String>,
        rank_api: &RankAPI,
        epic_id_api: &EpicIdAPI,
        name_api: &NameAPI,
    ) {
        self.state = updated.state;
        self.max_active_players = self.max_active_players.max(updated.players.len());
        self.score = updated.score;

        normalize_bot_player_ids(&mut updated.players);
        for player in &mut self.players {
            let updated_pos = updated
                .players
                .iter()
                .position(|p| p.platform_id == player.data.platform_id);
            if let Some(updated_pos) = updated_pos {
                let updated = updated.players.swap_remove(updated_pos);
                player.data = updated;
                player.left = false;
            } else {
                player.left = true;
            }
        }

        for remaining_player in updated.players {
            self.players.push(MatchPlayer {
                is_local_player: Some(&remaining_player.platform_id) == local_player_id.as_ref(),
                left: false,
                uncensored_name: None,
                epic_name: None,
                skill: None,
                data: remaining_player,
            });
        }

        self.update_ranks(rank_api);
        self.update_epic_ids(epic_id_api);
        self.uncensor_names(name_api);

        self.our_team = self
            .players
            .iter()
            .find(|p| p.is_local_player)
            .map_or(Team::Blue, |p| p.data.team);
        self.players.sort_by_key(|p| p.data.team != self.our_team);
    }

    fn on_each_player<F: Fn(&mut MatchPlayer), C: Fn(&MatchPlayer) -> bool>(
        &mut self,
        func: F,
        check: C,
    ) {
        for player in &mut self.players {
            if check(player) {
                func(player);
            }
        }
    }

    pub fn update_ranks(&mut self, api: &RankAPI) {
        self.on_each_player(
            |p| p.skill = api.get(&p.data.platform_id),
            |c| c.data.platform != Platform::Bot,
        );
    }

    pub fn update_epic_ids(&mut self, api: &EpicIdAPI) {
        self.on_each_player(
            |p| p.epic_name = api.get(&p.data.platform_id),
            |c| matches!(c.data.platform, Platform::Switch),
        );
    }

    pub fn uncensor_names(&mut self, api: &NameAPI) {
        self.on_each_player(
            |p| p.uncensored_name = api.get(&p.data.platform_id),
            |c| is_censored(c.display_name()),
        );
    }
}

fn is_censored(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c == '*')
}

fn normalize_bot_player_ids(players: &mut [PlayerData]) {
    // bots all share the same id so replace it for comparisons
    for player_or_bot_hmm in players {
        if player_or_bot_hmm.platform == Platform::Bot {
            player_or_bot_hmm.platform_id = player_or_bot_hmm.name.clone();
        }
    }
}
