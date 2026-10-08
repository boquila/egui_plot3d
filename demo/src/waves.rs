use crate::science::{GREEN, Scene, Series, surface};
use eframe::egui;
use std::f64::consts::{PI, TAU};

pub const DESCRIPTION: &str = "Two radial waves reinforce or cancel. Inferno shows amplitude from -2 to +2 in arbitrary units; opposite phases cancel along the centerline.";

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
        |z| crate::inferno::color(z * 0.25 + 0.5),
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

pub fn color_scale(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.small("Amplitude  -2");
        let (rect, _) = ui.allocate_exact_size(egui::vec2(130.0, 10.0), egui::Sense::hover());
        let mut mesh = egui::Mesh::default();
        for i in 0..=64 {
            let t = i as f64 / 64.0;
            let x = egui::lerp(rect.x_range(), t as f32);
            let color = crate::inferno::color(t);
            mesh.colored_vertex(egui::pos2(x, rect.top()), color);
            mesh.colored_vertex(egui::pos2(x, rect.bottom()), color);
            if i > 0 {
                let k = i * 2;
                mesh.add_triangle(k - 2, k - 1, k);
                mesh.add_triangle(k, k - 1, k + 1);
            }
        }
        ui.painter().add(egui::Shape::mesh(mesh));
        ui.small("+2  · Inferno");
    });
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
