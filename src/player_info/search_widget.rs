use eframe::egui;

use crate::{
    common::channel::Sender, player_info::PlayerInfoServiceCommand, rocket_league::Platform,
};

pub struct PlayerSearchWidget {
    selected_platform: Platform,
    player_name: String,
    sender: Sender<PlayerInfoServiceCommand>,
}

impl PlayerSearchWidget {
    pub fn new(sender: Sender<PlayerInfoServiceCommand>) -> Self {
        Self {
            player_name: String::new(),
            selected_platform: Platform::Epic,
            sender,
        }
    }
}

impl egui::Widget for &mut PlayerSearchWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical_centered_justified(|ui| {
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("player search platform selector")
                    .selected_text(self.selected_platform.to_string())
                    .show_ui(ui, |ui| {
                        for platform in [
                            Platform::Epic,
                            Platform::PlayStation,
                            Platform::Xbox,
                            Platform::Steam,
                            Platform::Switch,
                        ] {
                            let platform_str = platform.to_string();
                            ui.selectable_value(
                                &mut self.selected_platform,
                                platform,
                                platform_str,
                            );
                        }
                    });

                ui.centered_and_justified(|ui| ui.text_edit_singleline(&mut self.player_name));
            });

            if ui.button("Search").clicked() {
                self.sender.send(PlayerInfoServiceCommand::Open(
                    self.selected_platform,
                    self.player_name.clone(),
                ));
            }
        })
        .response
    }
}
