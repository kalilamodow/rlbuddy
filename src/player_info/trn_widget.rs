use std::str::FromStr;

use eframe::egui;

use crate::{
    player_info::trn::{PeakRatingSegment, PlayerKey, PlaylistSegment, ProfileData, Segment},
    rocket_league::Rank,
};

pub struct TrackerWidget<'a> {
    profile: &'a ProfileData,
    player: &'a PlayerKey,
}

impl<'a> TrackerWidget<'a> {
    pub fn new(profile: &'a ProfileData, player: &'a PlayerKey) -> Self {
        Self { profile, player }
    }

    fn render_overview(&self, ui: &mut egui::Ui) {
        let Some(overview) = self.profile.segments.iter().find_map(|s| {
            if let Segment::Overview(ov) = s {
                Some(ov)
            } else {
                None
            }
        }) else {
            return;
        };

        egui::Grid::new(format!(
            "overview for {}",
            self.profile.platform_info.platform_user_handle
        ))
        .show(ui, |ui| {
            ui.strong(&overview.stats.wins.display_name);
            ui.label(overview.stats.wins.value());
            ui.end_row();

            ui.strong(&overview.stats.goals.display_name);
            ui.label(overview.stats.goals.value());
            ui.end_row();

            ui.strong(&overview.stats.season_reward_level.display_name);
            ui.label(&overview.stats.season_reward_level.metadata.rank_name);
            ui.end_row();
        });
    }

    fn render_playlists(&self, ui: &mut egui::Ui) {
        let playlists: Vec<&PlaylistSegment> = self
            .profile
            .segments
            .iter()
            .filter_map(|s| {
                if let Segment::Playlist(ov) = s {
                    Some(ov)
                } else {
                    None
                }
            })
            .collect();

        let peak_ratings: Vec<&PeakRatingSegment> = self
            .profile
            .segments
            .iter()
            .filter_map(|s| {
                if let Segment::PeakRating(ov) = s {
                    Some(ov)
                } else {
                    None
                }
            })
            .collect();

        let find_peak_rating = |playlist_id: i8| {
            peak_ratings
                .iter()
                .find(|r| r.attributes.playlist_id == playlist_id)
        };

        egui::Grid::new(format!(
            "playlists for {}",
            self.profile.platform_info.platform_user_handle
        ))
        .striped(true)
        .min_col_width(20.0)
        .show(ui, |ui| {
            ui.small("Rank");
            ui.small("Playlist");
            ui.small("Streak");
            ui.small("Peak");
            ui.end_row();

            for playlist in playlists {
                if let Ok(rank) = Rank::from_str(&playlist.stats.tier.metadata.name) {
                    ui.add(
                        egui::Image::new(rank.to_image()).fit_to_exact_size(egui::vec2(20.0, 20.0)),
                    )
                    .on_hover_text(format!("MMR: {}", playlist.stats.rating.value()));
                } else {
                    ui.label("-");
                }

                ui.strong(&playlist.metadata.name);
                ui.label(playlist.stats.win_streak.value());

                if let Ok(peak_rating) = playlist.stats.peak_rating.value().parse::<i16>() {
                    let peak_rank = Rank::estimate_from_mmr(peak_rating);

                    ui.add(
                        egui::Image::new(peak_rank.to_image())
                            .fit_to_exact_size(egui::vec2(20.0, 20.0)),
                    )
                    .on_hover_text(format!("MMR: {peak_rating}"));
                } else if let Some(peak_rating) = find_peak_rating(playlist.attributes.playlist_id)
                    && let Ok(mut peak_rank) =
                        Rank::from_str(&peak_rating.stats.peak_rating.metadata.name)
                {
                    if matches!(peak_rank, Rank::Unranked) {
                        peak_rank = Rank::estimate_from_mmr(peak_rating.stats.peak_rating.rating);
                    }

                    ui.add(
                        egui::Image::new(peak_rank.to_image())
                            .fit_to_exact_size(egui::vec2(20.0, 20.0)),
                    )
                    .on_hover_text(format!(
                        "{}\nMMR: {}",
                        peak_rating.stats.peak_rating.metadata.season,
                        peak_rating.stats.peak_rating.rating
                    ));
                }

                ui.end_row();
            }
        });
    }
}

impl egui::Widget for TrackerWidget<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            self.render_overview(ui);
            ui.add_space(8.0);
            self.render_playlists(ui);
            ui.add_space(8.0);

            if ui.button("Open in TRN").clicked() {
                let _ = webbrowser::open(&self.player.trn_url());
            }
        })
        .response
    }
}
