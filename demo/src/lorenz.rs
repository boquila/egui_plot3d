use eframe::egui::{self, Color32};
use egui_plot3d::{Line, PlotBounds, PlotUi, PointBuffer, Points};

const DT: f64 = 0.005;
const START: [f64; 3] = [1.0, 1.0, 1.0];
const NEIGHBOR: [f64; 3] = [1.0001, 1.0, 1.0];

pub struct Lorenz {
    pub sigma: f64,
    pub rho: f64,
    pub beta: f64,
    pub count: usize,
    pub width: f32,
    pub trajectory: PointBuffer,
    pub neighbor: PointBuffer,
    pub show_neighbor: bool,
    drawing: bool,
    running: bool,
    animate_values: bool,
    speed: f64,
    pending: f64,
    trail: Vec<[f64; 3]>,
    neighbor_trail: Vec<[f64; 3]>,
}
impl Default for Lorenz {
    fn default() -> Self {
        let mut s = Self {
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
            count: 16_000,
            width: 1.4,
            trajectory: PointBuffer::from(Vec::new()),
            neighbor: PointBuffer::from(Vec::new()),
            show_neighbor: false,
            drawing: true,
            running: true,
            animate_values: true,
            speed: 1.0,
            pending: 0.0,
            trail: Vec::new(),
            neighbor_trail: Vec::new(),
        };
        s.regenerate();
        s
    }
}
impl Lorenz {
    pub fn regenerate(&mut self) {
        self.trajectory =
            integrate([1.0, 1.0, 1.0], self.count, self.sigma, self.rho, self.beta).into();
        self.neighbor = integrate(
            [1.0001, 1.0, 1.0],
            self.count,
            self.sigma,
            self.rho,
            self.beta,
        )
        .into();
        if self.drawing {
            self.restart();
        }
    }
    pub fn controls(&mut self, ui: &mut egui::Ui) {
        ui.label("Base values");
        if self.drawing && self.animate_values {
            let time = self.trail.len().saturating_sub(1) as f64 * DT;
            let [sigma, rho, beta] = parameters([self.sigma, self.rho, self.beta], time, true);
            ui.weak(format!(
                "Current: sigma {sigma:.2}, rho {rho:.2}, beta {beta:.2}"
            ));
        }
        let mut changed = ui
            .add(egui::Slider::new(&mut self.sigma, 1.0..=25.0).text("sigma"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.rho, 1.0..=60.0).text("rho"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.beta, 0.5..=6.0).text("beta"))
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut self.count, 1_000..=50_000)
                    .logarithmic(true)
                    .text("Samples"),
            )
            .changed();
        ui.add(
            egui::Slider::new(&mut self.width, 0.5..=4.0)
                .suffix(" px")
                .text("Line width"),
        );
        ui.add_space(8.0);
        ui.checkbox(&mut self.show_neighbor, "Nearby initial condition")
            .on_hover_text(
                "Compare x = 1 with x = 1.0001 to see sensitivity to initial conditions.",
            );
        ui.add(
            egui::Slider::new(&mut self.speed, 0.1..=5.0)
                .text("Speed")
                .suffix("×"),
        )
        .on_hover_text("1× = 400 samples per second.");
        if ui.button("Classic parameters").clicked() {
            self.animate_values = false;
            self.sigma = 10.0;
            self.rho = 28.0;
            self.beta = 8.0 / 3.0;
            changed = true;
        }
        if changed {
            self.regenerate();
        }
    }
    pub fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.checkbox(&mut self.drawing, "Draw trajectory").changed() {
                if self.drawing {
                    self.restart();
                } else {
                    self.running = false;
                    self.animate_values = false;
                }
            }
            ui.checkbox(&mut self.show_neighbor, "Nearby initial condition");
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.drawing && self.trail.len() < self.count,
                    egui::Button::new(if self.running { "Pause" } else { "Resume" }),
                )
                .clicked()
            {
                self.running = !self.running;
            }
            if ui.button("Restart drawing").clicked() {
                self.restart();
            }
            if ui
                .checkbox(&mut self.animate_values, "Animate values")
                .changed()
                && self.animate_values
                && (!self.drawing || self.trail.len() == self.count)
            {
                self.restart();
            }
        });
        if self.drawing {
            ui.label(format!(
                "{} / {} samples · t = {:.2}",
                self.trail.len(),
                self.count,
                self.trail.len().saturating_sub(1) as f64 * DT
            ));
        }
    }
    fn restart(&mut self) {
        self.drawing = true;
        self.running = true;
        self.pending = 0.0;
        self.trail.clear();
        self.neighbor_trail.clear();
    }
    pub fn is_animating(&self) -> bool {
        self.drawing && self.running
    }
    pub fn update(&mut self, dt: f32) {
        if !self.is_animating() || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.pending += dt as f64 * 400.0 * self.speed;
        let steps = (self.pending.floor() as usize).min(self.count - self.trail.len());
        self.pending -= steps as f64;
        let base = [self.sigma, self.rho, self.beta];
        for _ in 0..steps {
            let time = self.trail.len().saturating_sub(1) as f64 * DT;
            let advance = |p| {
                rk4(p, DT, |p, offset| {
                    let [sigma, rho, beta] = parameters(base, time + offset, self.animate_values);
                    derivative(p, sigma, rho, beta)
                })
            };
            self.trail
                .push(self.trail.last().copied().map_or(START, advance));
            self.neighbor_trail.push(
                self.neighbor_trail
                    .last()
                    .copied()
                    .map_or(NEIGHBOR, advance),
            );
        }
        if self.trail.len() == self.count {
            self.running = false;
            self.pending = 0.0;
        }
    }
    pub fn bounds(&self) -> PlotBounds {
        let mut bounds = self.trajectory.bounds();
        bounds.merge(self.neighbor.bounds());
        // Reserve room for the changing system before any samples are drawn.
        // This envelope stays fixed while drawing, pausing, and toggling modulation.
        let center = bounds.center();
        let size = bounds.size();
        PlotBounds::from_min_max(
            std::array::from_fn(|i| center[i] - size[i] * 0.7),
            std::array::from_fn(|i| center[i] + size[i] * 0.7),
        )
    }
    pub fn plot<'a>(&'a self, ui: &mut PlotUi<'a>) {
        let (trajectory, neighbor) = if self.drawing {
            (self.trail.as_slice(), self.neighbor_trail.as_slice())
        } else {
            (self.trajectory.as_slice(), self.neighbor.as_slice())
        };
        ui.line(
            Line::new("Lorenz trajectory", trajectory)
                .color(Color32::from_rgb(89, 198, 214))
                .width(self.width),
        );
        if self.show_neighbor {
            ui.line(
                Line::new("x + 0.0001", neighbor)
                    .color(Color32::from_rgb(236, 157, 94))
                    .width(self.width),
            );
        }
        if self.drawing
            && let Some(&p) = trajectory.last()
        {
            ui.points(
                Points::new("Drawing head", p)
                    .radius(6.0)
                    .color(Color32::from_rgb(240, 115, 35)),
            );
            if self.show_neighbor
                && let Some(&p) = neighbor.last()
            {
                ui.points(
                    Points::new("Nearby head", p)
                        .radius(5.0)
                        .color(Color32::from_rgb(170, 85, 200)),
                );
            }
        }
    }
}
fn parameters(base: [f64; 3], time: f64, animate: bool) -> [f64; 3] {
    if !animate {
        return base;
    }
    [
        base[0] * (1.0 + 0.20 * (time * 0.61).sin()),
        base[1] * (1.0 + 0.22 * (time * 0.37).sin()),
        base[2] * (1.0 + 0.12 * (time * 0.47).sin()),
    ]
}
fn derivative(p: [f64; 3], sigma: f64, rho: f64, beta: f64) -> [f64; 3] {
    [
        sigma * (p[1] - p[0]),
        p[0] * (rho - p[2]) - p[1],
        p[0] * p[1] - beta * p[2],
    ]
}
fn step(p: [f64; 3], dt: f64, sigma: f64, rho: f64, beta: f64) -> [f64; 3] {
    rk4(p, dt, |p, _| derivative(p, sigma, rho, beta))
}
fn rk4(p: [f64; 3], dt: f64, derivative: impl Fn([f64; 3], f64) -> [f64; 3]) -> [f64; 3] {
    let k1 = derivative(p, 0.0);
    let k2 = derivative(std::array::from_fn(|i| p[i] + k1[i] * dt * 0.5), dt * 0.5);
    let k3 = derivative(std::array::from_fn(|i| p[i] + k2[i] * dt * 0.5), dt * 0.5);
    let k4 = derivative(std::array::from_fn(|i| p[i] + k3[i] * dt), dt);
    std::array::from_fn(|i| p[i] + dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]))
}
fn integrate(mut p: [f64; 3], count: usize, sigma: f64, rho: f64, beta: f64) -> Vec<[f64; 3]> {
    let mut points = Vec::with_capacity(count);
    for _ in 0..count {
        points.push(p);
        p = step(p, 0.005, sigma, rho, beta);
    }
    points
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drawing_starts_empty_grows_pauses_and_finishes() {
        let mut l = Lorenz {
            animate_values: false,
            ..Default::default()
        };
        let bounds = l.bounds();
        l.restart();
        assert!(l.trail.is_empty());
        egui::__run_test_ui(|ui| {
            let response = egui_plot3d::Plot::new("empty draw")
                .default_bounds(bounds)
                .show(ui, |p| l.plot(p));
            assert_eq!(response.rendered_primitives, 0);
        });
        l.update(1.0);
        assert_eq!(l.trail.len(), 400);
        assert_eq!(l.trail, l.trajectory.as_slice()[..400]);
        assert_eq!(l.neighbor_trail, l.neighbor.as_slice()[..400]);
        assert_eq!(l.bounds(), bounds);
        l.running = false;
        let saved = l.trail.clone();
        l.update(1.0);
        assert_eq!(l.trail, saved);
        l.running = true;
        l.update(100.0);
        assert_eq!(l.trail.len(), l.count);
        assert!(!l.is_animating());
        assert_eq!(l.trail, l.trajectory.as_slice());
        l.restart();
        assert!(l.trail.is_empty() && l.neighbor_trail.is_empty());
        assert!(l.is_animating());
    }
    #[test]
    fn driven_system_changes_values_preserves_history_and_is_frame_rate_independent() {
        let mut a = Lorenz::default();
        let mut b = Lorenz::default();
        assert!(a.animate_values && a.is_animating());
        a.restart();
        b.restart();
        a.update(2.0);
        for _ in 0..16 {
            b.update(0.125);
        }
        assert_eq!(a.trail, b.trail);
        assert_ne!(a.trail, a.trajectory.as_slice()[..a.trail.len()]);
        let saved = a.trail.clone();
        a.update(2.0);
        assert_eq!(a.trail[..saved.len()], saved);
        assert!(a.trail.iter().flatten().all(|v| v.is_finite()));
    }
    #[test]
    fn classic_is_bounded() {
        let l = Lorenz::default();
        assert_eq!(l.trajectory.len(), 16_000);
        for p in l.trajectory.as_slice() {
            assert!(p.iter().all(|v| v.is_finite() && v.abs() < 100.0));
        }
        assert_ne!(
            l.trajectory.as_slice()[10_000],
            l.neighbor.as_slice()[10_000]
        );
    }
    #[test]
    fn rk4_converges() {
        let p = [1.0, 1.0, 1.0];
        let a = step(p, 0.01, 10.0, 28.0, 8.0 / 3.0);
        let b = step(
            step(p, 0.005, 10.0, 28.0, 8.0 / 3.0),
            0.005,
            10.0,
            28.0,
            8.0 / 3.0,
        );
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-5));
    }
}
