use crate::{
    common::{ReadWriteStateHandle, ThreadedReadonlyStateHandle, channel::Sender},
    music_control::{
        MusicControlCommand, MusicControlService, MusicControlServiceState, MusicControlSettings,
        controller::PlaybackStatus,
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
        let state = self.state.read();
        let Some(currently_playing) = state.playback_info.as_ref() else {
            ui.label("No track currently playing");
            return;
        };

        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(handle_null_string(currently_playing.track_name.as_ref()))
                    .size(16.0),
            );

            ui.label(handle_null_string(currently_playing.artist.as_ref()));

            if let Some(progress) = &currently_playing.progress
                && let Some(song_length) = &currently_playing.song_length
            {
                ui.add_space(4.0);
                ui.add(
                    egui::ProgressBar::new(progress.as_secs_f32() / song_length.as_secs_f32())
                        .desired_height(6.0),
                );
            }

            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if ui.button("Previous").clicked() {
                    self.commander.send(MusicControlCommand::Previous);
                }
                if ui.button("Next").clicked() {
                    self.commander.send(MusicControlCommand::Next);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    let Some(status) = &currently_playing.status else {
                        return;
                    };

                    if ui
                        .button(match status {
                            PlaybackStatus::Paused => "Play",
                            PlaybackStatus::Playing => "Pause",
                            _ => return,
                        })
                        .clicked()
                    {
                        match status {
                            PlaybackStatus::Paused => {
                                self.commander.send(MusicControlCommand::Play)
                            }
                            PlaybackStatus::Playing => {
                                self.commander.send(MusicControlCommand::Pause)
                            }
                            _ => unreachable!("Button shouldn't exist right now"),
                        }
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
