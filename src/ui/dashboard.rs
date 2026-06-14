use egui::{Color32, RichText};
use std::sync::{Arc, RwLock};

use crate::state::AppState;

// Catppuccin Mocha palette
const MAUVE: Color32 = Color32::from_rgb(203, 166, 247);
const BLUE: Color32 = Color32::from_rgb(137, 180, 250);
const GREEN: Color32 = Color32::from_rgb(166, 227, 161);
const YELLOW: Color32 = Color32::from_rgb(249, 226, 175);
const RED: Color32 = Color32::from_rgb(243, 139, 168);
const PEACH: Color32 = Color32::from_rgb(250, 179, 135);

pub fn show(ui: &mut egui::Ui, state: &Arc<RwLock<AppState>>) {
    let s = state.read().unwrap();

    ui.columns(2, |cols| {
        // ── CPU ──────────────────────────────────────────────────
        cols[0].group(|ui| {
            ui.set_min_height(280.0);
            ui.label(RichText::new("CPU — Ryzen 9 9950X3D").strong().size(14.0));
            ui.separator();

            if let Some(cpu) = &s.cpu {
                ui.horizontal(|ui| {
                    ui.label(format!("Total: {:.1}%", cpu.total_usage));
                    ui.separator();
                    let avg = if cpu.frequency_mhz.is_empty() {
                        0
                    } else {
                        cpu.frequency_mhz.iter().sum::<u64>() / cpu.frequency_mhz.len() as u64
                    };
                    ui.label(format!("Avg freq: {} MHz", avg));
                });

                ui.add_space(6.0);
                ui.label(RichText::new("CCD0 — X3D (Mauve) · CCD1 — standard (Blue)").small().color(Color32::GRAY));
                ui.add_space(4.0);

                let n = cpu.usage_per_core.len();
                egui::Grid::new("cpu_grid")
                    .num_columns(4)
                    .spacing([6.0, 4.0])
                    .show(ui, |ui| {
                        for (i, &usage) in cpu.usage_per_core.iter().enumerate() {
                            let color = if i < n / 2 { MAUVE } else { BLUE };
                            ui.vertical(|ui| {
                                ui.label(RichText::new(format!("C{i}")).color(color).small());
                                ui.add(
                                    egui::ProgressBar::new(usage / 100.0)
                                        .fill(color)
                                        .desired_width(52.0),
                                );
                                ui.label(RichText::new(format!("{:.0}%", usage)).small());
                            });
                            if (i + 1) % 4 == 0 {
                                ui.end_row();
                            }
                        }
                    });
            } else {
                ui.label("Collecting CPU data...");
            }
        });

        // ── GPU ──────────────────────────────────────────────────
        cols[1].group(|ui| {
            ui.set_min_height(280.0);
            ui.label(RichText::new("GPU — RTX 5090 32 GB").strong().size(14.0));
            ui.separator();

            if let Some(gpu) = &s.gpu {
                egui::Grid::new("gpu_grid")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Utilization");
                        ui.add(
                            egui::ProgressBar::new(gpu.utilization_pct as f32 / 100.0)
                                .fill(util_color(gpu.utilization_pct as f32))
                                .text(format!("{}%", gpu.utilization_pct))
                                .desired_width(200.0),
                        );
                        ui.end_row();

                        ui.label("VRAM");
                        let vram_pct = gpu.memory_used_mb as f32 / gpu.memory_total_mb.max(1) as f32;
                        ui.add(
                            egui::ProgressBar::new(vram_pct)
                                .fill(PEACH)
                                .text(format!("{} / {} MB", gpu.memory_used_mb, gpu.memory_total_mb))
                                .desired_width(200.0),
                        );
                        ui.end_row();

                        ui.label("Power");
                        let pow_pct = gpu.power_draw_w / gpu.power_limit_w.max(1.0);
                        ui.add(
                            egui::ProgressBar::new(pow_pct)
                                .fill(RED)
                                .text(format!("{:.0} / {:.0} W", gpu.power_draw_w, gpu.power_limit_w))
                                .desired_width(200.0),
                        );
                        ui.end_row();

                        ui.label("Temperature");
                        ui.label(
                            RichText::new(format!("{}°C", gpu.temperature))
                                .color(temp_color(gpu.temperature)),
                        );
                        ui.end_row();

                        ui.label("SM Clock");
                        ui.label(format!("{} MHz", gpu.clock_sm_mhz));
                        ui.end_row();

                        ui.label("Mem Clock");
                        ui.label(format!("{} MHz", gpu.clock_mem_mhz));
                        ui.end_row();
                    });
            } else {
                ui.label("NVML unavailable — install nvidia drivers");
            }
        });
    });

    ui.add_space(8.0);

    // ── Top Processes ─────────────────────────────────────────────
    ui.group(|ui| {
        ui.label(RichText::new("Top Processes").strong().size(14.0));
        ui.separator();
        egui::ScrollArea::vertical()
            .max_height(160.0)
            .show(ui, |ui| {
                egui::Grid::new("procs")
                    .num_columns(4)
                    .spacing([20.0, 2.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("PID").strong().small());
                        ui.label(RichText::new("Name").strong().small());
                        ui.label(RichText::new("CPU %").strong().small());
                        ui.label(RichText::new("RAM MB").strong().small());
                        ui.end_row();

                        for p in &s.top_processes {
                            ui.label(RichText::new(p.pid.to_string()).small());
                            ui.label(RichText::new(&p.name).small());
                            let cpu_color = if p.cpu_pct > 50.0 { RED } else { GREEN };
                            ui.label(RichText::new(format!("{:.1}", p.cpu_pct)).color(cpu_color).small());
                            ui.label(RichText::new(p.mem_mb.to_string()).small());
                            ui.end_row();
                        }
                    });
            });
    });
}

fn util_color(pct: f32) -> Color32 {
    if pct > 80.0 { RED } else if pct > 50.0 { YELLOW } else { GREEN }
}

fn temp_color(t: u32) -> Color32 {
    if t > 85 { RED } else if t > 70 { YELLOW } else { GREEN }
}
