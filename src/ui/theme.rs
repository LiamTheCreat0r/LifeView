use bevy_egui::egui;
use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals};

pub const BG: Color32 = Color32::from_rgb(9, 12, 17);
pub const PANEL: Color32 = Color32::from_rgb(15, 20, 28);
pub const ELEVATED: Color32 = Color32::from_rgb(22, 29, 40);
pub const TOPBAR: Color32 = Color32::from_rgb(12, 16, 23);
pub const BORDER: Color32 = Color32::from_rgb(38, 49, 64);
pub const TEXT: Color32 = Color32::from_rgb(225, 234, 244);
pub const MUTED: Color32 = Color32::from_rgb(136, 151, 169);
pub const ACCENT: Color32 = Color32::from_rgb(94, 234, 212);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(24, 74, 70);
pub const WARN: Color32 = Color32::from_rgb(245, 158, 11);

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut visuals = Visuals::dark();

    visuals.panel_fill = PANEL;
    visuals.window_fill = ELEVATED;
    visuals.extreme_bg_color = BG;
    visuals.faint_bg_color = Color32::from_rgb(19, 25, 34);
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = ACCENT_DIM;
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = WARN;
    visuals.slider_trailing_fill = true;
    visuals.handle_shape = egui::style::HandleShape::Circle;
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.button_frame = true;
    visuals.collapsing_header_frame = false;

    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, MUTED);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

    visuals.widgets.inactive.bg_fill = ELEVATED;
    visuals.widgets.inactive.weak_bg_fill = ELEVATED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(30, 41, 55);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(30, 41, 55);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

    visuals.widgets.active.bg_fill = ACCENT_DIM;
    visuals.widgets.active.weak_bg_fill = ACCENT_DIM;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);

    visuals.widgets.open.bg_fill = ELEVATED;
    visuals.widgets.open.weak_bg_fill = ELEVATED;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.open.corner_radius = CornerRadius::same(6);

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.interact_size.y = 24.0;
    style.spacing.slider_width = 120.0;
    style.spacing.combo_width = 130.0;
    style.spacing.indent = 14.0;
    style.spacing.scroll.bar_width = 8.0;

    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(16.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(11.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(12.0, FontFamily::Monospace),
        ),
    ]
    .into();

    ctx.set_style(style);
}
