use crate::common::savedata::{load_service_config, save_service_config};
use crate::core::app::SavedOpenPanelList;
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppData {
    pub app_settings: crate::core::app::AppSettings,
    pub saved_window_dimensions: Option<(egui::Pos2, egui::Vec2)>, // outer pos, inner size
    pub open_panels: SavedOpenPanelList,
}

impl AppData {
    pub fn load() -> Self {
        load_service_config("global_data")
    }

    pub fn save(&self) {
        save_service_config("global_data", self);
    }
}
