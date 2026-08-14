use super::apis::{NameAPI, RankAPI};
use super::{MatchInfo, MatchOverInfo};
use crate::common::eventsource::EventReceiver;
use crate::common::{ReadWriteStateHandle, ReadonlyStateHandle};
use crate::matches::StrippedMatchInfo;
use crate::matches::apis::{EpicIdAPI, new_epic_id_api, new_name_api, new_rank_api};
use crate::rocket_league::{Playlist, Team};
use crate::stats_api::{RLEvent, TeamScores};
use eframe::egui;
use std::borrow::Cow;
use std::time::SystemTime;

#[derive(Debug)]
pub enum MatchType<'a> {
    Session(Cow<'a, MatchInfo>),
    Old(Cow<'a, StrippedMatchInfo>),
}

impl MatchType<'_> {
    pub fn playlist(&self) -> Playlist {
        match self {
            Self::Old(o) => o.playlist,
            Self::Session(s) => s.playlist,
        }
    }

    pub fn started_at(&self) -> SystemTime {
        match self {
            Self::Old(o) => o.start_time,
            Self::Session(s) => s.started_at,
        }
    }

    pub fn score(&self) -> &TeamScores {
        match self {
            Self::Old(o) => &o.score,
            Self::Session(s) => &s.score,
        }
    }

    pub fn our_team(&self) -> Team {
        match self {
            Self::Old(o) => o.our_team(),
            Self::Session(s) => s.our_team,
        }
    }

    pub fn is_over(&self) -> bool {
        match self {
            Self::Old(_) => true,
            Self::Session(s) => s.finish.is_some(),
        }
    }

    pub fn player_qty(&self) -> usize {
        match self {
            Self::Old(o) => o.players.len(),
            Self::Session(s) => s.players.len(),
        }
    }
}

#[derive(Debug)]
pub struct MatchesServiceState {
    pub current_match: Option<MatchInfo>,
    pub prev_matches: Vec<MatchType<'static>>,
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
    pub fn new(
        ctx: &egui::Context,
        stats_api: EventReceiver<RLEvent>,
        prev_matches: Vec<StrippedMatchInfo>,
    ) -> Self {
        MatchesService {
            state: ReadWriteStateHandle::new(MatchesServiceState {
                current_match: None,
                prev_matches: prev_matches
                    .into_iter()
                    .map(|m| MatchType::Old(Cow::Owned(m)))
                    .collect(),
            }),
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

                    state
                        .prev_matches
                        .push(MatchType::Session(Cow::Owned(current_match)));
                    self.ctx.request_repaint();
                }
                RLEvent::Update(ref update) => {
                    if let Some(current_match) = state.current_match.as_mut() {
                        current_match.update(
                            update.clone(),
                            self.local_player_id.as_ref(),
                            &self.rank_api,
                            &self.epic_ids_api,
                            &self.names_api,
                        );
                    } else {
                        state.current_match = Some(MatchInfo::new(
                            update.clone(),
                            self.local_player_id.as_ref(),
                        ));
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

    pub fn stripped_history(&self) -> Vec<StrippedMatchInfo> {
        self.state
            .read()
            .prev_matches
            .iter()
            .map(|m| match m {
                MatchType::Session(sess) => StrippedMatchInfo::from(sess.clone().into_owned()),
                MatchType::Old(old) => old.clone().into_owned(),
            })
            .collect()
    }
}
