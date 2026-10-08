use eframe::egui::{self, Color32};
use egui_plot3d::{Line, PlotUi, PointBuffer, Points};

pub struct Lorenz {
    pub sigma: f64,
    pub rho: f64,
    pub beta: f64,
    pub count: usize,
    pub width: f32,
    pub trajectory: PointBuffer,
    pub neighbor: PointBuffer,
    pub show_neighbor: bool,
    pub animate: bool,
    pub head: f64,
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
            animate: false,
            head: 0.0,
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
        self.head = 0.0;
    }
    pub fn controls(&mut self, ui: &mut egui::Ui) {
        ui.label("System parameters");
        let mut changed = ui
            .add(egui::Slider::new(&mut self.sigma, 1.0..=25.0).text("sigma"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.rho, 1.0..=60.0).text("rho"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.beta, 0.5..=6.0).text("beta"))
            .changed();
        ui.add_space(10.0);
        ui.label("Trajectory samples");
        changed |= ui
            .add(egui::Slider::new(&mut self.count, 1_000..=50_000).logarithmic(true))
            .changed();
        ui.add_space(10.0);
        ui.label("Line width");
        ui.add(egui::Slider::new(&mut self.width, 0.5..=4.0).suffix(" px"));
        ui.add_space(8.0);
        ui.checkbox(&mut self.show_neighbor, "Nearby initial condition")
            .on_hover_text(
                "Compare x = 1 with x = 1.0001 to see sensitivity to initial conditions.",
            );
        ui.checkbox(&mut self.animate, "Animate particle");
        if ui.button("Restore classic parameters").clicked() {
            self.sigma = 10.0;
            self.rho = 28.0;
            self.beta = 8.0 / 3.0;
            changed = true;
        }
        if changed {
            self.regenerate();
        }
        ui.add_space(20.0);
        ui.strong("The Lorenz system");
        ui.monospace("dx/dt = sigma (y - x)\ndy/dt = x (rho - z) - y\ndz/dt = x y - beta z");
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Fourth-order Runge-Kutta\ndt = 0.005").weak());
    }
    pub fn plot(&mut self, ui: &mut PlotUi<'_>, dt: f32) {
        ui.line(
            Line::new("Lorenz trajectory", &self.trajectory)
                .color(Color32::from_rgb(89, 198, 214))
                .width(self.width),
        );
        if self.show_neighbor {
            ui.line(
                Line::new("x + 0.0001", &self.neighbor)
                    .color(Color32::from_rgb(236, 157, 94))
                    .width(self.width),
            );
        }
        if self.animate && !self.trajectory.is_empty() {
            self.head = (self.head + dt as f64 * 400.0) % self.trajectory.len() as f64;
            let p = self.trajectory.as_slice()[self.head as usize];
            ui.points(
                Points::new("Particle", vec![p])
                    .radius(5.0)
                    .color(Color32::WHITE),
            );
        }
    }
}
fn derivative(p: [f64; 3], sigma: f64, rho: f64, beta: f64) -> [f64; 3] {
    [
        sigma * (p[1] - p[0]),
        p[0] * (rho - p[2]) - p[1],
        p[0] * p[1] - beta * p[2],
    ]
}
fn step(p: [f64; 3], dt: f64, sigma: f64, rho: f64, beta: f64) -> [f64; 3] {
    let k1 = derivative(p, sigma, rho, beta);
    let k2 = derivative(
        std::array::from_fn(|i| p[i] + k1[i] * dt * 0.5),
        sigma,
        rho,
        beta,
    );
    let k3 = derivative(
        std::array::from_fn(|i| p[i] + k2[i] * dt * 0.5),
        sigma,
        rho,
        beta,
    );
    let k4 = derivative(std::array::from_fn(|i| p[i] + k3[i] * dt), sigma, rho, beta);
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
