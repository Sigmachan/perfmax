use egui::RichText;

use crate::config::Config;

pub fn show(ui: &mut egui::Ui, config: &mut Config) {
    ui.label(RichText::new("Settings").strong().size(14.0));
    ui.separator();

    egui::CollapsingHeader::new("AI")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("ai_settings")
                .num_columns(2)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Endpoint");
                    ui.text_edit_singleline(&mut config.ai.endpoint);
                    ui.end_row();

                    ui.label("Model");
                    ui.text_edit_singleline(&mut config.ai.model);
                    ui.end_row();

                    ui.label("Interval (s)");
                    ui.add(egui::DragValue::new(&mut config.ai.interval_secs).range(5..=600));
                    ui.end_row();

                    ui.label("Max tokens");
                    ui.add(egui::DragValue::new(&mut config.ai.max_tokens).range(64..=4096));
                    ui.end_row();
                });
        });

    ui.add_space(8.0);

    egui::CollapsingHeader::new("Optimizer")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("opt_settings")
                .num_columns(2)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Enabled");
                    ui.checkbox(&mut config.optimizer.enabled, "");
                    ui.end_row();

                    ui.label("Dry run (log only, don't execute)");
                    ui.checkbox(&mut config.optimizer.dry_run, "");
                    ui.end_row();

                    ui.label("ryzenadj path");
                    ui.text_edit_singleline(&mut config.optimizer.ryzenadj_path);
                    ui.end_row();
                });
        });

    ui.add_space(12.0);

    if ui.button("Save").clicked() {
        match config.save() {
            Ok(_) => { /* success */ }
            Err(e) => eprintln!("Config save error: {e}"),
        }
    }
}
