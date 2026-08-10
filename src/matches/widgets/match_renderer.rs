use super::super::{MatchInfo, MatchPlayer};
use crate::{
    common::channel::Sender,
    matches::apis::PlayerSkillInformation,
    player_info::PlayerInfoServiceCommand,
    rocket_league::{Platform, Playlist, Rank, Team},
    stats_api::TeamScores,
};
use eframe::egui::{self, Color32};
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub struct MatchRenderer<'a> {
    match_info: &'a MatchInfo,
    is_open: Option<&'a mut bool>,
    showing_stats_for: &'a mut Option<(String, String)>,
    player_info_sender: &'a Sender<PlayerInfoServiceCommand>,
}

impl<'a> MatchRenderer<'a> {
    pub fn new(
        match_info: &'a MatchInfo,
        is_open: Option<&'a mut bool>,
        showing_stats_for: &'a mut Option<(String, String)>,
        player_info_sender: &'a Sender<PlayerInfoServiceCommand>,
    ) -> MatchRenderer<'a> {
        MatchRenderer {
            match_info,
            is_open,
            showing_stats_for,
            player_info_sender,
        }
    }

    fn render_header(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            ui.label(format!("{}", self.match_info.playlist));

            if self.match_info.playlist.is_singleplayer() {
                return;
            }

            if let Some(finished) = &self.match_info.finish {
                if let Some(winner) = finished.winner.or_else(|| {
                    match self
                        .match_info
                        .score
                        .blue
                        .cmp(&self.match_info.score.orange)
                    {
                        Ordering::Greater => Some(Team::Blue),
                        Ordering::Less => Some(Team::Orange),
                        Ordering::Equal => None,
                    }
                }) {
                    ui.label(bold_text(if winner == self.match_info.our_team {
                        "Win"
                    } else {
                        "Loss"
                    }));
                }
            } else {
                ui.label("In progress");
            }

            score_labels(ui, &self.match_info.score, self.match_info.our_team);

            if let Some(finished) = &self.match_info.finish {
                let (text, refresh_in) = format_seconds(
                    SystemTime::now()
                        .duration_since(finished.timestamp)
                        .unwrap()
                        .as_secs(),
                );

                ui.label(text);
                ui.request_repaint_after(refresh_in);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if let Some(is_open) = &mut self.is_open {
                    let text = if **is_open { "Close" } else { "View" };
                    ui.toggle_value(is_open, text);
                }
            });
        })
        .response
    }

    fn render_player(&mut self, ui: &mut egui::Ui, match_player: &MatchPlayer) {
        // rank in this gamemode
        if let Some(skill) = &match_player.skill {
            self.render_player_rank_cell(ui, skill);
        } else {
            center_label(ui, "-");
        }

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;

            ui.horizontal(|ui| {
                let name_color = if match_player.left {
                    Color32::GRAY
                } else if match_player.is_local_player {
                    ui.visuals().strong_text_color()
                } else {
                    match match_player.data.team {
                        Team::Blue => Color32::from_rgb(64, 128, 255),
                        Team::Orange => Color32::ORANGE,
                    }
                };

                let name_label = ui.add(
                    egui::Label::new(
                        bold_text(match_player.display_name())
                            .color(name_color)
                            .size(15.0),
                    )
                    .sense(egui::Sense::CLICK)
                    .extend(),
                );

                name_label.context_menu(|ui| {
                    if ui.button("Copy player id").clicked() {
                        ui.ctx().copy_text(match_player.data.platform_id.clone());
                    }
                });

                if match_player.data.platform != Platform::Bot {
                    if name_label.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    if name_label.clicked() {
                        self.player_info_sender
                            .send(PlayerInfoServiceCommand::OpenPlayer(
                                match_player.data.clone(),
                            ));
                    }
                }

                ui.label(
                    egui::RichText::new(match_player.data.platform.to_string())
                        .color(ui.visuals().weak_text_color()),
                );
            });

            if let Some(skill) = &match_player.skill {
                MatchRenderer::render_rank_list(ui, match_player.left, skill);
            }
        });

        center_label(ui, match_player.data.stats.score.to_string());

        if ui
            .add_enabled(
                !matches!(match_player.data.platform, Platform::Bot),
                egui::Button::new("More"),
            )
            .clicked()
        {
            *self.showing_stats_for = Some((
                match_player.display_name().to_owned(),
                match_player.data.platform_id.clone(),
            ));
        }

        ui.end_row();
    }

    fn render_player_rank_cell(
        &mut self,
        ui: &mut egui::Ui,
        skill_info: &Arc<PlayerSkillInformation>,
    ) {
        let playlist_to_show = self
            .match_info
            .playlist
            .in_ranked()
            .or_else(|| Playlist::infer_from_player_count(self.match_info.players.len() as u8));

        let Some(playlist_to_show) = playlist_to_show else {
            center_label(ui, "-");
            return;
        };

        let Some(rank) = skill_info.get_playlist(playlist_to_show) else {
            center_label(ui, "-");
            return;
        };

        center_layout(ui, 28.0, |ui| {
            if rank.rank_is_estimate {
                ui.add(
                    egui::Image::new(Rank::Unranked.to_image())
                        .fit_to_exact_size(egui::vec2(28.0, 28.0)),
                )
                .on_hover_text(format!("Unranked in {playlist_to_show}"))
            } else {
                ui.add(
                    egui::Image::new(rank.rank.to_image())
                        .fit_to_exact_size(egui::vec2(28.0, 28.0)),
                )
                .on_hover_text(format!(
                    "{playlist_to_show} rank: {}{}",
                    rank.rank.as_str(),
                    rank.div
                ))
            }
        });
    }

    fn render_rank_list(ui: &mut egui::Ui, muted: bool, skill: &Arc<PlayerSkillInformation>) {
        ui.horizontal(|ui| {
            let modes = [
                skill.get_playlist(Playlist::RankedSoloDuel),
                skill.get_playlist(Playlist::RankedTeamDoubles),
                skill.get_playlist(Playlist::RankedStandard),
            ];

            for mode in modes {
                // per-rank mmr + icon
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;

                    if let Some(mode) = mode {
                        let image = ui.image(mode.rank.to_image());
                        if mode.rank_is_estimate {
                            image.on_hover_text("Estimated rank");
                        } else {
                            image.on_hover_text(
                                mode.rank.as_str().to_string() + &mode.div.to_string(),
                            );
                        }

                        if muted {
                            ui.label(mode.mmr.to_string());
                        } else {
                            ui.label(
                                egui::RichText::new(mode.mmr.to_string())
                                    .color(mode.rank.to_color()),
                            );
                        }
                    } else {
                        ui.image(Rank::Unranked.to_image());
                        ui.label(egui::RichText::new("---").color(Rank::Unranked.to_color()));
                    }
                });
            }
        });
    }

    fn render_stats_window(&self, ui: &mut egui::Ui, player: &(String, String)) -> bool {
        let mut window_is_open = true;

        let Some(player_details) = self
            .match_info
            .players
            .iter()
            .find(|p| p.data.platform_id == player.1)
        else {
            return false;
        };

        let window_title = format!("{}'s Stats", player.0);
        egui::Window::new(&window_title)
            .open(&mut window_is_open)
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                egui::Grid::new(&window_title)
                    .spacing(egui::vec2(8.0, 8.0))
                    .striped(true)
                    .show(ui, |ui| {
                        let stats = &player_details.data.stats;

                        ui.strong("Score");
                        center_label(ui, stats.score.to_string());
                        ui.end_row();

                        ui.strong("Goals");
                        center_label(ui, stats.goals.to_string());
                        ui.end_row();

                        ui.strong("Assists");
                        center_label(ui, stats.assists.to_string());
                        ui.end_row();

                        ui.strong("Saves");
                        center_label(ui, stats.saves.to_string());
                        ui.end_row();

                        ui.strong("Shots");
                        center_label(ui, stats.shots.to_string());
                        ui.end_row();

                        ui.strong("Touches");
                        center_label(ui, stats.touches.to_string());
                        ui.end_row();
                    })
            });

        window_is_open
    }
}

impl egui::Widget for MatchRenderer<'_> {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        let header_response = self.render_header(ui);
        if let Some(is_open) = &self.is_open
            && !**is_open
        {
            return header_response;
        }

        if let Some(player) = self.showing_stats_for.as_ref() {
            let stay_open = self.render_stats_window(ui, player);
            if !stay_open {
                *self.showing_stats_for = None;
            }
        }

        egui::Grid::new(self.match_info.started_at)
            .spacing(egui::vec2(8.0, 12.0))
            .striped(true)
            .show(ui, |ui| {
                center_label(ui, bold_text("Rank"));
                ui.label(bold_text("Player"));
                center_label(ui, bold_text("Score"));
                ui.label(""); // more button

                ui.end_row();

                if self.match_info.finish.is_some() {
                    for player in filter_useless_bots(&self.match_info.players) {
                        self.render_player(ui, player);
                    }
                } else {
                    for player in &self.match_info.players {
                        self.render_player(ui, player);
                    }
                }
            })
            .response
    }
}

fn score_labels(ui: &mut egui::Ui, scores: &TeamScores, priority: Team) {
    let blue_text = egui::RichText::new(scores.blue.to_string()).color(Color32::LIGHT_BLUE);
    let orange_text = egui::RichText::new(scores.orange.to_string()).color(Color32::LIGHT_RED);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if priority == Team::Blue {
            ui.label(blue_text);
            ui.label("-");
            ui.label(orange_text);
        } else {
            ui.label(orange_text);
            ui.label("-");
            ui.label(blue_text);
        }
    });
}

fn pluralize_ago(count: u64, word: &str, suffix: &str) -> String {
    format!(
        "{count} {word}{} {suffix}",
        if count == 1 { "" } else { "s" }
    )
}

const ONE_SECOND: Duration = Duration::from_secs(1);
const ONE_MINUTE: Duration = Duration::from_mins(1);

pub fn format_seconds(seconds: u64) -> (String, Duration) {
    match seconds {
        ..60 => (pluralize_ago(seconds, "second", "ago"), ONE_SECOND),
        60..3600 => (pluralize_ago(seconds / 60, "minute", "ago"), ONE_MINUTE),
        3600.. => (
            format!(
                "{}{}",
                pluralize_ago(seconds / 3600, "hour", ""),
                pluralize_ago((seconds % 3600) / 60, "minute", "ago")
            ),
            ONE_MINUTE,
        ),
    }
}

fn center_layout<R>(
    ui: &mut egui::Ui,
    height: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
        add_contents,
    )
}

fn center_label(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
) -> egui::InnerResponse<egui::Response> {
    center_layout(ui, 16.0, |ui| ui.label(text))
}

fn bold_text(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong()
}

fn filter_useless_bots(players: &[MatchPlayer]) -> impl Iterator<Item = &MatchPlayer> {
    players
        .iter()
        .filter(|p| p.data.platform != Platform::Bot || p.data.stats.score != 0)
}
