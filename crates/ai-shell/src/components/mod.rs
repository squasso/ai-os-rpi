// Componenti UI riusabili — card, badge, separatori stilizzati
use egui::{Ui, Frame, Color32, Rounding};

pub fn card(ui: &mut Ui, fill: Color32, content: impl FnOnce(&mut Ui)) {
    Frame::none()
        .fill(fill)
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, content);
}

pub fn badge(ui: &mut Ui, text: &str, color: Color32) {
    Frame::none()
        .fill(color.linear_multiply(0.2))
        .rounding(Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(6.0, 2.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).small().color(color));
        });
}
