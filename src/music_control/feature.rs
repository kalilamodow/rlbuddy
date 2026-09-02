use crate::common::savedata::{load_service_data, save_service_data};
use crate::core::app::{Feature, Service};
use crate::music_control::service::MusicControlService;
use crate::music_control::widget::MusicControlWidget;
use crate::stats_api::StatsApi;
use eframe::egui;
use eframe::egui::{Ui, Widget};

const DATA_ID: &str = "music_control_settings";

pub struct MusicControlFeature {
    service: MusicControlService,
    widget: MusicControlWidget,
}

impl MusicControlFeature {
    pub fn new(stats_api: &mut StatsApi) -> Self {
        let service = MusicControlService::new(load_service_data(DATA_ID), stats_api.subscribe());
        Self {
            widget: MusicControlWidget::new(&service),
            service,
        }
    }
}

impl Service for MusicControlFeature {
    fn update(&mut self) {
        self.service.update();
    }

    fn save(&self) {
        save_service_data(DATA_ID, self.service.settings_handle().read().clone())
    }
}

impl Feature for MusicControlFeature {
    fn name(&self) -> &'static str {
        "Music"
    }

    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        self.widget.ui(ui)
    }
}
