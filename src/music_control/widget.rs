use super::{
    controller::PlaybackStatus,
    service::{
        MusicControlCommand, MusicControlService, MusicControlServiceState, MusicControlSettings,
    },
};
use crate::common::{ReadWriteStateHandle, ThreadedReadonlyStateHandle, channel::Sender};
use crate::core::app::Panel;
use eframe::egui::{self, ImageSource, load::Bytes};
use std::time::{Duration, SystemTime};

fn handle_null_string(s: Option<&String>) -> &str {
    s.map_or("-", |s| s.as_str())
}

pub struct MusicControlWidget {
    state: ThreadedReadonlyStateHandle<MusicControlServiceState>,
    settings: ReadWriteStateHandle<MusicControlSettings>,
    commander: Sender<MusicControlCommand>,
    last_progress: (SystemTime, Duration), // had progress Duration at time SystemTime
}

impl MusicControlWidget {
    pub fn new(service: &MusicControlService) -> MusicControlWidget {
        MusicControlWidget {
            state: service.state_handle(),
            settings: service.settings_handle(),
            commander: service.sender(),
            last_progress: (SystemTime::now(), Duration::ZERO),
        }
    }

    fn render_currently_playing(&mut self, ui: &mut egui::Ui) {
        let state = self.state.read();
        let Some(currently_playing) = state.playback_info.as_ref() else {
            ui.label("No track currently playing");
            return;
        };

        ui.vertical(|ui| {
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.set_max_height(64.0);

                if let Some(thumbnail) = &currently_playing.thumbnail {
                    ui.add(
                        egui::Image::new(ImageSource::Bytes {
                            uri: format!(
                                "bytes://{}.{}",
                                handle_null_string(currently_playing.track_name.as_ref()),
                                thumbnail.extension
                            )
                            .into(),
                            bytes: Bytes::Shared(thumbnail.bytes.clone()),
                        })
                        .corner_radius(egui::CornerRadius::same(4))
                        .max_height(64.0),
                    );
                }

                let title_text =
                    egui::RichText::new(handle_null_string(currently_playing.track_name.as_ref()))
                        .size(16.0)
                        .strong()
                        .into();

                ui.add(TitleArtistStack {
                    title: title_text,
                    artist: handle_null_string(currently_playing.artist.as_ref()).into(),
                    max_height: 64.0,
                });
            });

            if let Some(progress) = currently_playing.progress
                && let Some(song_length) = &currently_playing.song_length
            {
                let real_progress =
                    if matches!(currently_playing.status, Some(PlaybackStatus::Playing))
                        && progress == self.last_progress.1
                    {
                        SystemTime::now()
                            .duration_since(self.last_progress.0)
                            .unwrap()
                            + self.last_progress.1
                    } else {
                        self.last_progress = (SystemTime::now(), progress);
                        progress
                    };

                ui.add_space(4.0);
                ui.add(
                    egui::ProgressBar::new(real_progress.as_secs_f32() / song_length.as_secs_f32())
                        .desired_height(6.0),
                );
            }

            ui.add_space(4.0);
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
                                self.commander.send(MusicControlCommand::Play);
                            }
                            PlaybackStatus::Playing => {
                                self.commander.send(MusicControlCommand::Pause);
                            }
                            _ => unreachable!("Button shouldn't exist right now"),
                        }
                    }
                });
            });
        });
    }
}

impl Panel for MusicControlWidget {
    fn name(&self) -> &'static str {
        "Music"
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
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

struct TitleArtistStack {
    title: egui::WidgetText,
    artist: egui::WidgetText,
    max_height: f32,
}

impl egui::Widget for TitleArtistStack {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let title_alloc = self.title.clone().into_galley(
            ui,
            None,
            ui.available_width(),
            egui::FontSelection::Default,
        );
        let artist_alloc = self.artist.clone().into_galley(
            ui,
            None,
            ui.available_width(),
            egui::FontSelection::Default,
        );
        let widgets_height = title_alloc.rect.height() + artist_alloc.rect.height();

        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), self.max_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.add_space((self.max_height - widgets_height) / 2.0);
                ui.label(self.title);
                ui.label(self.artist);
            },
        )
        .response
    }
}
