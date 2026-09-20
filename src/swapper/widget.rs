use eframe::egui;
use rfd::FileDialog;

use crate::{
    common::{ReadonlyStateHandle, channel::Sender},
    core::app::Panel,
    swapper::service::{SwapperCommand, SwapperService, SwapperServiceState},
};

pub struct SwapperWidget {
    state: ReadonlyStateHandle<SwapperServiceState>,
    sender: Sender<SwapperCommand>,
}

impl SwapperWidget {
    pub fn new(service: &SwapperService) -> Self {
        Self {
            state: service.state_handle(),
            sender: service.sender(),
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
                ui.label("Active swaps:");
            }
        })
        .response
    }
}
