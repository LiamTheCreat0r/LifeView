use std::cmp::min;

use bevy::prelude::*;

#[derive(Clone, Debug)]
pub struct ColorGradient {
    pub name: &'static str,
    pub colors: Vec<LinearRgba>,
}

impl ColorGradient {
    pub fn viridis() -> Self {
        Self {
            name: "Viridis",
            colors: vec![
                LinearRgba::new(0.267, 0.004, 0.329, 1.0),
                LinearRgba::new(0.282, 0.141, 0.458, 1.0),
                LinearRgba::new(0.254, 0.265, 0.530, 1.0),
                LinearRgba::new(0.164, 0.471, 0.558, 1.0),
                LinearRgba::new(0.134, 0.659, 0.518, 1.0),
                LinearRgba::new(0.478, 0.821, 0.318, 1.0),
                LinearRgba::new(0.993, 0.906, 0.144, 1.0),
            ],
        }
    }

    pub fn plasma() -> Self {
        Self {
            name: "Plasma",
            colors: vec![
                LinearRgba::new(0.050, 0.030, 0.528, 1.0),
                LinearRgba::new(0.369, 0.061, 0.699, 1.0),
                LinearRgba::new(0.679, 0.105, 0.575, 1.0),
                LinearRgba::new(0.895, 0.188, 0.377, 1.0),
                LinearRgba::new(0.990, 0.385, 0.148, 1.0),
                LinearRgba::new(0.991, 0.697, 0.195, 1.0),
                LinearRgba::new(0.940, 0.975, 0.131, 1.0),
            ],
        }
    }

    pub fn inferno() -> Self {
        Self {
            name: "Inferno",
            colors: vec![
                LinearRgba::new(0.001, 0.000, 0.014, 1.0),
                LinearRgba::new(0.243, 0.048, 0.424, 1.0),
                LinearRgba::new(0.524, 0.041, 0.468, 1.0),
                LinearRgba::new(0.734, 0.127, 0.290, 1.0),
                LinearRgba::new(0.929, 0.304, 0.122, 1.0),
                LinearRgba::new(0.987, 0.608, 0.089, 1.0),
                LinearRgba::new(0.988, 0.998, 0.645, 1.0),
            ],
        }
    }

    pub fn cividis() -> Self {
        Self {
            name: "Cividis",
            colors: vec![
                LinearRgba::new(0.000, 0.133, 0.200, 1.0),
                LinearRgba::new(0.122, 0.278, 0.337, 1.0),
                LinearRgba::new(0.318, 0.396, 0.388, 1.0),
                LinearRgba::new(0.502, 0.502, 0.376, 1.0),
                LinearRgba::new(0.682, 0.608, 0.341, 1.0),
                LinearRgba::new(0.851, 0.722, 0.278, 1.0),
                LinearRgba::new(1.000, 0.843, 0.196, 1.0),
            ],
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::viridis(),
            Self::plasma(),
            Self::inferno(),
            Self::cividis(),
        ]
    }

    pub fn lerp(&self, x: f32) -> LinearRgba {
        let ratio = (self.colors.len() - 1) as f32 * x;
        let min_idx = ratio.floor() as usize;
        let max_idx = min(min_idx + 1, self.colors.len() - 1);
        let t = ratio - ratio.floor();
        let a = self.colors[min_idx];
        let b = self.colors[max_idx];
        LinearRgba::new(
            a.red + (b.red - a.red) * t,
            a.green + (b.green - a.green) * t,
            a.blue + (b.blue - a.blue) * t,
            1.0,
        )
    }
}

#[derive(Clone, Debug)]
pub struct GridColoration {
    pub gradient: ColorGradient,
    pub smooth: bool,
}

impl Default for GridColoration {
    fn default() -> Self {
        Self {
            gradient: ColorGradient::inferno(),
            smooth: false,
        }
    }
}
