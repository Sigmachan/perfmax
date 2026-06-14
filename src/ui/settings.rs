use egui::{Color32, RichText};
use std::sync::{Arc, RwLock};

use crate::config::{Config, Provider};
use crate::model_manager;
use crate::server_manager;
use crate::state::AppState;

const GREEN: Color32 = Color32::from_rgb(166, 227, 161);
const RED: Color32 = Color32::from_rgb(243, 139, 168);
const YELLOW: Color32 = Color32::from_rgb(249, 226, 175);
const MAUVE: Color32 = Color32::from_rgb(203, 166, 247);

pub struct SettingsForm {
    pub new_name: String,
    pub new_endpoint: String,
    pub new_model: String,
    pub new_key: String,
    pub download_triggered: bool,
}

impl Default for SettingsForm {
    fn default() -> Self {
        Self {
            new_name: String::new(),
            new_endpoint: String::new(),
            new_model: String::new(),
            new_key: String::new(),
            download_triggered: false,
        }
    }
}

pub fn show(
    ui: &mut egui::Ui,
    config: &mut Config,
    state: &Arc<RwLock<AppState>>,
    form: &mut SettingsForm,
    download_tx: &tokio::sync::mpsc::UnboundedSender<()>,
) {
    let s = state.read().unwrap();

    // ── Local AI ────────────────────────────────────────────────────
    ui.group(|ui| {
        ui.label(RichText::new("Local AI — MiMo-7B-RL").strong().size(13.0));
        ui.separator();

        let model_ok = model_manager::model_exists();
        let server_bin = server_manager::find_llama_server();
        let server_online = s.local_server_online;

        egui::Grid::new("local_ai_status")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Model");
                if model_ok {
                    ui.label(RichText::new(format!(
                        "✓ {}",
                        model_manager::default_model_path().display()
                    )).color(GREEN).small());
                } else {
                    ui.label(RichText::new("✗ not downloaded").color(RED).small());
                }
                ui.end_row();

                ui.label("llama-server");
                match &server_bin {
                    Some(p) => ui.label(
                        RichText::new(format!("✓ {}", p.display())).color(GREEN).small()
                    ),
                    None => ui.label(
                        RichText::new("✗ not found — sudo pacman -S llama.cpp").color(RED).small()
                    ),
                };
                ui.end_row();

                ui.label("Server (port 8081)");
                if server_online {
                    ui.label(RichText::new("● online").color(GREEN).small());
                } else {
                    ui.label(RichText::new("○ offline").color(RED).small());
                }
                ui.end_row();
            });

        // Download progress
        if let Some(dl) = &s.model_download {
            if !dl.done {
                let pct = if dl.bytes_total > 0 {
                    dl.bytes_done as f32 / dl.bytes_total as f32
                } else {
                    0.0
                };
                let done_mb = dl.bytes_done / 1024 / 1024;
                let total_mb = dl.bytes_total / 1024 / 1024;
                ui.add(
                    egui::ProgressBar::new(pct)
                        .text(format!("Downloading… {done_mb}/{total_mb} MB"))
                        .animate(true),
                );
            } else {
                ui.label(RichText::new("✓ Download complete").color(GREEN));
            }
        }

        drop(s); // release read lock before buttons

        ui.horizontal(|ui| {
            if !model_manager::model_exists() {
                if ui.button("⬇ Download MiMo-7B-RL (Q8, ~7 GB)").clicked()
                    && !form.download_triggered
                {
                    form.download_triggered = true;
                    let _ = download_tx.send(());
                }
            }
            if server_manager::find_llama_server().is_some()
                && model_manager::model_exists()
            {
                let s2 = state.read().unwrap();
                if !s2.local_server_online && ui.button("▶ Start Server").clicked() {
                    // Launch happens in main via the server manager
                    // Just set a flag in state for now
                    drop(s2);
                    if let Ok(mut st) = state.write() {
                        st.optimize_trigger = false; // placeholder, actual start in bg
                    }
                }
            }
        });

        // Bring back the read guard for providers section
        let s = state.read().unwrap();
        drop(s);
    });

    ui.add_space(8.0);

    // ── Providers ───────────────────────────────────────────────────
    ui.group(|ui| {
        ui.label(RichText::new("AI Providers").strong().size(13.0));
        ui.separator();

        let s = state.read().unwrap();
        let mut remove_idx: Option<usize> = None;

        for (i, provider) in config.ai.providers.iter().enumerate() {
            let status = s.provider_status.get(&provider.name);
            let online = status.map(|(ok, _)| *ok).unwrap_or(false);
            let dot = if online {
                RichText::new("●").color(GREEN)
            } else {
                RichText::new("○").color(Color32::GRAY)
            };

            ui.horizontal(|ui| {
                ui.label(dot);
                let is_active = provider.name == config.ai.active;
                if ui
                    .selectable_label(is_active, RichText::new(&provider.name).strong())
                    .clicked()
                {
                    config.ai.active = provider.name.clone();
                }
                ui.label(
                    RichText::new(format!("  {}  [{}]", provider.endpoint, provider.model))
                        .small()
                        .color(Color32::GRAY),
                );
                if provider.api_key.is_some() {
                    ui.label(RichText::new("🔑").small());
                }
                if ui.small_button("✕").clicked() {
                    remove_idx = Some(i);
                }
            });

            // Show available models if online
            if online {
                if let Some((_, models)) = status {
                    if !models.is_empty() {
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            ui.label(
                                RichText::new(format!("models: {}", models.join(", ")))
                                    .small()
                                    .color(Color32::GRAY),
                            );
                        });
                    }
                }
            }
        }

        drop(s);

        if let Some(idx) = remove_idx {
            config.ai.providers.remove(idx);
        }

        ui.separator();

        // ── Add custom provider (BYOK) ───────────────────────────────
        ui.label(RichText::new("Add Custom Provider (BYOK)").strong());
        egui::Grid::new("byok_form")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut form.new_name);
                ui.end_row();

                ui.label("Endpoint");
                ui.text_edit_singleline(&mut form.new_endpoint);
                ui.end_row();

                ui.label("Model");
                ui.text_edit_singleline(&mut form.new_model);
                ui.end_row();

                ui.label("API Key (optional)");
                ui.add(egui::TextEdit::singleline(&mut form.new_key).password(true));
                ui.end_row();
            });

        // Quick-fill buttons for known cloud providers
        ui.horizontal(|ui| {
            ui.label(RichText::new("Quick fill:").small().color(Color32::GRAY));
            if ui.small_button("OpenRouter").clicked() {
                form.new_name = "OpenRouter".into();
                form.new_endpoint = "https://openrouter.ai/api/v1".into();
                form.new_model = "mistralai/mistral-7b-instruct".into();
            }
            if ui.small_button("Groq").clicked() {
                form.new_name = "Groq".into();
                form.new_endpoint = "https://api.groq.com/openai/v1".into();
                form.new_model = "llama-3.3-70b-versatile".into();
            }
            if ui.small_button("Together.ai").clicked() {
                form.new_name = "Together.ai".into();
                form.new_endpoint = "https://api.together.xyz/v1".into();
                form.new_model = "meta-llama/Llama-3-8b-chat-hf".into();
            }
            if ui.small_button("OpenAI").clicked() {
                form.new_name = "OpenAI".into();
                form.new_endpoint = "https://api.openai.com/v1".into();
                form.new_model = "gpt-4o-mini".into();
            }
        });

        let can_add = !form.new_name.is_empty()
            && !form.new_endpoint.is_empty()
            && !form.new_model.is_empty();

        if ui
            .add_enabled(can_add, egui::Button::new("＋ Add Provider"))
            .clicked()
        {
            let key = if form.new_key.is_empty() {
                None
            } else {
                Some(form.new_key.clone())
            };
            config.ai.providers.push(Provider {
                name: form.new_name.clone(),
                endpoint: form.new_endpoint.clone(),
                model: form.new_model.clone(),
                api_key: key,
            });
            form.new_name.clear();
            form.new_endpoint.clear();
            form.new_model.clear();
            form.new_key.clear();
        }
    });

    ui.add_space(8.0);

    // ── Optimizer ───────────────────────────────────────────────────
    ui.group(|ui| {
        ui.label(RichText::new("Optimizer").strong().size(13.0));
        ui.separator();
        egui::Grid::new("opt_settings")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("AI interval (s)");
                ui.add(egui::DragValue::new(&mut config.ai.interval_secs).range(5..=600));
                ui.end_row();

                ui.label("Max tokens");
                ui.add(egui::DragValue::new(&mut config.ai.max_tokens).range(64..=4096));
                ui.end_row();

                ui.label("Enabled");
                ui.checkbox(&mut config.optimizer.enabled, "");
                ui.end_row();

                ui.label("Dry run");
                ui.checkbox(&mut config.optimizer.dry_run, "");
                ui.end_row();

                ui.label("ryzenadj path");
                ui.text_edit_singleline(&mut config.optimizer.ryzenadj_path);
                ui.end_row();
            });
    });

    ui.add_space(8.0);

    if ui
        .button(RichText::new("Save").color(MAUVE))
        .clicked()
    {
        let _ = config.save();
    }
}
