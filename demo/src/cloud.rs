use eframe::egui::{self, Color32};
use egui_plot3d::{PlotUi, PointBuffer, Points};

pub const COLORS: [Color32; 5] = [
    Color32::from_rgb(89, 198, 214),
    Color32::from_rgb(236, 157, 94),
    Color32::from_rgb(174, 142, 235),
    Color32::from_rgb(130, 201, 144),
    Color32::from_rgb(225, 119, 153),
];
pub const NAMES: [&str; 5] = ["Canopy", "Meadow", "Wetland", "Woodland", "Alpine"];

pub struct Cloud {
    pub groups: Vec<PointBuffer>,
    pub count: usize,
    pub radius: f32,
    pub spread: f64,
    pub colored: bool,
    pub selected: Option<(String, usize, [f64; 3])>,
    seed: u64,
}
impl Default for Cloud {
    fn default() -> Self {
        let mut cloud = Self {
            groups: Vec::new(),
            count: 20_000,
            radius: 1.6,
            spread: 0.65,
            colored: true,
            selected: None,
            seed: 42,
        };
        cloud.regenerate();
        cloud
    }
}
impl Cloud {
    pub fn regenerate(&mut self) {
        let centers = [
            [-2.2, -1.4, 0.2],
            [1.6, -1.5, -0.8],
            [0.5, 2.0, 0.4],
            [-1.8, 1.0, 2.0],
            [1.8, 0.8, 2.6],
        ];
        let mut rng = Rng(self.seed);
        let mut groups = vec![Vec::new(); 5];
        for i in 0..self.count {
            let g = i % 5;
            let a = rng.normal();
            let b = rng.normal();
            let c = rng.normal();
            groups[g].push([
                centers[g][0] + self.spread * (a + 0.3 * b),
                centers[g][1] + self.spread * (b * 0.75 + 0.2 * c),
                centers[g][2] + self.spread * (c * 0.6 + 0.25 * a),
            ]);
        }
        self.groups = groups.into_iter().map(PointBuffer::from).collect();
        self.selected = None;
    }
    pub fn controls(&mut self, ui: &mut egui::Ui) {
        let mut changed = ui
            .add(
                egui::Slider::new(&mut self.count, 100..=100_000)
                    .logarithmic(true)
                    .text("Samples"),
            )
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut self.spread, 0.15..=1.5).text("Spread"))
            .changed();
        ui.add(
            egui::Slider::new(&mut self.radius, 0.8..=6.0)
                .suffix(" px")
                .text("Radius"),
        );
        ui.add_space(8.0);
        ui.checkbox(&mut self.colored, "Color by cluster");
        if ui.button("Regenerate samples").clicked() {
            self.seed = self.seed.wrapping_add(1);
            changed = true;
        }
        if changed {
            self.regenerate();
        }
        if let Some((name, index, p)) = &self.selected {
            ui.separator();
            ui.label(format!("{name} / {index}"));
            ui.monospace(format!("x  {:.4}\ny  {:.4}\nz  {:.4}", p[0], p[1], p[2]));
            if ui.small_button("Clear selection").clicked() {
                self.selected = None;
            }
        }
    }
    pub fn plot(&self, ui: &mut PlotUi<'_>) {
        for (i, group) in self.groups.iter().enumerate() {
            ui.points(
                Points::new(NAMES[i], group)
                    .color(if self.colored {
                        COLORS[i]
                    } else {
                        Color32::from_rgb(155, 185, 200)
                    })
                    .radius(self.radius),
            );
        }
        if let Some((_, _, point)) = self.selected {
            ui.points(
                Points::new("", vec![point])
                    .color(Color32::WHITE)
                    .radius(self.radius + 2.0)
                    .allow_hover(false),
            );
        }
    }
}
struct Rng(u64);
impl Rng {
    fn uniform(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    fn normal(&mut self) -> f64 {
        (-2.0 * self.uniform().ln()).sqrt() * (std::f64::consts::TAU * self.uniform()).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_finite_cloud() {
        let a = Cloud::default();
        let b = Cloud::default();
        assert_eq!(a.groups.iter().map(PointBuffer::len).sum::<usize>(), 20_000);
        assert_eq!(a.groups[0].as_slice(), b.groups[0].as_slice());
        assert!(a.groups.iter().all(|g| g.bounds().is_valid()));
    }
}
