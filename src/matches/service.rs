use super::apis::{NameAPI, RankAPI};
use super::{MatchInfo, MatchOverInfo};
use crate::common::eventsource::EventReceiver;
use crate::common::{ReadWriteStateHandle, ReadonlyStateHandle};
use crate::matches::apis::{EpicIdAPI, new_epic_id_api, new_name_api, new_rank_api};
use crate::stats_api::RLEvent;
use eframe::egui;
use std::time::SystemTime;

#[derive(Debug, Default)]
pub struct MatchesServiceState {
    pub current_match: Option<MatchInfo>,
    pub prev_matches: Vec<MatchInfo>,
}

pub struct MatchesService {
    state: ReadWriteStateHandle<MatchesServiceState>,
    stats_api: EventReceiver<RLEvent>,
    ctx: egui::Context,
    local_player_id: Option<String>,
    rank_api: RankAPI,
    names_api: NameAPI,
    epic_ids_api: EpicIdAPI,
}

impl MatchesService {
    pub fn new(ctx: &egui::Context, stats_api: EventReceiver<RLEvent>) -> Self {
        MatchesService {
            state: ReadWriteStateHandle::default(),
            stats_api,
            ctx: ctx.clone(),
            local_player_id: None,
            rank_api: new_rank_api(ctx.clone()),
            names_api: new_name_api(ctx.clone()),
            epic_ids_api: new_epic_id_api(ctx.clone()),
        }
    }

    pub fn state_handle(&self) -> ReadonlyStateHandle<MatchesServiceState> {
        ReadonlyStateHandle::over(&self.state)
    }

    pub fn update(&mut self) {
        while let Some(event) = self.stats_api.try_recv() {
            let mut state = self.state.write();
            match *event {
                RLEvent::MatchOver(winner) => {
                    if let Some(current_match) = state.current_match.as_mut() {
                        if current_match.players.len() <= 1 {
                            return;
                        }

                        current_match.finish = Some(MatchOverInfo {
                            timestamp: SystemTime::now(),
                            winner: Some(winner),
                        });
                    }
                    self.ctx.request_repaint();
                }
                RLEvent::MatchLeft => {
                    let Some(mut current_match) = state.current_match.take() else {
                        return;
                    };

                    self.rank_api
                        .invalidate(current_match.players.iter().map(|p| &p.data.platform_id));

                    if current_match.playlist.is_singleplayer() {
                        return;
                    }

                    if current_match.finish.is_none() {
                        current_match.finish = Some(MatchOverInfo {
                            timestamp: SystemTime::now(),
                            winner: current_match.score.guess_winner(),
                        });
                    }

                    state.prev_matches.push(current_match);
                    self.ctx.request_repaint();
                }
                RLEvent::Update(ref update) => {
                    if let Some(current_match) = state.current_match.as_mut() {
                        current_match.update(
                            update.clone(),
                            &self.local_player_id,
                            &self.rank_api,
                            &self.epic_ids_api,
                            &self.names_api,
                        );
                    } else {
                        state.current_match =
                            Some(MatchInfo::new(update.clone(), &self.local_player_id));
                    }

                    drop(state);
                    self.ctx.request_repaint();
                }
                RLEvent::OurPlayerId(ref id) => {
                    self.local_player_id = Some(id.clone());
                }
                _ => {}
            }
        }
    }
}
