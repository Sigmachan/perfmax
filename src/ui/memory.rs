use egui::RichText;
use std::sync::{Arc, RwLock};

use crate::state::AppState;

pub fn show(ui: &mut egui::Ui, state: &Arc<RwLock<AppState>>) {
    let s = state.read().unwrap();

    ui.label(RichText::new("Screen Memory").strong().size(14.0));
    ui.separator();

    ui.label("Active window tracking:");
    ui.group(|ui| {
        ui.label(RichText::new(&s.active_window).monospace());
    });

    ui.add_space(12.0);
    ui.label(RichText::new("Full OCR / screenpipe integration — coming next.").italics());
    ui.label("Will capture and index everything on screen, searchable via AI.");
}
