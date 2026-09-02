use crate::{
    settings::app_settings::AppSettingsWidget,
    toast_alert::{MatchNotificatorService, MatchNotificatorSettingsWidget, ToastAlertService},
};
use eframe::egui;
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Panel {
    AppSettings,
    MatchNotificator,
}

impl std::fmt::Display for Panel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Panel::AppSettings => "App",
                Panel::MatchNotificator => "Toast",
            }
        )
    }
}

const ALL_PANELS: [Panel; 2] = [Panel::AppSettings, Panel::MatchNotificator];

pub struct SettingsWidget {
    app: AppSettingsWidget,
    notificator: MatchNotificatorSettingsWidget,
}

impl SettingsWidget {
    pub fn new(
        match_notificator_service: &MatchNotificatorService,
        toast_service: &ToastAlertService,
        transparency: Rc<RefCell<u8>>,
    ) -> Self {
        Self {
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
                        Panel::AppSettings => ui.add(&self.app),
                        Panel::MatchNotificator => ui.add(&mut self.notificator),
                    };
                });
            }
        })
        .response
    }
}
