use bevy_egui::egui;
use egui::RichText;

use crate::grid::Grid;
use crate::grid_coloration::ColorGradient;

use super::{theme, widgets};

pub fn show(ui: &mut egui::Ui, grid: &mut Grid) {
    widgets::section(ui, "Color map");

    let current = grid.grid_coloration.gradient.name;
    for gradient in ColorGradient::all() {
        let selected = gradient.name == current;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(gradient.name)
                    .size(12.0)
                    .strong()
                    .color(if selected { theme::ACCENT } else { theme::TEXT }),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if selected {
                    ui.label(RichText::new("active").size(10.0).color(theme::ACCENT));
                }
            });
        });

        if widgets::gradient_bar(ui, &gradient, selected).clicked() {
            grid.grid_coloration.gradient = gradient;
        }
        ui.add_space(4.0);
    }

    ui.label(
        RichText::new("Keys 1–4 switch color maps.")
            .size(11.0)
            .color(theme::MUTED),
    );

    widgets::section(ui, "Rendering");

    ui.checkbox(&mut grid.grid_coloration.smooth, "Smooth cells")
        .on_hover_text("CPU bicubic interpolation. Trades detail for a fluid, smoke-like view.");

    if grid.grid_coloration.smooth {
        ui.label(
            RichText::new("Smooth mode resamples the grid into a texture capped at 800×800.")
                .size(11.0)
                .color(theme::MUTED),
        );
    } else {
        ui.label(
            RichText::new("Instanced cells, one draw call per grid.")
                .size(11.0)
                .color(theme::MUTED),
        );
    }

    widgets::section(ui, "Channels");

    let description = match grid.rule.num_channels {
        1 => "1 channel — state maps through the selected gradient.",
        2 => "2 channels — c₀ controls brightness, c₁ controls hue.",
        _ => "3 channels — c₀, c₁, c₂ map directly to red, green, blue.",
    };
    ui.label(RichText::new(description).size(11.0).color(theme::MUTED));

    ui.add_space(6.0);
    widgets::stat(ui, "Channels", format!("{}", grid.rule.num_channels));
    widgets::stat(
        ui,
        "Color map",
        grid.grid_coloration.gradient.name.to_string(),
    );
}
