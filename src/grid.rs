use bevy::prelude::*;

use crate::grid_coloration::GridColoration;
use crate::rule::{KernelDef, KernelKind, Rule, RuleMode, StateType, Transition};
use crate::shapes::Shape;

use rand::Rng;
use rayon::prelude::*;

#[derive(Resource, Debug)]
pub struct Grid {
    pub cell_data: Vec<f32>,
    pub next_cell_data: Vec<f32>,
    pub width: usize,
    pub height: usize,
    pub cell_size: f32,
    pub prev_cell_size: f32,
    pub rule: Rule,
    pub grid_coloration: GridColoration,
    pub paused: bool,
    pub generation_type: GenerationType,
    pub generations_per_second: f32,
    pub gen_accumulator: f32,
    pub generation_count: u64,
    pub step_once: bool,
    pub kernel_caches: Vec<KernelCache>,
    pub prev_kernel_sig: Vec<KernelSignature>,
}

#[derive(Debug, Clone)]
pub struct KernelCache {
    pub weights: Vec<(IVec2, f32)>,
    pub sum: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KernelSignature {
    pub base_radius: i32,
    pub relative_radius: f32,
    pub peaks: Vec<f32>,
    pub kernel_kind: KernelKind,
}

impl Grid {
    pub fn new(width: usize, height: usize, cell_size: f32) -> Self {
        let rule = Rule::default();
        let num_channels = rule.num_channels;
        let total_cells = width * height;

        let mut grid = Self {
            cell_data: vec![0.0; total_cells * num_channels],
            next_cell_data: vec![0.0; total_cells * num_channels],
            width,
            height,
            cell_size,
            prev_cell_size: cell_size,
            rule,
            grid_coloration: GridColoration::default(),
            paused: true,
            generation_type: GenerationType::RANDOM,
            generations_per_second: 64.0,
            gen_accumulator: 0.0,
            generation_count: 0,
            step_once: false,
            kernel_caches: Vec::new(),
            prev_kernel_sig: Vec::new(),
        };
        grid.rebuild_all_kernels();
        grid.init();
        grid
    }

    pub fn needs_rebuild(&self) -> bool {
        (self.cell_size - self.prev_cell_size).abs() > f32::EPSILON
    }

    pub fn kernels_need_rebuild(&self) -> bool {
        if self.rule.kernels.len() != self.prev_kernel_sig.len() {
            return true;
        }
        for (k, sig) in self.rule.kernels.iter().zip(&self.prev_kernel_sig) {
            let current_sig = KernelSignature {
                base_radius: k.base_radius,
                relative_radius: k.relative_radius,
                peaks: k.peaks.clone(),
                kernel_kind: k.effective_kernel_kind(),
            };
            if current_sig != *sig {
                return true;
            }
        }
        false
    }

    pub fn rebuild_all_kernels(&mut self) {
        self.kernel_caches.clear();
        self.prev_kernel_sig.clear();
        for kernel_def in &self.rule.kernels {
            let cache = Self::build_kernel(kernel_def);
            self.prev_kernel_sig.push(KernelSignature {
                base_radius: kernel_def.base_radius,
                relative_radius: kernel_def.relative_radius,
                peaks: kernel_def.peaks.clone(),
                kernel_kind: kernel_def.effective_kernel_kind(),
            });
            self.kernel_caches.push(cache);
        }
    }

    fn build_kernel(kernel_def: &KernelDef) -> KernelCache {
        let effective_r =
            ((kernel_def.base_radius as f32) * kernel_def.relative_radius).ceil() as i32;
        let r = effective_r.max(1);
        let mut weights = Vec::new();
        let mut sum = 0.0;

        for x in -r..=r {
            for y in -r..=r {
                let d = IVec2::new(x, y).as_vec2().length();
                if d > r as f32 || d == 0.0 {
                    continue;
                }

                let t = d / r as f32;
                let w =
                    Self::kernel_weight(t, &kernel_def.peaks, &kernel_def.effective_kernel_kind());
                weights.push((IVec2::new(x, y), w));
                sum += w;
            }
        }

        KernelCache { weights, sum }
    }

    fn kernel_weight(t: f32, peaks: &[f32], kernel_kind: &KernelKind) -> f32 {
        let (idx, frac) = if peaks.len() == 1 {
            (0, t)
        } else {
            let idx = ((t * peaks.len() as f32).floor() as usize).min(peaks.len() - 1);
            (idx, t * peaks.len() as f32 - idx as f32)
        };
        let peak = peaks[idx];

        match kernel_kind {
            KernelKind::Gaussian => {
                let bell_frac = (frac - 0.5) / 0.15;
                peak * (-bell_frac.powi(2) / 2.0).exp()
            }
            KernelKind::Quad4 => {
                let k = 4.0 * frac * (1.0 - frac);
                peak * k.powi(4)
            }
            KernelKind::Bump4 => {
                if frac <= 0.0 || frac >= 1.0 {
                    0.0
                } else {
                    peak * (4.0 - 1.0 / (frac * (1.0 - frac))).exp()
                }
            }
            KernelKind::Step => {
                if (0.25..=0.75).contains(&frac) {
                    peak
                } else {
                    0.0
                }
            }
        }
    }

    pub fn init(&mut self) {
        self.paused = true;
        self.gen_accumulator = 0.0;
        self.generation_count = 0;
        self.step_once = false;
        let num_channels = self.rule.num_channels;
        let total_cells = self.width * self.height;
        let cx = self.width as f32 / 2.0;
        let cy = self.height as f32 / 2.0;
        let r = self.width.min(self.height) as f32 / 6.0;

        if self.generation_type == GenerationType::EMPTY {
            for i in 0..total_cells * num_channels {
                self.cell_data[i] = 0.0;
            }
            return;
        }

        for i in 0..total_cells {
            let pos = self.idx_to_vector(i as i32);

            if self.generation_type == GenerationType::RANDOM {
                for c in 0..num_channels {
                    self.cell_data[i * num_channels + c] = rand::rng().random();
                }
                continue;
            }

            // BLOB
            let dx = pos.x as f32 - cx;
            let dy = pos.y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let state = (-0.5 * (dist / (r * 0.5)).powi(2)).exp();

            for c in 0..num_channels {
                self.cell_data[i * num_channels + c] = if c == 0 { state } else { 0.0 };
            }
        }
    }

    pub fn clear(&mut self) {
        self.paused = true;
        self.gen_accumulator = 0.0;
        self.generation_count = 0;
        self.step_once = false;
        let num_channels = self.rule.num_channels;
        let total_cells = self.width * self.height;
        for i in 0..total_cells {
            for c in 0..num_channels {
                self.cell_data[i * num_channels + c] = 0.0;
            }
        }
    }

    #[allow(dead_code)] // Kept as part of the simulation data API.
    pub fn life_around(&self, pos: IVec2, kernel_idx: usize, _channel: usize) -> f32 {
        let cache = &self.kernel_caches[kernel_idx];
        let kernel_def = &self.rule.kernels[kernel_idx];
        let source_channel = kernel_def.c0;
        let num_channels = self.rule.num_channels;

        let mut result: f32 = 0.0;
        for &(offset, w) in &cache.weights {
            let neighbour = self.wrap_pos(pos + offset);
            let grid_idx = self.vector_to_idx(neighbour) as usize;
            let cell_idx = grid_idx * num_channels + source_channel;
            let value = self.cell_data[cell_idx];
            result += value * w;
        }

        if cache.sum == 0.0 {
            0.0
        } else {
            result / cache.sum
        }
    }

    fn convolve_cell(
        pos: IVec2,
        kernel_idx: usize,
        width: usize,
        height: usize,
        num_channels: usize,
        kernel_caches: &[KernelCache],
        kernels: &[KernelDef],
        cell_data: &[f32],
    ) -> f32 {
        let cache = &kernel_caches[kernel_idx];
        let kernel_def = &kernels[kernel_idx];
        let source_channel = kernel_def.c0;

        let gw = width as i32;
        let gh = height as i32;
        let px = pos.x;
        let py = pos.y;

        let mut result = 0.0;
        for &(offset, weight) in &cache.weights {
            let nx = px + offset.x;
            let ny = py + offset.y;
            let nx = nx - (nx >= gw) as i32 * gw + (nx < 0) as i32 * gw;
            let ny = ny - (ny >= gh) as i32 * gh + (ny < 0) as i32 * gh;
            result += cell_data[(nx + ny * gw) as usize * num_channels + source_channel] * weight;
        }

        if cache.sum == 0.0 {
            0.0
        } else {
            result / cache.sum
        }
    }

    pub fn idx_to_vector(&self, idx: i32) -> IVec2 {
        IVec2::new(idx % self.width as i32, idx / self.width as i32)
    }

    pub fn vector_to_idx(&self, pos: IVec2) -> i32 {
        pos.x % self.width as i32 + pos.y * self.width as i32
    }

    pub fn wrap_pos(&self, pos: IVec2) -> IVec2 {
        let w = self.width as i32;
        let h = self.height as i32;
        IVec2::new(((pos.x % w) + w) % w, ((pos.y % h) + h) % h)
    }

    pub fn generation(&mut self) {
        let total_cells = self.width * self.height;
        let expected_size = total_cells * self.rule.num_channels;
        if self.cell_data.len() != expected_size {
            self.cell_data.resize(expected_size, 0.0);
        }
        if self.next_cell_data.len() != expected_size {
            self.next_cell_data.resize(expected_size, 0.0);
        }

        let transition = self.rule.transition.clone();
        let state_type = self.rule.state_type.clone();

        match transition {
            Transition::LeniaGrowth => {
                self.lenia_generation_inner();
            }
            Transition::BirthSurvive { birth, survive, .. } => {
                self.birth_survive_inner(&birth, &survive);
            }
        }

        if state_type == StateType::DISCRETE {
            self.quantize_state();
        }

        self.generation_count = self.generation_count.wrapping_add(1);
    }

    fn lenia_generation_inner(&mut self) {
        let num_channels = self.rule.num_channels;

        let cell_data = &self.cell_data;
        let rule = &self.rule;
        let kernel_caches = &self.kernel_caches;
        let width = self.width;
        let height = self.height;

        let sum_mode = rule.mode == RuleMode::Sum;

        self.next_cell_data
            .par_chunks_mut(num_channels)
            .enumerate()
            .for_each(|(idx, chunk)| {
                let pos = IVec2::new(idx as i32 % width as i32, idx as i32 / width as i32);
                for c in 0..num_channels {
                    let mut total = 0.0;
                    let mut count = 0;

                    for (ki, kernel_def) in rule.kernels.iter().enumerate() {
                        if kernel_def.c1 != c {
                            continue;
                        }

                        let u = Self::convolve_cell(
                            pos,
                            ki,
                            width,
                            height,
                            num_channels,
                            kernel_caches,
                            &rule.kernels,
                            cell_data,
                        );
                        let current_val = cell_data[idx * num_channels + c];
                        let g = if kernel_def.use_target {
                            rule.target(u, ki) - current_val
                        } else {
                            rule.growth(u, ki)
                        };

                        let value = kernel_def.height * g;
                        if sum_mode {
                            total += value;
                        } else if count == 0 {
                            total = value;
                        } else {
                            total *= value;
                        }
                        count += 1;
                    }

                    let growth = if count > 0 {
                        if sum_mode {
                            total / count as f32
                        } else {
                            total
                        }
                    } else {
                        0.0
                    };
                    let current_val = cell_data[idx * num_channels + c];
                    let new_value = current_val + growth * rule.delta;
                    chunk[c] = new_value.clamp(0.0, 1.0);
                }
            });

        std::mem::swap(&mut self.cell_data, &mut self.next_cell_data);
    }

    fn count_moore_neighbors(
        cell_data: &[f32],
        pos: IVec2,
        width: usize,
        height: usize,
        channel: usize,
        num_channels: usize,
    ) -> u8 {
        let gw = width as i32;
        let gh = height as i32;
        let mut count = 0u8;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = pos.x + dx;
                let ny = pos.y + dy;
                let nx = nx - (nx >= gw) as i32 * gw + (nx < 0) as i32 * gw;
                let ny = ny - (ny >= gh) as i32 * gh + (ny < 0) as i32 * gh;
                let idx = (nx + ny * gw) as usize * num_channels + channel;
                if cell_data[idx] > 0.0 {
                    count += 1;
                }
            }
        }
        count
    }

    fn birth_survive_inner(&mut self, birth: &[u8], survive: &[u8]) {
        let num_channels = self.rule.num_channels;
        let width = self.width;
        let height = self.height;
        let n_states = self.rule.num_discrete_states;
        let max_state_f = (n_states - 1) as f32;

        let cell_data = &self.cell_data;

        self.next_cell_data
            .par_chunks_mut(num_channels)
            .enumerate()
            .for_each(|(idx, chunk)| {
                let pos = IVec2::new(idx as i32 % width as i32, idx as i32 / width as i32);
                for c in 0..num_channels {
                    let current = cell_data[idx * num_channels + c];
                    let count =
                        Self::count_moore_neighbors(cell_data, pos, width, height, c, num_channels);
                    let new_val = if current <= 0.0 {
                        if birth.contains(&count) { 1.0 } else { 0.0 }
                    } else {
                        if survive.contains(&count) {
                            current
                        } else {
                            let cur_level = (current * max_state_f).round();
                            let new_level = (cur_level - 1.0).max(0.0);
                            new_level / max_state_f
                        }
                    };
                    chunk[c] = new_val;
                }
            });

        std::mem::swap(&mut self.cell_data, &mut self.next_cell_data);
    }

    fn quantize_state(&mut self) {
        let n = self.rule.num_discrete_states;
        if n <= 1 {
            return;
        }
        let n_minus_1 = (n - 1) as f32;
        for val in self.cell_data.iter_mut() {
            let q = (*val * n_minus_1).round() / n_minus_1;
            *val = q.clamp(0.0, 1.0);
        }
    }

    /// Request exactly one simulation step, even while paused.
    pub fn step(&mut self) {
        self.step_once = true;
    }

    /// Resize the grid while keeping the overlapping region of the simulation.
    ///
    /// Cells outside the old grid are filled by tiling the previous contents
    /// instead of leaving a black band where new rows/columns were added.
    pub fn resize_preserving(&mut self, width: usize, height: usize, cell_size: f32) {
        if width == self.width
            && height == self.height
            && (cell_size - self.cell_size).abs() <= f32::EPSILON
        {
            return;
        }

        let num_channels = self.rule.num_channels;
        let old_width = self.width;
        let old_height = self.height;
        let old_data = std::mem::take(&mut self.cell_data);
        let mut new_data = vec![0.0; width * height * num_channels];

        for y in 0..height {
            for x in 0..width {
                let source = if x < old_width && y < old_height {
                    (y * old_width + x) * num_channels
                } else if old_width > 0 && old_height > 0 {
                    ((y % old_height) * old_width + (x % old_width)) * num_channels
                } else {
                    continue;
                };
                let target = (y * width + x) * num_channels;
                for channel in 0..num_channels {
                    if source + channel < old_data.len() {
                        new_data[target + channel] = old_data[source + channel];
                    }
                }
            }
        }

        self.cell_data = new_data;
        self.next_cell_data = vec![0.0; width * height * num_channels];
        self.width = width;
        self.height = height;
        self.cell_size = cell_size;
        self.prev_cell_size = cell_size;
    }

    pub fn pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn spawn_shape(&mut self, shape_name: String, shapes: Vec<Shape>) {
        let mut shape: Shape = shapes[0].clone();
        for current_shape in shapes {
            if current_shape.name == shape_name {
                shape = current_shape;
            }
        }

        self.rule = shape.optimal_rule.clone();
        self.generation_count = 0;
        self.gen_accumulator = 0.0;
        self.step_once = false;
        if self.kernels_need_rebuild() {
            self.rebuild_all_kernels();
        }

        let num_channels = self.rule.num_channels;
        let total_cells = self.width * self.height;

        let expected_size = total_cells * num_channels;
        if self.cell_data.len() != expected_size {
            self.cell_data.resize(expected_size, 0.0);
            self.next_cell_data.resize(expected_size, 0.0);
        }

        for i in 0..expected_size {
            self.cell_data[i] = 0.0;
        }

        let grid_center: IVec2 = IVec2::new(self.width as i32 / 2, self.height as i32 / 2);
        for i in 0..shape.cells_state.len() {
            let pos = self.wrap_pos(grid_center + shape.cells_pos[i]);
            let idx: usize = self.vector_to_idx(pos) as usize;
            if idx < total_cells {
                let ch = if !shape.cells_channel.is_empty() {
                    shape.cells_channel[i]
                } else if num_channels == 1 {
                    0
                } else {
                    i % num_channels
                };
                if ch < num_channels {
                    self.cell_data[idx * num_channels + ch] = shape.cells_state[i];
                }
            }
        }
    }
}

pub fn update_generation(time: Res<Time>, grid: Option<ResMut<Grid>>) {
    let Some(mut grid) = grid else {
        return;
    };

    if grid.step_once {
        grid.step_once = false;
        grid.gen_accumulator = 0.0;
        if grid.kernels_need_rebuild() {
            grid.rebuild_all_kernels();
        }
        grid.generation();
        return;
    }

    if grid.paused {
        return;
    }

    grid.gen_accumulator += time.delta_secs();
    let interval = 1.0 / grid.generations_per_second.max(0.001);

    while grid.gen_accumulator >= interval {
        grid.gen_accumulator -= interval;
        if grid.kernels_need_rebuild() {
            grid.rebuild_all_kernels();
        }
        grid.generation();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_idx_to_vector() {
        let grid = Grid::new(100, 50, 5.0);

        assert_eq!(grid.idx_to_vector(0), IVec2::new(0, 0));
        assert_eq!(grid.idx_to_vector(100), IVec2::new(0, 1));
        assert_eq!(grid.idx_to_vector(101), IVec2::new(1, 1));
        assert_eq!(grid.idx_to_vector(1010), IVec2::new(10, 10));
    }

    #[test]
    fn grid_vector_to_idx() {
        let grid = Grid::new(100, 50, 5.0);

        assert_eq!(grid.vector_to_idx(IVec2::new(0, 0)), 0);
        assert_eq!(grid.vector_to_idx(IVec2::new(0, 1)), 100);
        assert_eq!(grid.vector_to_idx(IVec2::new(1, 1)), 101);
        assert_eq!(grid.vector_to_idx(IVec2::new(10, 10)), 1010);
    }

    #[test]
    fn grid_wrap_pos() {
        let grid = Grid::new(100, 50, 5.0);

        assert_eq!(grid.wrap_pos(IVec2::new(0, 0)), IVec2::new(0, 0));
        assert_eq!(grid.wrap_pos(IVec2::new(-1, 0)), IVec2::new(99, 0));
        assert_eq!(grid.wrap_pos(IVec2::new(-1, -1)), IVec2::new(99, 49));
        assert_eq!(grid.wrap_pos(IVec2::new(0, -1)), IVec2::new(0, 49));
    }

    #[test]
    fn generation_increments_counter() {
        let mut grid = Grid::new(20, 20, 5.0);
        assert_eq!(grid.generation_count, 0);

        grid.generation();

        assert_eq!(grid.generation_count, 1);
    }

    #[test]
    fn spawning_a_shape_keeps_the_simulation_running() {
        let mut grid = Grid::new(20, 20, 5.0);
        grid.paused = false;

        let shapes = crate::shapes::Shapes::load_embedded();
        let shape_name = shapes.0[0].name.clone();
        grid.spawn_shape(shape_name, shapes.0);

        assert!(!grid.paused);
    }

    fn run_shape(name: &str, size: usize, generations: usize) -> f32 {
        let mut grid = Grid::new(size, size, 5.0);
        let shapes = crate::shapes::Shapes::load_embedded();
        grid.spawn_shape(name.to_string(), shapes.0);

        let initial: f32 = grid.cell_data.iter().sum();
        assert!(initial > 0.0, "{name} spawned an empty grid");

        for _ in 0..generations {
            grid.generation();
        }

        grid.cell_data.iter().sum::<f32>() / initial
    }

    #[test]
    fn discutium_pachus_uses_the_step_rule_and_stays_bounded() {
        let ratio = run_shape("Discutium pachus", 96, 200);
        assert!(
            (0.5..2.0).contains(&ratio),
            "Discutium pachus mass ratio {ratio} outside the stable range"
        );
    }

    #[test]
    fn orbium_bicaudatus_ignis_stays_alive() {
        let ratio = run_shape("Orbium bicaudatus ignis", 96, 150);
        assert!(
            (0.5..2.0).contains(&ratio),
            "Orbium bicaudatus ignis mass ratio {ratio} outside the stable range"
        );
    }

    #[test]
    fn gyrorbium_revolvens_stays_alive() {
        let ratio = run_shape("Gyrorbium revolvens", 96, 150);
        assert!(
            (0.5..2.0).contains(&ratio),
            "Gyrorbium revolvens mass ratio {ratio} outside the stable range"
        );
    }

    #[test]
    fn resize_preserves_overlapping_cells() {
        let mut grid = Grid::new(20, 20, 5.0);
        grid.cell_data[0] = 0.75;

        grid.resize_preserving(10, 10, 8.0);

        assert_eq!(grid.width, 10);
        assert_eq!(grid.height, 10);
        assert_eq!(grid.cell_size, 8.0);
        assert_eq!(grid.cell_data[0], 0.75);
    }
}

#[derive(Resource, Debug, PartialEq, Clone, Copy)]
pub enum GenerationType {
    EMPTY,
    RANDOM,
    BLOB,
}
