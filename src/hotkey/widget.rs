use super::service::{HotkeyService, HotkeySettings, SelectableHotkey};
use crate::core::app::Panel;
use crate::{common::ThreadedReadWriteStateHandle, hotkey::service::SelectableControllerButton};
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

            egui::ComboBox::from_label("Hotkey (gamepad)")
                .selected_text(settings.button.as_str())
                .show_ui(ui, |ui| {
                    for key in [
                        SelectableControllerButton::LeftBumper,
                        SelectableControllerButton::RightBumper,
                        SelectableControllerButton::Select,
                        SelectableControllerButton::Start,
                        SelectableControllerButton::Disabled,
                    ] {
                        let key_str = key.as_str();
                        ui.selectable_value(&mut settings.button, key, key_str);
                    }
                });
        })
        .response
    }
}
