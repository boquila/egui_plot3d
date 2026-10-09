use crate::science::{IndexedSurface, Scene};
use egui_plot3d::PlotBounds;
use std::sync::{Arc, OnceLock};

pub const DESCRIPTION: &str = "Activation fronts curl into spirals in an excitable medium. Height and color show activation in the Barkley reaction–diffusion model.";
const N: usize = 96;
const DT: f64 = 0.006;
const INV_DX2: f64 = 16.0;

pub struct Model {
    pub params: [f64; 3],
    u: Vec<f64>,
    v: Vec<f64>,
    next: Vec<f64>,
    pending: f64,
}
impl Model {
    pub fn new(params: [f64; 3]) -> Self {
        let mut model = Self {
            params,
            u: vec![0.0; N * N],
            v: vec![0.0; N * N],
            next: vec![0.0; N * N],
            pending: 0.0,
        };
        // A broken planar wave: excitation on the left, refractory medium below.
        for y in 0..N {
            for x in 0..N {
                model.u[y * N + x] = if x < N / 2 { 1.0 } else { 0.0 };
                model.v[y * N + x] = if y < N / 2 { params[0] * 0.5 } else { 0.0 };
            }
        }
        for _ in 0..600 {
            model.step();
        }
        model
    }
    fn step(&mut self) {
        let [a, b, epsilon] = self.params;
        for y in 0..N {
            for x in 0..N {
                let i = y * N + x;
                let u = self.u[i];
                // Reflecting boundaries (zero normal flux), five-point Laplacian.
                let lap = self.u[y * N + x.saturating_sub(1)]
                    + self.u[y * N + (x + 1).min(N - 1)]
                    + self.u[y.saturating_sub(1) * N + x]
                    + self.u[(y + 1).min(N - 1) * N + x]
                    - 4.0 * u;
                self.next[i] =
                    u + DT * (lap * INV_DX2 + u * (1.0 - u) * (u - (self.v[i] + b) / a) / epsilon);
                self.v[i] += DT * (u - self.v[i]);
            }
        }
        std::mem::swap(&mut self.u, &mut self.next);
    }
    pub fn advance(&mut self, seconds: f64) {
        self.pending += seconds * 0.8;
        while self.pending + 1e-12 >= DT {
            self.step();
            self.pending -= DT;
        }
    }
    pub fn scene(&self, strength: f64) -> Scene {
        static INDICES: OnceLock<Arc<[[u32; 3]]>> = OnceLock::new();
        let indices = INDICES
            .get_or_init(|| egui_plot3d::SurfaceGrid::triangle_indices([N, N]))
            .clone();
        let vertices: Vec<_> = self
            .u
            .iter()
            .enumerate()
            .map(|(i, &u)| {
                [
                    10.0 * (i % N) as f64 / (N - 1) as f64 - 5.0,
                    10.0 * (i / N) as f64 / (N - 1) as f64 - 5.0,
                    strength * u,
                ]
            })
            .collect();
        let colors = indices
            .iter()
            .map(|f| {
                egui_plot3d::ColorMap::Mako
                    .color(f.iter().map(|&i| self.u[i as usize]).sum::<f64>() / 3.0)
            })
            .collect();
        Scene {
            mesh: Some(IndexedSurface {
                name: "Activation",
                legend_color: egui_plot3d::ColorMap::Mako.color(0.65),
                vertices: vertices.into(),
                indices,
                colors,
                shading: false,
                bounds: PlotBounds::from_min_max([-5.0, -5.0, 0.0], [5.0, 5.0, 1.05]),
            }),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equilibrium_and_fixed_step_evolution() {
        let mut a = Model::new([0.75, 0.02, 0.02]);
        let mut b = Model::new(a.params);
        a.advance(1.0);
        for _ in 0..8 {
            b.advance(0.125);
        }
        assert_eq!(a.u, b.u);
        assert_eq!(a.v, b.v);
        for p in [[0.65, 0.07, 0.02], [0.9, 0.01, 0.08]] {
            let mut m = Model::new(p);
            m.advance(5.0);
            assert!(
                m.u.iter()
                    .chain(&m.v)
                    .all(|v| v.is_finite() && *v >= -1e-8 && *v <= 1.00001)
            );
        }
        a.u.fill(0.0);
        a.v.fill(0.0);
        a.step();
        assert!(a.u.iter().chain(&a.v).all(|&v| v == 0.0));
    }
}
