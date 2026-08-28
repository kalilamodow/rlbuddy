use std::{fs, path::PathBuf};

use eframe::egui;
use rfd::FileDialog;

use crate::{
    common::{ReadonlyStateHandle, channel::Sender, data_dir::rlbuddy_data_dir},
    map_loader::service::{CustomMapId, MapLoaderCommand, MapLoaderService, MapLoaderServiceState},
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
        let state = self.state.read();

        ui.horizontal(|ui| {
            if ui.button("Import new map").clicked() {
                self.import_map();
            }

            if ui
                .add_enabled(state.loaded_map.is_some(), egui::Button::new("Unload"))
                .clicked()
            {
                self.command_sender.send(MapLoaderCommand::Unload);
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

                    // first, draw the background
                    let image_rect = match preview_image(&map.id) {
                        Some(preview_image) => {
                            ui.add(
                                egui::Image::new(egui::ImageSource::Uri(
                                    preview_image
                                        .to_string_lossy()
                                        .replace("\\\\?\\", "file://")
                                        .into(),
                                ))
                                .fit_to_exact_size(egui::vec2(200.0, 115.0))
                                .maintain_aspect_ratio(false)
                                .corner_radius(egui::CornerRadius::same(8)),
                            )
                            .rect
                        }
                        None => {
                            ui.allocate_space(egui::vec2(
                                200.0,
                                if map.description.is_some() {
                                    115.0
                                } else {
                                    75.0
                                },
                            ))
                            .1
                        }
                    };

                    // then, add a dark overlay for contrast
                    ui.painter().add(
                        egui::Frame::new()
                            .fill(egui::Color32::from_black_alpha(200))
                            .corner_radius(egui::CornerRadius::same(8))
                            .stroke(if this_map_is_selected {
                                egui::Stroke::new(0.5f32, egui::Color32::WHITE)
                            } else {
                                Default::default()
                            })
                            .paint(image_rect),
                    );

                    if image_rect.width() < 50.0 || image_rect.height() < 50.0 {
                        // probably loading, dont render content
                        continue;
                    }

                    // finally, put the actual content (shrink for margin)
                    let content_rect = image_rect.shrink(8.0);

                    ui.place(content_rect, |ui: &mut egui::Ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(map.id.as_str()).strong().size(15.0));
                            ui.add_space(2.0);

                            if let Some(description) = &map.description {
                                ui.label(description);
                            }

                            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                                ui.horizontal(|ui| {
                                    if let Some(author) = &map.author {
                                        ui.label(format!("By {}", author));
                                    }

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Max),
                                        |ui| {
                                            if ui
                                                .add_enabled(
                                                    !this_map_is_selected,
                                                    egui::Button::new("Load"),
                                                )
                                                .clicked()
                                            {
                                                self.command_sender
                                                    .send(MapLoaderCommand::Load(map.id.clone()))
                                            }

                                            if ui
                                                .add_enabled(
                                                    !this_map_is_selected,
                                                    egui::Button::new("Delete"),
                                                )
                                                .clicked()
                                            {
                                                self.command_sender
                                                    .send(MapLoaderCommand::Delete(map.id.clone()))
                                            }
                                        },
                                    );
                                });
                            });
                        })
                        .response
                    });
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
