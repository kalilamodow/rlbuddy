use crate::{
    common::{ReadonlyStateHandle, channel::Sender},
    core::app::Panel,
    rocket_league::{Item, ItemSlot, ItemsLoadStatus, get_items},
    swapper::service::{SwapperCommand, SwapperService, SwapperServiceState},
};
use eframe::egui;
use rfd::FileDialog;

pub struct SwapperWidget {
    state: ReadonlyStateHandle<SwapperServiceState>,
    sender: Sender<SwapperCommand>,
    to_swap_input: (Option<Item>, Option<Item>),
    to_swap_filter: ItemSlot,
}

impl SwapperWidget {
    pub fn new(service: &SwapperService) -> Self {
        Self {
            state: service.state_handle(),
            sender: service.sender(),
            to_swap_input: (None, None),
            to_swap_filter: ItemSlot::Boost,
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
        let ItemsLoadStatus::Loaded(items) = &*get_items() else {
            ui.spinner();
            return;
        };

        for swap in &state.active_swaps {
            let replaced = items.iter().find(|i| i.id == swap.replaced);
            let appearance = items.iter().find(|i| i.id == swap.appearance);

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{} looks like {}",
                        replaced.map(|r| r.name.as_str()).unwrap_or_default(),
                        appearance.map(|r| r.name.as_str()).unwrap_or_default()
                    ));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if ui
                            .add_enabled(replaced.is_some(), egui::Button::new("Remove"))
                            .clicked()
                        {
                            self.sender
                                .send(SwapperCommand::DeleteSwap(replaced.unwrap().clone()));
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
        let ItemsLoadStatus::Loaded(items) = &*get_items() else {
            ui.spinner();
            return;
        };

        egui::ComboBox::from_label("Appearance")
            .selected_text(
                self.to_swap_input
                    .0
                    .as_ref()
                    .map(|i| i.name.as_str())
                    .unwrap_or("Select..."),
            )
            .show_ui(ui, |ui| {
                for item in items.iter().filter(|i| i.slot == self.to_swap_filter) {
                    if ui
                        .selectable_label(
                            self.to_swap_input.0.as_ref().map(|i| i.id) == Some(item.id),
                            &item.name,
                        )
                        .clicked()
                    {
                        self.to_swap_input.0 = Some(item.clone());
                    }
                }
            });
        egui::ComboBox::from_label("Replaced item")
            .selected_text(
                self.to_swap_input
                    .1
                    .as_ref()
                    .map(|i| i.name.as_str())
                    .unwrap_or("Select..."),
            )
            .show_ui(ui, |ui| {
                for item in items.iter().filter(|i| i.slot == self.to_swap_filter) {
                    if ui
                        .selectable_label(
                            self.to_swap_input.1.as_ref().map(|i| i.id) == Some(item.id),
                            &item.name,
                        )
                        .clicked()
                    {
                        self.to_swap_input.1 = Some(item.clone());
                    }
                }
            });

        let mut swap_clicked = false;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            swap_clicked = ui.button("Swap").clicked();

            egui::ComboBox::from_label("Slot")
                .selected_text(self.to_swap_filter.as_str())
                .show_ui(ui, |ui| {
                    for slot in &[
                        ItemSlot::Antenna,
                        ItemSlot::Body,
                        ItemSlot::Boost,
                        ItemSlot::Explosion,
                        ItemSlot::PaintFinish,
                        ItemSlot::Topper,
                        ItemSlot::Trail,
                        ItemSlot::Wheel,
                    ] {
                        ui.selectable_value(&mut self.to_swap_filter, slot.clone(), slot.as_str());
                    }
                });
        });

        if !swap_clicked {
            return;
        }

        if let Some(appearance) = self.to_swap_input.0.take()
            && let Some(replaced) = self.to_swap_input.1.take()
        {
            self.sender.send(SwapperCommand::Swap {
                replaced,
                appearance,
            });
        }
    }

    fn render_footer(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.small("Thanks to ShinyEmii/Toga-Files for item definitions!");

            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Min),
                |ui| match &*get_items() {
                    ItemsLoadStatus::Error(err) => {
                        ui.colored_label(ui.visuals().error_fg_color, err.to_string());
                    }
                    ItemsLoadStatus::Loading | ItemsLoadStatus::NotLoaded => {
                        ui.label("Loading items...");
                    }
                    ItemsLoadStatus::Loaded(_) => {}
                },
            );
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
            }

            ui.separator();
            self.render_footer(ui);
        })
        .response
    }
}
