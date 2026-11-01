mod display;
mod rules;
mod shapes_panel;
mod simulate;
mod theme;
mod widgets;

use bevy::app::{App, Plugin};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::grid::Grid;
use crate::shapes::Shapes;

pub const PANEL_WIDTH: f32 = 300.0;
pub const TOPBAR_HEIGHT: f32 = 48.0;
pub const STATUS_HEIGHT: f32 = 24.0;

/// Layout values shared by the UI, camera, grid sizing, and mouse painting.
#[derive(Resource, Debug)]
pub struct UiMetrics {
    pub panel_width: f32,
    pub topbar_height: f32,
    pub status_height: f32,
}

impl Default for UiMetrics {
    fn default() -> Self {
        Self {
            panel_width: PANEL_WIDTH,
            topbar_height: TOPBAR_HEIGHT,
            status_height: STATUS_HEIGHT,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Simulate,
    Display,
    Shapes,
    Rules,
}

#[derive(Resource, Debug)]
pub struct UiState {
    pub tab: Tab,
    pub shape_search: String,
    pub selected_shape: Option<String>,
    pub panel_open: bool,
    theme_applied: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            tab: Tab::default(),
            shape_search: String::new(),
            selected_shape: None,
            panel_open: true,
            theme_applied: false,
        }
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        assert!(app.is_plugin_added::<bevy_egui::EguiPlugin>());

        app.init_resource::<UiMetrics>()
            .init_resource::<UiState>()
            .init_resource::<shapes_panel::ShapePreviewCache>()
            .add_systems(EguiPrimaryContextPass, ui);
    }
}

fn ui(
    mut contexts: EguiContexts,
    grid: Option<ResMut<Grid>>,
    shapes: Option<Res<Shapes>>,
    diagnostics: Res<DiagnosticsStore>,
    mut state: ResMut<UiState>,
    mut metrics: ResMut<UiMetrics>,
    mut previews: ResMut<shapes_panel::ShapePreviewCache>,
) -> Result {
    let Some(mut grid) = grid else {
        return Ok(());
    };
    let Some(shapes) = shapes else {
        return Ok(());
    };

    let ctx = contexts.ctx_mut()?;
    if !state.theme_applied {
        theme::apply(ctx);
        state.theme_applied = true;
    }

    handle_shortcuts(ctx, &mut grid, &mut state);

    // Bars first so they span the full window; the side panel fills the space
    // between them.
    topbar(ctx, &mut grid, &mut state);
    statusbar(ctx, &grid, &state, &diagnostics);

    if state.panel_open {
        let panel = egui::SidePanel::left("main_panel")
            .resizable(false)
            .exact_width(PANEL_WIDTH)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ctx, |ui| {
                tabs(ui, &mut state.tab);
                ui.add_space(6.0);
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(8.0);
                        match state.tab {
                            Tab::Simulate => simulate::show(ui, &mut grid),
                            Tab::Display => display::show(ui, &mut grid),
                            Tab::Shapes => shapes_panel::show(
                                ui,
                                &mut grid,
                                &shapes,
                                &mut state,
                                &mut previews,
                            ),
                            Tab::Rules => rules::show(ui, &mut grid),
                        }
                        ui.add_space(16.0);
                    });
            });
        metrics.panel_width = panel.response.rect.width();
    } else {
        metrics.panel_width = 0.0;
    }

    metrics.topbar_height = TOPBAR_HEIGHT;
    metrics.status_height = STATUS_HEIGHT;

    Ok(())
}

fn topbar(ctx: &egui::Context, grid: &mut Grid, state: &mut UiState) {
    egui::TopBottomPanel::top("topbar")
        .exact_height(TOPBAR_HEIGHT)
        .frame(
            egui::Frame::new()
                .fill(theme::TOPBAR)
                .inner_margin(egui::Margin::symmetric(12, 4)),
        )
        .show(ctx, |ui| {
            let width = ui.available_width();
            ui.horizontal_centered(|ui| {
                ui.label(
                    egui::RichText::new("LifeView")
                        .size(16.0)
                        .strong()
                        .color(theme::ACCENT),
                );
                if width > 560.0 {
                    ui.label(egui::RichText::new("LENIA").size(12.0).color(theme::MUTED));
                }
                ui.add_space(4.0);
                ui.separator();

                let pause_label = if grid.paused { "Play" } else { "Pause" };
                if widgets::topbar_button(ui, pause_label, "Play/pause (Space)").clicked() {
                    grid.pause();
                }
                if widgets::topbar_button(ui, "Step", "Advance one generation (S)").clicked() {
                    grid.step();
                }
                if width > 680.0 {
                    if widgets::topbar_button(ui, "Reset", "Reinitialize the grid (R)").clicked() {
                        grid.init();
                    }
                    if widgets::topbar_button(ui, "Clear", "Clear all cells (C)").clicked() {
                        grid.clear();
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let panel_label = if state.panel_open {
                        "Hide panel"
                    } else {
                        "Show panel"
                    };
                    if widgets::topbar_button(ui, panel_label, "Toggle inspector (H)").clicked() {
                        state.panel_open = !state.panel_open;
                    }

                    if ui.available_width() > 170.0 {
                        ui.spacing_mut().slider_width = 90.0;
                        ui.add(
                            egui::Slider::new(&mut grid.generations_per_second, 1.0..=200.0)
                                .show_value(false),
                        )
                        .on_hover_text("Simulation speed — generations per second");
                        ui.label(
                            egui::RichText::new(format!("{:.0}/s", grid.generations_per_second))
                                .monospace()
                                .size(11.0)
                                .color(theme::MUTED),
                        );
                    }

                    if ui.available_width() > 80.0 {
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!("gen {}", grid.generation_count))
                                .monospace()
                                .size(11.0)
                                .color(theme::MUTED),
                        );
                    }
                });
            });
        });
}

fn statusbar(ctx: &egui::Context, grid: &Grid, state: &UiState, diagnostics: &DiagnosticsStore) {
    egui::TopBottomPanel::bottom("statusbar")
        .exact_height(STATUS_HEIGHT)
        .frame(
            egui::Frame::new()
                .fill(theme::TOPBAR)
                .inner_margin(egui::Margin::symmetric(12, 0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                let (label, color) = if grid.paused {
                    ("Paused", theme::MUTED)
                } else {
                    ("Running", theme::ACCENT)
                };
                widgets::dot(ui, color);
                ui.label(egui::RichText::new(label).size(11.0).color(color));
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("{} × {}", grid.width, grid.height))
                        .monospace()
                        .size(11.0)
                        .color(theme::MUTED),
                );
                if ui.available_width() > 280.0 {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "{} cell{} · {} ch",
                            grid.width * grid.height,
                            if grid.width * grid.height == 1 {
                                ""
                            } else {
                                "s"
                            },
                            grid.rule.num_channels
                        ))
                        .monospace()
                        .size(11.0)
                        .color(theme::MUTED),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let fps = diagnostics
                        .get(&FrameTimeDiagnosticsPlugin::FPS)
                        .and_then(|diagnostic| diagnostic.smoothed());
                    if let Some(fps) = fps {
                        ui.label(
                            egui::RichText::new(format!("{fps:.0} fps"))
                                .monospace()
                                .size(11.0)
                                .color(theme::MUTED),
                        );
                    }
                    if let Some(name) = &state.selected_shape
                        && ui.available_width() > 120.0
                    {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(name)
                                    .size(11.0)
                                    .color(theme::ACCENT)
                                    .italics(),
                            )
                            .truncate(),
                        );
                    }
                });
            });
        });
}

fn tabs(ui: &mut egui::Ui, tab: &mut Tab) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);
        for (value, label) in [
            (Tab::Simulate, "Simulate"),
            (Tab::Display, "Display"),
            (Tab::Shapes, "Shapes"),
            (Tab::Rules, "Rules"),
        ] {
            let selected = *tab == value;
            let button =
                egui::Button::new(egui::RichText::new(label).size(12.0).color(if selected {
                    theme::ACCENT
                } else {
                    theme::MUTED
                }))
                .fill(if selected {
                    theme::ACCENT_DIM
                } else {
                    egui::Color32::TRANSPARENT
                })
                .stroke(egui::Stroke::NONE)
                .corner_radius(6);
            if ui.add(button).clicked() {
                *tab = value;
            }
        }
    });
}

fn handle_shortcuts(ctx: &egui::Context, grid: &mut Grid, state: &mut UiState) {
    if ctx.wants_keyboard_input() {
        return;
    }

    let tab_shortcut = ctx.input_mut(|input| {
        [
            egui::Key::Num1,
            egui::Key::Num2,
            egui::Key::Num3,
            egui::Key::Num4,
        ]
        .iter()
        .position(|key| input.consume_key(egui::Modifiers::COMMAND, *key))
    });
    if let Some(index) = tab_shortcut {
        state.tab = [Tab::Simulate, Tab::Display, Tab::Shapes, Tab::Rules][index];
    }

    let (pause, step, reset, clear, smooth, panel, gradient) = ctx.input(|input| {
        (
            input.key_pressed(egui::Key::Space),
            input.key_pressed(egui::Key::S),
            input.key_pressed(egui::Key::R),
            input.key_pressed(egui::Key::C),
            input.key_pressed(egui::Key::F),
            input.key_pressed(egui::Key::H),
            [
                input.key_pressed(egui::Key::Num1) && !input.modifiers.command,
                input.key_pressed(egui::Key::Num2) && !input.modifiers.command,
                input.key_pressed(egui::Key::Num3) && !input.modifiers.command,
                input.key_pressed(egui::Key::Num4) && !input.modifiers.command,
            ],
        )
    });

    if pause {
        grid.pause();
    }
    if step {
        grid.step();
    }
    if reset {
        grid.init();
    }
    if clear {
        grid.clear();
    }
    if smooth {
        grid.grid_coloration.smooth = !grid.grid_coloration.smooth;
    }
    if panel {
        state.panel_open = !state.panel_open;
    }
    if let Some(index) = gradient.iter().position(|pressed| *pressed) {
        let gradients = crate::grid_coloration::ColorGradient::all();
        if let Some(gradient) = gradients.get(index) {
            grid.grid_coloration.gradient = gradient.clone();
        }
    }
}
