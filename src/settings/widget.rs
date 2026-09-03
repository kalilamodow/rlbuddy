use crate::settings::app_settings::AppSettingsWidget;
use eframe::egui;
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Panel {
    AppSettings,
}

impl std::fmt::Display for Panel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Panel::AppSettings => "App",
            }
        )
    }
}

const ALL_PANELS: [Panel; 1] = [Panel::AppSettings];

pub struct SettingsWidget {
    app: AppSettingsWidget,
}

impl SettingsWidget {
    pub fn new(transparency: Rc<RefCell<u8>>) -> Self {
        Self {
            app: AppSettingsWidget::new(transparency),
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
                    };
                });
            }
        })
        .response
    }
}
