use eframe::egui;
use rfd::FileDialog;

use crate::{
    common::{ReadonlyStateHandle, channel::Sender},
    core::app::Panel,
    swapper::service::{ItemId, SwapperCommand, SwapperService, SwapperServiceState},
};

pub struct SwapperWidget {
    state: ReadonlyStateHandle<SwapperServiceState>,
    sender: Sender<SwapperCommand>,
    to_swap_input: (String, String),
}

impl SwapperWidget {
    pub fn new(service: &SwapperService) -> Self {
        Self {
            state: service.state_handle(),
            sender: service.sender(),
            to_swap_input: ("".into(), "".into()),
        }
    }

    fn render_exe_path_picker(&self, ui: &mut egui::Ui) {
        if !ui.button("Select executable path").clicked() {
            return;
        }

        let Some(selected_file) = FileDialog::new()
            .add_filter("Executable", &["exe"])
            .pick_file()
        else {
            return;
        };

        self.sender
            .send(SwapperCommand::SetExecutablePath(selected_file));
    }

    fn render_swap_list(&mut self, ui: &mut egui::Ui) {
        let state = self.state.read();
        for swap in &state.active_swaps {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{} looks like {}",
                        swap.replaced.id(),
                        swap.appearance.id()
                    ));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if ui.button("Remove").clicked() {
                            self.sender
                                .send(SwapperCommand::DeleteSwap(swap.replaced.clone()));
                        }
                    });
                });
            });
        }
    }

    fn render_current_error(&mut self, ui: &mut egui::Ui) {
        let state = self.state.read();
        if let Some(error) = &state.current_error {
            ui.horizontal(|ui| {
                ui.colored_label(ui.style().visuals.error_fg_color, error);
                if ui.button("OK").clicked() {
                    self.sender.send(SwapperCommand::ClearError);
                }
            });
        }
    }

    fn render_swap_inputs(&mut self, ui: &mut egui::Ui) {
        ui.label("Appearance:");
        ui.text_edit_singleline(&mut self.to_swap_input.0);
        ui.label("Replace:");
        ui.text_edit_singleline(&mut self.to_swap_input.1);

        if !ui.button("Swap").clicked() {
            return;
        }

        self.sender.send(SwapperCommand::Swap {
            appearance: ItemId::new(std::mem::take(&mut self.to_swap_input.0)),
            replaced: ItemId::new(std::mem::take(&mut self.to_swap_input.1)),
        });
    }
}

impl Panel for SwapperWidget {
    fn name(&self) -> &'static str {
        "Item Swapper"
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.vertical(|ui| {
            let has_exe_path = {
                let state = self.state.read();
                state.exe_path.is_some()
            };

            if !has_exe_path {
                self.render_exe_path_picker(ui);
            } else {
                let has_any_swaps = {
                    let state = self.state.read();
                    !state.active_swaps.is_empty()
                };
                ui.label(if has_any_swaps {
                    "Active swaps:"
                } else {
                    "No active swaps"
                });

                self.render_swap_list(ui);
                self.render_current_error(ui);
                ui.separator();
                self.render_swap_inputs(ui);
                ui.separator();
            }

            ui.small("Thanks to ShinyEmii/Toga-Files for aes keys!");
        })
        .response
    }
}
