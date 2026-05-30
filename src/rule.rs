use bevy::math::ops::exp;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub state_type: StateType,
    pub delta: f32,
    pub kernels: Vec<KernelDef>,
    pub num_channels: usize,
    #[serde(default)]
    pub mode: RuleMode,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KernelKind {
    Bump4,
    Quad4,
}

impl Default for KernelKind {
    fn default() -> Self {
        Self::Bump4
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RuleMode {
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "multiply")]
    Multiply,
}

impl Default for RuleMode {
    fn default() -> Self {
        Self::Sum
    }
}

impl Rule {
    pub fn single_channel(mu: f32, sigma: f32, radius: i32) -> Self {
        Self {
            state_type: StateType::CONTINUOUS,
            delta: 0.1,
            kernels: vec![KernelDef::default_single(mu, sigma, radius)],
            num_channels: 1,
            mode: RuleMode::Sum,
        }
    }

    pub fn multi_channel(kernels: Vec<KernelDef>, num_channels: usize) -> Self {
        Self {
            state_type: StateType::CONTINUOUS,
            delta: 0.1,
            kernels,
            num_channels,
            mode: RuleMode::Sum,
        }
    }

    pub fn growth(&self, u: f32, kernel_idx: usize) -> f32 {
        let k = &self.kernels[kernel_idx];
        if k.polynomial {
            let l = (u - k.mu).abs();
            let k_val = 3.0 * k.sigma;
            if l <= k_val {
                let ratio = l / k_val;
                2.0 * (1.0 - ratio * ratio).powf(k.alpha) - 1.0
            } else {
                -1.0
            }
        } else {
            2.0 * exp(-((u - k.mu).powi(2) / (2.0 * k.sigma * k.sigma))) - 1.0
        }
    }

    pub fn target(&self, u: f32, kernel_idx: usize) -> f32 {
        let k = &self.kernels[kernel_idx];
        exp(-((u - k.mu).powi(2) / (2.0 * k.sigma * k.sigma)))
    }

    pub fn effective_radius(&self, kernel_idx: usize) -> i32 {
        let k = &self.kernels[kernel_idx];
        ((k.base_radius as f32) * k.relative_radius).ceil() as i32
    }

    pub fn max_radius(&self) -> i32 {
        self.kernels
            .iter()
            .map(|k| ((k.base_radius as f32) * k.relative_radius).ceil() as i32)
            .max()
            .unwrap_or(13)
    }

    pub fn default_orbium() -> Self {
        Self::single_channel(0.15, 0.015, 13)
    }
}

impl Default for Rule {
    fn default() -> Self {
        Self::default_orbium()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KernelDef {
    pub mu: f32,
    pub sigma: f32,
    pub base_radius: i32,
    pub relative_radius: f32,
    pub height: f32,
    pub peaks: Vec<f32>,
    pub c0: usize,
    pub c1: usize,
    pub use_target: bool,
    pub polynomial: bool,
    pub alpha: f32,
    #[serde(default)]
    pub kernel_kind: KernelKind,
}

impl KernelDef {
    pub fn default_single(mu: f32, sigma: f32, radius: i32) -> Self {
        Self {
            mu,
            sigma,
            base_radius: radius,
            relative_radius: 1.0,
            height: 1.0,
            peaks: vec![1.0],
            c0: 0,
            c1: 0,
            use_target: false,
            polynomial: false,
            alpha: 4.0,
            kernel_kind: KernelKind::default(),
        }
    }

    pub fn new(
        mu: f32,
        sigma: f32,
        base_radius: i32,
        relative_radius: f32,
        height: f32,
        peaks: Vec<f32>,
        c0: usize,
        c1: usize,
        polynomial: bool,
    ) -> Self {
        Self {
            mu,
            sigma,
            base_radius,
            relative_radius,
            height,
            peaks,
            c0,
            c1,
            use_target: false,
            polynomial,
            alpha: 4.0,
            kernel_kind: KernelKind::default(),
        }
    }

    pub fn with_kernel_kind(mut self, kernel_kind: KernelKind) -> Self {
        self.kernel_kind = kernel_kind;
        self
    }

    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
}

    pub fn with_target(mut self, use_target: bool) -> Self {
        self.use_target = use_target;
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StateType {
    CONTINUOUS,
    DISCRETE,
}
