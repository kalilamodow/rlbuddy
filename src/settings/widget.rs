use std::{cell::RefCell, rc::Rc};

use eframe::egui;

use crate::{
    hotkey::{HotkeyService, HotkeySettingsWidget},
    settings::app_settings::AppSettingsWidget,
    toast_alert::{MatchNotificatorService, MatchNotificatorSettingsWidget, ToastAlertService},
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Panel {
    HotkeySettings,
    AppSettings,
    MatchNotificator,
}

impl std::fmt::Display for Panel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Panel::HotkeySettings => "Keybind",
                Panel::AppSettings => "App",
                Panel::MatchNotificator => "Toast",
            }
        )
    }
}

const ALL_PANELS: [Panel; 3] = [
    Panel::HotkeySettings,
    Panel::AppSettings,
    Panel::MatchNotificator,
];

pub struct SettingsWidget {
    hotkey: HotkeySettingsWidget,
    app: AppSettingsWidget,
    notificator: MatchNotificatorSettingsWidget,
}

impl SettingsWidget {
    pub fn new(
        hotkey_service: &HotkeyService,
        match_notificator_service: &MatchNotificatorService,
        toast_service: &ToastAlertService,
        transparency: Rc<RefCell<u8>>,
    ) -> Self {
        Self {
            hotkey: HotkeySettingsWidget::new(hotkey_service.settings_handle()),
            app: AppSettingsWidget::new(transparency),
            notificator: MatchNotificatorSettingsWidget::new(
                match_notificator_service,
                toast_service.sender(),
            ),
        }
    }
}

impl egui::Widget for &mut SettingsWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical_centered_justified(|ui| {
            for panel in ALL_PANELS {
                ui.add_space(4.0);
                ui.group(|ui| {
                    ui.strong(panel.to_string());
                    ui.add_space(4.0);

                    match panel {
                        Panel::HotkeySettings => ui.add(&self.hotkey),
                        Panel::AppSettings => ui.add(&self.app),
                        Panel::MatchNotificator => ui.add(&mut self.notificator),
                    };
                });
            }
        })
        .response
    }
}
