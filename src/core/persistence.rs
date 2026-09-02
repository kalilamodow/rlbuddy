use crate::common::savedata::rlbuddy_data_dir;
use crate::core::app::OpenLegacyPanelList;
use crate::discord;
use crate::gamepad::overlay::service::GamepadOverlayServiceSettings;
use crate::map_loader::MapLoaderServiceSavedata;
use crate::matches::StrippedMatchInfo;
use crate::my_stats::MyStatsWidgetSettings;
use crate::toast_alert::MatchNotificatorSettings;
use eframe::egui;
use std::fs;
use std::path::Path;

#[derive(Debug, Default)]
pub struct AppData {
    pub app_settings: crate::core::app::AppSettings,
    pub rich_presence_settings: discord::DiscordSettings,
    pub open_panels: OpenLegacyPanelList,
    pub matches: Vec<StrippedMatchInfo>,
    pub my_stats_settings: MyStatsWidgetSettings,
    pub match_notification_settings: MatchNotificatorSettings,
    pub saved_window_dimensions: Option<(egui::Pos2, egui::Vec2)>, // outer pos, inner size
    pub gamepad_overlay_savedata: GamepadOverlayServiceSettings,
    pub map_loader_savedata: MapLoaderServiceSavedata,
}

impl AppData {
    pub fn load() -> Self {
        let Some(data_dir) = rlbuddy_data_dir() else {
            return Self::default();
        };

        Self {
            app_settings: Self::load_setting(&data_dir, "app_settings"),
            rich_presence_settings: Self::load_setting(&data_dir, "drpc_settings"),
            open_panels: Self::load_setting(&data_dir, "open_panel_list"),
            matches: Self::load_setting(&data_dir, "matches"),
            my_stats_settings: Self::load_setting(&data_dir, "my_stats_settings"),
            match_notification_settings: Self::load_setting(
                &data_dir,
                "match_notifications_settings",
            ),
            saved_window_dimensions: Self::load_setting(&data_dir, "saved_window_dimensions"),
            gamepad_overlay_savedata: Self::load_setting(&data_dir, "gamepad_overlay_savedata"),
            map_loader_savedata: Self::load_setting(&data_dir, "map_loader_savedata"),
        }
    }

    fn load_setting<T>(data_dir: &Path, name: &str) -> T
    where
        T: serde::de::DeserializeOwned + Default,
    {
        let Ok(string) = fs::read_to_string(data_dir.join(format!("{name}.json"))) else {
            return T::default();
        };

        serde_json::from_str(&string).unwrap_or_default()
    }

    pub fn save(self) {
        let Some(data_dir) = rlbuddy_data_dir() else {
            return;
        };

        let _ = fs::create_dir_all(&data_dir);

        Self::write_setting(&data_dir, "app_settings", self.app_settings);
        Self::write_setting(&data_dir, "drpc_settings", self.rich_presence_settings);
        Self::write_setting(&data_dir, "open_panel_list", self.open_panels);
        Self::write_setting(&data_dir, "matches", self.matches);
        Self::write_setting(&data_dir, "my_stats_settings", self.my_stats_settings);
        Self::write_setting(
            &data_dir,
            "match_notifications_settings",
            self.match_notification_settings,
        );
        Self::write_setting(
            &data_dir,
            "saved_window_dimensions",
            self.saved_window_dimensions,
        );
        Self::write_setting(
            &data_dir,
            "gamepad_overlay_savedata",
            self.gamepad_overlay_savedata,
        );
        Self::write_setting(&data_dir, "map_loader_savedata", self.map_loader_savedata);
    }

    fn write_setting<T>(data_dir: &Path, name: &str, new: T)
    where
        T: serde::Serialize + Default,
    {
        let string = match serde_json::to_string(&new) {
            Ok(wtv) => wtv,
            Err(e) => {
                eprintln!("Failed to serialize settings for {name}: {e:?}");
                return;
            }
        };

        if let Err(error) = fs::write(data_dir.join(format!("{name}.json")), string) {
            eprintln!("Failed to write settings for {name}: {error:?}");
        }
    }
}
