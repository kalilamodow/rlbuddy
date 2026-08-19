use crate::{
    auto_setup::AutoSetupWidget,
    common::eventsource::EventReceiver,
    discord,
    hotkey::{HotkeyService, HotkeySettings},
    matches::{CurrentMatchWidget, MatchesService, PastMatchesWidget, StrippedMatchInfo},
    music_control::{self, MusicControlService, MusicControlSettings, MusicControlWidget},
    my_stats::{MyStatsWidget, MyStatsWidgetSettings},
    player_info::{PlayerInfoService, PlayerSearchWidget},
    settings::SettingsWidget,
    stats_api::{RLEvent, StatsApi},
    toast_alert::{MatchNotificatorService, MatchNotificatorSettings, ToastAlertService},
};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc, thread};
use std::{sync::mpsc, time::Duration};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
enum Panel {
    CurrentMatch,
    PastMatches,
    MusicControl,
    MyStats,
    Discord,
    PlayerSearch,
    AutoSetup,
    Settings,
}

impl std::fmt::Display for Panel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Panel::CurrentMatch => "Lobby",
                Panel::PastMatches => "History",
                Panel::MyStats => "Session",
                Panel::MusicControl => "Music",
                Panel::Discord => "Discord",
                Panel::PlayerSearch => "Player Search",
                Panel::AutoSetup => "Stats API Setup",
                Panel::Settings => "Settings",
            }
        )
    }
}

const OPENABLE_PANELS: [Panel; 8] = [
    Panel::CurrentMatch,
    Panel::Discord,
    Panel::MusicControl,
    Panel::MyStats,
    Panel::PastMatches,
    Panel::AutoSetup,
    Panel::PlayerSearch,
    Panel::Settings,
];

fn visuals_with_transparency(visuals: &mut egui::Visuals, transparency: u8) {
    visuals.panel_fill = egui::Color32::from_rgba_unmultiplied(
        visuals.panel_fill.r(),
        visuals.panel_fill.g(),
        visuals.panel_fill.b(),
        255 - transparency,
    );
}

#[derive(Debug, Serialize, Deserialize)]
struct AppData {
    transparency: u8,
    hotkey_settings: Option<HotkeySettings>,
    rich_presence_settings: Option<discord::DiscordSettings>,
    open_panels: Vec<Panel>,
    matches: Vec<StrippedMatchInfo>,
    my_stats_settings: Option<MyStatsWidgetSettings>,
    music_control_settings: Option<MusicControlSettings>,
    match_notifications: Option<MatchNotificatorSettings>,
}

impl Default for AppData {
    fn default() -> Self {
        AppData {
            transparency: 25,
            hotkey_settings: None,
            rich_presence_settings: None,
            open_panels: vec![Panel::CurrentMatch],
            matches: Vec::new(),
            music_control_settings: None,
            my_stats_settings: None,
            match_notifications: None,
        }
    }
}

pub struct RlBuddyApp {
    current_transparency: Rc<RefCell<u8>>,

    overlay_tx: mpsc::Sender<bool>,
    overlay_rx: mpsc::Receiver<bool>,
    prev_hide_pos: Option<egui::Pos2>,
    open_panels: Vec<Panel>,

    stats_api_events: EventReceiver<RLEvent>,
    stats_api_service: StatsApi,

    discord_service: discord::DiscordService,
    discord_widget: discord::DiscordWidget,

    music_control_service: music_control::MusicControlService,
    music_control_widget: music_control::MusicControlWidget,

    matches_service: MatchesService,
    current_match: CurrentMatchWidget,
    past_matches: PastMatchesWidget,
    my_stats_widget: MyStatsWidget,

    player_info_service: PlayerInfoService,
    player_search_widget: PlayerSearchWidget,

    match_notificator_service: MatchNotificatorService,
    toast_service: ToastAlertService,

    hotkey_service: HotkeyService,
    auto_setup_widget: AutoSetupWidget,
    settings_widget: SettingsWidget,
}

impl RlBuddyApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let ctx = cc.egui_ctx.clone();
        egui_system_fonts::set_auto(&ctx, egui_system_fonts::FontStyle::Sans);

        let app_data = if let Some(storage) = cc.storage
            && let Some(existing_state) = eframe::get_value::<AppData>(storage, eframe::APP_KEY)
        {
            existing_state
        } else {
            AppData::default()
        };

        let (overlay_tx, overlay_rx) = mpsc::channel();
        let mut stats_api_service = StatsApi::new();
        let toast_service = ToastAlertService::new(ctx.clone());
        let matches_service =
            MatchesService::new(&ctx, stats_api_service.subscribe(), app_data.matches);
        let match_notificator_service = MatchNotificatorService::new(
            app_data.match_notifications,
            matches_service.state_handle(),
            stats_api_service.subscribe(),
            toast_service.sender(),
        );
        let music_control_service = MusicControlService::new(
            app_data.music_control_settings,
            stats_api_service.subscribe(),
        );
        let discord_service = discord::DiscordService::new(
            app_data.rich_presence_settings,
            matches_service.state_handle(),
            stats_api_service.subscribe(),
        );
        let hotkey_service = HotkeyService::new(&overlay_tx, app_data.hotkey_settings);
        let player_info_service = PlayerInfoService::new(ctx.clone());

        let current_transparency = Rc::new(RefCell::new(app_data.transparency));
        RlBuddyApp {
            settings_widget: SettingsWidget::new(
                &hotkey_service,
                &match_notificator_service,
                &toast_service,
                Rc::clone(&current_transparency),
            ),

            overlay_tx,
            overlay_rx,
            current_transparency,
            prev_hide_pos: None,

            discord_widget: discord::DiscordWidget::new(
                discord_service.settings_handle(),
                discord_service.state_handle(),
            ),
            discord_service,

            music_control_widget: MusicControlWidget::new(&music_control_service),
            music_control_service,

            my_stats_widget: MyStatsWidget::new(
                matches_service.state_handle(),
                app_data.my_stats_settings.unwrap_or_default(),
            ),
            current_match: CurrentMatchWidget::new(
                matches_service.state_handle(),
                player_info_service.sender(),
            ),
            past_matches: PastMatchesWidget::new(
                matches_service.state_handle(),
                player_info_service.sender(),
            ),
            match_notificator_service,
            matches_service,

            stats_api_events: stats_api_service.subscribe(),
            stats_api_service,

            hotkey_service,
            player_search_widget: PlayerSearchWidget::new(player_info_service.sender()),
            player_info_service,

            toast_service,
            auto_setup_widget: AutoSetupWidget::new(),
            open_panels: app_data.open_panels,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        self.prev_hide_pos = ctx.input(|i| {
            i.viewport()
                .outer_rect
                .map(|outer_rect| egui::pos2(outer_rect.left(), outer_rect.top()))
        });

        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(8.0, 8.0)));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::Normal,
        ));
    }

    fn hide(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        if let Some(move_to) = self.prev_hide_pos {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(move_to));
        }
    }

    fn pop_up(&self) {
        self.overlay_tx.send(true).unwrap();
        let tx = self.overlay_tx.clone();

        thread::spawn(move || {
            thread::sleep(Duration::from_secs(3));
            tx.send(false).unwrap();
        });
    }
}

impl eframe::App for RlBuddyApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let data = AppData {
            transparency: *self.current_transparency.borrow(),
            hotkey_settings: Some(self.hotkey_service.settings_handle().read().clone()),
            rich_presence_settings: Some(self.discord_service.settings_handle().read().clone()),
            open_panels: self.open_panels.clone(),
            matches: self.matches_service.stripped_history(),
            my_stats_settings: Some(self.my_stats_widget.clone_settings()),
            music_control_settings: Some(
                self.music_control_service.settings_handle().read().clone(),
            ),
            match_notifications: Some(
                self.match_notificator_service
                    .settings_handle()
                    .read()
                    .clone(),
            ),
        };
        eframe::set_value(storage, eframe::APP_KEY, &data);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        visuals_with_transparency(ui.visuals_mut(), *self.current_transparency.borrow());

        egui::Panel::bottom("bottom_panel").show_inside(ui, |ui| {
            egui::ComboBox::from_label("")
                .selected_text("Widgets")
                .show_ui(ui, |ui| {
                    for panel in OPENABLE_PANELS {
                        let open = self.open_panels.contains(&panel);

                        if ui.selectable_label(open, panel.to_string()).clicked() {
                            if open {
                                self.open_panels.retain(|p| p != &panel);
                            } else {
                                self.open_panels.push(panel);
                            }
                        }
                    }
                });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    let mut to_swap: Option<(usize, usize)> = None; // index, move to
                    let mut to_close: Option<Panel> = None;

                    for (index, panel) in self.open_panels.iter().enumerate() {
                        let frame =
                            egui::Frame::group(ui.style()).fill(ui.style().visuals.faint_bg_color);

                        frame.show(ui, |ui| {
                            ui.columns_const(|[c1, c2]| {
                                c1.label(egui::RichText::new(panel.to_string()).strong());

                                c2.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Min),
                                    |c2| {
                                        if c2.small_button("X").clicked() {
                                            to_close = Some(*panel);
                                        }

                                        c2.add_enabled_ui(
                                            index != self.open_panels.len() - 1,
                                            |c2| {
                                                if c2.small_button("\\/").clicked() {
                                                    to_swap = Some((index, index + 1));
                                                }
                                            },
                                        );

                                        c2.add_enabled_ui(index != 0, |c2| {
                                            if c2.small_button("/\\").clicked() {
                                                to_swap = Some((index, index - 1));
                                            }
                                        });
                                    },
                                );
                            });

                            ui.separator();

                            match panel {
                                Panel::CurrentMatch => ui.add(&mut self.current_match),
                                Panel::Discord => ui.add(&mut self.discord_widget),
                                Panel::MyStats => ui.add(&mut self.my_stats_widget),
                                Panel::MusicControl => ui.add(&mut self.music_control_widget),
                                Panel::PastMatches => ui.add(&mut self.past_matches),
                                Panel::PlayerSearch => ui.add(&mut self.player_search_widget),
                                Panel::AutoSetup => ui.add(&mut self.auto_setup_widget),
                                Panel::Settings => ui.add(&mut self.settings_widget),
                            };
                        });

                        ui.add_space(4.0);
                    }

                    if let Some(to_close) = to_close {
                        self.open_panels.retain(|p| p != &to_close);
                    }
                    if let Some(to_shift) = to_swap {
                        self.open_panels.swap(to_shift.0, to_shift.1);
                    }
                })
            });
        });

        ui.add(&mut self.player_info_service);
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.stats_api_service.update();
        self.matches_service.update();
        self.player_info_service.update();
        self.discord_service.update();
        self.music_control_service.update();
        self.match_notificator_service.update();
        self.toast_service.update();

        while let Some(event) = self.stats_api_events.try_recv() {
            match *event {
                RLEvent::Connected => ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                    "rlbuddy (connected)".to_string(),
                )),
                RLEvent::Disconnected => ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                    "rlbuddy (not connected".to_string(),
                )),
                RLEvent::MatchStart => self.pop_up(),
                _ => {}
            }
        }

        if let Some(should_overlay) = self.overlay_rx.try_iter().last() {
            if should_overlay {
                self.show(ctx);
            } else {
                self.hide(ctx);
            }
        }

        ctx.request_repaint_after(Duration::from_millis(10));
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}
