use crate::science::{GREEN, Scene, Series, gradient, surface};
use std::f64::consts::{PI, TAU};

pub const DESCRIPTION: &str = "Two coherent sources emit radial waves. Their amplitudes add: orange peaks and blue troughs reinforce or cancel. Compare phase 0 with pi: along x = 0, equal distances give complete cancellation at pi. Wavelength sets fringe spacing. This illustrative model uses softened radial decay, not a fluid simulation. Coordinates and amplitude are in arbitrary units.";

pub fn amplitude(x: f64, y: f64, p: [f64; 3], time: f64) -> f64 {
    let r1 = (x - p[2] * 0.5).hypot(y);
    let r2 = (x + p[2] * 0.5).hypot(y);
    (TAU * r1 / p[1] - time * 2.0).sin() / (1.0 + r1).sqrt()
        + (TAU * r2 / p[1] - time * 2.0 + p[0] * PI).sin() / (1.0 + r2).sqrt()
}
pub fn build(p: [f64; 3], time: f64) -> Scene {
    let mut mesh = surface(
        "Wave amplitude",
        4.0,
        65,
        |x, y| amplitude(x, y, p, time),
        |z| gradient(z * 0.3 + 0.5),
    );
    // Fixed vertical bounds keep the camera still during animation.
    mesh.bounds = egui_plot3d::PlotBounds::from_min_max([-4.0, -4.0, -2.0], [4.0, 4.0, 2.0]);
    Scene {
        surface: Some(mesh),
        dots: vec![Series::new(
            "Sources",
            vec![[-p[2] * 0.5, 0.0, 0.0], [p[2] * 0.5, 0.0, 0.0]],
            GREEN,
        )],
        ..Default::default()
    }
}

/// Smooth parameter modulation makes fringes merge and separate as waves travel.
pub fn animated_parameters(p: [f64; 3], time: f64, strength: f64) -> [f64; 3] {
    [
        p[0] + strength * (time * 0.65).sin(),
        p[1] * (1.0 + 0.22 * strength * (time * 0.43).sin()),
        p[2] * (1.0 + 0.30 * strength * (time * 0.57).sin()),
    ]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opposite_phase_cancels_on_bisector() {
        for y in [-3.0, -0.3, 0.0, 2.1] {
            for time in [0.0, 0.7, 4.0] {
                assert!(amplitude(0.0, y, [1.0, 1.5, 2.4], time).abs() < 1e-12);
            }
        }
        assert!(amplitude(0.0, 0.0, [0.0, 1.5, 2.4], 0.0).abs() > 0.5);
    }
}
