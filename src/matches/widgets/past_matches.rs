use super::{super::service::MatchesServiceState, match_renderer::MatchRenderer};
use crate::{
    common::{ReadonlyStateHandle, channel::Sender},
    matches::widgets::match_renderer::BuddyStatsOption,
    player_info::PlayerInfoServiceCommand,
};
use eframe::egui;
use std::{collections::HashMap, time::SystemTime};

pub struct PastMatchesWidget {
    state: ReadonlyStateHandle<MatchesServiceState>,
    open: HashMap<SystemTime, bool>,
    opened_stats: HashMap<SystemTime, Option<(String, String)>>, // display name, player id
    player_info_sender: Sender<PlayerInfoServiceCommand>,
}

impl PastMatchesWidget {
    pub fn new(
        state: ReadonlyStateHandle<MatchesServiceState>,
        player_info_sender: Sender<PlayerInfoServiceCommand>,
    ) -> Self {
        PastMatchesWidget {
            state,
            open: HashMap::new(),
            player_info_sender,
            opened_stats: HashMap::new(),
        }
    }
}

impl egui::Widget for &mut PastMatchesWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.vertical(|ui| {
                for prev_match in self.state.read().prev_matches.iter().rev() {
                    ui.group(|ui| {
                        ui.add(MatchRenderer::new(
                            prev_match,
                            Some(&mut self.open.entry(prev_match.started_at()).or_insert(false)),
                            self.opened_stats
                                .entry(prev_match.started_at())
                                .or_default(),
                            &self.player_info_sender,
                            BuddyStatsOption::No,
                        ))
                    });
                }
            })
            .response
        })
        .response
    }
}
