use crate::{
    common::{ReadonlyStateHandle, timefmt::format_seconds},
    matches::{MatchType, MatchesServiceState, StrippedPlayerType},
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
pub struct PlayerAllTimeStats {
    goals: u64,
    assists: u64,
    saves: u64,
    shots: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MyStatsWidgetSettings {
    pub session_only: bool,
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

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.checkbox(&mut self.settings.session_only, "Session only");
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
            .filter(|m| !self.settings.session_only || matches!(m, MatchType::Session(_)))
            .filter_map(|m| {
                (match m {
                    MatchType::Old(o) => o
                        .players
                        .iter()
                        .find(|p| matches!(p.player_type, StrippedPlayerType::LocalPlayer(_)))
                        .and_then(|p| p.rank_in_mode.as_ref()),
                    MatchType::Session(s) => s
                        .players
                        .iter()
                        .find(|p| p.is_local_player)
                        .and_then(|p| p.skill.as_ref())
                        .and_then(|sk| {
                            sk.get_playlist(s.playlist.in_ranked().unwrap_or(s.playlist))
                        }),
                })
                .map(|skill| PlotPoint {
                    x: m.started_at()
                        .duration_since(UNIX_EPOCH)
                        .expect("its before 1970")
                        .as_secs_f64(),
                    y: skill.mmr.into(),
                })
            })
            .collect();

        if points.len() <= 1 {
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
        let wins = state.prev_matches.iter().filter(|m| match m {
            MatchType::Old(o) => o.winner == o.our_team(),
            MatchType::Session(s) => s.finish.as_ref().and_then(|f| f.winner) == Some(m.our_team()),
        });
        let losses = state.prev_matches.iter().filter(|m| match m {
            MatchType::Old(o) => o.winner != o.our_team(),
            MatchType::Session(s) => s.finish.as_ref().and_then(|f| f.winner) != Some(m.our_team()),
        });

        let wins_qty = if self.settings.session_only {
            wins.filter(|w| matches!(w, MatchType::Session(_))).count()
        } else {
            wins.count()
        };

        let losses_qty = if self.settings.session_only {
            losses
                .filter(|w| matches!(w, MatchType::Session(_)))
                .count()
        } else {
            losses.count()
        };

        let total_games = wins_qty + losses_qty;

        ui.horizontal(|ui| {
            ui.strong("W/L");
            if total_games == 0 {
                ui.label("-");
                return;
            }
            ui.label(format!(
                "{wins_qty}/{losses_qty} ({}%)",
                (wins_qty * 100 / total_games)
            ));
        });
    }

    fn render_stats(&mut self, ui: &mut egui::Ui) {
        let state = self.matches_state.read();
        let all_stats = state.prev_matches.iter().filter_map(|m| match m {
            MatchType::Old(_) if self.settings.session_only => None,
            MatchType::Session(s) => s
                .players
                .iter()
                .find_map(|p| p.is_local_player.then(|| &p.data.stats)),
            MatchType::Old(o) => o.players.iter().find_map(|p| match &p.player_type {
                StrippedPlayerType::LocalPlayer(l) => Some(l),
                StrippedPlayerType::RemotePlayer => None,
            }),
        });

        let totals: PlayerAllTimeStats =
            all_stats.fold(PlayerAllTimeStats::default(), |mut total, stats| {
                total.goals += stats.goals as u64;
                total.shots += stats.shots as u64;
                total.assists += stats.assists as u64;
                total.saves += stats.saves as u64;
                total
            });

        ui.horizontal(|ui| {
            ui.strong("Goals");
            ui.label(totals.goals.to_string());

            ui.strong("Assists");
            ui.label(totals.assists.to_string());

            ui.strong("Shots");
            ui.label(totals.shots.to_string());

            ui.strong("Saves");
            ui.label(totals.saves.to_string());
        });
    }
}

impl egui::Widget for &mut MyStatsWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical_centered_justified(|ui| {
            self.render_settings_header(ui);

            ui.horizontal(|ui| {
                self.render_win_loss(ui);
                ui.add_space(2.0);
                self.render_stats(ui);
            });

            self.render_mmr_graph(ui);
        })
        .response
    }
}
