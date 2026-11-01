use bevy_egui::egui;
use egui::RichText;

use crate::grid::{GenerationType, Grid};

use super::{theme, widgets};

pub fn show(ui: &mut egui::Ui, grid: &mut Grid) {
    widgets::section(ui, "Grid");

    ui.add(egui::Slider::new(&mut grid.cell_size, 4.0..=32.0).text("Cell size"))
        .on_hover_text("Pixels per cell. The grid is resized to fill the viewport.");

    widgets::stat(
        ui,
        "Dimensions",
        format!("{} × {}", grid.width, grid.height),
    );
    widgets::stat(
        ui,
        "Cells",
        format!("{}", grid.width.saturating_mul(grid.height)),
    );

    widgets::section(ui, "Initialization");

    egui::ComboBox::from_id_salt("generation_type")
        .selected_text(match grid.generation_type {
            GenerationType::EMPTY => "Empty",
            GenerationType::RANDOM => "Random",
            GenerationType::BLOB => "Blob",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut grid.generation_type, GenerationType::EMPTY, "Empty");
            ui.selectable_value(&mut grid.generation_type, GenerationType::RANDOM, "Random");
            ui.selectable_value(&mut grid.generation_type, GenerationType::BLOB, "Blob");
        });

    ui.horizontal(|ui| {
        if ui.button("Reinitialize").clicked() {
            grid.init();
        }
        if ui.button("Clear").clicked() {
            grid.clear();
        }
    });

    widgets::section(ui, "Simulation");

    ui.horizontal(|ui| {
        let pause_label = if grid.paused { "Play" } else { "Pause" };
        if ui.button(pause_label).clicked() {
            grid.pause();
        }
        if ui.button("Step").clicked() {
            grid.step();
        }
        if ui.button("Reset").clicked() {
            grid.init();
        }
    });

    ui.add(egui::Slider::new(&mut grid.generations_per_second, 1.0..=200.0).text("Gen/s"))
        .on_hover_text("Target simulation speed. One generation updates every cell once.");

    widgets::stat(ui, "Generation", format!("{}", grid.generation_count));
    widgets::stat(ui, "Status", if grid.paused { "Paused" } else { "Running" });

    widgets::section(ui, "Topology");

    ui.label(
        RichText::new("Toroidal — the left/right and top/bottom edges wrap.")
            .size(11.0)
            .color(theme::MUTED),
    );
    ui.label(
        RichText::new("Finite-boundary mode is not implemented.")
            .size(11.0)
            .color(theme::MUTED),
    );

    ui.add_space(8.0);
    ui.label(
        RichText::new("Left-drag paints cells · right-drag erases")
            .size(11.0)
            .color(theme::MUTED),
    );
}
