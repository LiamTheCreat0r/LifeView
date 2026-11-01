use bevy_egui::egui;
use egui::RichText;

use crate::grid::Grid;
use crate::rule::{KernelDef, KernelKind, RuleMode, StateType, Transition};

use super::{theme, widgets};

pub fn show(ui: &mut egui::Ui, grid: &mut Grid) {
    let is_conway = matches!(grid.rule.transition, Transition::BirthSurvive { .. });

    widgets::section(ui, "Transition");

    let mut transition = if is_conway { 1 } else { 0 };
    egui::ComboBox::from_id_salt("transition")
        .selected_text(if is_conway { "Conway" } else { "Lenia growth" })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut transition, 0, "Lenia growth");
            ui.selectable_value(&mut transition, 1, "Conway");
        });

    if transition == 1 && !is_conway {
        set_conway(grid);
    } else if transition == 0 && is_conway {
        set_lenia(grid);
    }

    if matches!(grid.rule.transition, Transition::BirthSurvive { .. }) {
        if let Transition::BirthSurvive {
            ref mut birth,
            ref mut survive,
        } = grid.rule.transition
        {
            ui.label(RichText::new("Birth").size(11.0).color(theme::MUTED));
            birth_checkboxes(ui, birth);
            ui.label(RichText::new("Survive").size(11.0).color(theme::MUTED));
            birth_checkboxes(ui, survive);
        }
        return;
    }

    widgets::section(ui, "State");

    egui::ComboBox::from_id_salt("state_type")
        .selected_text(match grid.rule.state_type {
            StateType::CONTINUOUS => "Continuous",
            StateType::DISCRETE => "Discrete",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(
                &mut grid.rule.state_type,
                StateType::CONTINUOUS,
                "Continuous",
            );
            ui.selectable_value(&mut grid.rule.state_type, StateType::DISCRETE, "Discrete");
        });

    if grid.rule.state_type == StateType::DISCRETE {
        let mut states = grid.rule.num_discrete_states as i32;
        if ui
            .add(egui::Slider::new(&mut states, 2..=256).text("States"))
            .changed()
        {
            grid.rule.num_discrete_states = states as usize;
        }
    }

    ui.add(egui::Slider::new(&mut grid.rule.delta, 0.0..=0.5).text("Δt"))
        .on_hover_text("Time step applied to each growth value.");

    egui::ComboBox::from_id_salt("rule_mode")
        .selected_text(match grid.rule.mode {
            RuleMode::Sum => "Sum",
            RuleMode::Multiply => "Multiply",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut grid.rule.mode, RuleMode::Sum, "Sum");
            ui.selectable_value(&mut grid.rule.mode, RuleMode::Multiply, "Multiply");
        });

    widgets::section(ui, "Channels");

    let mut channels = grid.rule.num_channels as i32;
    if ui
        .add(egui::Slider::new(&mut channels, 1..=3).text("Count"))
        .changed()
    {
        change_channel_count(grid, channels as usize);
    }
    ui.label(
        RichText::new("Changing the count copies existing channels and clamps kernel routes.")
            .size(11.0)
            .color(theme::MUTED),
    );

    widgets::section(ui, "Kernels");

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} kernel(s)", grid.rule.kernels.len()))
                .size(11.0)
                .color(theme::MUTED),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("+ Add").clicked() {
                add_default_kernel(grid);
            }
        });
    });

    let num_kernels = grid.rule.kernels.len();
    let num_channels = grid.rule.num_channels;
    let mut remove: Option<usize> = None;
    let mut weights_changed = false;

    for index in 0..num_kernels {
        let mut card_remove = false;
        let mut kind_changed = false;

        {
            let kernel = &mut grid.rule.kernels[index];
            egui::CollapsingHeader::new(format!(
                "Kernel {}  ·  c{} -> c{}",
                index + 1,
                kernel.c0,
                kernel.c1
            ))
            .default_open(num_kernels == 1)
            .show(ui, |ui| {
                widgets::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "effective radius {:.1}",
                                kernel.base_radius as f32 * kernel.relative_radius
                            ))
                            .size(10.0)
                            .color(theme::MUTED),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Remove").clicked() {
                                card_remove = true;
                            }
                        });
                    });

                    ui.horizontal(|ui| {
                        ui.spacing_mut().combo_width = 68.0;
                        egui::ComboBox::from_id_salt(("c0", index))
                            .selected_text(format!("c{}", kernel.c0))
                            .show_ui(ui, |ui| {
                                for channel in 0..num_channels {
                                    ui.selectable_value(
                                        &mut kernel.c0,
                                        channel,
                                        format!("c{channel}"),
                                    );
                                }
                            });
                        ui.label("->");
                        egui::ComboBox::from_id_salt(("c1", index))
                            .selected_text(format!("c{}", kernel.c1))
                            .show_ui(ui, |ui| {
                                for channel in 0..num_channels {
                                    ui.selectable_value(
                                        &mut kernel.c1,
                                        channel,
                                        format!("c{channel}"),
                                    );
                                }
                            });
                    });

                    ui.add(egui::Slider::new(&mut kernel.mu, 0.0..=1.0).text("μ peak"))
                        .on_hover_text("Neighborhood value where growth is maximal.");

                    ui.add(egui::Slider::new(&mut kernel.sigma, 0.001..=0.2).text("σ width"))
                        .on_hover_text("Width of the growth peak.");

                    let radius =
                        ui.add(egui::Slider::new(&mut kernel.base_radius, 1..=20).text("radius"));
                    if radius.changed() {
                        weights_changed = true;
                    }
                    radius.on_hover_text("Kernel radius in cells.");

                    let relative = ui.add(
                        egui::Slider::new(&mut kernel.relative_radius, 0.1..=1.5)
                            .text("ρ relative"),
                    );
                    if relative.changed() {
                        weights_changed = true;
                    }
                    relative.on_hover_text("Multiplier applied to the kernel radius.");

                    ui.add(egui::Slider::new(&mut kernel.height, 0.0..=1.0).text("η height"))
                        .on_hover_text("Weight of this kernel's contribution.");

                    let mut kind = kernel.kernel_kind.clone();
                    egui::ComboBox::from_id_salt(("kind", index))
                        .selected_text(match kind {
                            KernelKind::Bump4 => "Bump4",
                            KernelKind::Quad4 => "Quad4",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut kind, KernelKind::Bump4, "Bump4");
                            ui.selectable_value(&mut kind, KernelKind::Quad4, "Quad4");
                        });
                    if kind != kernel.kernel_kind {
                        kernel.kernel_kind = kind;
                        kind_changed = true;
                    }

                    ui.checkbox(&mut kernel.use_target, "Asymptotic (target mode)")
                        .on_hover_text(
                            "Drive cells toward the kernel target instead of adding growth.",
                        );
                    ui.checkbox(&mut kernel.polynomial, "Polynomial growth")
                        .on_hover_text(
                            "Use the compact quartic growth curve instead of a Gaussian.",
                        );

                    if kernel.polynomial {
                        ui.add(egui::Slider::new(&mut kernel.alpha, 1.0..=12.0).text("α exponent"))
                            .on_hover_text("Steepness of the polynomial falloff.");
                    }

                    if kernel.peaks.len() > 1 {
                        let peaks = kernel
                            .peaks
                            .iter()
                            .map(|peak| format!("{peak:.3}"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        ui.label(
                            RichText::new(format!("peaks [{peaks}]"))
                                .size(10.0)
                                .color(theme::MUTED),
                        );
                    }
                });
            });
        }

        if card_remove {
            remove = Some(index);
        }
        weights_changed |= kind_changed;
    }

    if let Some(index) = remove
        && grid.rule.kernels.len() > 1
    {
        grid.rule.kernels.remove(index);
        weights_changed = true;
    }

    if weights_changed {
        grid.rebuild_all_kernels();
    }

    ui.add_space(6.0);
    ui.label(
        RichText::new("Radius and kernel-kind changes rebuild the convolution cache; μ, σ, η and growth flags apply live.")
            .size(10.0)
            .color(theme::MUTED),
    );
}

fn set_lenia(grid: &mut Grid) {
    grid.rule.transition = Transition::LeniaGrowth;
    grid.rule.state_type = StateType::CONTINUOUS;
    grid.rule.num_discrete_states = 2;
    grid.rule.kernels = vec![KernelDef::default_single(0.15, 0.015, 13)];
    grid.rebuild_all_kernels();
    grid.clear();
}

fn set_conway(grid: &mut Grid) {
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

fn change_channel_count(grid: &mut Grid, new_count: usize) {
    if new_count == grid.rule.num_channels {
        return;
    }

    let old_channels = grid.rule.num_channels;
    let total_cells = grid.width * grid.height;
    let mut new_data = vec![0.0; total_cells * new_count];

    for index in 0..total_cells {
        let old_base = index * old_channels;
        let new_base = index * new_count;
        let copy_len = new_count.min(old_channels);
        for channel in 0..copy_len {
            new_data[new_base + channel] = grid.cell_data[old_base + channel];
        }
    }

    grid.cell_data = new_data;
    grid.next_cell_data = vec![0.0; total_cells * new_count];
    grid.rule.num_channels = new_count;

    for kernel in &mut grid.rule.kernels {
        kernel.c0 = kernel.c0.min(new_count - 1);
        kernel.c1 = kernel.c1.min(new_count - 1);
    }

    grid.rebuild_all_kernels();
}

fn add_default_kernel(grid: &mut Grid) {
    let new_kernel = if let Some(kernel) = grid.rule.kernels.first() {
        KernelDef {
            mu: kernel.mu,
            sigma: kernel.sigma,
            base_radius: kernel.base_radius,
            relative_radius: kernel.relative_radius,
            height: kernel.height,
            peaks: kernel.peaks.clone(),
            c0: 0,
            c1: 0,
            use_target: false,
            polynomial: kernel.polynomial,
            alpha: kernel.alpha,
            kernel_kind: kernel.kernel_kind.clone(),
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
            for column in 0..3 {
                let count = row * 3 + column;
                let checked = counts.contains(&count);
                if ui.selectable_label(checked, format!("{count}")).clicked() {
                    if checked {
                        counts.retain(|value| *value != count);
                    } else {
                        counts.push(count);
                        counts.sort();
                    }
                }
            }
        });
    }
}
