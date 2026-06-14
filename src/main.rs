use std::sync::{Arc, RwLock};
use tracing::info;

mod ai;
mod app;
mod config;
mod discovery;
mod metrics;
mod model_manager;
mod optimizer;
mod server_manager;
mod state;
mod ui;

use app::{PerfMaxApp, PerfMaxTray};
use config::Config;
use server_manager::ServerManager;
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
    let server_mgr = Arc::new(ServerManager::new());

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(6)
        .enable_all()
        .build()
        .expect("tokio runtime");

    // Model download channel (UI → background task)
    let (dl_tx, mut dl_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    // Metrics
    rt.spawn(metrics::run(shared.clone(), config.metrics.interval_ms));

    // AI
    rt.spawn(ai::run(shared.clone(), config.clone()));

    // Optimizer
    rt.spawn(optimizer::run(shared.clone(), config.optimizer.dry_run));

    // Provider discovery (probes all configured endpoints)
    rt.spawn(discovery::run(shared.clone(), config.clone()));

    // Server health monitor + auto-restart
    {
        let mgr = server_mgr.clone();
        rt.spawn(server_manager::run(shared.clone(), mgr));
    }

    // Model download listener
    {
        let dl_state = shared.clone();
        rt.spawn(async move {
            while dl_rx.recv().await.is_some() {
                let state = dl_state.clone();
                tokio::spawn(async move {
                    if let Err(e) = model_manager::download_model(state.clone()).await {
                        if let Ok(mut s) = state.write() {
                            if let Some(dl) = &mut s.model_download {
                                dl.error = Some(e.to_string());
                            }
                        }
                    }
                });
            }
        });
    }

    // Auto-start local server if model exists and binary found
    if model_manager::model_exists() && server_manager::find_llama_server().is_some() {
        if let Err(e) = server_mgr.start() {
            info!("Local server auto-start skipped: {e}");
        }
    }

    // KDE tray
    let tray_state = shared.clone();
    std::thread::spawn(move || {
        let svc = ksni::TrayService::new(PerfMaxTray { state: tray_state });
        let _ = svc.run();
    });

    // egui window — blocks main thread
    eframe::run_native(
        "PerfMax",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("PerfMax")
                .with_inner_size([1000.0, 660.0])
                .with_min_inner_size([720.0, 480.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(PerfMaxApp::new(shared, config, dl_tx)))
        }),
    )
    .expect("eframe failed");
}

fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = egui::Color32::from_rgb(30, 30, 46);
    v.window_fill = egui::Color32::from_rgb(36, 39, 58);
    v.extreme_bg_color = egui::Color32::from_rgb(24, 24, 37);
    v.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(49, 50, 68);
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(58, 60, 78);
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(69, 71, 90);
    v.widgets.active.bg_fill = egui::Color32::from_rgb(203, 166, 247);
    v.selection.bg_fill = egui::Color32::from_rgb(203, 166, 247);
    v.hyperlink_color = egui::Color32::from_rgb(137, 180, 250);
    v.override_text_color = Some(egui::Color32::from_rgb(205, 214, 244));
    ctx.set_visuals(v);
}
