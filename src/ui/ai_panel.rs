use egui::{Color32, RichText};
use std::sync::{Arc, RwLock};

use crate::state::AppState;

const GREEN: Color32 = Color32::from_rgb(166, 227, 161);
const RED: Color32 = Color32::from_rgb(243, 139, 168);
const YELLOW: Color32 = Color32::from_rgb(249, 226, 175);

pub fn show(ui: &mut egui::Ui, state: &Arc<RwLock<AppState>>) {
    let s = state.read().unwrap();

    ui.label(RichText::new("AI Optimizer — MiMo-7B-RL").strong().size(14.0));
    ui.separator();

    // Status row
    ui.horizontal(|ui| {
        if s.ai.thinking {
            ui.spinner();
            ui.label("Analyzing system and generating tuning commands...");
        } else if let Some(err) = &s.ai.error {
            ui.label(RichText::new(format!("⚠ {err}")).color(RED).small());
        } else if let Some(updated) = s.ai.last_updated {
            let secs = updated.elapsed().as_secs();
            ui.label(RichText::new(format!("✓ Last update {}s ago", secs)).color(GREEN).small());
        } else {
            ui.label(RichText::new("Waiting for first optimization cycle...").color(YELLOW).small());
        }
    });

    ui.add_space(8.0);

    ui.columns(2, |cols| {
        // Last recommendation (raw AI output)
        cols[0].group(|ui| {
            ui.label(RichText::new("AI Output").strong());
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("ai_raw")
                .max_height(300.0)
                .show(ui, |ui| {
                    if s.ai.last_recommendation.is_empty() {
                        ui.label(RichText::new("No output yet").color(Color32::GRAY));
                    } else {
                        ui.add(
                            egui::TextEdit::multiline(
                                &mut s.ai.last_recommendation.as_str(),
                            )
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY),
                        );
                    }
                });
        });

        // Parsed commands
        cols[1].group(|ui| {
            ui.label(RichText::new("Commands Queued").strong());
            ui.separator();
            if s.ai.last_commands.is_empty() {
                ui.label(RichText::new("None").color(Color32::GRAY));
            } else {
                for cmd in &s.ai.last_commands {
                    ui.label(RichText::new(cmd).monospace().small());
                }
            }
        });
    });

    ui.add_space(8.0);

    // Command history
    ui.group(|ui| {
        ui.label(RichText::new("Execution History").strong());
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("cmd_history")
            .max_height(200.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for entry in &s.command_history {
                    let color = if entry.contains("[OK]") {
                        GREEN
                    } else if entry.contains("[ERR]") || entry.contains("[FAIL]") {
                        RED
                    } else if entry.contains("[DRY]") {
                        YELLOW
                    } else {
                        Color32::GRAY
                    };
                    ui.label(RichText::new(entry).color(color).monospace().small());
                }
            });
    });
}
