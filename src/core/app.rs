use crate::core::persistence::AppData;
use crate::gamepad::GamepadService;
use crate::gamepad::overlay::service::GamepadOverlayService;
use crate::gamepad::overlay::widget::GamepadOverlayWidget;
use crate::hotkey::HotkeyService;
use crate::music_control::MusicControlService;
use crate::{
    auto_setup::AutoSetupWidget,
    common::eventsource::EventReceiver,
    discord,
    map_loader::{MapLoaderService, MapLoaderWidget},
    matches::{CurrentMatchWidget, MatchesService, PastMatchesWidget},
    my_stats::MyStatsWidget,
    player_info::PlayerInfoService,
    settings::SettingsWidget,
    stats_api::{RLEvent, StatsApi},
    toast_alert::{MatchNotificatorService, ToastAlertService},
};
use eframe::egui::{self, Ui, ViewportCommand};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc, thread};
use std::{sync::mpsc, time::Duration};

pub trait Service {
    fn update(&mut self);
    fn save(&self) {}

    // for egui windows or whatever
    fn render(&mut self, _ui: &mut Ui) {}
}

pub trait ServiceWithUi: Service {
    // 'static otherwise it thinks the panel needs a reference to the Service for
    // its whole lifetime
    fn panel(&self) -> impl Panel + 'static;
}

pub trait Panel {
    fn name(&self) -> &'static str;
    fn ui(&mut self, ui: &mut Ui) -> egui::Response;
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegacyPanel {
    CurrentMatch,
    PastMatches,
    MyStats,
    Discord,
    MapLoader,
    AutoSetup,
    GamepadOverlay,
    Settings,
}

impl std::fmt::Display for LegacyPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LegacyPanel::CurrentMatch => "Lobby",
                LegacyPanel::PastMatches => "History",
                LegacyPanel::MyStats => "Session",
                LegacyPanel::Discord => "Discord",
                LegacyPanel::MapLoader => "Custom Maps",
                LegacyPanel::AutoSetup => "Stats API Setup",
                LegacyPanel::GamepadOverlay => "Gamepad Overlay",
                LegacyPanel::Settings => "Settings",
            }
        )
    }
}

const OPENABLE_LEGAGY_PANELS: [LegacyPanel; 8] = [
    LegacyPanel::CurrentMatch,
    LegacyPanel::Discord,
    LegacyPanel::MyStats,
    LegacyPanel::PastMatches,
    LegacyPanel::MapLoader,
    LegacyPanel::AutoSetup,
    LegacyPanel::GamepadOverlay,
    LegacyPanel::Settings,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenLegacyPanelList(Vec<LegacyPanel>);

impl Default for OpenLegacyPanelList {
    fn default() -> Self {
        Self(vec![LegacyPanel::CurrentMatch, LegacyPanel::AutoSetup])
    }
}

impl std::ops::Deref for OpenLegacyPanelList {
    type Target = Vec<LegacyPanel>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for OpenLegacyPanelList {
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

struct AppPanel {
    name: &'static str,
    open: bool,
    panel: Box<dyn Panel>,
}

impl AppPanel {
    fn new<F>(feature: F) -> Self
    where
        F: Panel + 'static,
    {
        Self {
            name: feature.name(),
            open: false,
            panel: Box::new(feature),
        }
    }
}

impl egui::Widget for &mut AppPanel {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        self.panel.ui(ui)
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
    open_panels: OpenLegacyPanelList,

    stats_api_events: EventReceiver<RLEvent>,

    discord_service: discord::DiscordService,
    discord_widget: discord::DiscordWidget,

    matches_service: MatchesService,
    current_match: CurrentMatchWidget,
    past_matches: PastMatchesWidget,
    my_stats_widget: MyStatsWidget,

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
    panels: Vec<AppPanel>,
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

        let hotkey_service = HotkeyService::new(&mut gamepad_service, &overlay_tx);
        let music_service = MusicControlService::new(&mut stats_api_service);

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

            map_loader_widget: MapLoaderWidget::new(&map_loader_service),
            map_loader_service,

            gamepad_overlay_widget: GamepadOverlayWidget::new(&gamepad_overlay_service),
            gamepad_overlay_service,

            gamepad_service,

            toast_service,
            auto_setup_widget: AutoSetupWidget::new(),
            open_panels: app_data.open_panels,

            panels: vec![
                AppPanel::new(music_service.panel()),
                AppPanel::new(hotkey_service.panel()),
                AppPanel::new(player_info_service.panel()),
            ],
            services: vec![
                Box::new(hotkey_service),
                Box::new(music_service),
                Box::new(stats_api_service),
                Box::new(player_info_service),
            ],
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
        for service in &self.services {
            service.save();
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
    }
}

impl eframe::App for RlBuddyApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.gamepad_service.update();
        self.matches_service.update();
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

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        visuals_with_transparency(ui.visuals_mut(), *self.current_transparency.borrow());

        egui::Panel::bottom("bottom_panel").show_inside(ui, |ui| {
            egui::ComboBox::from_label("")
                .selected_text("Widgets")
                .show_ui(ui, |ui| {
                    for panel in OPENABLE_LEGAGY_PANELS {
                        let open = self.open_panels.contains(&panel);

                        if ui.selectable_label(open, panel.to_string()).clicked() {
                            if open {
                                self.open_panels.retain(|p| p != &panel);
                            } else {
                                self.open_panels.push(panel);
                            }
                        }
                    }

                    for feature in &mut self.panels {
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
                    let mut to_close: Option<LegacyPanel> = None;

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
                                LegacyPanel::CurrentMatch => ui.add(&mut self.current_match),
                                LegacyPanel::Discord => ui.add(&mut self.discord_widget),
                                LegacyPanel::MyStats => ui.add(&mut self.my_stats_widget),
                                LegacyPanel::PastMatches => ui.add(&mut self.past_matches),
                                LegacyPanel::MapLoader => ui.add(&mut self.map_loader_widget),
                                LegacyPanel::AutoSetup => ui.add(&mut self.auto_setup_widget),
                                LegacyPanel::GamepadOverlay => {
                                    ui.add(&mut self.gamepad_overlay_widget)
                                }
                                LegacyPanel::Settings => ui.add(&mut self.settings_widget),
                            };
                        });

                        ui.add_space(4.0);
                    }

                    for panel in self.panels.iter_mut().filter(|f| f.open) {
                        let frame =
                            egui::Frame::group(ui.style()).fill(ui.style().visuals.faint_bg_color);

                        frame.show(ui, |ui| {
                            ui.columns_const(|[c1, c2]| {
                                c1.label(egui::RichText::new(panel.name).strong());
                                c2.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Min),
                                    |c2| {
                                        if c2.small_button("X").clicked() {
                                            panel.open = false;
                                        }
                                    },
                                );
                            });

                            ui.separator();
                            ui.add(panel);
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

        for service in &mut self.services {
            service.render(ui);
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}
