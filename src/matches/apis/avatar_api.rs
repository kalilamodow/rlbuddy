// platform id -> epic name

use crate::{common::CachedHttpApi, matches::MatchPlayer, rocket_league::Platform};
use eframe::egui;
use serde::Deserialize;
use std::sync::Arc;

const API_URL: &str = "https://rl-avatars-api.kmdw.dev/avatar";

#[derive(Debug, Deserialize)]
pub struct AvatarResponse {
    url: Option<String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum AvatarPlatform {
    Steam,
    Playstation,
    Xbox,
}

impl AvatarPlatform {
    fn service_name(&self) -> &'static str {
        match self {
            Self::Steam => "steam",
            Self::Playstation => "psn",
            Self::Xbox => "xbox",
        }
    }

    fn id_tag_name(&self) -> &'static str {
        match self {
            Self::Steam => "steamid",
            Self::Playstation => "psnid",
            Self::Xbox => "gamertag",
        }
    }
}

impl TryFrom<Platform> for AvatarPlatform {
    type Error = ();
    fn try_from(value: Platform) -> Result<Self, Self::Error> {
        match value {
            Platform::Steam => Ok(Self::Steam),
            Platform::PlayStation => Ok(Self::Playstation),
            Platform::Xbox => Ok(Self::Xbox),
            _ => Err(()),
        }
    }
}

fn get_platform_id<'a>(player_id: &'a str) -> &'a str {
    player_id.split('|').nth(1).unwrap()
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct AvatarKey {
    platform: AvatarPlatform,
    id: String,
}

impl TryFrom<&MatchPlayer> for AvatarKey {
    type Error = ();
    fn try_from(value: &MatchPlayer) -> Result<Self, Self::Error> {
        let Ok(platform) = value.data.platform.try_into() else {
            return Err(());
        };

        Ok(Self {
            id: match platform {
                AvatarPlatform::Xbox => {
                    if !value.display_name_is_censored() {
                        value.display_name().to_owned()
                    } else {
                        return Err(());
                    }
                }
                _ => get_platform_id(&value.data.platform_id).to_owned(),
            },
            platform,
        })
    }
}

pub type AvatarAPI = CachedHttpApi<AvatarKey, String, AvatarResponse>;

pub fn new_avatar_api(context: egui::Context) -> AvatarAPI {
    CachedHttpApi::new(
        context,
        Box::new(|key| {
            format!(
                "{}?service={}&{}={}",
                API_URL,
                key.platform.service_name(),
                key.platform.id_tag_name(),
                key.id
            )
        }),
        Arc::new(|resp| resp.url),
    )
}
