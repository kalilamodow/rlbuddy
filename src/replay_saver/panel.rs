use crate::{
    core::app::Panel,
    replay_saver::{
        ReplaySaverService,
        service::{ChoosableKey, ReplaySaverSettingsHandle},
    },
};
use eframe::egui;

pub struct ReplaySaverWidget {
    settings: ReplaySaverSettingsHandle,
}

impl ReplaySaverWidget {
    pub fn new(service: &ReplaySaverService) -> Self {
        Self {
            settings: service.settings_handle(),
        }
    }
}

impl Panel for ReplaySaverWidget {
    fn name(&self) -> &'static str {
        "Replay Saver"
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let mut state = self.settings.write();
        ui.vertical(|ui| {
            egui::ComboBox::from_label("In-game keybind")
                .selected_text(
                    state
                        .chosen_key
                        .as_ref()
                        .map(|k| k.as_str())
                        .unwrap_or("Disable"),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.chosen_key, None, "Disable");
                    for key in ChoosableKey::ALL {
                        ui.selectable_value(&mut state.chosen_key, Some(key), key.as_str());
                    }
                });
            ui.small("This should match the key \"Save Replay\" is binded to in-game.");

            ui.add_space(8.0);
            ui.checkbox(&mut state.save_non_comp, "Save non-Competitive games");
            ui.small("If checked, casual and private matches will also be saved.");
        })
        .response
    }
}
