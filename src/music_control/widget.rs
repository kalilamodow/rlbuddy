use crate::{
    common::{ReadWriteStateHandle, ThreadedReadonlyStateHandle, channel::Sender},
    music_control::{
        MusicControlCommand, MusicControlService, MusicControlServiceState, MusicControlSettings,
    },
};
use eframe::egui;

fn handle_null_string<'a>(s: Option<&'a String>) -> &'a str {
    s.map_or("-", |s| s.as_str())
}

pub struct MusicControlWidget {
    state: ThreadedReadonlyStateHandle<MusicControlServiceState>,
    settings: ReadWriteStateHandle<MusicControlSettings>,
    commander: Sender<MusicControlCommand>,
}

impl MusicControlWidget {
    pub fn new(service: &MusicControlService) -> MusicControlWidget {
        MusicControlWidget {
            state: service.state_handle(),
            settings: service.settings_handle(),
            commander: service.sender(),
        }
    }

    fn render_currently_playing(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut has_song = true;

            ui.vertical(|ui| {
                let state = self.state.read();
                let Some(currently_playing) = state.playback_info.as_ref() else {
                    has_song = false;
                    ui.label("No track currently playing");
                    return;
                };

                ui.small("Now playing:");
                ui.label(
                    egui::RichText::new(handle_null_string(currently_playing.track_name.as_ref()))
                        .size(16.0),
                );

                ui.label(handle_null_string(currently_playing.artist.as_ref()));
            });

            ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                if !has_song {
                    return;
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("Next").clicked() {
                        self.commander.send(MusicControlCommand::Next);
                    }
                    if ui.button("Previous").clicked() {
                        self.commander.send(MusicControlCommand::Previous);
                    }
                });
            });
        });
    }
}

impl egui::Widget for &mut MusicControlWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            self.render_currently_playing(ui);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                let mut settings = self.settings.write();
                ui.checkbox(&mut settings.pause_for_anthems, "Pause during anthems");
            });
        })
        .response
    }
}
