use std::collections::BTreeMap;

use bevy::{
    ecs::system::{Commands, ResMut},
    math::IVec2,
};
use include_dir::{Dir, include_dir};
use serde::Deserialize;

use crate::rule::Rule;

static SHAPES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/shapes");

#[derive(Clone, Debug)]
pub struct Shape {
    pub category: String,
    pub name: String,
    pub genus: String,
    pub genus_display: String,
    pub optimal_rule: Rule,
    pub cells_state: Vec<f32>,
    pub cells_pos: Vec<IVec2>,
    pub cells_channel: Vec<usize>,
}

#[derive(Deserialize)]
struct ShapeJson {
    name: String,
    genus: Option<String>,
    rule: Rule,
    channels: Vec<Vec<Vec<f32>>>,
}

const GENUS_DISPLAY_NAMES: &[(&str, &str)] = &[
    ("Orbium", "Orbium"),
    ("Gyrorbium", "Gyrorbium"),
    ("Vagorbium", "Vagorbium"),
    ("Synorbium", "Synorbium"),
    ("Trisynorbium", "Trisynorbium"),
    ("Parorbium", "Parorbium"),
    ("Triparorbium", "Triparorbium"),
    ("Scutium", "Scutium"),
    ("Discutium", "Discutium"),
    ("Triscutium", "Triscutium"),
    ("Hydrogeminium", "Hydrogeminium"),
    ("Tessellatium", "Tessellatium"),
    ("Flos", "Flos"),
    ("Asteria", "Asteria"),
    ("Spirillum", "Spirillum"),
    ("Angula", "Angula"),
    ("Caterpillar", "Caterpillar"),
    ("Platyhelminthes", "Platyhelminthes"),
    ("Lacuna", "Lacuna"),
    ("Medusa", "Medusa"),
    ("Anemone", "Anemone"),
    ("StillLife", "Still Life"),
    ("Oscillator", "Oscillator"),
    ("Spaceship", "Spaceship"),
    ("Methuselah", "Methuselah"),
    ("Gun", "Gun"),
];

fn get_genus_display(genus: &str) -> &str {
    for (g, display) in GENUS_DISPLAY_NAMES {
        if *g == genus {
            return display;
        }
    }
    genus
}

fn extract_genus(name: &str) -> String {
    name.split_whitespace().next().unwrap_or(name).to_string()
}

impl Shape {
    pub fn new(
        name: String,
        optimal_rule: Rule,
        cells_state: Vec<f32>,
        cells_pos: Vec<IVec2>,
        cells_channel: Vec<usize>,
    ) -> Self {
        let genus = extract_genus(&name);
        let genus_display = get_genus_display(&genus).to_string();
        Self {
            category: String::new(),
            name,
            genus,
            genus_display,
            optimal_rule,
            cells_state,
            cells_pos,
            cells_channel,
        }
    }

    #[allow(dead_code)]
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
        Self::new(
            name.into(),
            optimal_rule,
            cells_state,
            cells_pos,
            cells_channel,
        )
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
        Self::new(
            name.into(),
            optimal_rule,
            cells_state,
            cells_pos,
            cells_channel,
        )
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

        if let Some(ref genus) = parsed.genus {
            shape.genus = genus.clone();
            shape.genus_display = get_genus_display(genus).to_string();
        }

        for (ch_idx, channel_grid) in parsed.channels.iter().enumerate() {
            let refs: Vec<&[f32]> = channel_grid.iter().map(|row| row.as_slice()).collect();
            let channel_shape = Self::from_channel_grid("", rule.clone(), ch_idx, &refs);
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

    #[allow(dead_code)]
    pub fn by_genus(&self) -> BTreeMap<String, Vec<&Shape>> {
        let mut map: BTreeMap<String, Vec<&Shape>> = BTreeMap::new();
        for shape in &self.0 {
            map.entry(shape.genus.clone()).or_default().push(shape);
        }
        map
    }

    pub fn by_category_and_genus(&self) -> BTreeMap<String, BTreeMap<String, Vec<&Shape>>> {
        let mut map: BTreeMap<String, BTreeMap<String, Vec<&Shape>>> = BTreeMap::new();
        for shape in &self.0 {
            map.entry(shape.category.clone())
                .or_default()
                .entry(shape.genus.clone())
                .or_default()
                .push(shape);
        }
        map
    }

    pub fn genus_display(&self, genus: &str) -> String {
        self.0
            .iter()
            .find(|s| s.genus == genus)
            .map(|s| s.genus_display.clone())
            .unwrap_or_else(|| genus.to_string())
    }

    /// Load every shape embedded at compile time.
    ///
    /// Using `include_dir` instead of `std::fs` keeps the shape browser working on
    /// WASM, where the filesystem is unavailable.
    pub fn load_embedded() -> Self {
        let mut shapes = Shapes::default();

        for dir in SHAPES_DIR.dirs() {
            let category = dir
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Uncategorized");

            let mut files: Vec<_> = dir.files().collect();
            files.sort_by_key(|file| file.path());

            for file in files {
                if file.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
                    continue;
                }

                let Some(content) = file.contents_utf8() else {
                    eprintln!("Shape file is not valid UTF-8: {:?}", file.path());
                    continue;
                };

                match Shape::from_json(content) {
                    Ok(mut shape) => {
                        shape.category = category.to_string();
                        shapes.add(shape);
                    }
                    Err(error) => {
                        eprintln!("Failed to parse {:?}: {}", file.path(), error);
                    }
                }
            }
        }

        eprintln!("Loaded {} shapes from embedded assets", shapes.0.len());
        shapes
    }
}

pub fn insert_shapes(mut commands: Commands) {
    commands.insert_resource(Shapes::default());
}

pub fn load_shapes(mut shapes: ResMut<Shapes>) {
    shapes.0 = Shapes::load_embedded().0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::{GrowthKind, KernelKind};

    #[test]
    fn embedded_shapes_load() {
        let shapes = Shapes::load_embedded();

        assert!(shapes.0.len() >= 60, "expected the bundled shape library");
        assert!(shapes.0.iter().any(|shape| shape.category == "Conway"));
        assert!(shapes.0.iter().any(|shape| shape.category == "Lenia"));
    }

    #[test]
    fn discutium_pachus_uses_the_official_step_rule() {
        let shapes = Shapes::load_embedded();
        let shape = shapes
            .0
            .iter()
            .find(|shape| shape.name == "Discutium pachus")
            .expect("Discutium pachus should be bundled");

        let kernel = &shape.optimal_rule.kernels[0];
        assert_eq!(kernel.effective_kernel_kind(), KernelKind::Step);
        assert_eq!(kernel.effective_growth_kind(), GrowthKind::Step);
    }

    #[test]
    fn official_quad4_shapes_derive_from_the_legacy_flag() {
        let shapes = Shapes::load_embedded();
        let shape = shapes
            .0
            .iter()
            .find(|shape| shape.name == "Orbium unicaudatus")
            .expect("Orbium unicaudatus should be bundled");

        let kernel = &shape.optimal_rule.kernels[0];
        assert_eq!(kernel.effective_kernel_kind(), KernelKind::Quad4);
        assert_eq!(kernel.effective_growth_kind(), GrowthKind::Polynomial);
    }
}
