use std::sync::Arc;

use eframe::egui;
use num_enum::{TryFromPrimitive as _, TryFromPrimitiveError};
use serde::{Deserialize, Serialize};

use crate::{
    common::CachedHttpApi,
    rocket_league::{Division, Playlist, Rank},
};

const API_URL: &str = "https://mmr.kmdw.dev/get-skills";

#[derive(Deserialize, Debug)]
struct GetPlayerSkillsPlaylistData {
    id: u8,
    mmr: i16,
    tier: u8,
    division: u8,
}

#[derive(Deserialize, Debug)]
pub struct GetPlayerSkillsResponse {
    playlists: Vec<GetPlayerSkillsPlaylistData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistSkillInformation {
    pub playlist: Playlist,
    pub rank: Rank,
    pub div: Division,
    pub mmr: i16,
    pub rank_is_estimate: bool,
}

impl PlaylistSkillInformation {
    fn try_from_data(
        value: &GetPlayerSkillsPlaylistData,
    ) -> Result<Self, TryFromPrimitiveError<Playlist>> {
        let actual_rank = Rank::try_from_primitive(value.tier).expect("Failed to convert rank");
        let use_estimate = actual_rank == Rank::Unranked;

        let playlist = Playlist::try_from_primitive(value.id)?;

        Ok(Self {
            playlist,
            rank: if use_estimate {
                Rank::estimate_from_mmr(value.mmr)
            } else {
                actual_rank
            },
            div: Division::from(value.division),
            mmr: value.mmr,
            rank_is_estimate: use_estimate,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PlayerSkillInformation {
    playlists: Vec<PlaylistSkillInformation>,
}

impl PlayerSkillInformation {
    pub fn get_playlist(&self, playlist: Playlist) -> Option<&PlaylistSkillInformation> {
        self.playlists.iter().find(|p| p.playlist == playlist)
    }
}

impl From<GetPlayerSkillsResponse> for PlayerSkillInformation {
    fn from(value: GetPlayerSkillsResponse) -> Self {
        Self {
            playlists: value
                .playlists
                .into_iter()
                .filter_map(|v| PlaylistSkillInformation::try_from_data(&v).ok())
                .collect(),
        }
    }
}

pub type RankAPI = CachedHttpApi<String, PlayerSkillInformation, GetPlayerSkillsResponse>;

pub fn new_rank_api(context: egui::Context) -> RankAPI {
    CachedHttpApi::new(
        context,
        Box::new(|player_id| format!("{}?playerId={}", API_URL, urlencoding::encode(player_id))),
        Arc::new(|response| Some(response.into())),
    )
}
