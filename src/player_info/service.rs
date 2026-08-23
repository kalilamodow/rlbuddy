use super::trn::{PlayerKey, TrackerAPI, new_tracker_api};
use crate::{
    common::channel::{Receiver, Sender},
    player_info::{trn::TRNError, trn_widget::TrackerWidget},
    rocket_league::Platform,
};
use eframe::egui;

pub enum PlayerInfoServiceCommand {
    Open(Platform, String),
}

#[derive(Debug)]
struct OpenedPlayer {
    data: PlayerKey,
    open: bool,
}

pub struct PlayerInfoService {
    trn: TrackerAPI,
    open_players: Vec<OpenedPlayer>,
    command_receiver: Receiver<PlayerInfoServiceCommand>,
}

impl PlayerInfoService {
    pub fn new(context: egui::Context) -> Self {
        Self {
            trn: new_tracker_api(context),
            open_players: Vec::new(),
            command_receiver: Receiver::new(),
        }
    }

    pub fn update(&mut self) {
        while let Some(command) = self.command_receiver.try_recv() {
            self.process_command(command);
        }
    }

    pub fn sender(&self) -> Sender<PlayerInfoServiceCommand> {
        self.command_receiver.send()
    }

    fn process_command(&mut self, command: PlayerInfoServiceCommand) {
        match command {
            PlayerInfoServiceCommand::Open(platform, platform_id) => {
                self.open_players.push(OpenedPlayer {
                    data: PlayerKey {
                        platform,
                        platform_id,
                    },
                    open: true,
                });
            }
        }
    }
}

impl egui::Widget for &mut PlayerInfoService {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        for player in &mut self.open_players {
            let OpenedPlayer { open, data } = player;
            let profile = self.trn.get(data);

            let display = if let Some(profile_result) = profile.as_ref()
                && let Ok(profile) = profile_result.as_ref()
            {
                &profile.platform_info.platform_user_handle
            } else {
                &data.platform_id
            };

            egui::Window::new(display)
                .open(open)
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    if let Some(profile) = profile {
                        match profile.as_ref() {
                            Ok(profile) => {
                                ui.add(TrackerWidget::new(profile, data));
                            }
                            Err(error) => {
                                match error {
                                    TRNError::NotFound => ui.colored_label(
                                        ui.visuals().error_fg_color,
                                        "Player not found.",
                                    ),
                                    TRNError::Other(e) => ui.colored_label(
                                        ui.visuals().error_fg_color,
                                        format!("Unexpected error: \"{e}\""),
                                    ),
                                };
                            }
                        }
                    } else {
                        ui.spinner();
                    }
                });
        }

        self.open_players.retain(|w| w.open);
        ui.allocate_response(egui::Vec2::ZERO, egui::Sense::empty())
    }
}
