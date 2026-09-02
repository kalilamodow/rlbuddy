use crate::core::persistence::AppData;
use crate::gamepad::GamepadService;
use crate::gamepad::overlay::service::GamepadOverlayService;
use crate::gamepad::overlay::widget::GamepadOverlayWidget;
use crate::hotkey::HotkeyFeature;
use crate::music_control::feature::MusicControlFeature;
use crate::{
    auto_setup::AutoSetupWidget,
    common::eventsource::EventReceiver,
    discord,
    map_loader::{MapLoaderService, MapLoaderWidget},
    matches::{CurrentMatchWidget, MatchesService, PastMatchesWidget},
    my_stats::MyStatsWidget,
    player_info::{PlayerInfoService, PlayerSearchWidget},
    settings::SettingsWidget,
    stats_api::{RLEvent, StatsApi},
    toast_alert::{MatchNotificatorService, ToastAlertService},
};
use eframe::egui::{self, ViewportCommand};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc, thread};
use std::{sync::mpsc, time::Duration};

pub trait Service {
    fn update(&mut self);
    fn save(&self) {}
}

pub trait Feature: Service {
    fn name(&self) -> &'static str;
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response;
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Panel {
    CurrentMatch,
    PastMatches,
    MyStats,
    Discord,
    PlayerSearch,
    MapLoader,
    AutoSetup,
    GamepadOverlay,
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
                Panel::Discord => "Discord",
                Panel::PlayerSearch => "Player Search",
                Panel::MapLoader => "Custom Maps",
                Panel::AutoSetup => "Stats API Setup",
                Panel::GamepadOverlay => "Gamepad Overlay",
                Panel::Settings => "Settings",
            }
        )
    }
}

const OPENABLE_PANELS: [Panel; 9] = [
    Panel::CurrentMatch,
    Panel::Discord,
    Panel::MyStats,
    Panel::PastMatches,
    Panel::MapLoader,
    Panel::AutoSetup,
    Panel::PlayerSearch,
    Panel::GamepadOverlay,
    Panel::Settings,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPanelList(Vec<Panel>);

impl Default for OpenPanelList {
    fn default() -> Self {
        Self(vec![Panel::CurrentMatch, Panel::AutoSetup])
    }
}

impl std::ops::Deref for OpenPanelList {
    type Target = Vec<Panel>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for OpenPanelList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn visuals_with_transparency(visuals: &mut egui::Visuals, transparency: u8) {
    visuals.panel_fill = egui::Color32::from_rgba_unmultiplied(
        visuals.panel_fill.r(),
        visuals.panel_fill.g(),
        visuals.panel_fill.b(),
        255 - transparency,
    );
}

struct AppFeature {
    name: &'static str,
    open: bool,
    feature: Box<dyn Feature>,
}

impl AppFeature {
    fn new<F: Feature + 'static>(feature: F) -> Self {
        Self {
            name: feature.name(),
            open: false,
            feature: Box::new(feature),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AppSettings {
    pub transparency: u8,
}

pub struct RlBuddyApp {
    current_transparency: Rc<RefCell<u8>>,

    overlay_tx: mpsc::Sender<bool>,
    overlay_rx: mpsc::Receiver<bool>,
    prev_hide_pos: Option<egui::Pos2>,
    open_panels: OpenPanelList,

    stats_api_events: EventReceiver<RLEvent>,
    stats_api_service: StatsApi,

    discord_service: discord::DiscordService,
    discord_widget: discord::DiscordWidget,

    matches_service: MatchesService,
    current_match: CurrentMatchWidget,
    past_matches: PastMatchesWidget,
    my_stats_widget: MyStatsWidget,

    player_info_service: PlayerInfoService,
    player_search_widget: PlayerSearchWidget,

    map_loader_widget: MapLoaderWidget,
    map_loader_service: MapLoaderService,

    match_notificator_service: MatchNotificatorService,
    toast_service: ToastAlertService,

    gamepad_overlay_service: GamepadOverlayService,
    gamepad_overlay_widget: GamepadOverlayWidget,

    gamepad_service: GamepadService,
    auto_setup_widget: AutoSetupWidget,
    settings_widget: SettingsWidget,

    services: Vec<Box<dyn Service>>,
    features: Vec<AppFeature>,
}

impl RlBuddyApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let ctx = cc.egui_ctx.clone();
        egui_system_fonts::set_auto(&ctx, egui_system_fonts::FontStyle::Sans);

        let app_data = AppData::load();

        if let Some(remembered_dimensions) = app_data.saved_window_dimensions {
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(remembered_dimensions.0));
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(remembered_dimensions.1));
        }

        let (overlay_tx, overlay_rx) = mpsc::channel();
        let mut stats_api_service = StatsApi::new();
        let toast_service = ToastAlertService::new(ctx.clone());
        let matches_service =
            MatchesService::new(&ctx, stats_api_service.subscribe(), app_data.matches);
        let match_notificator_service = MatchNotificatorService::new(
            app_data.match_notification_settings,
            matches_service.state_handle(),
            stats_api_service.subscribe(),
            toast_service.sender(),
        );
        let discord_service = discord::DiscordService::new(
            app_data.rich_presence_settings,
            matches_service.state_handle(),
            stats_api_service.subscribe(),
        );
        let mut gamepad_service = GamepadService::new();
        let gamepad_overlay_service = GamepadOverlayService::new(
            app_data.gamepad_overlay_savedata,
            ctx.clone(),
            &gamepad_service,
        );
        let player_info_service = PlayerInfoService::new(ctx.clone());
        let map_loader_service = MapLoaderService::new(app_data.map_loader_savedata);

        let hotkey = HotkeyFeature::new(&mut gamepad_service, &overlay_tx);
        let music_control = MusicControlFeature::new(&mut stats_api_service);

        let current_transparency = Rc::new(RefCell::new(app_data.app_settings.transparency));
        let app = RlBuddyApp {
            settings_widget: SettingsWidget::new(
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

            my_stats_widget: MyStatsWidget::new(
                matches_service.state_handle(),
                app_data.my_stats_settings,
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

            map_loader_widget: MapLoaderWidget::new(&map_loader_service),
            map_loader_service,

            gamepad_overlay_widget: GamepadOverlayWidget::new(&gamepad_overlay_service),
            gamepad_overlay_service,

            gamepad_service,
            player_search_widget: PlayerSearchWidget::new(player_info_service.sender()),
            player_info_service,

            toast_service,
            auto_setup_widget: AutoSetupWidget::new(),
            open_panels: app_data.open_panels,

            services: Vec::new(),
            features: vec![AppFeature::new(hotkey), AppFeature::new(music_control)],
        };

        app
    }

    fn show(&mut self, ctx: &egui::Context) {
        // if not minimized, dont bother
        if ctx.input(|i| !i.viewport().minimized.unwrap_or_default()) {
            return;
        }

        self.prev_hide_pos = ctx.input(|i| {
            i.viewport()
                .outer_rect
                .map(|outer_rect| egui::pos2(outer_rect.left(), outer_rect.top()))
        });

        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(8.0, 8.0)));
        ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
    }

    fn hide(&self, ctx: &egui::Context) {
        if ctx.input(|i| i.focused) {
            return;
        }

        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
        if let Some(move_to) = self.prev_hide_pos {
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(move_to));
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

    fn on_close(&self, ctx: &egui::Context) {
        for feature in &self.features {
            feature.feature.save();
        }

        AppData {
            app_settings: AppSettings {
                transparency: *self.current_transparency.borrow(),
            },
            rich_presence_settings: self.discord_service.settings_handle().read().clone(),
            open_panels: self.open_panels.clone(),
            matches: self.matches_service.stripped_history(),
            my_stats_settings: self.my_stats_widget.clone_settings(),
            match_notification_settings: self
                .match_notificator_service
                .settings_handle()
                .read()
                .clone(),
            saved_window_dimensions: ctx.input(|i| {
                i.viewport().outer_rect.and_then(|outer| {
                    i.viewport()
                        .inner_rect
                        .map(|inner| (outer.left_top(), inner.size()))
                })
            }),
            gamepad_overlay_savedata: self
                .gamepad_overlay_service
                .settings_handle()
                .read()
                .clone(),
            map_loader_savedata: self.map_loader_service.save(),
        }
        .save();
    }

    fn update_services(&mut self) {
        for s in &mut self.services {
            s.update();
        }
        for f in &mut self.features {
            f.feature.update();
        }
    }
}

impl eframe::App for RlBuddyApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.gamepad_service.update();
        self.stats_api_service.update();
        self.matches_service.update();
        self.player_info_service.update();
        self.discord_service.update();
        self.match_notificator_service.update();
        self.toast_service.update();
        self.map_loader_service.update();
        self.gamepad_overlay_service.update();

        self.update_services();

        while let Some(event) = self.stats_api_events.try_recv() {
            match *event {
                RLEvent::Connected => {
                    ctx.send_viewport_cmd(ViewportCommand::Title("rlbuddy (connected)".to_string()))
                }
                RLEvent::Disconnected => ctx.send_viewport_cmd(ViewportCommand::Title(
                    "rlbuddy (not connected)".to_string(),
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

        if ctx.input(|i| i.viewport().close_requested()) {
            self.on_close(ctx);
        }
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

                    for feature in &mut self.features {
                        if ui.selectable_label(feature.open, feature.name).clicked() {
                            feature.open = !feature.open;
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
                                Panel::PastMatches => ui.add(&mut self.past_matches),
                                Panel::PlayerSearch => ui.add(&mut self.player_search_widget),
                                Panel::MapLoader => ui.add(&mut self.map_loader_widget),
                                Panel::AutoSetup => ui.add(&mut self.auto_setup_widget),
                                Panel::GamepadOverlay => ui.add(&mut self.gamepad_overlay_widget),
                                Panel::Settings => ui.add(&mut self.settings_widget),
                            };
                        });

                        ui.add_space(4.0);
                    }

                    for feature in self.features.iter_mut().filter(|f| f.open) {
                        let frame =
                            egui::Frame::group(ui.style()).fill(ui.style().visuals.faint_bg_color);

                        frame.show(ui, |ui| {
                            ui.columns_const(|[c1, c2]| {
                                c1.label(egui::RichText::new(feature.name).strong());
                                c2.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Min),
                                    |c2| {
                                        if c2.small_button("X").clicked() {
                                            feature.open = false;
                                        }
                                    },
                                );
                            });

                            ui.separator();

                            feature.feature.ui(ui);
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

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}
