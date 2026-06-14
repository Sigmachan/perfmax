use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

use crate::{config::Config, state::AppState, ui, ui::settings::SettingsForm};

// Signals from tray → egui (static atomics, no GTK/channel needed)
pub static WINDOW_VISIBLE: AtomicBool = AtomicBool::new(true);
pub static OPTIMIZE_NOW: AtomicBool = AtomicBool::new(false);
pub static QUIT_APP: AtomicBool = AtomicBool::new(false);

// ── KDE StatusNotifierItem tray ──────────────────────────────────

pub struct PerfMaxTray {
    pub state: Arc<RwLock<AppState>>,
}

impl ksni::Tray for PerfMaxTray {
    fn icon_name(&self) -> String {
        "utilities-system-monitor".into()
    }

    fn title(&self) -> String {
        let s = self.state.read().unwrap();
        if s.ai.thinking {
            "PerfMax — optimizing…".into()
        } else {
            "PerfMax".into()
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "Show / Hide".into(),
                activate: Box::new(|_| {
                    let v = WINDOW_VISIBLE.load(Ordering::Relaxed);
                    WINDOW_VISIBLE.store(!v, Ordering::Relaxed);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Optimize Now".into(),
                activate: Box::new(|_| {
                    OPTIMIZE_NOW.store(true, Ordering::Relaxed);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|_| {
                    QUIT_APP.store(true, Ordering::Relaxed);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

// ── egui App ─────────────────────────────────────────────────────

#[derive(PartialEq)]
enum Tab {
    Dashboard,
    Ai,
    Memory,
    Settings,
}

pub struct PerfMaxApp {
    state: Arc<RwLock<AppState>>,
    config: Config,
    tab: Tab,
    settings_form: SettingsForm,
    download_tx: tokio::sync::mpsc::UnboundedSender<()>,
}

impl PerfMaxApp {
    pub fn new(
        state: Arc<RwLock<AppState>>,
        config: Config,
        download_tx: tokio::sync::mpsc::UnboundedSender<()>,
    ) -> Self {
        Self {
            state,
            config,
            tab: Tab::Dashboard,
            settings_form: SettingsForm::default(),
            download_tx,
        }
    }
}

impl eframe::App for PerfMaxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll tray signals
        if QUIT_APP.load(Ordering::Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if OPTIMIZE_NOW.swap(false, Ordering::Relaxed) {
            if let Ok(mut s) = self.state.write() {
                s.optimize_trigger = true;
            }
        }

        // Hide to tray on window close
        if ctx.input(|i| i.viewport().close_requested()) {
            WINDOW_VISIBLE.store(false, Ordering::Relaxed);
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        let visible = WINDOW_VISIBLE.load(Ordering::Relaxed);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(visible));

        // Repaint every second for live metrics
        ctx.request_repaint_after(std::time::Duration::from_secs(1));

        if !visible {
            return;
        }

        // ── Top bar ──
        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⚡ PerfMax")
                        .strong()
                        .size(16.0)
                        .color(egui::Color32::from_rgb(203, 166, 247)),
                );
                ui.separator();
                ui.selectable_value(&mut self.tab, Tab::Dashboard, "Dashboard");
                ui.selectable_value(&mut self.tab, Tab::Ai, "AI");
                ui.selectable_value(&mut self.tab, Tab::Memory, "Memory");
                ui.selectable_value(&mut self.tab, Tab::Settings, "Settings");

                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        let thinking = self.state.read().unwrap().ai.thinking;
                        if thinking {
                            ui.spinner();
                            ui.label("Optimizing...");
                        } else if ui.button("⚡ Optimize Now").clicked() {
                            if let Ok(mut s) = self.state.write() {
                                s.optimize_trigger = true;
                            }
                        }
                    },
                );
            });
        });

        // ── Status bar ──
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let s = self.state.read().unwrap();
                if let Some(cpu) = &s.cpu {
                    ui.label(
                        egui::RichText::new(format!("CPU {:.0}%", cpu.total_usage))
                            .small()
                            .color(egui::Color32::from_rgb(203, 166, 247)),
                    );
                    ui.separator();
                }
                if let Some(gpu) = &s.gpu {
                    ui.label(
                        egui::RichText::new(format!(
                            "GPU {}% · {:.0}W · {} MB",
                            gpu.utilization_pct, gpu.power_draw_w, gpu.memory_used_mb
                        ))
                        .small()
                        .color(egui::Color32::from_rgb(137, 180, 250)),
                    );
                    ui.separator();
                }
                if !s.active_window.is_empty() {
                    let win: String = s.active_window.chars().take(60).collect();
                    ui.label(egui::RichText::new(format!("🪟 {win}")).small());
                }
            });
        });

        // ── Content ──
        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Dashboard => ui::dashboard::show(ui, &self.state),
            Tab::Ai => ui::ai_panel::show(ui, &self.state),
            Tab::Memory => ui::memory::show(ui, &self.state),
            Tab::Settings => ui::settings::show(
                ui,
                &mut self.config,
                &self.state,
                &mut self.settings_form,
                &self.download_tx,
            ),
        });
    }
}
