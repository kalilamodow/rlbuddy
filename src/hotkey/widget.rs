use super::service::{HotkeyService, HotkeySettings, SelectableHotkey};
use crate::common::ThreadedReadWriteStateHandle;
use crate::core::app::Panel;
use crate::hotkey::service::SelectedButtonState;
use eframe::egui;

pub struct HotkeySettingsWidget {
    settings: ThreadedReadWriteStateHandle<HotkeySettings>,
}

impl HotkeySettingsWidget {
    pub fn new(service: &HotkeyService) -> Self {
        HotkeySettingsWidget {
            settings: service.settings_handle(),
        }
    }
}

impl Panel for HotkeySettingsWidget {
    fn name(&self) -> &'static str {
        "Hotkey Settings"
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let mut settings = self.settings.write();

        ui.vertical_centered_justified(|ui| {
            egui::ComboBox::from_label("Hotkey (keyboard)")
                .selected_text(settings.key.as_str())
                .show_ui(ui, |ui| {
                    for key in [
                        SelectableHotkey::Alt,
                        SelectableHotkey::LShift,
                        SelectableHotkey::LCtrl,
                        SelectableHotkey::Tab,
                        SelectableHotkey::Super,
                        SelectableHotkey::Disabled,
                    ] {
                        let key_str = key.as_str();
                        ui.selectable_value(&mut settings.key, key, key_str);
                    }
                });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let mut new_button: Option<SelectedButtonState> = None;
                match &settings.button {
                    SelectedButtonState::Selected(button) => {
                        ui.label("Selected gamepad hotkey:");
                        ui.strong(format!("{button:?}"));

                        if ui.button("Change").clicked() {
                            new_button = Some(SelectedButtonState::Choosing);
                        }
                        if ui.button("Remove").clicked() {
                            new_button = Some(SelectedButtonState::Disabled);
                        }
                    }
                    SelectedButtonState::Choosing => {
                        ui.label("Choosing gamepad hotkey...");
                        if ui.button("Cancel").clicked() {
                            new_button = Some(SelectedButtonState::Disabled);
                        }
                    }
                    SelectedButtonState::Disabled => {
                        ui.label("Gamepad hotkey disabled");
                        if ui.button("Choose").clicked() {
                            new_button = Some(SelectedButtonState::Choosing);
                        }
                    }
                }

                if let Some(new_button) = new_button {
                    settings.button = new_button;
                }
            });
        })
        .response
    }
}
