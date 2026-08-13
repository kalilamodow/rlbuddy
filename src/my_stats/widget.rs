use crate::{common::ReadonlyStateHandle, matches::MatchesServiceState};
use eframe::egui::{self};
use egui_plot::{Line, Plot, PlotPoints};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MyStatsWidgetSettings {
    pub session_only: bool,
}

pub struct MyStatsWidget {
    matches_state: ReadonlyStateHandle<MatchesServiceState>,
    settings: MyStatsWidgetSettings,
}

impl MyStatsWidget {
    pub fn new(
        matches_state: ReadonlyStateHandle<MatchesServiceState>,
        settings: MyStatsWidgetSettings,
    ) -> Self {
        Self {
            matches_state,
            settings,
        }
    }

    pub fn clone_settings(&self) -> MyStatsWidgetSettings {
        self.settings.clone()
    }

    fn render_settings_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.settings.session_only, "Session only");
        });
    }

    fn render_mmr_graph(&mut self, ui: &mut egui::Ui) {
        let prev_matches = self.matches_state.read();
        let line = Line::new(
            "MMR",
            PlotPoints::new(vec![[1f64, 0f64], [2f64, 1f64], [4f64, 10f64]]),
        );
        Plot::new("mmr graph plot").show(ui, |ui| {
            ui.line(line);
        });
    }
}

impl egui::Widget for &mut MyStatsWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical_centered_justified(|ui| {
            self.render_settings_header(ui);
            self.render_mmr_graph(ui);
        })
        .response
    }
}
