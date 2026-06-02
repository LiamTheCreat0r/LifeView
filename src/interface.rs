use bevy::app::{App, Plugin};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

use crate::grid::GenerationType;
use crate::grid::Grid;
use crate::grid_coloration::ColorGradient;
use crate::rule::{KernelDef, Transition, StateType};
use crate::shapes::Shapes;

pub const PANEL_WIDTH: f32 = 280.;
pub const TOPBAR_HEIGHT: f32 = 32.;

pub struct UiPlugin;
impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        assert!(app.is_plugin_added::<EguiPlugin>());
        app.add_systems(EguiPrimaryContextPass, ui);
    }
}

pub fn ui(
    mut contexts: EguiContexts,
    grid: Option<ResMut<Grid>>,
    shapes: Option<Res<Shapes>>,
    diagnostics: Res<DiagnosticsStore>,
) -> Result {
    let Some(mut grid) = grid else {
        return Ok(());
    };
    let Some(shapes) = shapes else {
        return Ok(());
    };

    // ── Top bar ───────────────────────────────────────────────────
    egui::TopBottomPanel::top("topbar")
        .exact_height(TOPBAR_HEIGHT)
        .show(contexts.ctx_mut()?, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(egui::RichText::new("LifeView").strong());

                ui.separator();

                if ui.button("⏸ Pause").clicked() {
                    grid.pause();
                }
                if ui.button("↺ Reset").clicked() {
                    grid.init();
                }
                if ui.button("✕ Clear").clicked() {
                    grid.clear();
                }

                ui.separator();

                if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
                    && let Some(value) = fps.smoothed()
                {
                    ui.label(format!("FPS: {value:.0}"));
                }
            });
        });

    // ── Side panel ────────────────────────────────────────────────
    egui::SidePanel::left("main_panel")
        .resizable(false)
        .exact_width(PANEL_WIDTH)
        .show(contexts.ctx_mut()?, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // ── Grid ──────────────────────────────────────────
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Grid").heading());
                ui.add_space(4.0);

                ui.add(egui::Slider::new(&mut grid.cell_size, 4.0..=32.0).text("Cell size"));

                egui::ComboBox::from_label("Topology")
                    .selected_text("Toroidal (loop)")
                    .show_ui(ui, |ui| {
                        ui.selectable_label(true, "Toroidal (loop)");
                        ui.selectable_label(false, "Finite");
                    });

                egui::ComboBox::from_label("Init")
                    .selected_text(format!("{:?}", grid.generation_type))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut grid.generation_type,
                            GenerationType::EMPTY,
                            "Empty",
                        );
                        ui.selectable_value(
                            &mut grid.generation_type,
                            GenerationType::RANDOM,
                            "Random",
                        );
                        ui.selectable_value(
                            &mut grid.generation_type,
                            GenerationType::BLOB,
                            "Blob",
                        );
                    });

                ui.separator();

                let is_conway = matches!(grid.rule.transition, Transition::BirthSurvive { .. });

                if !is_conway {
                    // ── Channels ──────────────────────────────────────
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Channels").heading());
                    ui.add_space(4.0);

                    let mut num_channels = grid.rule.num_channels as i32;
                    ui.add(egui::Slider::new(&mut num_channels, 1..=3).text("Count"));
                    let num_channels = num_channels as usize;
                    if num_channels != grid.rule.num_channels {
                        change_channel_count(&mut grid, num_channels);
                    }
                }

                ui.separator();

                // ── Global Rules ──────────────────────────────────
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Global Rules").heading());
                ui.add_space(4.0);

                egui::ComboBox::from_label("Transition")
                    .selected_text(if is_conway { "Conway" } else { "Lenia Growth" })
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut (), (), "Lenia Growth").clicked() && is_conway {
                            grid.rule.transition = Transition::LeniaGrowth;
                            grid.rule.kernels = vec![KernelDef::default_single(0.15, 0.015, 13)];
                            grid.rebuild_all_kernels();
                            grid.clear();
                        }
                        if ui.selectable_value(&mut (), (), "Conway").clicked() && !is_conway {
                            grid.rule.transition = Transition::conway();
                            grid.rule.num_channels = 1;
                            grid.rule.state_type = StateType::DISCRETE;
                            grid.rule.num_discrete_states = 2;
                            grid.rule.kernels.clear();
                            grid.kernel_caches.clear();
                            grid.prev_kernel_sig.clear();
                            let total_cells = grid.width * grid.height;
                            grid.cell_data.resize(total_cells, 0.0);
                            grid.next_cell_data.resize(total_cells, 0.0);
                            grid.clear();
                        }
                    });

                if is_conway {
                    if let Transition::BirthSurvive { ref mut birth, ref mut survive } = grid.rule.transition {
                        ui.label("Birth:");
                        birth_checkboxes(ui, birth);

                        ui.label("Survive:");
                        birth_checkboxes(ui, survive);
                    }
                } else {
                    egui::ComboBox::from_label("State")
                        .selected_text(match grid.rule.state_type {
                            StateType::CONTINUOUS => "Continuous",
                            StateType::DISCRETE => "Discrete",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut grid.rule.state_type, StateType::CONTINUOUS, "Continuous");
                            ui.selectable_value(&mut grid.rule.state_type, StateType::DISCRETE, "Discrete");
                        });

                    if grid.rule.state_type == StateType::DISCRETE && grid.rule.num_discrete_states < 2 {
                        grid.rule.num_discrete_states = 2;
                    }

                    ui.add(egui::Slider::new(&mut grid.rule.delta, 0.0..=0.5).text("Δt"));

                    egui::ComboBox::from_label("Mode")
                        .selected_text(format!("{:?}", grid.rule.mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut grid.rule.mode, crate::rule::RuleMode::Sum, "Sum");
                            ui.selectable_value(&mut grid.rule.mode, crate::rule::RuleMode::Multiply, "Multiply");
                        });
                }

                if grid.rule.state_type == StateType::DISCRETE {
                    let mut n = grid.rule.num_discrete_states as i32;
                    ui.add(egui::Slider::new(&mut n, 2..=256).text("States"));
                    grid.rule.num_discrete_states = n as usize;
                }

                ui.separator();

                // ── Display ────────────────────────────────────────
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Display").heading());
                ui.add_space(4.0);

                ui.checkbox(&mut grid.grid_coloration.smooth, "Smooth cells");

                ui.label("Color map");
                let gradients = ColorGradient::all();
                let current_name = grid.grid_coloration.gradient.name;
                for gradient in &gradients {
                    let selected = gradient.name == current_name;
                    if ui.selectable_label(selected, gradient.name).clicked() {
                        grid.grid_coloration.gradient = gradient.clone();
                    }
                }

                ui.separator();

                // ── Shapes ────────────────────────────────────────
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Shapes").heading());
                ui.add_space(4.0);

                let mut search: String = ui.data_mut(|d| {
                    d.get_persisted(egui::Id::new("shape_search")).unwrap_or_default()
                });
                if ui.text_edit_singleline(&mut search).changed() {
                    ui.data_mut(|d| d.insert_persisted(egui::Id::new("shape_search"), search.clone()));
                }

                ui.separator();

                let category_map = shapes.by_category_and_genus();
                for (category, genus_map) in &category_map {
                    egui::CollapsingHeader::new(category)
                        .open((!search.is_empty()).then_some(true))
                        .show(ui, |ui| {
                            for (genus, genus_shapes) in genus_map {
                                let display = shapes.genus_display(genus);

                                let filtered: Vec<_> = if search.is_empty() {
                                    genus_shapes.iter().copied().collect()
                                } else {
                                    genus_shapes.iter()
                                        .filter(|s| s.name.to_lowercase().contains(&search.to_lowercase()))
                                        .copied()
                                        .collect()
                                };

                                if filtered.is_empty() {
                                    continue;
                                }

                                let header_label = if search.is_empty() {
                                    format!("{} ({})", display, filtered.len())
                                } else {
                                    format!("{} ({}/{})", display, filtered.len(), genus_shapes.len())
                                };

                                egui::CollapsingHeader::new(&header_label)
                                    .open((!search.is_empty()).then_some(true))
                                    .show(ui, |ui| {
                                        egui::Grid::new(format!("shapes_{}_{}", category, genus))
                                            .num_columns(2)
                                            .spacing([6.0, 6.0])
                                            .show(ui, |ui| {
                                                for (i, shape) in filtered.iter().enumerate() {
                                                    if ui.button(&shape.name).clicked() {
                                                        grid.clear();
                                                        grid.spawn_shape(shape.name.clone(), shapes.0.clone());
                                                    }
                                                    if i % 2 == 1 {
                                                        ui.end_row();
                                                    }
                                                }
                                            });
                                    });
                            }
                        });
                }

                ui.separator();

                if !is_conway {
                    // ── Kernels ───────────────────────────────────────
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Kernels").heading());
                        ui.add_space(8.0);
                        if ui.button("+ Add").clicked() {
                            add_default_kernel(&mut grid);
                        }
                    });
                    ui.add_space(4.0);

                    let num_kernels = grid.rule.kernels.len();
                    let mut to_remove: Option<usize> = None;
                    let mut kernels_changed = false;

                    for ki in 0..num_kernels {
                        let kernel = &mut grid.rule.kernels[ki];
                        let header_label = if num_kernels == 1 {
                            format!("Kernel")
                        } else {
                            format!("Kernel #{}", ki + 1)
                        };

                        egui::CollapsingHeader::new(&header_label)
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.label(format!("c{} → c{}", kernel.c0, kernel.c1));

                                ui.add(
                                    egui::Slider::new(&mut kernel.mu, 0.0..=1.0)
                                        .text("μ micro"),
                                );
                                ui.add(
                                    egui::Slider::new(&mut kernel.sigma, 0.001..=0.2)
                                        .text("σ sigma"),
                                );
                                ui.add(
                                    egui::Slider::new(&mut kernel.base_radius, 1..=20)
                                        .text("Radius"),
                                );
                                ui.add(
                                    egui::Slider::new(&mut kernel.relative_radius, 0.1..=1.5)
                                        .text("ρ rel. radius"),
                                );
                                ui.add(
                                    egui::Slider::new(&mut kernel.height, 0.0..=1.0)
                                        .text("η height"),
                                );

                                ui.checkbox(&mut kernel.use_target, "Asymptotic (target mode)");
                                ui.checkbox(&mut kernel.polynomial, "Polynomial growth");

                                if ui.small_button("Remove").clicked() {
                                    to_remove = Some(ki);
                                }

                                kernels_changed = true;
                            });
                    }

                    if let Some(ki) = to_remove {
                        if grid.rule.kernels.len() > 1 {
                            grid.rule.kernels.remove(ki);
                            kernels_changed = true;
                        }
                    }

                    if kernels_changed {
                        grid.rebuild_all_kernels();
                    }
                }
            });
        });

    Ok(())
}

fn change_channel_count(grid: &mut Grid, new_count: usize) {
    if new_count == grid.rule.num_channels {
        return;
    }

    let old_channels = grid.rule.num_channels;
    let total_cells = grid.width * grid.height;

    let mut new_data = vec![0.0; total_cells * new_count];

    for i in 0..total_cells {
        let old_base = i * old_channels;
        let new_base = i * new_count;
        let copy_len = new_count.min(old_channels);
        for c in 0..copy_len {
            new_data[new_base + c] = grid.cell_data[old_base + c];
        }
    }

    grid.cell_data = new_data;
    grid.next_cell_data = vec![0.0; total_cells * new_count];
    grid.rule.num_channels = new_count;

    // Clamp all kernel channel indices to valid range
    for k in &mut grid.rule.kernels {
        k.c0 = k.c0.min(new_count - 1);
        k.c1 = k.c1.min(new_count - 1);
    }

    grid.rebuild_all_kernels();
}

fn add_default_kernel(grid: &mut Grid) {
    let _num_channels = grid.rule.num_channels;
    let first_kernel = grid.rule.kernels.first();

    let new_kernel = if let Some(k) = first_kernel {
        KernelDef {
            mu: k.mu,
            sigma: k.sigma,
            base_radius: k.base_radius,
            relative_radius: k.relative_radius,
            height: k.height,
            peaks: k.peaks.clone(),
            c0: 0,
            c1: 0,
            use_target: false,
            polynomial: k.polynomial,
            alpha: k.alpha,
            kernel_kind: k.kernel_kind.clone(),
        }
    } else {
        KernelDef::default_single(0.15, 0.015, 13)
    };

    grid.rule.kernels.push(new_kernel);
    grid.rebuild_all_kernels();
}

fn birth_checkboxes(ui: &mut egui::Ui, counts: &mut Vec<u8>) {
    for row in 0..3 {
        ui.horizontal(|ui| {
            for col in 0..3 {
                let n = row * 3 + col;
                let checked = counts.contains(&n);
                if ui.selectable_label(checked, format!("{n}")).clicked() {
                    if checked {
                        counts.retain(|&x| x != n);
                    } else {
                        counts.push(n);
                        counts.sort();
                    }
                }
            }
        });
    }
}
