use crate::science::{IndexedSurface, Scene, Series};
use eframe::egui::Color32;
use egui_plot3d::{ColorMap, ColorScale, PlotBounds, SurfaceGrid};
use std::sync::OnceLock;

pub const DESCRIPTION: &str = "Give the network a point's x and y coordinates. Its job: predict whether that point is inside the ring. It learns from 256 labeled examples.";
const WIDTH: usize = 8;
const PARAMETERS: usize = 105;
const GRID: usize = 48;
const SAMPLES: usize = 256;
const STEP_SECONDS: f64 = 1.0 / 60.0;

pub fn bounds() -> PlotBounds {
    PlotBounds::from_min_max([-1.0, -1.0, 0.0], [1.0, 1.0, 1.0])
}

// A 2 → 8 → 8 → 1 MLP: tanh hidden units, sigmoid output.
// Parameter blocks: W1[16], b1[8], W2[64], b2[8], W3[8], b3[1].
pub struct Model {
    pub params: [f64; 3],
    weights: [f64; PARAMETERS],
    first_moment: [f64; PARAMETERS],
    second_moment: [f64; PARAMETERS],
    pending: f64,
    pub steps: usize,
    pub loss: f64,
    pub accuracy: f64,
    samples: Vec<([f64; 2], f64)>,
}

fn sigmoid(x: f64) -> f64 {
    0.5 + 0.5 * (0.5 * x).tanh()
}

fn target([x, y]: [f64; 2], width: f64) -> f64 {
    f64::from((x.hypot(y) - 0.60).abs() < width)
}

impl Model {
    pub fn new(params: [f64; 3]) -> Self {
        let mut state = params[2] as u64 + 12345;
        let mut weights = [0.0; PARAMETERS];
        for (i, w) in weights.iter_mut().enumerate() {
            if (16..24).contains(&i) || (88..96).contains(&i) || i == 104 {
                continue;
            }
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let uniform = (state >> 11) as f64 / (1u64 << 53) as f64;
            *w = (uniform * 2.0 - 1.0) * if i < 16 { 1.0 } else { 0.65 };
        }
        let samples = (0..SAMPLES)
            .map(|i| {
                let p = [
                    ((i % 16) as f64 + 0.5) / 8.0 - 1.0,
                    ((i / 16) as f64 + 0.5) / 8.0 - 1.0,
                ];
                (p, target(p, params[1]))
            })
            .collect();
        let mut model = Self {
            params,
            weights,
            first_moment: [0.0; PARAMETERS],
            second_moment: [0.0; PARAMETERS],
            pending: 0.0,
            steps: 0,
            loss: 0.0,
            accuracy: 0.0,
            samples,
        };
        model.measure();
        model
    }

    fn forward_logits(&self, x: [f64; 2]) -> ([f64; WIDTH], [f64; WIDTH], f64) {
        let a: [f64; WIDTH] = std::array::from_fn(|j| {
            (self.weights[16 + j] + self.weights[j * 2] * x[0] + self.weights[j * 2 + 1] * x[1])
                .tanh()
        });
        let b: [f64; WIDTH] = std::array::from_fn(|j| {
            (self.weights[88 + j]
                + (0..WIDTH)
                    .map(|k| self.weights[24 + j * WIDTH + k] * a[k])
                    .sum::<f64>())
            .tanh()
        });
        let out = self.weights[104] + (0..WIDTH).map(|j| self.weights[96 + j] * b[j]).sum::<f64>();
        (a, b, out)
    }

    pub fn forward(&self, x: [f64; 2]) -> ([f64; WIDTH], [f64; WIDTH], f64) {
        let (a, b, logit) = self.forward_logits(x);
        (a, b, sigmoid(logit))
    }

    fn gradient(&self) -> ([f64; PARAMETERS], f64, f64) {
        let mut g = [0.0; PARAMETERS];
        let mut loss = 0.0;
        let mut correct = 0;
        for &(x, y) in &self.samples {
            let (a, b, logit) = self.forward_logits(x);
            let probability = sigmoid(logit);
            // Binary cross-entropy directly from logits stays accurate even when
            // the sigmoid rounds to 0 or 1. Its logit derivative is p - y.
            loss += logit.max(0.0) - logit * y + (-logit.abs()).exp().ln_1p();
            correct += usize::from((probability >= 0.5) == (y == 1.0));
            let delta = probability - y;
            g[104] += delta;
            let mut da = [0.0; WIDTH];
            for j in 0..WIDTH {
                g[96 + j] += delta * b[j];
                let db = delta * self.weights[96 + j] * (1.0 - b[j] * b[j]);
                g[88 + j] += db;
                for k in 0..WIDTH {
                    g[24 + j * WIDTH + k] += db * a[k];
                    da[k] += db * self.weights[24 + j * WIDTH + k];
                }
            }
            for j in 0..WIDTH {
                let d = da[j] * (1.0 - a[j] * a[j]);
                g[16 + j] += d;
                g[j * 2] += d * x[0];
                g[j * 2 + 1] += d * x[1];
            }
        }
        (
            g.map(|v| v / SAMPLES as f64),
            loss / SAMPLES as f64,
            correct as f64 / SAMPLES as f64,
        )
    }

    fn measure(&mut self) {
        let (_, loss, accuracy) = self.gradient();
        self.loss = loss;
        self.accuracy = accuracy;
    }

    fn step(&mut self) {
        let (gradient, _, _) = self.gradient();
        self.steps += 1;
        let correction1 = 1.0 - 0.9_f64.powf(self.steps as f64);
        let correction2 = 1.0 - 0.999_f64.powf(self.steps as f64);
        for (i, g) in gradient.into_iter().enumerate() {
            self.first_moment[i] = 0.9 * self.first_moment[i] + 0.1 * g;
            self.second_moment[i] = 0.999 * self.second_moment[i] + 0.001 * g * g;
            self.weights[i] -= self.params[0] * (self.first_moment[i] / correction1)
                / ((self.second_moment[i] / correction2).sqrt() + 1e-8);
        }
    }

    pub fn advance(&mut self, seconds: f64) {
        self.pending += seconds;
        while self.pending + 1e-12 >= STEP_SECONDS {
            self.step();
            self.pending -= STEP_SECONDS;
        }
        self.measure();
    }

    pub fn scene(&self, input: [f64; 2]) -> Scene {
        static GRID_TEMPLATE: OnceLock<SurfaceGrid> = OnceLock::new();
        let mut grid = GRID_TEMPLATE
            .get_or_init(|| {
                SurfaceGrid::from_fn(-1.0..=1.0, -1.0..=1.0, [GRID + 1, GRID + 1], |_, _| 0.0)
            })
            .clone();
        grid.resample(|x, y| [x, y, self.forward([x, y]).2]);
        let scale = ColorScale::new(0.0..=1.0, ColorMap::Viridis);
        let mut mesh = IndexedSurface::from_grid("Prediction: P(inside)", &grid, &scale);
        mesh.bounds = bounds();
        for color in &mut mesh.colors {
            *color = color.gamma_multiply(0.85);
        }
        let mut boundary = Series::new(
            "Decision boundary (50%)",
            grid.contour(0.5).as_slice().to_vec(),
            Color32::from_rgb(245, 65, 145),
        );
        boundary.width = 2.0;
        let mut scene = Scene {
            mesh: Some(mesh),
            lines: vec![boundary],
            ..Default::default()
        };
        // Ground-truth labels are fixed at z=0 or z=1, independently of prediction.
        for label in [0.0, 1.0] {
            let points = self
                .samples
                .iter()
                .filter(|(_, y)| *y == label)
                .map(|&(x, y)| [x[0], x[1], y])
                .collect();
            let mut samples = Series::new(
                if label == 0.0 {
                    "Answer: outside (0)"
                } else {
                    "Answer: inside (1)"
                },
                points,
                label_color(label),
            );
            samples.radius = 2.8;
            scene.dots.push(samples);
        }
        let probability = self.forward(input).2;
        let probe = [input[0], input[1], probability];
        let mut dot = Series::new(
            "Selected input",
            vec![probe],
            Color32::from_rgb(245, 65, 145),
        );
        dot.radius = 6.0;
        scene.dots.push(dot);
        let mut stem = Series::new(
            "",
            vec![[input[0], input[1], 0.0], probe],
            Color32::from_rgb(245, 65, 145),
        );
        stem.width = 1.0;
        scene.lines.push(stem);
        scene
    }

    pub fn step_once(&mut self) {
        self.step();
        self.measure();
    }

    pub fn samples(&self) -> &[([f64; 2], f64)] {
        &self.samples
    }
    pub fn weights(&self) -> &[f64; PARAMETERS] {
        &self.weights
    }
    pub fn answer(&self, input: [f64; 2]) -> bool {
        target(input, self.params[1]) == 1.0
    }
}

pub fn label_color(label: f64) -> Color32 {
    if label == 0.0 {
        Color32::from_rgb(30, 105, 180)
    } else {
        Color32::from_rgb(255, 160, 35)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backprop_matches_finite_differences() {
        let mut model = Model::new([0.02, 0.22, 1.0]);
        check_gradients(&mut model);
        model.advance(5.0);
        check_gradients(&mut model);
    }

    fn check_gradients(model: &mut Model) {
        let (gradient, _, _) = model.gradient();
        for (i, analytic) in gradient.into_iter().enumerate() {
            let original = model.weights[i];
            model.weights[i] = original + 1e-5;
            let plus = model.gradient().1;
            model.weights[i] = original - 1e-5;
            let minus = model.gradient().1;
            model.weights[i] = original;
            assert!(
                (analytic - (plus - minus) / 2e-5).abs() < 1e-7,
                "parameter {i}"
            );
        }
    }
    #[test]
    fn known_output_and_saturated_loss_are_correct() {
        let mut model = Model::new([0.02, 0.22, 1.0]);
        model.weights.fill(0.0);
        model.weights[104] = 3.0_f64.ln();
        let positive_fraction = model.samples.iter().map(|(_, y)| y).sum::<f64>() / SAMPLES as f64;
        for point in [[0.0, 0.0], [1.0, -1.0]] {
            assert!((model.forward(point).2 - 0.75).abs() < 1e-12);
        }
        let (gradient, loss, _) = model.gradient();
        let expected =
            -positive_fraction * 0.75_f64.ln() - (1.0 - positive_fraction) * 0.25_f64.ln();
        assert!((loss - expected).abs() < 1e-12);
        assert!((gradient[104] - (0.75 - positive_fraction)).abs() < 1e-12);
        assert!(gradient[..104].iter().all(|g| *g == 0.0));
        // Confidently wrong predictions must retain their full loss and gradient.
        model.weights[104] = 1000.0;
        let (gradient, loss, _) = model.gradient();
        assert!((loss - 1000.0 * (1.0 - positive_fraction)).abs() < 1e-9);
        assert!((gradient[104] - (1.0 - positive_fraction)).abs() < 1e-12);
    }
    #[test]
    fn training_learns_and_is_frame_rate_independent() {
        let mut a = Model::new([0.02, 0.22, 1.0]);
        let initial_loss = a.loss;
        a.advance(15.0);
        assert!(a.loss < initial_loss * 0.25, "loss {}", a.loss);
        assert!(a.accuracy > 0.93, "accuracy {}", a.accuracy);
        let mut b = Model::new(a.params);
        for _ in 0..150 {
            b.advance(0.1);
        }
        assert_eq!(a.steps, b.steps);
        assert_eq!(a.weights, b.weights);
    }
}
