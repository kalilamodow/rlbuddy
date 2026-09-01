use crate::common::ReadWriteStateHandle;
use crate::gamepad::GamepadService;
use crate::gamepad::service::GamepadStateHandle;
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
}

impl GamepadOverlayService {
    pub fn new(
        savedata: GamepadOverlayServiceSettings,
        ctx: egui::Context,
        gamepad_service: &GamepadService,
    ) -> Self {
        Self {
            settings: ReadWriteStateHandle::new(savedata),
            gamepad: gamepad_service.gamepad_state_handle(),
            ctx,
        }
    }

    pub fn update(&mut self) {
        {
            let settings = self.settings.read();
            if !settings.enabled {
                return;
            }
        }

        self.render();
    }

    fn render(&self) {
        let gamepad_state = self.gamepad.read();
        let mut settings = self.settings.write();

        self.ctx.show_viewport_immediate(
            ViewportId::from_hash_of("gamepad overlay"),
            ViewportBuilder::default()
                .with_inner_size(egui::vec2(300.0, 175.0))
                .with_transparent(true)
                .with_taskbar(false)
                .with_position(settings.window_pos)
                .with_always_on_top(),
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

                        let Some(gp) = gamepad_state.as_ref() else {
                            return;
                        };

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
                    });
            },
        );
    }

    pub fn settings_handle(&self) -> ReadWriteStateHandle<GamepadOverlayServiceSettings> {
        self.settings.clone()
    }
}
