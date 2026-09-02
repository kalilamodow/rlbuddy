use crate::common::savedata::{load_service_data, save_service_data};
use crate::core::app::{Feature, Service};
use crate::gamepad::GamepadService;
use crate::hotkey::service::HotkeyService;
use crate::hotkey::widget::HotkeySettingsWidget;
use eframe::egui::{Response, Ui, Widget};
use std::sync::mpsc;

const DATA_ID: &str = "hotkey_settings";

pub struct HotkeyFeature {
    service: HotkeyService,
    widget: HotkeySettingsWidget,
}

impl HotkeyFeature {
    pub fn new(gamepad_service: &mut GamepadService, overlay_tx: &mpsc::Sender<bool>) -> Self {
        let service = HotkeyService::new(gamepad_service, overlay_tx, load_service_data(DATA_ID));
        Self {
            widget: HotkeySettingsWidget::new(&service),
            service,
        }
    }
}

impl Service for HotkeyFeature {
    fn update(&mut self) {
        self.service.update();
    }

    fn save(&self) {
        save_service_data(DATA_ID, self.service.settings_handle().read().clone());
    }
}

impl Feature for HotkeyFeature {
    fn name(&self) -> &'static str {
        "Hotkey"
    }

    fn ui(&mut self, ui: &mut Ui) -> Response {
        self.widget.ui(ui)
    }
}
