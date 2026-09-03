use crate::common::savedata::{load_service_data, rlbuddy_data_dir, save_service_data};
use eframe::egui;
use std::fs;

#[derive(Debug, Default)]
pub struct AppData {
    pub app_settings: crate::core::app::AppSettings,
    pub saved_window_dimensions: Option<(egui::Pos2, egui::Vec2)>, // outer pos, inner size
}

impl AppData {
    pub fn load() -> Self {
        Self {
            app_settings: load_service_data("app_settings"),
            saved_window_dimensions: load_service_data("saved_window_dimensions"),
        }
    }

    pub fn save(self) {
        let Some(data_dir) = rlbuddy_data_dir() else {
            return;
        };

        let _ = fs::create_dir_all(&data_dir);

        save_service_data("app_settings", self.app_settings);
        save_service_data("saved_window_dimensions", self.saved_window_dimensions);
    }
}
