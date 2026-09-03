use super::service::{DiscordServiceState, DiscordSettings};
use crate::common::{ReadWriteStateHandle, ReadonlyStateHandle};
use crate::core::app::Panel;
use eframe::egui;

pub struct DiscordWidget {
    settings: ReadWriteStateHandle<DiscordSettings>,
    state: ReadonlyStateHandle<DiscordServiceState>,
}

impl DiscordWidget {
    pub fn new(
        settings: ReadWriteStateHandle<DiscordSettings>,
        state: ReadonlyStateHandle<DiscordServiceState>,
    ) -> Self {
        DiscordWidget { settings, state }
    }
}

impl Panel for DiscordWidget {
    fn name(&self) -> &'static str {
        "Discord Presence"
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            let state = self.state.read();

            ui.vertical(|ui| {
                if state.busy {
                    ui.disable();
                }

                let mut settings = self.settings.write();
                ui.checkbox(&mut settings.disable, "Disabled");

                ui.add_enabled_ui(!settings.disable, |ui| {
                    ui.checkbox(&mut settings.hide_score, "Hide score");
                });
            });

            ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                if state.connected {
                    ui.colored_label(egui::Color32::DARK_GREEN, "Connected");
                } else {
                    ui.label("Not connected");
                }

                if state.busy {
                    ui.spinner();
                }
            });
        })
        .response
    }
}
