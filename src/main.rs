use std::sync::{Arc, RwLock};
use tracing::info;

mod ai;
mod app;
mod config;
mod metrics;
mod optimizer;
mod state;
mod ui;

use app::{PerfMaxApp, PerfMaxTray};
use config::Config;
use state::AppState;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "perfmax=info".into()),
        )
        .init();

    info!("PerfMax starting");

    let config = Config::load().unwrap_or_default();
    let shared = Arc::new(RwLock::new(AppState::default()));

    // Background tokio runtime (separate from egui's event loop)
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    // Metrics task
    rt.spawn(metrics::run(shared.clone(), config.metrics.interval_ms));

    // AI task
    rt.spawn(ai::run(shared.clone(), config.clone()));

    // Optimizer task
    rt.spawn(optimizer::run(shared.clone(), config.optimizer.dry_run));

    // KDE tray (ksni 0.2.x — run() is blocking, not async)
    let tray_state = shared.clone();
    std::thread::spawn(move || {
        let svc = ksni::TrayService::new(PerfMaxTray {
            state: tray_state,
        });
        let _ = svc.run();
    });

    // egui window — blocks main thread
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PerfMax")
            .with_inner_size([980.0, 640.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "PerfMax",
        native_options,
        Box::new(|cc| {
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(PerfMaxApp::new(shared, config)))
        }),
    )
    .expect("eframe failed");
}

fn apply_theme(ctx: &egui::Context) {
    // Catppuccin Mocha — manual (no external crate dep)
    let mut v = egui::Visuals::dark();
    v.panel_fill = egui::Color32::from_rgb(30, 30, 46);
    v.window_fill = egui::Color32::from_rgb(36, 39, 58);
    v.extreme_bg_color = egui::Color32::from_rgb(24, 24, 37);
    v.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(49, 50, 68);
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(58, 60, 78);
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(69, 71, 90);
    v.widgets.active.bg_fill = egui::Color32::from_rgb(203, 166, 247); // mauve
    v.selection.bg_fill = egui::Color32::from_rgb(203, 166, 247);
    v.hyperlink_color = egui::Color32::from_rgb(137, 180, 250);
    v.override_text_color = Some(egui::Color32::from_rgb(205, 214, 244));
    ctx.set_visuals(v);
}
