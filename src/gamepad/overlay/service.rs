use crate::common::ReadWriteStateHandle;
use crate::common::savedata::{load_service_data, save_service_data};
use crate::core::app::{Panel, Service, ServiceWithUi};
use crate::gamepad::GamepadService;
use crate::gamepad::overlay::widget::GamepadOverlayWidget;
use crate::gamepad::service::GamepadStateHandle;
use crate::gamepad::service::NO_GAMEPAD_STATE;
use eframe::egui;
use eframe::egui::{Color32, CornerRadius, Frame, Stroke, ViewportBuilder, ViewportId};
use eframe::epaint::StrokeKind;
use emath::Rect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GamepadOverlayServiceSettings {
    pub enabled: bool,
    pub window_pos: egui::Pos2,
}

pub struct GamepadOverlayService {
    settings: ReadWriteStateHandle<GamepadOverlayServiceSettings>,
    gamepad: GamepadStateHandle,
    ctx: egui::Context,

    // to only reposition when first opening which prevents glitchy movement
    was_enabled_before: bool,
}

const DATA_ID: &str = "gamepad_overlay_savedata";

impl GamepadOverlayService {
    #[must_use]
    pub fn new(ctx: egui::Context, gamepad_service: &GamepadService) -> Self {
        Self {
            settings: ReadWriteStateHandle::new(load_service_data(DATA_ID)),
            gamepad: gamepad_service.gamepad_state_handle(),
            ctx,
            was_enabled_before: false,
        }
    }

    pub fn update(&mut self) {
        let mut reposition = false;

        {
            let settings = self.settings.read();
            if !settings.enabled {
                self.was_enabled_before = false;
                return;
            }
        }

        if !self.was_enabled_before {
            reposition = true;
            self.was_enabled_before = true;
        }

        self.render_viewport(reposition);
    }

    fn render_viewport(&self, reposition: bool) {
        let gamepad_state = self.gamepad.read();
        let mut settings = self.settings.write();

        let mut viewport = ViewportBuilder::default()
            .with_title("Gamepad Overlay")
            .with_inner_size(egui::vec2(300.0, 175.0))
            .with_transparent(true)
            .with_taskbar(false)
            .with_always_on_top();

        if reposition {
            viewport = viewport.with_position(settings.window_pos);
        }

        self.ctx.show_viewport_immediate(
            ViewportId::from_hash_of("gamepad overlay"),
            viewport,
            |ui, _| {
                egui::CentralPanel::default()
                    .frame(Frame::canvas(ui.style()))
                    .show_inside(ui, |ui| {
                        if let Some(outer_rect) = ui.ctx().input(|i| i.viewport().outer_rect) {
                            settings.window_pos = outer_rect.min;
                        }

                        let stroke = Stroke::new(1.5f32, Color32::WHITE);
                        let painter = ui.painter();

                        painter.hline(50.0..=250.0, 50.0, stroke); // top part
                        painter.vline(50.0, 50.0..=160.0, stroke); // left side
                        painter.vline(250.0, 50.0..=160.0, stroke); // right side

                        painter.hline(50.0..=100.0, 160.0, stroke); // left bottom
                        painter.hline(200.0..=250.0, 160.0, stroke); // right bottom

                        painter.vline(100.0, 110.0..=160.0, stroke); // left inner side
                        painter.vline(200.0, 110.0..=160.0, stroke); // right inner side

                        painter.hline(100.0..=200.0, 110.0, stroke); // bottom inner

                        let gp = gamepad_state.as_ref().unwrap_or(&NO_GAMEPAD_STATE);

                        let draw_joystick = |draw_x, joy_x: f32, joy_y: f32| {
                            painter.circle_stroke(egui::pos2(draw_x, 110.0), 15.0, stroke); // outline
                            painter.circle_filled(
                                egui::pos2(draw_x + (joy_x * 8.0), 110.0 + (joy_y * -8.0)),
                                14.0,
                                Color32::WHITE,
                            );
                        };

                        draw_joystick(100.0, gp.joy_left_x, gp.joy_left_y);
                        draw_joystick(200.0, gp.joy_right_x, gp.joy_right_y);

                        // main buttons
                        let circle_if = |coordinates, condition| {
                            if condition {
                                // bigger because the outline wraps it
                                painter.circle_filled(coordinates, 7.0, Color32::WHITE);
                            } else {
                                painter.circle_stroke(coordinates, 5.0, stroke);
                            }
                        };

                        circle_if(egui::pos2(72.5, 65.0), gp.up);
                        circle_if(egui::pos2(72.5, 90.0), gp.down);
                        circle_if(egui::pos2(60.0, 77.5), gp.left);
                        circle_if(egui::pos2(85.0, 77.5), gp.right);

                        circle_if(egui::pos2(227.5, 65.0), gp.north);
                        circle_if(egui::pos2(227.5, 90.0), gp.south);
                        circle_if(egui::pos2(215.0, 77.5), gp.west);
                        circle_if(egui::pos2(240.0, 77.5), gp.east);

                        let rect_if = |rect, condition| {
                            if condition {
                                painter.rect_filled(rect, CornerRadius::default(), Color32::WHITE);
                            } else {
                                painter.rect_stroke(
                                    rect,
                                    CornerRadius::default(),
                                    stroke,
                                    StrokeKind::Inside,
                                );
                            }
                        };

                        rect_if(
                            Rect::from_x_y_ranges(70.0..=100.0, 39.0..=50.0),
                            gp.bumper_left,
                        );
                        rect_if(
                            Rect::from_x_y_ranges(70.0..=100.0, 20.0..=38.0),
                            gp.trigger_left,
                        );
                        rect_if(
                            Rect::from_x_y_ranges(200.0..=230.0, 39.0..=50.0),
                            gp.bumper_right,
                        );
                        rect_if(
                            Rect::from_x_y_ranges(200.0..=230.0, 20.0..=38.0),
                            gp.trigger_right,
                        );

                        rect_if(Rect::from_x_y_ranges(110.0..=130.0, 65.0..=75.0), gp.select);
                        rect_if(Rect::from_x_y_ranges(170.0..=190.0, 65.0..=75.0), gp.start);
                    });

                if ui.ctx().input(|i| {
                    i.viewport().close_requested() || i.viewport().minimized.unwrap_or_default()
                }) {
                    settings.enabled = false;
                }

                // NO maximize >:(
                if ui
                    .ctx()
                    .input(|i| i.viewport().maximized.unwrap_or_default())
                {
                    ui.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                }
            },
        );
    }

    #[must_use]
    pub fn settings_handle(&self) -> ReadWriteStateHandle<GamepadOverlayServiceSettings> {
        self.settings.clone()
    }
}

impl Service for GamepadOverlayService {
    fn update(&mut self) {
        self.update();
    }

    fn save(&self) {
        save_service_data(DATA_ID, &*self.settings.read());
    }
}

impl ServiceWithUi for GamepadOverlayService {
    fn panel(&self) -> impl Panel + 'static {
        GamepadOverlayWidget::new(self)
    }
}
