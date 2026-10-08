use crate::science::{GREEN, IndexedSurface, Scene, Series};
use eframe::egui;
use egui_plot3d::{ColorBar, ColorMap, ColorScale, SurfaceGrid};
use std::f64::consts::{PI, TAU};
use std::sync::OnceLock;

pub const DESCRIPTION: &str = "Two radial waves reinforce or cancel. Inferno shows amplitude from -2 to +2 in arbitrary units; opposite phases cancel along the centerline.";

pub fn amplitude(x: f64, y: f64, p: [f64; 3], time: f64) -> f64 {
    let r1 = (x - p[2] * 0.5).hypot(y);
    let r2 = (x + p[2] * 0.5).hypot(y);
    (TAU * r1 / p[1] - time * 2.0).sin() / (1.0 + r1).sqrt()
        + (TAU * r2 / p[1] - time * 2.0 + p[0] * PI).sin() / (1.0 + r2).sqrt()
}
pub fn build(p: [f64; 3], time: f64) -> Scene {
    static GRID: OnceLock<SurfaceGrid> = OnceLock::new();
    let mut grid = GRID
        .get_or_init(|| SurfaceGrid::from_fn(-4.0..=4.0, -4.0..=4.0, [65, 65], |_, _| 0.0))
        .clone();
    grid.resample(|x, y| [x, y, amplitude(x, y, p, time)]);
    let mut mesh = IndexedSurface::from_grid("Wave amplitude", &grid, &scale());
    // Fixed vertical bounds keep the camera still during animation.
    mesh.bounds = egui_plot3d::PlotBounds::from_min_max([-4.0, -4.0, -2.0], [4.0, 4.0, 2.0]);
    Scene {
        mesh: Some(mesh),
        dots: vec![Series::new(
            "Sources",
            vec![[-p[2] * 0.5, 0.0, 0.0], [p[2] * 0.5, 0.0, 0.0]],
            GREEN,
        )],
        ..Default::default()
    }
}

fn scale() -> ColorScale {
    ColorScale::new(-2.0..=2.0, ColorMap::Inferno)
}
pub fn color_scale(ui: &mut egui::Ui) {
    ui.add(ColorBar::new("Amplitude", &scale()).width(180.0));
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
