// theres no gsmtc on linux, so this is a fallback service which just says its unsupported.

use crate::{
    core::app::{Panel, Service, ServiceWithUi},
    stats_api::StatsApi,
};

pub struct MusicControlService {}

impl MusicControlService {
    pub fn new(_stats_api: &mut StatsApi) -> Self {
        Self {}
    }
}

impl Service for MusicControlService {
    fn update(&mut self) {}
}

impl ServiceWithUi for MusicControlService {
    fn panel(&self) -> impl crate::core::app::Panel + 'static {
        MusicControlBackupPanel {}
    }
}

pub struct MusicControlBackupPanel {}
impl Panel for MusicControlBackupPanel {
    fn name(&self) -> &'static str {
        "Music"
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                "Unfortunately, the music widget isn't supported on Linux :(\n\
            If you have any experience with Rust development, a PR would be highly appreciated! <3",
            )
        })
        .response
    }
}
