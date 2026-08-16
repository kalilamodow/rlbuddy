use crate::{
    common::{ReadonlyStateHandle, timefmt::format_seconds},
    matches::{MatchType, MatchesServiceState},
    rocket_league::Playlist,
};
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoint, PlotPoints};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Default)]
pub struct PlayerLongtimeStats {
    goals: u64,
    assists: u64,
    saves: u64,
    shots: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MyStatsWidgetSettings {
    pub selected_playlist: Option<Playlist>,
}

pub struct MyStatsWidget {
    matches_state: ReadonlyStateHandle<MatchesServiceState>,
    settings: MyStatsWidgetSettings,
}

impl MyStatsWidget {
    pub fn new(
        matches_state: ReadonlyStateHandle<MatchesServiceState>,
        settings: MyStatsWidgetSettings,
    ) -> Self {
        Self {
            matches_state,
            settings,
        }
    }

    pub fn clone_settings(&self) -> MyStatsWidgetSettings {
        self.settings.clone()
    }

    fn render_streak_header(&self, ui: &mut egui::Ui) {
        let state = self.matches_state.read();
        let session_matches = || {
            state.prev_matches.iter().filter_map(|m| match m {
                MatchType::Session(s) => Some(s),
                MatchType::Old(_) => None,
            })
        };

        let count = session_matches().count();
        ui.horizontal(|ui| {
            ui.strong(format!("{} matches", count));
            if count < 1 {
                return;
            }

            let won_last_match = session_matches().last().unwrap().is_win();
            let streak = session_matches()
                .rev()
                .take_while(|m| m.is_win() == won_last_match)
                .count();

            ui.label(format!(
                "{} {streak}",
                if won_last_match { "🔥" } else { "❄️" }
            ));

            if !won_last_match && streak > 4 {
                ui.label("<!> Consider taking a break");
            }
        });
    }

    fn render_settings_header(&mut self, ui: &mut egui::Ui) {
        let state = self.matches_state.read();
        let mut choosable_playlists: Vec<Playlist> = state
            .prev_matches
            .iter()
            .map(MatchType::playlist)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        choosable_playlists.sort();

        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Choose playlist")
                .selected_text(
                    self.settings
                        .selected_playlist
                        .map_or("None", Playlist::as_str),
                )
                .show_ui(ui, |ui| {
                    for playlist in choosable_playlists {
                        ui.selectable_value(
                            &mut self.settings.selected_playlist,
                            Some(playlist),
                            playlist.as_str(),
                        );
                    }
                });
        });
    }

    fn render_mmr_graph(&mut self, ui: &mut egui::Ui) {
        let state = self.matches_state.read();
        if state.prev_matches.is_empty() {
            ui.label("No data. Play with rlbuddy open to see your progress!");
            return;
        }

        let Some(selected_playlist) = self.settings.selected_playlist else {
            ui.label("Select a playlist");
            return;
        };

        let points: Vec<PlotPoint> = state
            .prev_matches
            .iter()
            .filter(|m| m.playlist() == selected_playlist)
            .filter_map(|m| match m {
                MatchType::Old(_) => None,
                MatchType::Session(s) => s
                    .players
                    .iter()
                    .find(|p| p.is_local_player)
                    .and_then(|p| p.skill.as_ref())
                    .and_then(|sk| sk.get_playlist(s.playlist.in_ranked().unwrap_or(s.playlist)))
                    .map(|skill| PlotPoint {
                        x: m.started_at()
                            .duration_since(UNIX_EPOCH)
                            .expect("its before 1970")
                            .as_secs_f64(),
                        y: skill.mmr.into(),
                    }),
            })
            .collect();

        if points.len() <= 1 {
            ui.label("No points to graph");
            return;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("its before 1970")
            .as_secs_f64();

        Plot::new("mmr graph plot")
            .allow_axis_zoom_drag(false)
            .allow_zoom(false)
            .allow_boxed_zoom(false)
            .allow_scroll(false)
            .allow_drag(false)
            .view_aspect(1f32)
            .cursor_color(egui::Color32::TRANSPARENT)
            .show_crosshair(false)
            .clamp_grid(true)
            // space on actual points
            .x_grid_spacer(|_| {
                points
                    .iter()
                    .zip(points.iter().skip(1))
                    .map(|p| egui_plot::GridMark {
                        value: p.0.x,
                        step_size: p.1.x - p.0.x,
                    })
                    .collect()
            })
            .x_axis_formatter(|point, _| format_seconds((now - point.value).round() as u64, true).0)
            .show(ui, |ui| {
                ui.line(Line::new("MMR", PlotPoints::Borrowed(&points)));
            });
    }

    fn render_win_loss(&mut self, ui: &mut egui::Ui) {
        let state = self.matches_state.read();
        let session_matches = || {
            state.prev_matches.iter().filter_map(|m| match m {
                MatchType::Session(s) => Some(s),
                MatchType::Old(_) => None,
            })
        };

        let total_games = session_matches().count();
        let wins = session_matches()
            .filter(|s| s.finish.as_ref().and_then(|f| f.winner) == Some(s.our_team))
            .count();

        ui.horizontal(|ui| {
            ui.label("W/L");
            if total_games == 0 {
                ui.label("-");
                return;
            }
            ui.label(format!(
                "{wins}/{} ({}%)",
                total_games - wins,
                (wins * 100 / total_games)
            ));
        });
    }

    fn render_stats(&mut self, ui: &mut egui::Ui) {
        let state = self.matches_state.read();
        let all_stats = state.prev_matches.iter().filter_map(|m| match m {
            MatchType::Session(s) => s
                .players
                .iter()
                .find_map(|p| p.is_local_player.then(|| &p.data.stats)),
            MatchType::Old(_) => None,
        });

        let totals: PlayerLongtimeStats =
            all_stats.fold(PlayerLongtimeStats::default(), |mut total, stats| {
                total.goals += stats.goals as u64;
                total.shots += stats.shots as u64;
                total.assists += stats.assists as u64;
                total.saves += stats.saves as u64;
                total
            });

        ui.horizontal(|ui| {
            ui.label("Goals:");
            ui.label(totals.goals.to_string());

            ui.label("Assists:");
            ui.label(totals.assists.to_string());

            ui.label("Shots:");
            ui.label(totals.shots.to_string());

            ui.label("Saves:");
            ui.label(totals.saves.to_string());
        });
    }
}

impl egui::Widget for &mut MyStatsWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            self.render_streak_header(ui);
            ui.horizontal(|ui| {
                self.render_stats(ui);
                self.render_win_loss(ui);
            });

            ui.separator();

            self.render_settings_header(ui);
            self.render_mmr_graph(ui);
        })
        .response
    }
}
