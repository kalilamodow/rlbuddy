use crate::common::ReadWriteStateHandle;
use crate::core::persistence::AppData;
use crate::gamepad::GamepadService;
use crate::gamepad::overlay::GamepadOverlayService;
use crate::hotkey::HotkeyService;
use crate::music_control::MusicControlService;
use crate::my_stats::MyStatsWidget;
use crate::{
    auto_setup::AutoSetupWidget,
    common::eventsource::EventReceiver,
    discord,
    map_loader::MapLoaderService,
    matches::{CurrentMatchWidget, MatchesService, PastMatchesWidget},
    player_info::PlayerInfoService,
    stats_api::{RLEvent, StatsApi},
    toast_alert::{MatchNotificatorService, ToastAlertService},
};
use discord::DiscordService;
use eframe::egui::{self, Response, Ui, ViewportCommand};
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hasher};
use std::thread;
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
    fn ui(&mut self, ui: &mut Ui) -> Response;
}

fn visuals_with_transparency(visuals: &mut egui::Visuals, transparency: u8) {
    visuals.panel_fill = egui::Color32::from_rgba_unmultiplied(
        visuals.panel_fill.r(),
        visuals.panel_fill.g(),
        visuals.panel_fill.b(),
        255 - transparency,
    );
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PanelId(u64);

impl PanelId {
    fn from_name(name: &str) -> PanelId {
        let mut hasher = DefaultHasher::new();
        hasher.write(name.as_bytes());
        PanelId(hasher.finish())
    }
}

struct AppPanel {
    name: &'static str,
    id: PanelId,
    panel: Box<dyn Panel>,
}

impl AppPanel {
    fn new<F>(feature: F) -> Self
    where
        F: Panel + 'static,
    {
        Self {
            name: feature.name(),
            id: PanelId::from_name(feature.name()),
            panel: Box::new(feature),
        }
    }
}

impl egui::Widget for &mut AppPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        self.panel.ui(ui)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AppSettings {
    pub transparency: u8,
}

pub struct RlBuddyApp {
    app_settings: ReadWriteStateHandle<AppSettings>,
    open_panels: Vec<PanelId>,

    overlay_tx: mpsc::Sender<bool>,
    overlay_rx: mpsc::Receiver<bool>,
    prev_hide_pos: Option<egui::Pos2>,
    stats_api_events: EventReceiver<RLEvent>,

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

        let app_settings = ReadWriteStateHandle::new(app_data.app_settings);
        let (overlay_tx, overlay_rx) = mpsc::channel();

        let mut stats_api_service = StatsApi::new();
        let toast_service = ToastAlertService::new(ctx.clone());
        let matches_service = MatchesService::new(&ctx, &mut stats_api_service);
        let match_notificator_service =
            MatchNotificatorService::new(&matches_service, &mut stats_api_service, &toast_service);
        let discord_service = DiscordService::new(&matches_service, &mut stats_api_service);
        let mut gamepad_service = GamepadService::new();
        let gamepad_overlay_service = GamepadOverlayService::new(ctx.clone(), &gamepad_service);
        let map_loader_service = MapLoaderService::new();
        let player_info_service = PlayerInfoService::new(ctx.clone());
        let hotkey_service = HotkeyService::new(&mut gamepad_service, &overlay_tx);
        let music_service = MusicControlService::new(&mut stats_api_service);

        let app = RlBuddyApp {
            overlay_tx,
            overlay_rx,
            prev_hide_pos: None,
            open_panels: app_data.open_panels,

            stats_api_events: stats_api_service.subscribe(),
            panels: vec![
                AppPanel::new(CurrentMatchWidget::new(
                    &matches_service,
                    &player_info_service,
                )),
                AppPanel::new(PastMatchesWidget::new(
                    &matches_service,
                    &player_info_service,
                )),
                AppPanel::new(MyStatsWidget::new(&matches_service)),
                AppPanel::new(music_service.panel()),
                AppPanel::new(player_info_service.panel()),
                AppPanel::new(AutoSetupWidget::new()),
                AppPanel::new(map_loader_service.panel()),
                AppPanel::new(gamepad_overlay_service.panel()),
                AppPanel::new(match_notificator_service.panel()),
                AppPanel::new(hotkey_service.panel()),
                AppPanel::new(discord_service.panel()),
                AppPanel::new(AppSettingsWidget::new(app_settings.clone())),
            ],
            services: vec![
                Box::new(hotkey_service),
                Box::new(music_service),
                Box::new(stats_api_service),
                Box::new(player_info_service),
                Box::new(matches_service),
                Box::new(toast_service),
                Box::new(match_notificator_service),
                Box::new(discord_service),
                Box::new(gamepad_service),
                Box::new(gamepad_overlay_service),
                Box::new(map_loader_service),
            ],

            app_settings,
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
            app_settings: self.app_settings.read().clone(),
            saved_window_dimensions: ctx.input(|i| {
                i.viewport().outer_rect.and_then(|outer| {
                    i.viewport()
                        .inner_rect
                        .map(|inner| (outer.left_top(), inner.size()))
                })
            }),
            open_panels: self.open_panels.clone(),
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
        visuals_with_transparency(ui.visuals_mut(), self.app_settings.read().transparency);

        egui::Panel::bottom("bottom_panel").show_inside(ui, |ui| {
            egui::ComboBox::from_label("")
                .selected_text("Widgets")
                .show_ui(ui, |ui| {
                    for panel in &mut self.panels {
                        let is_open = self.open_panels.contains(&panel.id);
                        if ui.selectable_label(is_open, panel.name).clicked() {
                            if is_open {
                                self.open_panels.retain(|p| *p != panel.id);
                            } else {
                                self.open_panels.push(panel.id);
                            }
                        }
                    }
                });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(PanelsWidget {
                    open_panels: &mut self.open_panels,
                    panels: &mut self.panels,
                });
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

pub struct AppSettingsWidget {
    handle: ReadWriteStateHandle<AppSettings>,
}

impl AppSettingsWidget {
    pub fn new(handle: ReadWriteStateHandle<AppSettings>) -> Self {
        AppSettingsWidget { handle }
    }
}

impl Panel for AppSettingsWidget {
    fn name(&self) -> &'static str {
        "Settings"
    }

    fn ui(&mut self, ui: &mut Ui) -> Response {
        ui.vertical_centered_justified(|ui| {
            let mut settings = self.handle.write();
            ui.add(
                egui::Slider::new(&mut settings.transparency, u8::MIN..=u8::MAX)
                    .text("App transparency"),
            );
        })
        .response
    }
}

pub struct PanelsWidget<'a> {
    open_panels: &'a mut Vec<PanelId>,
    panels: &'a mut Vec<AppPanel>,
}

impl<'a> egui::Widget for PanelsWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical_centered_justified(|ui| {
            let mut to_close: Option<usize> = None;
            let mut to_swap: Option<(usize, usize)> = None;

            for (index, panel_id) in self.open_panels.iter().enumerate() {
                let Some(panel) = self.panels.iter_mut().find(|p| p.id == *panel_id) else {
                    eprintln!("could not find panel with id '{panel_id:?}'");
                    continue;
                };

                let frame = egui::Frame::group(ui.style()).fill(ui.style().visuals.faint_bg_color);

                frame.show(ui, |ui| {
                    ui.columns_const(|[c1, c2]| {
                        c1.label(egui::RichText::new(panel.name).strong());
                        c2.with_layout(egui::Layout::right_to_left(egui::Align::Min), |c2| {
                            if c2.small_button("X").clicked() {
                                to_close = Some(index);
                            }

                            c2.add_enabled_ui(index != self.open_panels.len() - 1, |c2| {
                                if c2.small_button("\\/").clicked() {
                                    to_swap = Some((index, index + 1));
                                }
                            });

                            c2.add_enabled_ui(index != 0, |c2| {
                                if c2.small_button("/\\").clicked() {
                                    to_swap = Some((index, index - 1));
                                }
                            });
                        });
                    });

                    ui.separator();
                    ui.add(panel);
                });

                ui.add_space(4.0);
            }

            if let Some(to_close) = to_close {
                self.open_panels.remove(to_close);
            }
            if let Some(to_swap) = to_swap {
                self.open_panels.swap(to_swap.0, to_swap.1);
            }
        })
        .response
    }
}
