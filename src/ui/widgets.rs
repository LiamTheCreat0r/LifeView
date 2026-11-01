use bevy::color::Srgba;
use bevy_egui::egui;
use egui::{Color32, Response, RichText, Sense, Stroke, Ui};

use crate::grid_coloration::ColorGradient;

use super::theme;

pub fn section(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .size(10.0)
            .strong()
            .color(theme::MUTED),
    );
    ui.add_space(1.0);
    ui.separator();
    ui.add_space(2.0);
}

pub fn topbar_button(ui: &mut Ui, label: &str, tooltip: &str) -> Response {
    ui.add(egui::Button::new(RichText::new(label).size(12.0)).min_size(egui::vec2(0.0, 26.0)))
        .on_hover_text(tooltip)
}

pub fn card<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    egui::Frame::new()
        .fill(theme::ELEVATED)
        .stroke(Stroke::new(1.0_f32, theme::BORDER))
        .corner_radius(8)
        .inner_margin(10)
        .show(ui, add_contents)
}

pub fn dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.5, color);
}

pub fn stat(ui: &mut Ui, label: &str, value: impl Into<String>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.0).color(theme::MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(value.into())
                    .size(11.0)
                    .monospace()
                    .color(theme::TEXT),
            );
        });
    });
}

pub fn gradient_bar(ui: &mut Ui, gradient: &ColorGradient, selected: bool) -> Response {
    let width = ui.available_width().max(80.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 18.0), Sense::click());
    let painter = ui.painter();

    const SEGMENTS: usize = 48;
    for index in 0..SEGMENTS {
        let start = index as f32 / SEGMENTS as f32;
        let end = (index + 1) as f32 / SEGMENTS as f32;
        let color = gradient_color(gradient, (start + end) * 0.5);
        // Inset the fill slightly so the rounded border stays clean at the ends.
        let mut min = egui::pos2(rect.left() + rect.width() * start, rect.top() + 1.0);
        let mut max = egui::pos2(rect.left() + rect.width() * end, rect.bottom() - 1.0);
        if index == 0 {
            min.x += 1.0;
        }
        if index == SEGMENTS - 1 {
            max.x -= 1.0;
        }
        painter.rect_filled(egui::Rect::from_min_max(min, max), 0.0, color);
    }

    painter.rect_stroke(
        rect,
        4.0,
        if selected {
            Stroke::new(2.0_f32, theme::ACCENT)
        } else {
            Stroke::new(1.0_f32, theme::BORDER)
        },
        egui::StrokeKind::Inside,
    );

    response.on_hover_text(format!("Use the {} color map", gradient.name))
}

pub fn gradient_color(gradient: &ColorGradient, value: f32) -> Color32 {
    let linear = gradient.lerp(value.clamp(0.0, 1.0));
    let srgb = Srgba::from(linear);
    Color32::from_rgba_unmultiplied(
        (srgb.red.clamp(0.0, 1.0) * 255.0) as u8,
        (srgb.green.clamp(0.0, 1.0) * 255.0) as u8,
        (srgb.blue.clamp(0.0, 1.0) * 255.0) as u8,
        (srgb.alpha.clamp(0.0, 1.0) * 255.0) as u8,
    )
}
