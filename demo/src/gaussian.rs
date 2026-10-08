use crate::science::{GREEN, ORANGE, Scene, Series, gradient, surface};
use std::f64::consts::{PI, TAU};

pub const DESCRIPTION: &str = "The height is a normalized bivariate Gaussian probability density, not a probability at a point. Flip correlation to turn the ridge; zero correlation removes the tilt. Green and orange contours mark Mahalanobis radii 1 and 2, enclosing about 39% and 86% of the full distribution. The contour lines are lifted slightly above the surface for visibility. X standard deviation is 1. Z is magnified 8x; ticks show actual density. The displayed surface is truncated at x,y = +/-4.5.";

pub fn density(x: f64, y: f64, rho: f64, sy: f64) -> f64 {
    let q = 1.0 - rho * rho;
    (-(x * x - 2.0 * rho * x * y / sy + (y / sy).powi(2)) / (2.0 * q)).exp()
        / (2.0 * PI * sy * q.sqrt())
}
pub fn build(p: [f64; 3]) -> Scene {
    let peak = density(0.0, 0.0, p[0], p[1]);
    Scene {
        surface: Some(surface(
            "Probability density",
            4.5,
            65,
            |x, y| density(x, y, p[0], p[1]),
            |z| gradient(z / peak),
        )),
        lines: [
            (1.0, GREEN, "Radius 1: 39%"),
            (2.0, ORANGE, "Radius 2: 86%"),
        ]
        .into_iter()
        .map(|(r, c, name)| {
            Series::new(
                name,
                (0..=160)
                    .map(|i| {
                        let t = TAU * i as f64 / 160.0;
                        [
                            r * t.cos(),
                            r * p[1] * (p[0] * t.cos() + (1.0 - p[0] * p[0]).sqrt() * t.sin()),
                            peak * (-0.5_f64 * r * r).exp() + 0.003,
                        ]
                    })
                    .collect(),
                c,
            )
        })
        .collect(),
        ..Default::default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn density_normalizes_and_reflects_with_correlation() {
        for rho in [-0.9, 0.0, 0.9] {
            let step = 0.05;
            let mut mass = 0.0;
            for i in -160..160 {
                for j in -160..160 {
                    mass += density((i as f64 + 0.5) * step, (j as f64 + 0.5) * step, rho, 1.5)
                        * step
                        * step;
                }
            }
            assert!((mass - 1.0).abs() < 1e-6, "mass {mass}");
            assert!((density(0.7, 1.2, rho, 1.5) - density(-0.7, 1.2, -rho, 1.5)).abs() < 1e-12);
        }
    }
}
