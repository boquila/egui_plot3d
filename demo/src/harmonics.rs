use crate::science::{BLUE, GREEN, ORANGE, Scene, Series};
use std::f64::consts::TAU;

pub const DESCRIPTION: &str = "Three sinusoidal coordinates form a 3D Lissajous curve. The base functions are x = sin(fx t + 0.4), y = sin(fy t), z = sin(fz t), with amplitude 1 in arbitrary units. Integer frequency ratios close the curve at t = 2 pi. Animate values changes the phases and amplitudes and blends in higher harmonics, making the whole curve morph while staying closed. The faint floor and side curves show coordinate projections.";

pub fn position(t: f64, f: [f64; 3]) -> [f64; 3] {
    [(f[0] * t + 0.4).sin(), (f[1] * t).sin(), (f[2] * t).sin()]
}
pub fn build(f: [f64; 3]) -> Scene {
    build_animated(f, 0.0, 1.0)
}

/// Every sample changes with `time`; integer spatial frequencies keep the curve closed.
pub fn animated_position(t: f64, f: [f64; 3], time: f64, strength: f64) -> [f64; 3] {
    if time == 0.0 || strength == 0.0 {
        return position(t, f);
    }
    let phase = [
        0.4 + strength * time.sin(),
        strength * (time * 0.73).sin(),
        strength * (time * 0.51).sin(),
    ];
    std::array::from_fn(|i| {
        let modulation = strength * (time * (0.6 + i as f64 * 0.17)).sin();
        (1.0 + 0.2 * modulation) * (f[i] * t + phase[i]).sin()
            + 0.22 * modulation * ((f[i] + f[(i + 1) % 3]) * t).sin()
    })
}

pub fn build_animated(f: [f64; 3], time: f64, strength: f64) -> Scene {
    let points: Vec<_> = (0..=2400)
        .map(|i| animated_position(TAU * i as f64 / 2400.0, f, time, strength))
        .collect();
    let floor = points.iter().map(|p| [p[0], p[1], -1.6]).collect();
    let side = points.iter().map(|p| [-1.6, p[1], p[2]]).collect();
    Scene {
        lines: vec![
            Series::new("3D trajectory", points, BLUE),
            Series::new("XY projection", floor, GREEN.gamma_multiply(0.55)),
            Series::new("YZ projection", side, ORANGE.gamma_multiply(0.55)),
        ],
        ..Default::default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn integer_frequency_paths_close_and_stay_bounded() {
        for f in [[2.0, 3.0, 5.0], [8.0, 7.0, 1.0]] {
            for (a, b) in position(0.0, f).into_iter().zip(position(TAU, f)) {
                assert!((a - b).abs() < 1e-12);
            }
            for i in 0..1000 {
                assert!(
                    position(i as f64 * 0.1, f)
                        .into_iter()
                        .all(|v| v.abs() <= 1.0)
                );
            }
        }
    }
}
