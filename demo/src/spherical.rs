use crate::science::{IndexedSurface, Scene};
use egui_plot3d::PlotBounds;
use std::{
    f64::consts::{PI, TAU},
    sync::{Arc, OnceLock},
};

pub const DESCRIPTION: &str = "A spherical harmonic makes alternating lobes expand and contract. Degree and order set the nodal pattern; colors show signed displacement.";
const LAT: usize = 48;
const LON: usize = 96;

// Associated Legendre polynomial with the Condon–Shortley phase.
fn legendre(l: usize, m: usize, x: f64) -> f64 {
    let mut pmm = 1.0;
    for k in 1..=m {
        pmm *= -((2 * k - 1) as f64) * (1.0 - x * x).max(0.0).sqrt();
    }
    if l == m {
        return pmm;
    }
    let mut previous = pmm;
    let mut current = x * (2 * m + 1) as f64 * pmm;
    for n in m + 2..=l {
        let next =
            ((2 * n - 1) as f64 * x * current - (n + m - 1) as f64 * previous) / (n - m) as f64;
        previous = current;
        current = next;
    }
    current
}

fn topology() -> Arc<[[u32; 3]]> {
    static INDICES: OnceLock<Arc<[[u32; 3]]>> = OnceLock::new();
    INDICES
        .get_or_init(|| {
            let mut faces = Vec::new();
            for y in 0..LAT {
                for x in 0..LON {
                    let a = (y * LON + x) as u32;
                    let b = (y * LON + (x + 1) % LON) as u32;
                    let c = a + LON as u32;
                    let d = b + LON as u32;
                    if y > 0 {
                        faces.push([a, b, c]);
                    }
                    if y + 1 < LAT {
                        faces.push([b, d, c]);
                    }
                }
            }
            faces.into()
        })
        .clone()
}

pub fn build(p: [f64; 3], time: f64, strength: f64) -> Scene {
    let l = p[0] as usize;
    let m = (p[1] as usize).min(l);
    let basis: Vec<_> = (0..=LAT)
        .map(|y| legendre(l, m, (PI * y as f64 / LAT as f64).cos()))
        .collect();
    // Peak-normalized for visible, comparable displacement across degrees.
    let peak = basis
        .iter()
        .fold(0.0_f64, |a, &b| a.max(b.abs()))
        .max(1e-12);
    let oscillation = (time * 1.8).cos();
    let mut values = Vec::with_capacity((LAT + 1) * LON);
    let mut vertices = Vec::with_capacity((LAT + 1) * LON);
    for (y, &latitude) in basis.iter().enumerate() {
        let theta = PI * y as f64 / LAT as f64;
        for x in 0..LON {
            let phi = TAU * x as f64 / LON as f64;
            let value = latitude / peak * (m as f64 * phi).cos() * oscillation;
            let radius = 2.1 + p[2] * strength * value;
            vertices.push([
                radius * theta.sin() * phi.cos(),
                radius * theta.sin() * phi.sin(),
                radius * theta.cos(),
            ]);
            values.push(value);
        }
    }
    let indices = topology();
    let colors = indices
        .iter()
        .map(|face| {
            let value = face.iter().map(|&i| values[i as usize]).sum::<f64>() / 3.0;
            crate::inferno::color(value * 0.5 + 0.5)
        })
        .collect();
    Scene {
        mesh: Some(IndexedSurface {
            name: "Radial displacement",
            vertices: vertices.into(),
            indices,
            colors,
            shading: false,
            bounds: PlotBounds::from_min_max([-3.2; 3], [3.2; 3]),
        }),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_harmonics_and_poles() {
        for x in [-1.0, -0.3, 0.0, 0.7, 1.0] {
            assert!((legendre(2, 0, x) - (3.0 * x * x - 1.0) * 0.5).abs() < 1e-12);
            assert!((legendre(2, 2, x) - 3.0 * (1.0 - x * x)).abs() < 1e-12);
        }
        for l in 1..=7 {
            for m in 1..=l {
                assert_eq!(legendre(l, m, 1.0), 0.0);
                assert_eq!(legendre(l, m, -1.0), 0.0);
            }
        }
    }
    #[test]
    fn opposite_phases_reverse_radial_displacement() {
        let a = build([4.0, 3.0, 0.85], 0.0, 1.0).mesh.unwrap();
        let b = build([4.0, 3.0, 0.85], PI / 1.8, 1.0).mesh.unwrap();
        for (a, b) in a.vertices.as_slice().iter().zip(b.vertices.as_slice()) {
            let radius = |p: &[f64; 3]| p[0].hypot(p[1]).hypot(p[2]);
            assert!((radius(a) + radius(b) - 4.2).abs() < 1e-10);
        }
    }
}
