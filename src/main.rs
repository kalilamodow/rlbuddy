#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod auto_setup;
mod common;
mod discord;
mod hotkey;
mod matches;
mod music_control;
mod my_stats;
mod player_info;
mod rocket_league;
mod settings;
mod stats_api;
mod toast_alert;

use eframe::egui;

fn main() -> eframe::Result {
    let gui_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([350.0, 600.0])
            .with_transparent(true)
            .with_title("rlbuddy (Not connected)"),
        ..Default::default()
    };

    eframe::run_native(
        "rlbuddy",
        gui_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app::RlBuddyApp::new(cc)))
        }),
    )
}
