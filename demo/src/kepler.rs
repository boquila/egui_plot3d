use crate::science::{BLUE, GREEN, ORANGE, Scene, Series};
use std::f64::consts::{PI, TAU};

pub const DESCRIPTION: &str = "An elliptical orbit with the star at a focus. Dots mark equal time intervals: wider gaps mean faster motion.";

pub fn eccentric_anomaly(mean: f64, e: f64) -> f64 {
    let mean = (mean + PI).rem_euclid(TAU) - PI;
    let mut value = mean;
    for _ in 0..15 {
        value -= (value - e * value.sin() - mean) / (1.0 - e * value.cos());
    }
    value
}
pub fn position(mean: f64, e: f64, inclination: f64) -> [f64; 3] {
    let anomaly = eccentric_anomaly(mean, e);
    let x = 3.0 * (anomaly.cos() - e);
    let y = 3.0 * (1.0 - e * e).sqrt() * anomaly.sin();
    [x, y * inclination.cos(), y * inclination.sin()]
}
pub fn build(p: [f64; 3]) -> Scene {
    let at = |m| position(m, p[0], p[1].to_radians());
    Scene {
        lines: vec![Series::new(
            "Orbit",
            (0..=512).map(|i| at(TAU * i as f64 / 512.0)).collect(),
            BLUE,
        )],
        dots: vec![
            Series::new(
                "Equal time intervals",
                (0..24).map(|i| at(TAU * i as f64 / 24.0)).collect(),
                GREEN,
            ),
            Series::new("Star at focus", vec![[0.0; 3]], ORANGE),
        ],
        ..Default::default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn solves_kepler_and_preserves_orbit_extrema() {
        for e in [0.0, 0.65, 0.85] {
            for i in -100..100 {
                let m = PI * i as f64 / 100.0;
                let a = eccentric_anomaly(m, e);
                assert!((a - e * a.sin() - m).abs() < 1e-12);
            }
            assert!((position(0.0, e, 0.7)[0] - 3.0 * (1.0 - e)).abs() < 1e-12);
            assert!((position(PI, e, 0.7)[0] + 3.0 * (1.0 + e)).abs() < 1e-12);
        }
    }
}
