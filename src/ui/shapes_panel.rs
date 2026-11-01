use bevy_egui::egui;
use std::collections::HashMap;

use bevy::prelude::*;
use egui::RichText;

use crate::grid::Grid;
use crate::grid_coloration::ColorGradient;
use crate::shapes::{Shape, Shapes};

use super::{UiState, theme, widgets};

#[derive(Resource, Default)]
pub struct ShapePreviewCache {
    textures: HashMap<(String, &'static str), egui::TextureHandle>,
}

pub fn show(
    ui: &mut egui::Ui,
    grid: &mut Grid,
    shapes: &Shapes,
    state: &mut UiState,
    previews: &mut ShapePreviewCache,
) {
    let gradient = grid.grid_coloration.gradient.clone();

    ui.horizontal(|ui| {
        let clear_width = if state.shape_search.is_empty() {
            0.0
        } else {
            26.0
        };
        let width = (ui.available_width() - clear_width).max(80.0);
        ui.add_sized(
            egui::vec2(width, 24.0),
            egui::TextEdit::singleline(&mut state.shape_search).hint_text("Search shapes…"),
        );
        if !state.shape_search.is_empty() && ui.small_button("×").clicked() {
            state.shape_search.clear();
        }
    });

    if let Some(selected) = state.selected_shape.clone()
        && let Some(shape) = shapes.0.iter().find(|shape| shape.name == selected)
    {
        ui.add_space(4.0);
        selected_card(ui, shape, &gradient, previews);
    }

    ui.add_space(6.0);
    let query = state.shape_search.trim().to_lowercase();
    let category_map = shapes.by_category_and_genus();
    let total_matches: usize = category_map
        .values()
        .flat_map(|genus_map| genus_map.values())
        .map(|genus_shapes| filter_shapes(genus_shapes, &query).len())
        .sum();

    for (category, genus_map) in &category_map {
        let category_count: usize = genus_map
            .values()
            .map(|genus_shapes| filter_shapes(genus_shapes, &query).len())
            .sum();

        if category_count == 0 {
            continue;
        }

        egui::CollapsingHeader::new(format!("{category} ({category_count})"))
            .default_open(true)
            .open((!query.is_empty()).then_some(true))
            .show(ui, |ui| {
                for (genus, genus_shapes) in genus_map {
                    let filtered = filter_shapes(genus_shapes, &query);
                    if filtered.is_empty() {
                        continue;
                    }

                    let display = shapes.genus_display(genus);
                    let header = if query.is_empty() {
                        format!("{} ({})", display, filtered.len())
                    } else {
                        format!("{} ({}/{})", display, filtered.len(), genus_shapes.len())
                    };

                    egui::CollapsingHeader::new(header)
                        .default_open(!query.is_empty())
                        .open((!query.is_empty()).then_some(true))
                        .show(ui, |ui| {
                            for shape in filtered {
                                let response = ui.add_sized(
                                    egui::vec2(ui.available_width(), 24.0),
                                    egui::Button::new(RichText::new(&shape.name).size(12.0))
                                        .truncate(),
                                );

                                let clicked = response.clicked();
                                response.on_hover_ui(|ui| {
                                    ui.set_max_width(150.0);
                                    let texture =
                                        preview_texture(ui.ctx(), previews, shape, &gradient);
                                    ui.add(
                                        egui::Image::new((texture.id(), egui::vec2(128.0, 128.0)))
                                            .fit_to_exact_size(egui::vec2(128.0, 128.0)),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {} cells",
                                            shape.genus,
                                            shape.cells_state.len()
                                        ))
                                        .size(10.0)
                                        .color(theme::MUTED),
                                    );
                                });

                                if clicked {
                                    grid.clear();
                                    grid.spawn_shape(shape.name.clone(), shapes.0.clone());
                                    state.selected_shape = Some(shape.name.clone());
                                }
                            }
                        });
                }
            });
    }

    if total_matches == 0 {
        ui.add_space(12.0);
        ui.label(
            RichText::new("No shapes match this search.")
                .size(11.0)
                .color(theme::MUTED),
        );
    }
}

fn filter_shapes<'a>(shapes: &[&'a Shape], query: &str) -> Vec<&'a Shape> {
    if query.is_empty() {
        return shapes.to_vec();
    }

    shapes
        .iter()
        .copied()
        .filter(|shape| {
            shape.name.to_lowercase().contains(query) || shape.genus.to_lowercase().contains(query)
        })
        .collect()
}

fn selected_card(
    ui: &mut egui::Ui,
    shape: &Shape,
    gradient: &ColorGradient,
    previews: &mut ShapePreviewCache,
) {
    widgets::card(ui, |ui| {
        ui.horizontal(|ui| {
            let texture = preview_texture(ui.ctx(), previews, shape, gradient);
            egui::Frame::new()
                .fill(theme::BG)
                .stroke(egui::Stroke::new(1.0_f32, theme::BORDER))
                .corner_radius(6)
                .inner_margin(4)
                .show(ui, |ui| {
                    ui.add(
                        egui::Image::new((texture.id(), egui::vec2(64.0, 64.0)))
                            .fit_to_exact_size(egui::vec2(64.0, 64.0)),
                    );
                });

            ui.vertical(|ui| {
                ui.label(
                    RichText::new(&shape.name)
                        .size(12.0)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.label(
                    RichText::new(&shape.genus_display)
                        .size(10.0)
                        .color(theme::MUTED),
                );
                ui.label(
                    RichText::new(format!("{} cells", shape.cells_state.len()))
                        .size(10.0)
                        .color(theme::MUTED),
                );
            });
        });
    });
}

fn preview_texture(
    ctx: &egui::Context,
    previews: &mut ShapePreviewCache,
    shape: &Shape,
    gradient: &ColorGradient,
) -> egui::TextureHandle {
    let key = (shape.name.clone(), gradient.name);
    if let Some(texture) = previews.textures.get(&key) {
        return texture.clone();
    }

    const SIZE: usize = 96;
    let mut pixels = vec![egui::Color32::TRANSPARENT; SIZE * SIZE];

    if !shape.cells_pos.is_empty() {
        let min_x = shape.cells_pos.iter().map(|pos| pos.x).min().unwrap_or(0);
        let max_x = shape.cells_pos.iter().map(|pos| pos.x).max().unwrap_or(0);
        let min_y = shape.cells_pos.iter().map(|pos| pos.y).min().unwrap_or(0);
        let max_y = shape.cells_pos.iter().map(|pos| pos.y).max().unwrap_or(0);
        let span_x = (max_x - min_x + 1).max(1) as f32;
        let span_y = (max_y - min_y + 1).max(1) as f32;
        let scale = (SIZE as f32 - 6.0) / span_x.max(span_y);
        let offset_x = (SIZE as f32 - span_x * scale) / 2.0;
        let offset_y = (SIZE as f32 - span_y * scale) / 2.0;

        for (pos, value) in shape.cells_pos.iter().zip(&shape.cells_state) {
            let px = (offset_x + (pos.x - min_x) as f32 * scale) as i32;
            let py = (offset_y + (max_y - pos.y) as f32 * scale) as i32;
            if (0..SIZE as i32).contains(&px) && (0..SIZE as i32).contains(&py) {
                pixels[py as usize * SIZE + px as usize] =
                    widgets::gradient_color(gradient, *value);
            }
        }
    }

    let image = egui::ColorImage::new([SIZE, SIZE], pixels);
    let texture = ctx.load_texture(
        format!("shape:{}:{}", shape.name, gradient.name),
        image,
        egui::TextureOptions::NEAREST,
    );
    previews.textures.insert(key, texture.clone());
    texture
}
