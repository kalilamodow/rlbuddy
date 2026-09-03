use crate::common::savedata::{load_service_data, save_service_data};
use crate::core::app::PanelId;
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppData {
    pub app_settings: crate::core::app::AppSettings,
    pub saved_window_dimensions: Option<(egui::Pos2, egui::Vec2)>, // outer pos, inner size
    pub open_panels: Vec<PanelId>,
}

impl AppData {
    pub fn load() -> Self {
        load_service_data("global_data")
    }

    pub fn save(self) {
        save_service_data("global_data", self);
    }
}
