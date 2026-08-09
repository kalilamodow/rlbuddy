use std::{fmt, str::FromStr};

use eframe::egui::{self, Color32};
use num_enum::{FromPrimitive, TryFromPrimitive};

const RANK_NAMES: [&str; 23] = [
    "Unranked",
    "Bronze I",
    "Bronze II",
    "Bronze III",
    "Silver I",
    "Silver II",
    "Silver III",
    "Gold I",
    "Gold II",
    "Gold III",
    "Platinum I",
    "Platinum II",
    "Platinum III",
    "Diamond I",
    "Diamond II",
    "Diamond III",
    "Champion I",
    "Champion II",
    "Champion III",
    "Grand Champion I",
    "Grand Champion II",
    "Grand Champion III",
    "Supersonic Legend",
];

#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum Rank {
    Unranked,
    Bronze1,
    Bronze2,
    Bronze3,
    Silver1,
    Silver2,
    Silver3,
    Gold1,
    Gold2,
    Gold3,
    Plat1,
    Plat2,
    Plat3,
    Diamond1,
    Diamond2,
    Diamond3,
    Champ1,
    Champ2,
    Champ3,
    GC1,
    GC2,
    GC3,
    Ssl,
}

impl Rank {
    pub fn as_str(self) -> &'static str {
        RANK_NAMES[self as usize]
    }

    pub fn to_image(self) -> egui::ImageSource<'static> {
        match self {
            Rank::Unranked => egui::include_image!("../../assets/Unranked_icon.webp"),
            Rank::Bronze1 => egui::include_image!("../../assets/Bronze1_rank_icon.webp"),
            Rank::Bronze2 => egui::include_image!("../../assets/Bronze2_rank_icon.webp"),
            Rank::Bronze3 => egui::include_image!("../../assets/Bronze3_rank_icon.webp"),
            Rank::Silver1 => egui::include_image!("../../assets/Silver1_rank_icon.webp"),
            Rank::Silver2 => egui::include_image!("../../assets/Silver2_rank_icon.webp"),
            Rank::Silver3 => egui::include_image!("../../assets/Silver3_rank_icon.webp"),
            Rank::Gold1 => egui::include_image!("../../assets/Gold1_rank_icon.webp"),
            Rank::Gold2 => egui::include_image!("../../assets/Gold2_rank_icon.webp"),
            Rank::Gold3 => egui::include_image!("../../assets/Gold3_rank_icon.webp"),
            Rank::Plat1 => egui::include_image!("../../assets/Platinum1_rank_icon.webp"),
            Rank::Plat2 => egui::include_image!("../../assets/Platinum2_rank_icon.webp"),
            Rank::Plat3 => egui::include_image!("../../assets/Platinum3_rank_icon.webp"),
            Rank::Diamond1 => egui::include_image!("../../assets/Diamond1_rank_icon.webp"),
            Rank::Diamond2 => egui::include_image!("../../assets/Diamond2_rank_icon.webp"),
            Rank::Diamond3 => egui::include_image!("../../assets/Diamond3_rank_icon.webp"),
            Rank::Champ1 => egui::include_image!("../../assets/Champion1_rank_icon.webp"),
            Rank::Champ2 => egui::include_image!("../../assets/Champion2_rank_icon.webp"),
            Rank::Champ3 => egui::include_image!("../../assets/Champion3_rank_icon.webp"),
            Rank::GC1 => egui::include_image!("../../assets/Grand_Champion1_rank_icon.webp"),
            Rank::GC2 => egui::include_image!("../../assets/Grand_Champion2_rank_icon.webp"),
            Rank::GC3 => egui::include_image!("../../assets/Grand_Champion3_rank_icon.webp"),
            Rank::Ssl => egui::include_image!("../../assets/Supersonic_Legend_rank_icon.webp"),
        }
    }

    pub fn to_color(self) -> Color32 {
        match self {
            Rank::Unranked => Color32::DARK_GRAY,
            Rank::Bronze1 | Rank::Bronze2 | Rank::Bronze3 => Color32::BROWN,
            Rank::Silver1 | Rank::Silver2 | Rank::Silver3 => Color32::GRAY,
            Rank::Gold1 | Rank::Gold2 | Rank::Gold3 => Color32::YELLOW,
            Rank::Plat1 | Rank::Plat2 | Rank::Plat3 => Color32::LIGHT_BLUE,
            Rank::Diamond1 | Rank::Diamond2 | Rank::Diamond3 => Color32::BLUE,
            Rank::Champ1 | Rank::Champ2 | Rank::Champ3 => Color32::PURPLE,
            Rank::GC1 | Rank::GC2 | Rank::GC3 => Color32::RED,
            Rank::Ssl => Color32::WHITE,
        }
    }

    // uses f2p season 23 1v1
    pub fn estimate_from_mmr(mmr: i16) -> Rank {
        #[allow(clippy::match_overlapping_arm)]
        match mmr {
            ..=156 => Rank::Bronze1,
            ..=213 => Rank::Bronze2,
            ..=274 => Rank::Bronze3,
            ..=334 => Rank::Silver1,
            ..=394 => Rank::Silver2,
            ..=454 => Rank::Silver3,
            ..=514 => Rank::Gold1,
            ..=574 => Rank::Gold2,
            ..=634 => Rank::Gold3,
            ..=694 => Rank::Plat1,
            ..=753 => Rank::Plat2,
            ..=808 => Rank::Plat3,
            ..=874 => Rank::Diamond1,
            ..=930 => Rank::Diamond2,
            ..=994 => Rank::Diamond3,
            ..=1052 => Rank::Champ1,
            ..=1114 => Rank::Champ2,
            ..=1170 => Rank::Champ3,
            ..=1232 => Rank::GC1,
            ..=1295 => Rank::GC2,
            ..=1351 => Rank::GC3,
            _ => Rank::Ssl,
        }
    }
}

impl FromStr for Rank {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let index = RANK_NAMES.iter().position(|rank| *rank == s).ok_or(())?;
        #[allow(clippy::cast_possible_truncation)]
        Rank::try_from(index as u8).map_err(|_| ())
    }
}

#[derive(Debug, FromPrimitive)]
#[repr(u8)]
pub enum Division {
    #[num_enum(default)]
    None,
    One,
    Two,
    Three,
    Four,
}

impl fmt::Display for Division {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Division::None => "",
                Division::One => " Div I",
                Division::Two => " Div II",
                Division::Three => " Div III",
                Division::Four => " Div IV",
            }
        )
    }
}
