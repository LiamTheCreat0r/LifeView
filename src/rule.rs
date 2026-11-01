use bevy::math::ops::exp;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub state_type: StateType,
    #[serde(default = "default_discrete_states")]
    pub num_discrete_states: usize,
    #[serde(default)]
    pub transition: Transition,
    pub delta: f32,
    pub kernels: Vec<KernelDef>,
    pub num_channels: usize,
    #[serde(default)]
    pub mode: RuleMode,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Transition {
    #[serde(rename = "lenia_growth")]
    LeniaGrowth,
    #[serde(rename = "birth_survive")]
    BirthSurvive { birth: Vec<u8>, survive: Vec<u8> },
}

impl Default for Transition {
    fn default() -> Self {
        Self::LeniaGrowth
    }
}

impl Transition {
    pub fn conway() -> Self {
        Self::BirthSurvive {
            birth: vec![3],
            survive: vec![2, 3],
        }
    }
}

fn default_discrete_states() -> usize {
    2
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KernelKind {
    /// Smooth Gaussian bump centred on the middle of the radius.
    Gaussian,
    /// Official Lenia polynomial kernel `(4t(1-t))^4`.
    Quad4,
    /// Official Lenia exponential bump `exp(4 - 1/(t(1-t)))`.
    Bump4,
    /// Official Lenia step kernel, flat on `t in [0.25, 0.75]`.
    Step,
}

impl Default for KernelKind {
    fn default() -> Self {
        Self::Gaussian
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GrowthKind {
    /// Compact quartic growth curve.
    Polynomial,
    /// Gaussian growth curve.
    Gaussian,
    /// Official Lenia step growth: `+1` inside `mu +/- sigma`, `-1` outside.
    Step,
}

impl Default for GrowthKind {
    fn default() -> Self {
        Self::Gaussian
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
            num_discrete_states: 2,
            transition: Transition::LeniaGrowth,
            delta: 0.1,
            kernels: vec![KernelDef::default_single(mu, sigma, radius)],
            num_channels: 1,
            mode: RuleMode::Sum,
        }
    }

    #[allow(dead_code)]
    pub fn multi_channel(kernels: Vec<KernelDef>, num_channels: usize) -> Self {
        Self {
            state_type: StateType::CONTINUOUS,
            num_discrete_states: 2,
            transition: Transition::LeniaGrowth,
            delta: 0.1,
            kernels,
            num_channels,
            mode: RuleMode::Sum,
        }
    }

    #[allow(dead_code)]
    pub fn conway() -> Self {
        Self {
            state_type: StateType::DISCRETE,
            num_discrete_states: 2,
            transition: Transition::conway(),
            delta: 1.0,
            kernels: Vec::new(),
            num_channels: 1,
            mode: RuleMode::Sum,
        }
    }

    pub fn growth(&self, u: f32, kernel_idx: usize) -> f32 {
        let k = &self.kernels[kernel_idx];
        match k.effective_growth_kind() {
            GrowthKind::Polynomial => {
                let l = (u - k.mu).abs();
                let k_val = 3.0 * k.sigma;
                if l <= k_val {
                    let ratio = l / k_val;
                    2.0 * (1.0 - ratio * ratio).powf(k.alpha) - 1.0
                } else {
                    -1.0
                }
            }
            GrowthKind::Gaussian => {
                2.0 * exp(-((u - k.mu).powi(2) / (2.0 * k.sigma * k.sigma))) - 1.0
            }
            GrowthKind::Step => {
                if (u - k.mu).abs() <= k.sigma {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }

    pub fn target(&self, u: f32, kernel_idx: usize) -> f32 {
        let k = &self.kernels[kernel_idx];
        exp(-((u - k.mu).powi(2) / (2.0 * k.sigma * k.sigma)))
    }

    #[allow(dead_code)]
    pub fn effective_radius(&self, kernel_idx: usize) -> i32 {
        let k = &self.kernels[kernel_idx];
        ((k.base_radius as f32) * k.relative_radius).ceil() as i32
    }

    #[allow(dead_code)]
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
    /// Legacy flag kept for old shape files. When `kernel_kind` or `growth_kind`
    /// are absent they fall back to `polynomial`: `true` means Quad4 + polynomial.
    pub polynomial: bool,
    pub alpha: f32,
    #[serde(default)]
    pub kernel_kind: Option<KernelKind>,
    #[serde(default)]
    pub growth_kind: Option<GrowthKind>,
}

impl KernelDef {
    /// Kernel kind actually used by the convolution, honouring the legacy
    /// `polynomial` flag when no explicit kind was loaded.
    pub fn effective_kernel_kind(&self) -> KernelKind {
        self.kernel_kind.clone().unwrap_or(if self.polynomial {
            KernelKind::Quad4
        } else {
            KernelKind::Gaussian
        })
    }

    /// Growth kind actually used by the update, honouring the legacy
    /// `polynomial` flag when no explicit kind was loaded.
    pub fn effective_growth_kind(&self) -> GrowthKind {
        self.growth_kind.clone().unwrap_or(if self.polynomial {
            GrowthKind::Polynomial
        } else {
            GrowthKind::Gaussian
        })
    }

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
            kernel_kind: None,
            growth_kind: None,
        }
    }

    #[allow(dead_code)]
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
            kernel_kind: None,
            growth_kind: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_kernel_kind(mut self, kernel_kind: KernelKind) -> Self {
        self.kernel_kind = Some(kernel_kind);
        self
    }

    #[allow(dead_code)]
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    #[allow(dead_code)]
    pub fn with_target(mut self, use_target: bool) -> Self {
        self.use_target = use_target;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StateType {
    CONTINUOUS,
    DISCRETE,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kernel_from_json(polynomial: bool, extra: &str) -> KernelDef {
        let json = format!(
            r#"{{
                "mu": 0.545,
                "sigma": 0.186,
                "base_radius": 13,
                "relative_radius": 1.0,
                "height": 1.0,
                "peaks": [1.0],
                "c0": 0,
                "c1": 0,
                "use_target": false,
                "polynomial": {polynomial},
                "alpha": 4.0
                {extra}
            }}"#
        );
        serde_json::from_str(&json).unwrap()
    }

    fn rule_with(kernel: KernelDef) -> Rule {
        Rule {
            state_type: StateType::CONTINUOUS,
            num_discrete_states: 2,
            transition: Transition::LeniaGrowth,
            delta: 0.1,
            kernels: vec![kernel],
            num_channels: 1,
            mode: RuleMode::Sum,
        }
    }

    #[test]
    fn legacy_polynomial_flag_selects_quad4_and_polynomial() {
        let kernel = kernel_from_json(true, "");
        assert_eq!(kernel.effective_kernel_kind(), KernelKind::Quad4);
        assert_eq!(kernel.effective_growth_kind(), GrowthKind::Polynomial);
    }

    #[test]
    fn legacy_default_selects_gaussian_kernel_and_growth() {
        let kernel = kernel_from_json(false, "");
        assert_eq!(kernel.effective_kernel_kind(), KernelKind::Gaussian);
        assert_eq!(kernel.effective_growth_kind(), GrowthKind::Gaussian);
    }

    #[test]
    fn explicit_kinds_override_the_legacy_flag() {
        let kernel = kernel_from_json(true, r#", "kernel_kind": "step", "growth_kind": "step""#);
        assert_eq!(kernel.effective_kernel_kind(), KernelKind::Step);
        assert_eq!(kernel.effective_growth_kind(), GrowthKind::Step);
    }

    #[test]
    fn step_growth_is_positive_inside_sigma_and_negative_outside() {
        let kernel = kernel_from_json(false, r#", "growth_kind": "step""#);
        let rule = rule_with(kernel);
        // mu = 0.545, sigma = 0.186
        assert_eq!(rule.growth(0.545, 0), 1.0);
        assert_eq!(rule.growth(0.545 + 0.186, 0), 1.0);
        assert_eq!(rule.growth(0.545 - 0.186, 0), 1.0);
        assert_eq!(rule.growth(0.545 + 0.187, 0), -1.0);
        assert_eq!(rule.growth(0.545 - 0.187, 0), -1.0);
    }

    #[test]
    fn step_growth_is_stable_for_discutium_pachus_values() {
        let kernel = kernel_from_json(false, r#", "growth_kind": "step""#);
        let rule = rule_with(kernel);
        // Around the growth peak the step rule must be +1, not 0.
        assert_eq!(rule.growth(0.5, 0), 1.0);
        assert_eq!(rule.growth(0.7, 0), 1.0);
        assert_eq!(rule.growth(0.1, 0), -1.0);
    }
}
