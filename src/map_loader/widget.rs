use crate::{
    common::{ThreadedReadonlyStateHandle, channel::Sender, data_dir::rlbuddy_data_dir},
    map_loader::{
        map_card_widget::MapCardWidget,
        service::{CustomMapId, MapLoaderCommand, MapLoaderService, MapLoaderServiceState},
    },
};
use eframe::egui;
use rfd::FileDialog;
use std::{fs, path::PathBuf};

pub struct MapLoaderWidget {
    state: ThreadedReadonlyStateHandle<MapLoaderServiceState>,
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
        let state = self.state.read();
        let is_importing = state.import_progress.is_some();

        ui.horizontal(|ui| {
            if ui
                .add_enabled(!is_importing, egui::Button::new("Import new map"))
                .clicked()
            {
                self.import_map();
            }

            if ui
                .add_enabled(state.loaded_map.is_some(), egui::Button::new("Unload"))
                .clicked()
            {
                self.command_sender.send(MapLoaderCommand::Unload);
            }

            if let Some(import_progress) = state.import_progress {
                ui.add(egui::ProgressBar::new(import_progress).text("Importing..."));
            }
        });
        ui.add_space(4.0);
    }

    fn render_map_list(&self, ui: &mut egui::Ui) {
        let state = self.state.read();

        ui.with_layout(
            egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
            |ui| {
                for map in &state.maps {
                    let this_map_is_selected = state
                        .loaded_map
                        .as_ref()
                        .is_some_and(|m| m.as_str() == map.id.as_str());

                    ui.add(MapCardWidget::new(
                        &map.id,
                        map.author.as_deref(),
                        map.description.as_deref(),
                        preview_image(&map.id)
                            .map(|i| i.to_string_lossy().replace("\\\\?\\", "file://"))
                            .as_deref(),
                        |ui| {
                            if ui
                                .add_enabled(!this_map_is_selected, egui::Button::new("Load"))
                                .clicked()
                            {
                                self.command_sender
                                    .send(MapLoaderCommand::Load(map.id.clone()))
                            }

                            if ui
                                .add_enabled(!this_map_is_selected, egui::Button::new("Delete"))
                                .clicked()
                            {
                                self.command_sender
                                    .send(MapLoaderCommand::Delete(map.id.clone()))
                            }
                        },
                        this_map_is_selected,
                    ));
                }
            },
        );
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
            self.render_map_list(ui);
        })
        .response
    }
}

fn preview_image(map_id: &CustomMapId) -> Option<PathBuf> {
    let data_dir = rlbuddy_data_dir()?;

    data_dir
        .join("custom maps")
        .join(map_id.as_str())
        .join("preview.jpg")
        .canonicalize()
        .ok()
}
