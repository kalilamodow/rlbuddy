use crate::core::app::Panel;
use eframe::egui;
use rfd::FileDialog;
use std::{fs, io, path::PathBuf};
use sysinfo::ProcessesToUpdate;

fn rewrite_ini(inipath: &PathBuf) -> io::Result<()> {
    let contents = fs::read_to_string(inipath)?;
    let new_contents = contents.replace("PacketSendRate=0", "PacketSendRate=1");
    fs::write(inipath, new_contents)?;
    Ok(())
}

fn is_rocket_league_open() -> bool {
    let mut system = sysinfo::System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    system
        .processes_by_name(std::ffi::OsStr::new("RocketLeague.exe"))
        .next()
        .is_some()
}

pub struct AutoSetupWidget {
    success: Option<Result<Option<&'static str>, String>>,
}

impl AutoSetupWidget {
    pub fn new() -> Self {
        AutoSetupWidget { success: None }
    }

    fn do_setup(&mut self) {
        let Some(selected_file) = FileDialog::new()
            .add_filter("Executable", &["exe"])
            .pick_file()
        else {
            return;
        };

        let Some(binary_dir) = selected_file.parent() else {
            self.success = Some(Err("invalid path".to_string()));
            return;
        };

        let stats_api_config_path = match binary_dir
            .join("../../TAGame/Config/DefaultStatsAPI.ini")
            .canonicalize()
        {
            Ok(p) => p,
            Err(error) => {
                self.success = Some(Err(error.to_string()));
                return;
            }
        };

        if let Err(error) = rewrite_ini(&stats_api_config_path) {
            self.success = Some(Err(error.to_string()));
        } else {
            self.success = Some(Ok(
                is_rocket_league_open().then_some("You will need to restart Rocket League.")
            ));
        }
    }
}

impl Panel for AutoSetupWidget {
    fn name(&self) -> &'static str {
        "Stats API Setup"
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            if let Some(result) = &self.success {
                match result {
                    Ok(success_msg) => {
                        ui.label("Success!");
                        if let Some(msg) = success_msg {
                            ui.label(*msg);
                        }
                    }
                    Err(error) => {
                        ui.label(format!("Error: {error}"));
                    }
                }
            } else {
                ui.label("Select RocketLeague.exe path");
                if ui.button("Select file").clicked() {
                    self.do_setup();
                }
            }
        })
        .response
    }
}
