use crate::{
    core::app::Panel,
    rocket_league::{get_rl_exe_path, set_rl_exe_path},
};
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

    fn do_setup(&mut self, exe_path: Option<PathBuf>) {
        let Some(selected_file) = exe_path.or_else(|| {
            let f = FileDialog::new()
                .add_filter("Executable", &["exe"])
                .pick_file();
            f
        }) else {
            return;
        };

        set_rl_exe_path(selected_file.clone());

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

    fn open_by_default(&self) -> bool {
        true
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
                // do_setup handles None ex_path
                let exe_path = get_rl_exe_path();

                if exe_path.is_none() {
                    ui.label("Select RocketLeague.exe path");
                }

                if ui
                    .button(if exe_path.is_some() {
                        "Set up automatically"
                    } else {
                        "Select file"
                    })
                    .clicked()
                {
                    self.do_setup(exe_path);
                }
            }
        })
        .response
    }
}
