use crate::science::{IndexedSurface, Scene};
use egui_plot3d::PlotBounds;
use std::f64::consts::TAU;

pub const DESCRIPTION: &str = "Windings form a torus knot; shared factors create linked loops. Traveling waves deform the tubes without cutting the centerlines.";
const STEPS: usize = 384;
const SIDES: usize = 12;

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}
fn center(u: f64, p: f64, q: f64, offset: f64, time: f64, strength: f64) -> [f64; 3] {
    let phi = p * u;
    // A shared deformation of the containing torus preserves the link.
    let minor = 0.85 + 0.20 * strength * (3.0 * phi - time * 1.3).sin();
    let theta = q * u + offset + 0.45 * strength * (time * 0.6).sin();
    let r = 2.15 + minor * theta.cos();
    [r * phi.cos(), r * phi.sin(), minor * theta.sin()]
}
fn unit(v: [f64; 3]) -> [f64; 3] {
    let n = v[0].hypot(v[1]).hypot(v[2]);
    v.map(|v| v / n)
}

pub fn build(p: [f64; 3], time: f64, strength: f64) -> Scene {
    let components = gcd(p[0] as usize, p[1] as usize);
    let turns = [p[0] / components as f64, p[1] / components as f64];
    let mut vertices = Vec::with_capacity(components * STEPS * SIDES);
    let mut indices = Vec::with_capacity(components * STEPS * SIDES * 2);
    let mut colors = Vec::with_capacity(components * STEPS * SIDES * 2);
    for component in 0..components {
        let offset = TAU * component as f64 / p[0];
        let base = vertices.len() as u32;
        for i in 0..STEPS {
            let u = TAU * i as f64 / STEPS as f64;
            let c = center(u, turns[0], turns[1], offset, time, strength);
            let before = center(u - 1e-4, turns[0], turns[1], offset, time, strength);
            let after = center(u + 1e-4, turns[0], turns[1], offset, time, strength);
            let tangent = unit(std::array::from_fn(|j| after[j] - before[j]));
            let radial = [(turns[0] * u).cos(), (turns[0] * u).sin(), 0.0];
            let dot = (0..3).map(|j| radial[j] * tangent[j]).sum::<f64>();
            let normal = unit(std::array::from_fn(|j| radial[j] - dot * tangent[j]));
            let binormal = [
                tangent[1] * normal[2] - tangent[2] * normal[1],
                tangent[2] * normal[0] - tangent[0] * normal[2],
                tangent[0] * normal[1] - tangent[1] * normal[0],
            ];
            // Thin crowded links so the visible tubes remain separated.
            let thickness = p[2].min(0.50 / p[0].max(p[1]))
                * (1.0 + 0.18 * strength * (5.0 * u - time * 2.0).sin());
            for j in 0..SIDES {
                let angle = TAU * j as f64 / SIDES as f64;
                vertices.push(std::array::from_fn(|k| {
                    c[k] + thickness * (normal[k] * angle.cos() + binormal[k] * angle.sin())
                }));
                let a = base + (i * SIDES + j) as u32;
                let b = base + (i * SIDES + (j + 1) % SIDES) as u32;
                let c = base + (((i + 1) % STEPS) * SIDES + j) as u32;
                let d = base + (((i + 1) % STEPS) * SIDES + (j + 1) % SIDES) as u32;
                indices.extend([[a, b, c], [b, d, c]]);
                let color =
                    crate::inferno::color(0.5 + 0.48 * (3.0 * u + offset - time * 0.6).cos());
                colors.extend([color; 2]);
            }
        }
    }
    Scene {
        mesh: Some(IndexedSurface {
            name: "Knot surface",
            vertices: vertices.into(),
            indices: indices.into(),
            colors,
            shading: true,
            bounds: PlotBounds::from_min_max([-3.6, -3.6, -1.7], [3.6, 3.6, 1.7]),
        }),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn centers_close_and_mesh_edges_are_manifold() {
        use std::collections::HashMap;
        for p in [[2.0, 3.0, 0.2], [3.0, 3.0, 0.2], [7.0, 6.0, 0.1]] {
            let components = gcd(p[0] as usize, p[1] as usize);
            for c in 0..components {
                let f = |u| {
                    center(
                        u,
                        p[0] / components as f64,
                        p[1] / components as f64,
                        TAU * c as f64 / p[0],
                        1.7,
                        1.0,
                    )
                };
                assert!(
                    f(0.0)
                        .iter()
                        .zip(f(TAU))
                        .all(|(a, b)| (a - b).abs() < 1e-10)
                );
            }
            let mesh = build(p, 1.7, 1.0).mesh.unwrap();
            let mut edges = HashMap::new();
            for &[a, b, c] in mesh.indices.iter() {
                for (a, b) in [(a, b), (b, c), (c, a)] {
                    *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
                }
            }
            assert!(edges.values().all(|&n| n == 2));
        }
    }
}
