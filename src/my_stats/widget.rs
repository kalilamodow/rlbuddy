use crate::common::channel::Sender;
use crate::common::{ReadonlyStateHandle, timefmt::format_seconds};
use crate::core::app::Panel;
use crate::my_stats::MyStatsService;
use crate::my_stats::service::{
    MmrGraphState, MyStatsServiceCommand, MyStatsServiceState, StreakType,
};
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use std::borrow::Cow;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct MyStatsWidget {
    state: ReadonlyStateHandle<MyStatsServiceState>,
    sender: Sender<MyStatsServiceCommand>,
}

impl MyStatsWidget {
    pub fn new(service: &MyStatsService) -> Self {
        Self {
            state: service.state_handle(),
            sender: service.sender(),
        }
    }

    fn render_streak_header(&self, ui: &mut egui::Ui) {
        let state = self.state.read();
        ui.horizontal(|ui| {
            let Some(streak) = &state.streak else {
                ui.strong("No matches");
                return;
            };

            ui.label(format!(
                "{} {}",
                match streak.cat {
                    StreakType::Win => '🔥',
                    StreakType::Loss => '❄',
                },
                streak.count
            ));

            if matches!(streak.cat, StreakType::Loss) && streak.count > 4 {
                ui.label("<!> Consider taking a break");
            }
        });
    }

    fn render_settings_header(&mut self, ui: &mut egui::Ui) {
        let state = self.state.read();

        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Choose playlist")
                .selected_text(
                    state
                        .selected_graph_playlist
                        .map_or(Cow::Borrowed("None"), |p| p.as_str()),
                )
                .show_ui(ui, |ui| {
                    for &playlist in &state.choosable_playlists {
                        if ui
                            .selectable_label(
                                state.selected_graph_playlist == Some(playlist),
                                playlist.as_str(),
                            )
                            .clicked()
                        {
                            self.sender
                                .send(MyStatsServiceCommand::SetSelectedGraphPlaylist(playlist));
                        }
                    }
                });
        });
    }

    fn render_mmr_graph(&mut self, ui: &mut egui::Ui) {
        let state = self.state.read();
        let (points, times) = match &state.graph {
            MmrGraphState::NoPlaylist => {
                ui.label("Select a playlist to graph");
                return;
            }
            MmrGraphState::NotEnoughData => {
                ui.label("Play a bit more to see data in this playlist!");
                return;
            }
            MmrGraphState::Graph(points) => points,
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("its before 1970")
            .as_secs_f64();

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        Plot::new("mmr graph plot")
            .allow_axis_zoom_drag(false)
            .allow_zoom(false)
            .allow_boxed_zoom(false)
            .allow_scroll(false)
            .allow_drag(false)
            .view_aspect(1.0)
            .cursor_color(egui::Color32::TRANSPARENT)
            .show_crosshair(false)
            .clamp_grid(true)
            .x_axis_formatter(|point, _| {
                let x = point.value as usize;
                let Some(time) = times[x] else {
                    return "".into();
                };

                format_seconds(
                    (now - time.duration_since(UNIX_EPOCH).unwrap().as_secs_f64()) as u64,
                    true,
                )
                .into_owned()
            })
            .show(ui, |ui| {
                ui.line(Line::new("MMR", PlotPoints::Borrowed(points)));
            });
    }

    fn render_win_loss(&mut self, ui: &mut egui::Ui) {
        let state = self.state.read();

        ui.horizontal(|ui| {
            let Some(wl) = &state.wl else {
                return;
            };
            ui.label("W/L");

            let percent = {
                let ratio = wl.wins as f32 * 100.0 / (wl.wins + wl.losses) as f32;
                let pct = (ratio * 100.0).round() / 100.0;

                if pct.is_nan() {
                    None
                } else {
                    Some(pct.to_string())
                }
            };

            ui.label(format!(
                "{}/{} ({}%)",
                wl.wins,
                wl.losses,
                percent.as_deref().unwrap_or("-")
            ));
        });
    }

    fn render_stats(&mut self, ui: &mut egui::Ui) {
        let state = self.state.read();

        #[allow(clippy::cast_precision_loss)]
        let stat_text = |text: &str, val: u64| {
            format!(
                "{text}: {val} ({:.2}pg)",
                val as f32 / state.number_of.games as f32
            )
        };

        ui.horizontal(|ui| {
            // also prevents a div by 0 error in stat_text()
            if state.number_of.games == 0 {
                return;
            }

            ui.label(stat_text("Goals", state.number_of.goals));
            ui.label(stat_text("Assists", state.number_of.assists));
            ui.label(stat_text("Shots", state.number_of.shots));
            ui.label(stat_text("Saves", state.number_of.saves));
        });
    }
}

impl Panel for MyStatsWidget {
    fn name(&self) -> &'static str {
        "Session Stats"
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            self.render_streak_header(ui);
            self.render_stats(ui);
            self.render_win_loss(ui);

            ui.separator();

            self.render_settings_header(ui);
            self.render_mmr_graph(ui);
        })
        .response
    }
}
