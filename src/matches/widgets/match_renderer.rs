use super::super::MatchPlayer;
use crate::{
    common::{ReadonlyStateHandle, channel::Sender, timefmt::format_seconds},
    matches::{
        MatchesServiceState, StrippedPlayer, StrippedPlayerType,
        apis::{PlayerSkillInformation, PlaylistSkillInformation},
        service::MatchType,
        widgets::buddy_badges::get_badges,
    },
    player_info::PlayerInfoServiceCommand,
    rocket_league::{Platform, Playlist, Rank, Team},
    stats_api::TeamScores,
};
use eframe::egui::{self, Color32, include_image};
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::SystemTime;

pub enum BuddyStatsOption<'a> {
    Yes(&'a ReadonlyStateHandle<MatchesServiceState>),
    No,
}

const DEFAULT_AVATAR_EXTRA_SCALE: f32 = {
    let image_heght = 128.0;
    let icon_height = 92.0;
    let total_padding = image_heght - icon_height;
    let one_size_padding = total_padding / 2.0;
    one_size_padding / 4.0
};

pub struct MatchRenderer<'a> {
    match_info: &'a MatchType<'a>,
    is_open: Option<&'a mut bool>,
    showing_stats_for: &'a mut Option<(String, String)>,
    player_info_sender: &'a Sender<PlayerInfoServiceCommand>,
    buddy_stats: BuddyStatsOption<'a>,
}

impl<'a> MatchRenderer<'a> {
    pub fn new(
        match_info: &'a MatchType<'a>,
        is_open: Option<&'a mut bool>,
        showing_stats_for: &'a mut Option<(String, String)>,
        player_info_sender: &'a Sender<PlayerInfoServiceCommand>,
        buddy_stats: BuddyStatsOption<'a>,
    ) -> MatchRenderer<'a> {
        MatchRenderer {
            match_info,
            is_open,
            showing_stats_for,
            player_info_sender,
            buddy_stats,
        }
    }

    fn ranked_playlist(&self) -> Playlist {
        self.match_info
            .playlist()
            .in_ranked()
            .or_else(|| Playlist::infer_from_player_count(self.match_info.player_qty()))
            .unwrap_or_else(|| self.match_info.playlist())
    }

    fn render_header(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            let playlist = match self.match_info {
                MatchType::Old(o) => o.playlist,
                MatchType::Session(s) => s.playlist,
            };

            ui.label(format!("{playlist}"));

            if playlist.is_singleplayer() {
                return;
            }

            // if its an old match: use the old match winner
            // otherwise if session match is finished:
            //    try: get existing match winner
            //    otherwise: calculate winner based on score
            if let Some(winner) = match self.match_info {
                MatchType::Old(o) => Some(o.winner),
                MatchType::Session(s) => s.finish.as_ref().and_then(|f| {
                    f.winner
                        .or_else(|| match s.score.blue.cmp(&s.score.orange) {
                            Ordering::Greater => Some(Team::Blue),
                            Ordering::Less => Some(Team::Orange),
                            Ordering::Equal => None,
                        })
                }),
            } {
                ui.label(bold_text(if winner == self.match_info.our_team() {
                    "Win"
                } else {
                    "Loss"
                }));
            } else {
                ui.label("In progress");
            }

            score_labels(ui, self.match_info.score(), self.match_info.our_team());

            if let Some(end_time) = match self.match_info {
                MatchType::Old(o) => Some(o.end_time),
                MatchType::Session(s) => s.finish.as_ref().map(|f| f.timestamp),
            } {
                let (text, refresh_in) = format_seconds(
                    SystemTime::now()
                        .duration_since(end_time)
                        .unwrap()
                        .as_secs(),
                    false,
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
        if let Some(player_skills) = &match_player.skill
            && let Some(rank) = player_skills.get_playlist(self.ranked_playlist())
        {
            render_player_rank_cell(ui, rank);
        } else {
            center_label(ui, "-");
        }

        render_avatar(ui, match_player.avatar_url.as_ref().map(|u| u.as_str()));

        ui.vertical(|ui| {
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

                    if ui
                        .add_enabled(
                            match_player.avatar_url.is_some(),
                            egui::Button::new("Copy avatar url"),
                        )
                        .clicked()
                    {
                        ui.ctx().copy_text(
                            match_player
                                .avatar_url
                                .as_ref()
                                .unwrap()
                                .as_ref()
                                .to_owned(),
                        );
                    }
                });

                if !matches!(match_player.data.platform, Platform::Bot) {
                    if name_label.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    if name_label.clicked()
                        && let Some(cmd) = match_player.open_player_info_command()
                    {
                        self.player_info_sender.send(cmd);
                    }
                }

                self.show_buddy_stat_icon_maybe(ui, match_player);

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

    fn show_buddy_stat_icon_maybe(&self, ui: &mut egui::Ui, other_player: &MatchPlayer) {
        if other_player.is_local_player {
            return;
        }

        let BuddyStatsOption::Yes(match_service_state) = self.buddy_stats else {
            return;
        };

        let prev_matches = &match_service_state.read().prev_matches;
        let badges = get_badges(other_player, prev_matches);

        for badge in badges {
            ui.label(badge.badge).on_hover_text(badge.detail_text);
        }
    }

    fn render_stripped_player(&mut self, ui: &mut egui::Ui, player: &StrippedPlayer) {
        if let Some(rank) = &player.rank_in_mode {
            render_player_rank_cell(ui, rank);
        } else {
            center_label(ui, "-");
        }

        render_avatar(ui, player.avatar_url.as_ref().map(|u| u.as_str()));

        let name_color = if player.is_local_player() {
            ui.visuals().strong_text_color()
        } else {
            match player.team {
                Team::Blue => Color32::from_rgb(64, 128, 255),
                Team::Orange => Color32::ORANGE,
            }
        };

        let name_label = ui.add(
            egui::Label::new(bold_text(&player.name).color(name_color).size(15.0))
                .sense(egui::Sense::CLICK)
                .extend(),
        );

        name_label.context_menu(|ui| {
            if ui.button("Copy player id").clicked() {
                ui.ctx().copy_text(player.player_id.clone());
            }
        });

        if !matches!(player.platform, Platform::Bot) {
            if name_label.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            if name_label.clicked() {
                self.player_info_sender.send(PlayerInfoServiceCommand::Open(
                    player.platform,
                    match player.platform {
                        Platform::Steam => player.player_id.split('|').nth(1).unwrap().to_string(),
                        _ => player.name.clone(),
                    },
                ));
            }
        }

        ui.label(
            egui::RichText::new(player.platform.to_string()).color(ui.visuals().weak_text_color()),
        );

        ui.end_row();
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

    fn render_match_mmr_info(&self, ui: &mut egui::Ui) {
        let (avg_mmr, local_mmr) = match self.match_info {
            MatchType::Old(o) => (
                o.players
                    .iter()
                    .filter_map(|p| p.rank_in_mode.as_ref().map(|r| r.mmr))
                    .sum::<i16>()
                    / i16::try_from(
                        o.players
                            .iter()
                            .filter(|p| p.rank_in_mode.is_some())
                            .count()
                            .max(1),
                    )
                    .unwrap_or(1),
                o.players
                    .iter()
                    .find(|p| p.is_local_player())
                    .and_then(|p| p.rank_in_mode.as_ref())
                    .map(|r| r.mmr),
            ),
            MatchType::Session(s) => (
                s.players
                    .iter()
                    .filter_map(|p| {
                        p.skill
                            .as_ref()
                            .and_then(|sk| sk.get_playlist(self.ranked_playlist()))
                            .map(|sk| sk.mmr)
                    })
                    .sum::<i16>()
                    / i16::try_from(
                        s.players
                            .iter()
                            .filter(|p| {
                                p.skill.as_ref().is_some_and(|sk| {
                                    sk.get_playlist(self.ranked_playlist()).is_some()
                                })
                            })
                            .count()
                            .max(1),
                    )
                    .unwrap_or(1),
                s.players.iter().find(|p| p.is_local_player).and_then(|p| {
                    p.skill
                        .as_ref()
                        .and_then(|sk| sk.get_playlist(self.ranked_playlist()))
                        .map(|sk| sk.mmr)
                }),
            ),
        };

        ui.horizontal(|ui| {
            ui.label(format!("Lobby average: {avg_mmr}"));
            if let Some(local_mmr) = local_mmr {
                ui.label(format!("Yours: {local_mmr}"));
                let diff = local_mmr - avg_mmr;

                match local_mmr.cmp(&avg_mmr) {
                    Ordering::Greater => ui.colored_label(egui::Color32::GREEN, format!("+{diff}")),
                    Ordering::Less => ui.colored_label(egui::Color32::RED, format!("{diff}")),
                    Ordering::Equal => ui.label("+0"),
                };
            }
        });
    }

    fn render_stats_window(&self, ui: &mut egui::Ui, player: &(String, String)) -> bool {
        let mut window_is_open = true;

        let Some(stats) = (match self.match_info {
            MatchType::Session(s) => s
                .players
                .iter()
                .find(|p| p.data.platform_id == player.1)
                .map(|p| &p.data.stats),
            MatchType::Old(o) => o
                .players
                .iter()
                .find(|p| p.player_id == player.1)
                .and_then(|p| match &p.player_type {
                    // stats are only there if its a local player
                    StrippedPlayerType::LocalPlayer(stats) => Some(stats),
                    StrippedPlayerType::RemotePlayer => None,
                }),
        }) else {
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

        ui.vertical(|ui| {
            egui::Grid::new(self.match_info.started_at())
                .spacing(egui::vec2(8.0, 12.0))
                .striped(true)
                .min_col_width(0.0)
                .show(ui, |ui| {
                    center_label(ui, bold_text("Rank"));
                    {
                        // so it doesnt add space between the avatar and player details
                        let mut rect = ui
                            .allocate_space(egui::vec2(
                                24.0,
                                ui.style().text_styles[&egui::TextStyle::Body].size + 2.0,
                            ))
                            .1;
                        rect.min.x += 12.0;

                        ui.place(rect, egui::Label::new(bold_text("Player")).extend());
                    }

                    ui.label("");
                    if matches!(self.match_info, MatchType::Session(_)) {
                        center_label(ui, bold_text("Score"));
                        ui.label(""); // more button
                    }

                    ui.end_row();

                    match self.match_info {
                        MatchType::Old(o) => {
                            for player in filter_useless_bots(&o.players, |p| p.platform, |_| 1) {
                                self.render_stripped_player(ui, player);
                            }
                        }
                        MatchType::Session(s) => {
                            for player in filter_useless_bots(
                                &s.players,
                                |p| p.data.platform,
                                |p| p.data.stats.score,
                            ) {
                                self.render_player(ui, player);
                            }
                        }
                    }
                });

            ui.add_space(4.0);
            self.render_match_mmr_info(ui);
        })
        .response
    }
}

fn render_avatar(ui: &mut egui::Ui, url: Option<&str>) {
    if let Some(avatar_url) = url {
        ui.add(
            egui::Image::new(avatar_url)
                .fit_to_exact_size(egui::vec2(28.0, 28.0))
                .corner_radius(egui::CornerRadius::same(4)),
        );
    } else {
        let rect = ui
            .allocate_space(egui::vec2(28.0, 28.0))
            .1
            .expand(DEFAULT_AVATAR_EXTRA_SCALE);
        egui::Image::new(include_image!("../../../assets/Avatar_icon.webp"))
            .corner_radius(egui::CornerRadius::same(4))
            .paint_at(ui, rect);
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

fn filter_useless_bots<T, GP: Fn(&T) -> Platform + 'static, GS: Fn(&T) -> u16 + 'static>(
    players: &[T],
    get_platform: GP,
    get_score: GS,
) -> impl Iterator<Item = &T> {
    players
        .iter()
        .filter(move |p| get_platform(p) != Platform::Bot || get_score(p) != 0)
}

fn render_player_rank_cell(ui: &mut egui::Ui, rank: &PlaylistSkillInformation) {
    center_layout(ui, 28.0, |ui| {
        if rank.rank_is_estimate {
            ui.add(
                egui::Image::new(Rank::Unranked.to_image())
                    .fit_to_exact_size(egui::vec2(28.0, 28.0)),
            )
            .on_hover_text(format!("Unranked in {}", rank.playlist))
        } else {
            ui.add(egui::Image::new(rank.rank.to_image()).fit_to_exact_size(egui::vec2(28.0, 28.0)))
                .on_hover_text(format!(
                    "{} rank: {}{}",
                    rank.playlist,
                    rank.rank.as_str(),
                    rank.div
                ))
        }
    });
}
