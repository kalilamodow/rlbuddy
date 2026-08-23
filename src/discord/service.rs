use super::rpc::{PresenceData, RichPresenceController};
use crate::{
    common::{ReadWriteStateHandle, ReadonlyStateHandle, eventsource::EventReceiver},
    matches::MatchesServiceState,
    rocket_league::{Playlist, Team},
    stats_api::{MatchState, RLEvent},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchData {
    pub team_score: u8,
    pub opp_score: u8,
    pub playlist: Playlist,
    pub arena: &'static str,
    pub state: MatchState,
}

impl MatchData {
    pub fn generate_presence(&self, include_score: bool) -> PresenceData {
        let mut details = format!("{} in {}", self.playlist, self.arena);
        let mut state: Option<String> = None;

        if !self.playlist.is_singleplayer() && include_score {
            details += format!(" | {}-{}", self.team_score, self.opp_score).as_str();
            state = Some(self.state.as_str().to_owned());
        }

        PresenceData { details, state }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameState {
    Lobby,
    InGame(MatchData),
}

impl GameState {
    fn to_presence(&self, show_score: bool) -> PresenceData {
        match &self {
            GameState::Lobby => PresenceData {
                details: "Main menu".to_owned(),
                state: None,
            },
            GameState::InGame(game) => game.generate_presence(show_score),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DiscordSettings {
    pub disable: bool,
    pub hide_score: bool,
}

#[derive(Debug, Default, Clone)]
pub struct DiscordServiceState {
    pub connected: bool,
    pub busy: bool,
}

pub struct DiscordService {
    state: ReadWriteStateHandle<DiscordServiceState>,
    settings: ReadWriteStateHandle<DiscordSettings>,
    controller: RichPresenceController,
    current: GameState,
    matches_handle: ReadonlyStateHandle<MatchesServiceState>,
    stats_api: EventReceiver<RLEvent>,
    is_rl_open: bool,
}

impl DiscordService {
    pub fn new(
        settings: DiscordSettings,
        matches_handle: ReadonlyStateHandle<MatchesServiceState>,
        stats_api: EventReceiver<RLEvent>,
    ) -> Self {
        DiscordService {
            state: ReadWriteStateHandle::new(DiscordServiceState::default()),
            settings: ReadWriteStateHandle::new(settings),
            controller: RichPresenceController::new(),
            current: GameState::Lobby,
            matches_handle,
            stats_api,
            is_rl_open: false,
        }
    }

    pub fn update(&mut self) {
        while let Some(event) = self.stats_api.try_recv() {
            match event.as_ref() {
                RLEvent::Connected => self.is_rl_open = true,
                RLEvent::Disconnected => self.is_rl_open = false,
                _ => {}
            }
        }

        self.current = if let Some(current_match) = &self.matches_handle.read().current_match {
            let (our_score, their_score) = match current_match.our_team {
                Team::Blue => (current_match.score.blue, current_match.score.orange),
                Team::Orange => (current_match.score.orange, current_match.score.blue),
            };

            GameState::InGame(MatchData {
                team_score: our_score,
                opp_score: their_score,
                playlist: current_match.playlist,
                arena: current_match.arena,
                state: current_match.state.clone(),
            })
        } else {
            GameState::Lobby
        };

        self.send_current();
        self.update_state();
    }

    fn send_current(&mut self) {
        let settings = self.settings.read();

        if settings.disable || !self.is_rl_open {
            self.controller.ensure_disconnected();
            return;
        }
        self.controller.ensure_connected();

        let presence = self.current.to_presence(!settings.hide_score);
        self.controller.set_presence(presence);
    }

    fn update_state(&self) {
        let mut state = self.state.write();
        state.connected = self.controller.is_connected();
        state.busy = self.controller.is_busy();
    }

    pub fn state_handle(&self) -> ReadonlyStateHandle<DiscordServiceState> {
        ReadonlyStateHandle::over(&self.state)
    }

    pub fn settings_handle(&self) -> ReadWriteStateHandle<DiscordSettings> {
        ReadWriteStateHandle::clone(&self.settings)
    }
}
