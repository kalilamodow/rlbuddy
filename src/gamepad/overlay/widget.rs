// settings widget basically, the actual one is rendered in the service

use crate::common::ReadWriteStateHandle;
use crate::core::app::Panel;
use crate::gamepad::overlay::service::{GamepadOverlayService, GamepadOverlayServiceSettings};
use eframe::egui;

pub struct GamepadOverlayWidget {
    settings_handle: ReadWriteStateHandle<GamepadOverlayServiceSettings>,
}

impl GamepadOverlayWidget {
    pub fn new(service: &GamepadOverlayService) -> Self {
        Self {
            settings_handle: service.settings_handle(),
        }
    }
}

impl Panel for GamepadOverlayWidget {
    fn name(&self) -> &'static str {
        "Gamepad Overlay"
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            let mut state = self.settings_handle.write();
            ui.checkbox(&mut state.enabled, "Enable");
        })
        .response
    }
}
