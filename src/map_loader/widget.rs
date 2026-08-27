use std::fs;

use eframe::egui;
use rfd::FileDialog;

use crate::{
    common::{ReadonlyStateHandle, channel::Sender},
    map_loader::service::{MapLoaderCommand, MapLoaderService, MapLoaderServiceState},
};

pub struct MapLoaderWidget {
    state: ReadonlyStateHandle<MapLoaderServiceState>,
    command_sender: Sender<MapLoaderCommand>,
}

impl MapLoaderWidget {
    pub fn new(service: &MapLoaderService) -> Self {
        Self {
            state: service.state_handle(),
            command_sender: service.sender(),
        }
    }

    fn render_setup(&self, ui: &mut egui::Ui) {
        if !ui.button("Select RocketLeague.exe").clicked() {
            return;
        }

        let Some(binary_path) = FileDialog::new()
            .add_filter("Executable", &["exe"])
            .pick_file()
        else {
            return;
        };

        let underpass_path =
            binary_path.join("../../../TAGame/CookedPCConsole/Labs_Underpass_P.upk");
        let Ok(underpass_path) = fs::canonicalize(underpass_path) else {
            return;
        };

        let underpass_path = match fs::canonicalize(underpass_path) {
            Ok(wtv) => wtv,
            Err(error) => {
                eprintln!("{error}");
                return;
            }
        };

        self.command_sender
            .send(MapLoaderCommand::UpdateUnderpassPath(underpass_path));
    }

    fn import_map(&self) {
        let Some(zip_path) = FileDialog::new()
            .add_filter("Custom Map", &["zip"])
            .pick_file()
        else {
            return;
        };

        let zip_path = match fs::canonicalize(zip_path) {
            Ok(wtv) => wtv,
            Err(error) => {
                eprintln!("{error}");
                return;
            }
        };

        self.command_sender.send(MapLoaderCommand::Import(zip_path));
    }

    fn render_error_header(&self, ui: &mut egui::Ui) {
        let state = self.state.read();
        if let Some(err) = state.current_error.as_ref() {
            ui.horizontal(|ui| {
                ui.colored_label(ui.style().visuals.error_fg_color, err);
                if ui.small_button("X").clicked() {
                    self.command_sender.send(MapLoaderCommand::ClearError);
                }
            });
        }
    }

    fn render_header(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Import new map").clicked() {
                self.import_map();
            }
        });
    }
}

impl egui::Widget for &MapLoaderWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            {
                let set_up = self.state.read().underpass_path.is_some();
                if !set_up {
                    self.render_setup(ui);
                    return;
                }
            }

            self.render_error_header(ui);
            self.render_header(ui);
        })
        .response
    }
}
