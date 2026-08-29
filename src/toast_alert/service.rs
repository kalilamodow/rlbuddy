use crate::common::channel::{Receiver, Sender};
use eframe::egui;
use std::time::{Duration, Instant};

// how long to show for
const SHOW_FOR: Duration = Duration::from_secs(3);
// in between close and destroying window reference, egui poops itself if
// we stop rendering the deferred viewport before it gets a chance to close normally
const CLOSE_DELAY: Duration = Duration::from_secs(1);

// from edge of screen
const TOAST_OFFSET: f32 = 16.0;
const TOAST_WIDTH: f32 = 200.0;
const TOAST_HEIGHT: f32 = 75.0;

#[derive(Debug, Clone)]
pub struct Toast {
    message: String,
    created_at: Instant,
    closed_at: Option<Instant>,
}

impl Toast {
    pub fn new(message: String) -> Self {
        Self {
            message,
            created_at: Instant::now(),
            closed_at: None,
        }
    }
}

pub struct ToastAlertService {
    receiver: Receiver<Toast>,
    active: Vec<Toast>,
    ctx: egui::Context,
}

impl ToastAlertService {
    pub fn new(ctx: egui::Context) -> Self {
        Self {
            receiver: Receiver::new(),
            active: Vec::new(),
            ctx,
        }
    }

    pub fn sender(&self) -> Sender<Toast> {
        self.receiver.send()
    }

    pub fn update(&mut self) {
        self.create_new_toasts();
        self.clear_old_toasts();
        self.render_toasts();
    }

    fn render_toasts(&self) {
        for toast in &self.active {
            // try to position from the right of the screen, otherwise from the left of the screen
            let toast_x = self
                .ctx
                .input(|i| i.viewport().monitor_size)
                .map_or(TOAST_OFFSET, |monitor| {
                    monitor.x - TOAST_WIDTH - TOAST_OFFSET
                });

            let text = toast.message.clone();
            self.ctx.show_viewport_deferred(
                egui::ViewportId::from_hash_of(toast.created_at),
                egui::ViewportBuilder::default()
                    .with_always_on_top()
                    .with_inner_size(egui::vec2(TOAST_WIDTH, TOAST_HEIGHT))
                    .with_position(egui::pos2(toast_x, TOAST_OFFSET))
                    .with_decorations(false)
                    .with_mouse_passthrough(true),
                move |ui, _| {
                    egui::CentralPanel::no_frame()
                        .frame(
                            egui::Frame::canvas(ui.style())
                                .corner_radius(egui::CornerRadius::same(4)),
                        )
                        .show(ui, |ui| {
                            ui.centered_and_justified(|ui| {
                                ui.label(egui::RichText::new(&text).line_height(Some(20.0)))
                            })
                        });
                },
            );
        }
    }

    fn create_new_toasts(&mut self) {
        while let Some(new_toast) = self.receiver.try_recv() {
            self.active.push(new_toast);
        }
    }

    fn clear_old_toasts(&mut self) {
        let now = Instant::now();

        // not closed in the first place, or is within grace period
        self.active.retain(|toast| {
            toast
                .closed_at
                .is_none_or(|closed_at| now.duration_since(closed_at) <= CLOSE_DELAY)
        });

        for toast in &mut self.active {
            if toast.closed_at.is_none() && now.duration_since(toast.created_at) >= SHOW_FOR {
                self.ctx.send_viewport_cmd_to(
                    egui::ViewportId::from_hash_of(toast.created_at),
                    egui::ViewportCommand::Close,
                );

                toast.closed_at = Some(Instant::now());
            }
        }
    }
}
