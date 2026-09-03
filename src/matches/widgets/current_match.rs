use super::{super::service::MatchesServiceState, match_renderer::MatchRenderer};
use crate::core::app::Panel;
use crate::matches::MatchesService;
use crate::player_info::PlayerInfoService;
use crate::{
    common::{ReadonlyStateHandle, channel::Sender},
    matches::{service::MatchType, widgets::match_renderer::BuddyStatsOption},
    player_info::PlayerInfoServiceCommand,
};
use eframe::egui;
use std::borrow::Cow;

pub struct CurrentMatchWidget {
    state: ReadonlyStateHandle<MatchesServiceState>,
    player_info_sender: Sender<PlayerInfoServiceCommand>,
    opened_stats: Option<(String, String)>,
}

impl CurrentMatchWidget {
    pub fn new(matches_service: &MatchesService, player_info_service: &PlayerInfoService) -> Self {
        CurrentMatchWidget {
            state: matches_service.state_handle(),
            player_info_sender: player_info_service.sender(),
            opened_stats: None,
        }
    }
}

impl Panel for CurrentMatchWidget {
    fn name(&self) -> &'static str {
        "Lobby"
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            if let Some(current_match) = &self.state.read().current_match {
                ui.add(MatchRenderer::new(
                    &MatchType::Session(Cow::Borrowed(current_match)),
                    None,
                    &mut self.opened_stats,
                    &self.player_info_sender,
                    BuddyStatsOption::Yes(&self.state),
                ));
            } else {
                ui.label("Not in a match");
            }
        })
        .response
    }
}
