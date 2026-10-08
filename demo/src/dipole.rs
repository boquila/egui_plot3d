use crate::science::{BLUE, ORANGE, Scene, Series};
use std::f64::consts::{PI, TAU};

pub const DESCRIPTION: &str = "Ideal magnetic dipole field lines. Arrows show direction, not strength. Animation rotates the dipole and varies the sampled lines.";

pub fn field(p: [f64; 3]) -> [f64; 3] {
    let r2 = p.iter().map(|v| v * v).sum::<f64>();
    let r = r2.sqrt();
    [
        3.0 * p[2] * p[0] / r.powi(5),
        3.0 * p[2] * p[1] / r.powi(5),
        (3.0 * p[2] * p[2] / r2 - 1.0) / r.powi(3),
    ]
}
fn tilt(p: [f64; 3], angle: f64) -> [f64; 3] {
    [
        p[0] * angle.cos() + p[2] * angle.sin(),
        p[1],
        -p[0] * angle.sin() + p[2] * angle.cos(),
    ]
}
pub fn build(p: [f64; 3]) -> Scene {
    build_animated(p, 0.0, 1.0)
}

/// A sequence of ideal dipole snapshots, with changing orientation and field-line sampling.
/// The breathing changes which field lines are drawn, not the dipole field equation.
pub fn build_animated(p: [f64; 3], time: f64, strength: f64) -> Scene {
    let angle = p[0].to_radians() + strength * 0.55 * (time * 0.6).sin();
    let azimuth = strength * time * 0.45;
    let orient = |point| {
        let [x, y, z] = tilt(point, angle);
        [
            x * azimuth.cos() - y * azimuth.sin(),
            x * azimuth.sin() + y * azimuth.cos(),
            z,
        ]
    };
    let mut lines = Vec::new();
    let mut origins = Vec::new();
    let mut tips = Vec::new();
    for l in [1.3, 2.3, 3.3] {
        // Different rates let the shells breathe independently while each line
        // still satisfies the instantaneous ideal-dipole geometry.
        let scale = 1.0 + 0.22 * strength * (time * (0.55 + 0.12 * l)).sin();
        let radius = l * scale;
        let mut path = Vec::new();
        let start = (0.3_f64 / radius).sqrt().asin();
        for meridian in 0..p[1] as usize {
            if !path.is_empty() {
                path.push([f64::NAN; 3]);
            }
            let phi = TAU * meridian as f64 / p[1];
            for i in 0..=128 {
                let theta = start + (PI - 2.0 * start) * i as f64 / 128.0;
                let r = radius * theta.sin().powi(2);
                let point = [
                    r * theta.sin() * phi.cos(),
                    r * theta.sin() * phi.sin(),
                    r * theta.cos(),
                ];
                path.push(orient(point));
                if [32, 64, 96].contains(&i) {
                    let b = field(point);
                    let norm = b.iter().map(|v| v * v).sum::<f64>().sqrt();
                    origins.push(orient(point));
                    tips.push(orient(std::array::from_fn(|j| {
                        point[j] + 0.24 * b[j] / norm
                    })));
                }
            }
        }
        lines.push(Series::new(
            match l {
                1.3 => "Inner field lines",
                2.3 => "Middle field lines",
                _ => "Outer field lines",
            },
            path,
            BLUE,
        ));
    }
    Scene {
        lines,
        dots: vec![
            Series::new("Dipole + axis", vec![orient([0.0, 0.0, 0.3])], ORANGE),
            Series::new("Dipole - axis", vec![orient([0.0, 0.0, -0.3])], BLUE),
        ],
        arrows: Some((origins.into(), tips.into())),
        ..Default::default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn field_has_correct_direction_and_cubic_decay() {
        assert_eq!(field([1.0, 0.0, 0.0]), [0.0, 0.0, -1.0]);
        assert_eq!(field([0.0, 0.0, 1.0]), [0.0, 0.0, 2.0]);
        assert_eq!(field([0.0, 0.0, 2.0]), [0.0, 0.0, 0.25]);
        let p = tilt([1.0, 2.0, 3.0], 0.7);
        assert!((p.iter().map(|v| v * v).sum::<f64>() - 14.0).abs() < 1e-12);
    }
}
