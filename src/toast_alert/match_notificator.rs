use crate::{
    common::{
        ReadWriteStateHandle, ReadonlyStateHandle, channel::Sender, eventsource::EventReceiver,
    },
    matches::MatchesServiceState,
    rocket_league::Playlist,
    stats_api::RLEvent,
    toast_alert::Toast,
};
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct MatchNotificatorSettings {
    disable_goal_speed: bool,
    disable_crossbar_hit_speed: bool,
    training_only: bool,
}

pub struct MatchNotificatorService {
    settings_handle: ReadWriteStateHandle<MatchNotificatorSettings>,

    matches_handle: ReadonlyStateHandle<MatchesServiceState>,
    stats_api: EventReceiver<RLEvent>,
    toasts: Sender<Toast>,
}

impl MatchNotificatorService {
    pub fn new(
        settings: Option<MatchNotificatorSettings>,
        matches_handle: ReadonlyStateHandle<MatchesServiceState>,
        stats_api: EventReceiver<RLEvent>,
        toasts: Sender<Toast>,
    ) -> Self {
        Self {
            settings_handle: ReadWriteStateHandle::new(settings.unwrap_or_default()),
            matches_handle,
            stats_api,
            toasts,
        }
    }

    pub fn settings_handle(&self) -> ReadWriteStateHandle<MatchNotificatorSettings> {
        self.settings_handle.clone()
    }

    pub fn update(&self) {
        let matches_state = self.matches_handle.read();
        if matches_state
            .current_match
            .as_ref()
            .is_some_and(|m| matches!(m.playlist, Playlist::Training))
            && self.settings_handle.read().training_only
        {
            return;
        }

        while let Some(event) = self.stats_api.try_recv() {
            match event.as_ref() {
                RLEvent::Goal {
                    ball_speed,
                    release_speed,
                } => self.toasts.send(Toast::new(format!(
                    "Nice shot! {ball_speed}km/h\nRelease: {release_speed}km/h"
                ))),
                RLEvent::CrossbarHit {
                    impact_speed,
                    release_speed,
                } => self.toasts.send(Toast::new(format!(
                    "Close one! {impact_speed}km/h\nRelease: {release_speed}km/h"
                ))),
                _ => {}
            }
        }
    }
}

pub struct MatchNotificatorSettingsWidget {
    settings_handle: ReadWriteStateHandle<MatchNotificatorSettings>,
    toasts: Sender<Toast>,
}

impl MatchNotificatorSettingsWidget {
    pub fn new(service: &MatchNotificatorService, toasts: Sender<Toast>) -> Self {
        Self {
            settings_handle: service.settings_handle(),
            toasts,
        }
    }
}

impl egui::Widget for &MatchNotificatorSettingsWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            let mut settings = self.settings_handle.write();
            ui.columns_const(|[c1, c2]| {
                c1.checkbox(
                    &mut settings.disable_crossbar_hit_speed,
                    "Disable crossbar notification",
                );
                c1.checkbox(&mut settings.disable_goal_speed, "Disable goal speed");
                c1.checkbox(&mut settings.training_only, "Training only");

                c2.with_layout(egui::Layout::right_to_left(egui::Align::Min), |c2| {
                    if c2.small_button("Send test notification").clicked() {
                        self.toasts.send(Toast::new(format!("{settings:?}")));
                    }
                });
            });
        })
        .response
    }
}
