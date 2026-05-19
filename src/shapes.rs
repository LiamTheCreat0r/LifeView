use std::fs;
use std::path::Path;

use bevy::{
    ecs::system::{Commands, ResMut},
    math::IVec2,
};
use serde::Deserialize;

use crate::rule::Rule;

#[derive(Clone, Debug)]
pub struct Shape {
    pub name: String,
    pub optimal_rule: Rule,
    pub cells_state: Vec<f32>,
    pub cells_pos: Vec<IVec2>,
    pub cells_channel: Vec<usize>,
}

#[derive(Deserialize)]
struct ShapeJson {
    name: String,
    rule: Rule,
    channels: Vec<Vec<Vec<f32>>>,
}

impl Shape {
    pub fn new(
        name: String,
        optimal_rule: Rule,
        cells_state: Vec<f32>,
        cells_pos: Vec<IVec2>,
        cells_channel: Vec<usize>,
    ) -> Self {
        Self {
            name,
            optimal_rule,
            cells_state,
            cells_pos,
            cells_channel,
        }
    }

    pub fn ring(
        name: impl Into<String>,
        optimal_rule: Rule,
        r_inner: i32,
        r_outer: i32,
        state_fn: impl Fn(f32) -> f32,
    ) -> Self {
        let mut cells_state = Vec::new();
        let mut cells_pos = Vec::new();
        let mut cells_channel = Vec::new();
        for x in -r_outer..=r_outer {
            for y in -r_outer..=r_outer {
                let dist = ((x * x + y * y) as f32).sqrt();
                if dist >= r_inner as f32 && dist <= r_outer as f32 {
                    let t = (dist - r_inner as f32) / (r_outer - r_inner).max(1) as f32;
                    cells_state.push(state_fn(t).clamp(0.0, 1.0));
                    cells_pos.push(IVec2::new(x, y));
                    cells_channel.push(0);
                }
            }
        }
        Self::new(name.into(), optimal_rule, cells_state, cells_pos, cells_channel)
    }

    pub fn merge_channel(mut self, other: Self) -> Self {
        self.cells_state.extend(other.cells_state);
        self.cells_pos.extend(other.cells_pos);
        self.cells_channel.extend(other.cells_channel);
        self
    }

    fn from_channel_grid(
        name: impl Into<String>,
        optimal_rule: Rule,
        channel: usize,
        grid: &[&[f32]],
    ) -> Self {
        let rows = grid.len() as i32;
        let cols = grid[0].len() as i32;
        let origin = IVec2::new(cols / 2, rows / 2);
        let mut cells_state = Vec::new();
        let mut cells_pos = Vec::new();
        let mut cells_channel = Vec::new();
        for (y, row) in grid.iter().enumerate() {
            for (x, &val) in row.iter().enumerate() {
                if val > 0.0 {
                    cells_state.push(val);
                    cells_pos.push(IVec2::new(x as i32, rows - 1 - y as i32) - origin);
                    cells_channel.push(channel);
                }
            }
        }
        Self::new(name.into(), optimal_rule, cells_state, cells_pos, cells_channel)
    }

    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let parsed: ShapeJson = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
        let rule = parsed.rule.clone();

        let mut shape = Shape::new(
            parsed.name.clone(),
            rule.clone(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );

        for (ch_idx, channel_grid) in parsed.channels.iter().enumerate() {
            let refs: Vec<&[f32]> = channel_grid.iter().map(|row| row.as_slice()).collect();
            let channel_shape = Self::from_channel_grid(
                "",
                rule.clone(),
                ch_idx,
                &refs,
            );
            shape = shape.merge_channel(channel_shape);
        }

        Ok(shape)
    }
}

#[derive(bevy::prelude::Resource, Debug, Default)]
pub struct Shapes(pub Vec<Shape>);

impl Shapes {
    pub fn add(&mut self, shape: Shape) {
        self.0.push(shape);
    }

    pub fn load_from_dir(dir_path: impl AsRef<Path>) -> Self {
        let mut shapes = Shapes::default();
        let dir = dir_path.as_ref();

        if !dir.exists() {
            eprintln!("Shapes directory not found: {:?}", dir);
            return shapes;
        }

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Failed to read shapes directory: {}", e);
                return shapes;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Failed to read directory entry: {}", e);
                    continue;
                }
            };

            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to read {:?}: {}", path, e);
                    continue;
                }
            };

            match Shape::from_json(&content) {
                Ok(shape) => {
                    eprintln!("Loaded shape: {}", shape.name);
                    shapes.add(shape);
                }
                Err(e) => {
                    eprintln!("Failed to parse {:?}: {}", path, e);
                }
            }
        }

        shapes
    }
}

pub fn insert_shapes(mut commands: Commands) {
    commands.insert_resource(Shapes::default());
}

pub fn load_shapes(mut shapes: ResMut<Shapes>) {
    let loaded = Shapes::load_from_dir("assets/shapes/patterns");
    shapes.0 = loaded.0;
}
