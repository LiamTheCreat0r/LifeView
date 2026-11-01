use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use bevy_embedded_assets::{EmbeddedAssetPlugin, PluginMode};

use crate::grid::update_generation;
use crate::instancing::CellMaterialPlugin;
use crate::instancing::update_instance_data;
use crate::shapes::insert_shapes;
use crate::shapes::load_shapes;
use crate::ui::{UiMetrics, UiPlugin};
use bevy_egui::{EguiContexts, EguiPlugin};

mod grid;
mod grid_coloration;
mod instancing;
mod rule;
mod shapes;
mod ui;

const DRAW_STRENGTH: f32 = 0.05;

fn main() {
    App::new()
        .add_plugins(EmbeddedAssetPlugin {
            mode: PluginMode::ReplaceDefault,
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "LifeView".to_string(),
                fit_canvas_to_parent: true,
                canvas: Some("#bevy".to_string()),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(EguiPlugin {
            ..EguiPlugin::default()
        })
        .add_plugins(CellMaterialPlugin)
        .add_plugins(UiPlugin)
        .add_systems(Startup, insert_shapes)
        .add_systems(Startup, load_shapes.after(insert_shapes))
        .add_systems(FixedUpdate, update_generation)
        .add_systems(
            FixedUpdate,
            update_instance_data.after(crate::grid::update_generation),
        )
        .add_systems(FixedUpdate, mouse_click.after(update_generation))
        .run();
}

fn mouse_click(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    grid: Option<ResMut<grid::Grid>>,
    metrics: Res<UiMetrics>,
    mut contexts: EguiContexts,
) {
    let Some(grid) = grid else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    if contexts
        .ctx_mut()
        .map(|ctx| ctx.wants_pointer_input())
        .unwrap_or(false)
    {
        return;
    }
    if mouse.pressed(MouseButton::Left) {
        if let Some(position) = window.cursor_position() {
            draw(position, grid, 1.0, window, &metrics);
        }
    } else if mouse.pressed(MouseButton::Right) {
        if let Some(position) = window.cursor_position() {
            draw(position, grid, -1.0, window, &metrics);
        }
    }
}

fn draw(
    position: Vec2,
    mut grid: ResMut<grid::Grid>,
    pressure: f32,
    window: &Window,
    metrics: &UiMetrics,
) {
    if position.x < metrics.panel_width
        || position.y < metrics.topbar_height
        || position.y > window.height() - metrics.status_height
    {
        return;
    }
    let gx = ((position.x - metrics.panel_width) / grid.cell_size).floor() as i32;
    let grid_height = grid.height as f32 * grid.cell_size;
    let gy = ((metrics.topbar_height + grid_height - position.y) / grid.cell_size).floor() as i32;
    let true_pos = grid.wrap_pos(IVec2::new(gx, gy));
    let idx: usize = grid.vector_to_idx(true_pos) as usize;
    let num_channels = grid.rule.num_channels;
    for c in 0..num_channels {
        let cell_idx = idx * num_channels + c;
        grid.cell_data[cell_idx] += DRAW_STRENGTH * pressure * grid.rule.delta;
        grid.cell_data[cell_idx] = grid.cell_data[cell_idx].clamp(0.0, 1.0);
    }
}
