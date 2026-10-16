//! Shared gallery plumbing; each model and its explanation live in their own module.
use eframe::egui::{self, Color32};
use egui_plot3d::{Arrows, ColorScale, Line, PlotBounds, PlotUi, PointBuffer, Points, SurfaceGrid};
use std::sync::Arc;

pub const BLUE: Color32 = Color32::from_rgb(70, 160, 215);
pub const ORANGE: Color32 = Color32::from_rgb(232, 145, 65);
pub const GREEN: Color32 = Color32::from_rgb(80, 185, 140);

pub struct IndexedSurface {
    pub name: &'static str,
    pub legend_color: Color32,
    pub vertices: PointBuffer,
    pub indices: Arc<[[u32; 3]]>,
    pub colors: Vec<Color32>,
    pub shading: bool,
    pub bounds: PlotBounds,
}
impl IndexedSurface {
    pub fn from_grid(name: &'static str, grid: &SurfaceGrid, scale: &ColorScale) -> Self {
        Self {
            name,
            legend_color: scale.color(scale.value_at(0.65).unwrap_or(0.0)),
            vertices: grid.vertices().clone(),
            indices: grid.indices().clone(),
            colors: grid.colors(scale),
            shading: false,
            bounds: grid.vertices().bounds(),
        }
    }
}

pub struct Series {
    pub name: &'static str,
    pub data: PointBuffer,
    pub color: Color32,
    pub width: f32,
    pub radius: f32,
}
impl Series {
    pub fn new(name: &'static str, data: Vec<[f64; 3]>, color: Color32) -> Self {
        Self {
            name,
            data: data.into(),
            color,
            width: 1.7,
            radius: 4.0,
        }
    }
}
#[derive(Default)]
pub struct Scene {
    pub mesh: Option<IndexedSurface>,
    pub lines: Vec<Series>,
    pub dots: Vec<Series>,
    pub arrows: Option<(PointBuffer, PointBuffer)>,
}
impl Scene {
    pub fn bounds(&self) -> PlotBounds {
        let mut bounds = PlotBounds::NOTHING;
        for s in self.lines.iter().chain(&self.dots) {
            bounds.merge(s.data.bounds());
        }
        if let Some(mesh) = &self.mesh {
            bounds.merge(mesh.bounds);
        }
        bounds
    }
    pub fn plot<'a>(&'a self, ui: &mut PlotUi<'a>) {
        if let Some(mesh) = &self.mesh {
            ui.surface(
                egui_plot3d::Surface::new(mesh.name, &mesh.vertices, mesh.indices.as_ref())
                    .color(mesh.legend_color)
                    .colors(&mesh.colors)
                    .shading(mesh.shading),
            );
        }
        for (i, s) in self.lines.iter().enumerate() {
            ui.line(
                Line::new(s.name, &s.data)
                    .id(("line", i))
                    .color(s.color)
                    .width(s.width),
            );
        }
        for (i, s) in self.dots.iter().enumerate() {
            ui.points(
                Points::new(s.name, &s.data)
                    .id(("dots", i))
                    .color(s.color)
                    .radius(s.radius),
            );
        }
        if let Some((origins, tips)) = &self.arrows {
            ui.arrows(Arrows::new("Field direction", origins, tips).color(ORANGE));
        }
    }
    fn preview(
        &self,
        rect: egui::Rect,
        aspect: [f64; 3],
        camera: egui_plot3d::Camera,
    ) -> Vec<egui::Shape> {
        let mut shapes = Vec::new();
        let t = egui_plot3d::PlotTransform::new(rect, self.bounds(), camera, true)
            .with_axis_aspect(aspect);
        if let Some(surface) = &self.mesh {
            let projected: Vec<_> = surface
                .vertices
                .as_slice()
                .iter()
                .map(|&p| t.project(p.into()))
                .collect();
            let mut faces: Vec<_> = surface
                .indices
                .iter()
                .enumerate()
                .filter_map(|(i, indices)| {
                    let [Some(a), Some(b), Some(c)] = indices.map(|i| projected[i as usize]) else {
                        return None;
                    };
                    Some(([a.0, b.0, c.0], surface.colors[i], a.1 + b.1 + c.1))
                })
                .collect();
            faces.sort_unstable_by(|a, b| b.2.total_cmp(&a.2));
            let mut mesh = egui::Mesh::default();
            for (points, color, _) in faces {
                let base = mesh.vertices.len() as u32;
                for p in points {
                    mesh.colored_vertex(p, color);
                }
                mesh.add_triangle(base, base + 1, base + 2);
            }
            shapes.push(egui::Shape::mesh(mesh));
        }
        for s in &self.lines {
            for path in s
                .data
                .as_slice()
                .split(|p| p.iter().any(|v| !v.is_finite()))
            {
                let points = path
                    .iter()
                    .step_by((path.len() / 100).max(1))
                    .filter_map(|&p| t.position_from_point(p))
                    .collect();
                shapes.push(egui::Shape::line(points, egui::Stroke::new(0.8, s.color)));
            }
        }
        for s in &self.dots {
            for &p in s.data.as_slice() {
                if let Some(pos) = t.position_from_point(p) {
                    shapes.push(egui::Shape::circle_filled(pos, 1.7, s.color));
                }
            }
        }
        shapes
    }
}

pub struct Example {
    pub title: &'static str,
    pub name: &'static str,
    pub tags: &'static [&'static str],
    pub source: &'static str,
    pub description: &'static str,
    pub reference: &'static str,
}
macro_rules! example {
    ($module:ident, $title:literal, $tags:expr, $reference:literal) => {
        Example {
            title: $title,
            name: stringify!($module),
            tags: $tags,
            source: include_str!(concat!(stringify!($module), ".rs")),
            description: crate::$module::DESCRIPTION,
            reference: $reference,
        }
    };
}
pub struct Science {
    pub kind: usize,
    pub params: [f64; 3],
    pub animate: bool,
    pub animate_values: bool,
    value_time: f64,
    motion_speed: f64,
    motion_strength: f64,
    time: f64,
    pub scene: Scene,
    preview: Option<(egui::Rect, Vec<egui::Shape>)>,
    reaction: Option<crate::spirals::Model>,
    neural: Option<crate::neural::Model>,
    neural_input: [f64; 2],
}
impl Science {
    pub const EXAMPLES: [Example; 9] = [
        example!(
            waves,
            "Wave interference",
            &["surface", "waves"],
            "https://openstax.org/books/university-physics-volume-1/pages/16-5-interference-of-waves"
        ),
        example!(
            kepler,
            "Kepler orbits",
            &["orbit", "astronomy"],
            "https://science.nasa.gov/solar-system/orbits-and-keplers-laws/"
        ),
        example!(
            gaussian,
            "Correlated Gaussian",
            &["surface", "statistics"],
            "https://mathworld.wolfram.com/BivariateNormalDistribution.html"
        ),
        example!(
            harmonics,
            "Harmonic motion",
            &["lines", "frequencies"],
            "https://mathworld.wolfram.com/LissajousCurve.html"
        ),
        example!(
            dipole,
            "Magnetic dipole",
            &["vectors", "magnetism"],
            "https://farside.ph.utexas.edu/teaching/jk1/Electromagnetism/node52.html"
        ),
        example!(
            spherical,
            "Spherical harmonics",
            &["surface", "modes"],
            "https://dlmf.nist.gov/14.30"
        ),
        example!(
            spirals,
            "Spiral waves",
            &["surface", "excitable media"],
            "https://www.scholarpedia.org/article/Barkley_model"
        ),
        example!(
            knots,
            "Torus knots",
            &["surface", "topology"],
            "https://mathworld.wolfram.com/TorusKnot.html"
        ),
        example!(
            neural,
            "Tiny neural network",
            &["network", "learning"],
            "https://arxiv.org/abs/1412.6980"
        ),
    ];
    fn defaults(kind: usize) -> [f64; 3] {
        match kind {
            0 => [0.0, 1.5, 2.4],
            1 => [0.65, 30.0, 0.0],
            2 => [0.65, 1.0, 0.0],
            3 => [2.0, 3.0, 5.0],
            4 => [20.0, 8.0, 0.0],
            5 => [4.0, 3.0, 0.85],
            6 => [0.75, 0.02, 0.02],
            7 => [2.0, 3.0, 0.20],
            _ => [0.02, 0.22, 1.0],
        }
    }
    pub fn new(kind: usize) -> Self {
        let mut s = Self {
            kind,
            params: Self::defaults(kind),
            animate: false,
            animate_values: matches!(kind, 0 | 3..=7),
            value_time: 0.0,
            motion_speed: 1.0,
            motion_strength: 1.0,
            time: 0.0,
            scene: Scene::default(),
            preview: None,
            reaction: None,
            neural: None,
            neural_input: [0.3, 0.2],
        };
        s.rebuild();
        s
    }
    pub fn aspect(&self) -> [f64; 3] {
        match self.kind {
            2 => [1.0, 1.0, 8.0],
            6 => [1.0, 1.0, 2.4],
            _ => [1.0; 3],
        }
    }
    pub fn camera(kind: usize) -> egui_plot3d::Camera {
        egui_plot3d::Camera {
            pitch: if kind == 7 {
                0.9
            } else if kind == 1 {
                1.25
            } else if matches!(kind, 0 | 2 | 6 | 8) {
                0.75
            } else {
                0.45
            },
            zoom: if kind == 8 {
                1.05
            } else if matches!(kind, 5 | 7) {
                1.15
            } else {
                1.0
            },
            ..Default::default()
        }
    }
    pub fn preview(&mut self, painter: &egui::Painter, rect: egui::Rect) {
        if self.preview.as_ref().is_none_or(|(r, _)| *r != rect) {
            self.preview = Some((
                rect,
                self.scene
                    .preview(rect, self.aspect(), Self::camera(self.kind)),
            ));
        }
        painter.extend(self.preview.as_ref().unwrap().1.iter().cloned());
    }
    pub fn axes(&self) -> [&'static str; 3] {
        match self.kind {
            0 => ["x", "y", "amplitude"],
            2 => ["x", "y", "density"],
            6 => ["x", "y", "activation"],
            8 => ["input x", "input y", "P(inside)"],
            _ => ["x", "y", "z"],
        }
    }
    pub fn controls(&mut self, ui: &mut egui::Ui) {
        let labels = match self.kind {
            0 => [
                ("Phase / pi", 0.0..=2.0),
                ("Wavelength", 0.6..=3.0),
                ("Source separation", 0.5..=4.0),
            ],
            1 => [
                ("Eccentricity", 0.0..=0.85),
                ("Inclination (°)", 0.0..=75.0),
                ("Mean anomaly / pi", 0.0..=2.0),
            ],
            2 => [
                ("Correlation", -0.9..=0.9),
                ("Y standard deviation", 0.5..=1.5),
                ("Unused", 0.0..=1.0),
            ],
            3 => [
                ("X frequency", 1.0..=8.0),
                ("Y frequency", 1.0..=8.0),
                ("Z frequency", 1.0..=8.0),
            ],
            4 => [
                ("Tilt (°)", 0.0..=90.0),
                ("Meridian planes", 3.0..=12.0),
                ("Unused", 0.0..=1.0),
            ],
            5 => [
                ("Degree l", 1.0..=7.0),
                ("Order m", 0.0..=self.params[0]),
                ("Amplitude", 0.1..=1.0),
            ],
            6 => [
                ("Recovery a", 0.65..=0.9),
                ("Threshold b", 0.01..=0.07),
                ("Epsilon", 0.02..=0.08),
            ],
            7 => [
                ("Toroidal turns", 1.0..=7.0),
                ("Poloidal turns", 1.0..=7.0),
                ("Tube radius", 0.08..=0.24),
            ],
            _ => [
                ("Learning rate", 0.003..=0.04),
                ("Ring width", 0.10..=0.35),
                ("Seed", 1.0..=20.0),
            ],
        };
        let mut changed = false;
        for (i, (label, range)) in
            labels
                .into_iter()
                .enumerate()
                .take(if self.kind == 2 || self.kind == 4 {
                    2
                } else {
                    3
                })
        {
            let slider = egui::Slider::new(&mut self.params[i], range).text(label);
            changed |= ui
                .add(
                    if self.kind == 3
                        || (self.kind == 4 && i == 1)
                        || (matches!(self.kind, 5 | 7) && i < 2)
                        || (self.kind == 8 && i == 2)
                    {
                        slider.integer()
                    } else {
                        slider
                    },
                )
                .changed();
        }
        if self.kind == 5 {
            self.params[1] = self.params[1].min(self.params[0]);
        }
        if ui.button("Restore defaults").clicked() {
            self.params = Self::defaults(self.kind);
            self.time = 0.0;
            self.value_time = 0.0;
            self.animate = false;
            self.animate_values = self.has_value_animation();
            self.motion_speed = 1.0;
            self.motion_strength = 1.0;
            self.reaction = None;
            self.neural = None;
            changed = true;
        }
        if self.has_value_animation() {
            ui.separator();
            ui.strong("Value animation");
            ui.add(
                egui::Slider::new(&mut self.motion_speed, 0.1..=3.0)
                    .text("Speed")
                    .suffix("x"),
            );
            changed |= ui
                .add(egui::Slider::new(&mut self.motion_strength, 0.0..=1.0).text("Intensity"))
                .changed();
        }
        if self.kind == 8 {
            ui.add(
                egui::Slider::new(&mut self.motion_speed, 0.1..=3.0)
                    .text("Training speed")
                    .suffix("x"),
            );
        }
        if changed {
            self.rebuild();
        }
    }
    pub fn toolbar(&mut self, ui: &mut egui::Ui) {
        if self.kind == 8 {
            ui.strong("Learn which points are inside the ring");
            return;
        }
        if ui.button("Compare setting").clicked() {
            let alternatives = [1.0, 0.0, -0.65, 4.0, 75.0, 6.0, 0.85, 3.0, 5.0];
            let parameter = if self.kind == 8 { 1 } else { 0 };
            let d = Self::defaults(self.kind)[parameter];
            self.params[parameter] = if (self.params[parameter] - d).abs() < 1e-6 {
                alternatives[self.kind]
            } else {
                d
            };
            self.rebuild();
        }
        if matches!(self.kind, 0 | 1 | 3) {
            ui.checkbox(
                &mut self.animate,
                if self.kind == 0 {
                    "Travel waves"
                } else {
                    "Move particle"
                },
            );
        }
        ui.label(match self.kind {
            0 => format!("Phase: {:.2} π", self.params[0]),
            1 => format!("e = {:.2}", self.params[0]),
            2 => format!("Correlation: {:.2} · Z ×8", self.params[0]),
            3 => format!(
                "{:.0}:{:.0}:{:.0}",
                self.params[0], self.params[1], self.params[2]
            ),
            4 => format!("Tilt: {:.0}°", self.params[0]),
            5 => format!("l = {:.0}, m = {:.0}", self.params[0], self.params[1]),
            6 => "Barkley model · Z ×2.4".to_owned(),
            7 => format!("({:.0}, {:.0})", self.params[0], self.params[1]),
            _ => "2–8–8–1".to_owned(),
        });
    }
    fn has_value_animation(&self) -> bool {
        matches!(self.kind, 0 | 3..=7)
    }
    fn motion_description(&self) -> &'static str {
        match self.kind {
            0 => "Vary phase, wavelength, and source separation. Sliders set the base values.",
            3 => "Vary phases, amplitudes, and higher harmonics while keeping the curve closed.",
            4 => "Rotate the dipole and vary the sampled field-line radii.",
            5 => "Oscillate a spherical harmonic mode.",
            6 => "Evolve activation and recovery; intensity controls the height.",
            7 => "Send deformation waves along the tubes.",
            _ => "Train with backpropagation and Adam; show live activations for the pink probe.",
        }
    }
    pub fn motion_toolbar(&mut self, ui: &mut egui::Ui) {
        if self.kind == 8 {
            self.neural_toolbar(ui);
            return;
        }
        if !self.has_value_animation() {
            return;
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .checkbox(&mut self.animate_values, "Animate values")
                .on_hover_text(self.motion_description())
                .changed()
            {
                self.preview = None;
            }
            if ui.button("Reset motion").clicked() {
                self.reset_motion();
            }
        });
    }

    fn neural_toolbar(&mut self, ui: &mut egui::Ui) {
        let steps = self.neural.as_ref().map_or(0, |m| m.steps);
        ui.horizontal_wrapped(|ui| {
            let label = if self.animate_values {
                "Pause training"
            } else if steps == 0 {
                "Start training"
            } else {
                "Resume training"
            };
            if ui.button(label).clicked() {
                self.animate_values = !self.animate_values;
            }
            if ui
                .add_enabled(!self.animate_values, egui::Button::new("One update"))
                .clicked()
            {
                self.neural.as_mut().unwrap().step_once();
                self.rebuild();
            }
            if ui.button("Reset training").clicked() {
                self.reset_motion();
            }
        });
        if let Some(model) = &self.neural {
            ui.label(if model.steps == 0 {
                "Ready: random weights. Press Start training.".to_owned()
            } else {
                format!(
                    "{} · {} updates",
                    if self.animate_values {
                        "Training"
                    } else {
                        "Paused"
                    },
                    model.steps
                )
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Training accuracy: {:.0}%", model.accuracy * 100.0))
                    .on_hover_text("Correct classifications among the 256 training examples. This is not a test-set score.");
                ui.label(format!("Loss: {:.3}", model.loss)).on_hover_text("Binary cross-entropy: how wrong the predictions are. Lower is better.");
            });
        }
        ui.label("Dots = fixed answers (0 or 1). Surface height = predicted probability.");
        ui.label("Training moves the surface toward the dots. Click a dot to inspect its input.");
        if ui.ctx().content_rect().width() < 1024.0 {
            ui.collapsing("See examples & network", |ui| {
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| self.neural_inspector(ui));
            });
        }
    }

    pub fn neural_inspector(&mut self, ui: &mut egui::Ui) {
        if let Some(model) = &self.neural
            && crate::neural_view::inspector(ui, model, &mut self.neural_input)
        {
            self.rebuild();
        }
    }

    pub fn select_neural_input(&mut self, point: [f64; 3]) {
        self.neural_input = [point[0].clamp(-1.0, 1.0), point[1].clamp(-1.0, 1.0)];
        self.rebuild();
    }
    pub fn animation_bounds(&self) -> Option<PlotBounds> {
        match self.kind {
            0 => Some(PlotBounds::from_min_max(
                [-4.0, -4.0, -2.0],
                [4.0, 4.0, 2.0],
            )),
            3 => Some(PlotBounds::from_min_max([-1.6; 3], [1.6; 3])),
            4 => Some(PlotBounds::from_min_max([-4.4; 3], [4.4; 3])),
            5 => Some(PlotBounds::from_min_max([-3.2; 3], [3.2; 3])),
            6 => Some(PlotBounds::from_min_max(
                [-5.0, -5.0, 0.0],
                [5.0, 5.0, 1.05],
            )),
            7 => Some(PlotBounds::from_min_max(
                [-3.6, -3.6, -1.7],
                [3.6, 3.6, 1.7],
            )),
            8 => Some(crate::neural::bounds()),
            _ => None,
        }
    }
    fn reset_motion(&mut self) {
        self.animate_values = false;
        self.value_time = 0.0;
        self.reaction = None;
        self.neural = None;
        self.rebuild();
    }
    pub fn update(&mut self, dt: f32) {
        if self.animate {
            self.time = (self.time + dt as f64).rem_euclid(std::f64::consts::TAU * 100.0);
        }
        if self.animate_values {
            self.value_time += dt as f64 * self.motion_speed;
            if let Some(model) = &mut self.reaction {
                model.advance(dt as f64 * self.motion_speed);
            }
            if let Some(model) = &mut self.neural {
                model.advance(dt as f64 * self.motion_speed);
            }
        }
        if self.animate_values || (self.kind == 0 && self.animate) {
            // Keep cached thumbnails while playing; regenerate the main data only.
            self.rebuild_scene();
        }
    }
    pub fn plot<'a>(&'a self, ui: &mut PlotUi<'a>) {
        self.scene.plot(ui);
        let point = match self.kind {
            1 => Some(crate::kepler::position(
                self.params[2] * std::f64::consts::PI + self.time * 0.6,
                self.params[0],
                self.params[1].to_radians(),
            )),
            3 if self.animate => Some(crate::harmonics::animated_position(
                self.time,
                self.params,
                self.value_time,
                self.motion_strength,
            )),
            _ => None,
        };
        if let Some(p) = point {
            ui.points(
                Points::new("Moving sample", vec![p])
                    .color(ORANGE)
                    .radius(6.0),
            );
        }
    }
    fn rebuild(&mut self) {
        self.preview = None;
        self.rebuild_scene();
    }
    fn rebuild_scene(&mut self) {
        self.scene = match self.kind {
            0 => crate::waves::build(
                crate::waves::animated_parameters(
                    self.params,
                    self.value_time,
                    self.motion_strength,
                ),
                self.time + self.value_time * self.motion_strength,
            ),
            1 => crate::kepler::build(self.params),
            2 => crate::gaussian::build(self.params),
            3 if self.value_time == 0.0 => crate::harmonics::build(self.params),
            3 => {
                crate::harmonics::build_animated(self.params, self.value_time, self.motion_strength)
            }
            4 if self.value_time == 0.0 => crate::dipole::build(self.params),
            4 => crate::dipole::build_animated(self.params, self.value_time, self.motion_strength),
            5 => crate::spherical::build(self.params, self.value_time, self.motion_strength),
            6 => {
                if self
                    .reaction
                    .as_ref()
                    .is_none_or(|m| m.params != self.params)
                {
                    self.reaction = Some(crate::spirals::Model::new(self.params));
                }
                self.reaction.as_ref().unwrap().scene(self.motion_strength)
            }
            7 => crate::knots::build(self.params, self.value_time, self.motion_strength),
            _ => {
                if self
                    .neural
                    .as_ref()
                    .is_none_or(|m| m.params[1..] != self.params[1..])
                {
                    self.animate_values = false;
                    self.neural = Some(crate::neural::Model::new(self.params));
                }
                let model = self.neural.as_mut().unwrap();
                model.params[0] = self.params[0];
                model.scene(self.neural_input)
            }
        };
    }
}

#[cfg(test)]
mod animation_tests {
    use super::*;
    fn vertices(scene: &Scene) -> Vec<[f64; 3]> {
        let mut result: Vec<[f64; 3]> = Vec::new();
        if let Some(mesh) = &scene.mesh {
            result.extend(mesh.vertices.as_slice());
        }
        for series in &scene.lines {
            result.extend(series.data.as_slice());
        }
        result
            .into_iter()
            .filter(|p| p.iter().all(|x| x.is_finite()))
            .collect()
    }
    #[test]
    fn value_animation_moves_geometry_then_freezes_and_resets() {
        for kind in [0, 3, 4, 5, 7] {
            let mut model = Science::new(kind);
            let initial = vertices(&model.scene);
            assert!(model.animate_values);
            for _ in 0..90 {
                model.update(1.0 / 60.0);
            }
            let moving = vertices(&model.scene);
            assert_eq!(initial.len(), moving.len());
            let changed = initial
                .iter()
                .zip(&moving)
                .filter(|(a, b)| (0..3).any(|i| (a[i] - b[i]).abs() > 0.01))
                .count();
            assert!(
                changed > initial.len() / 2,
                "model {kind} moved too few samples: {changed}"
            );
            model.animate_values = false;
            model.update(1.0);
            assert_eq!(
                moving,
                vertices(&model.scene),
                "pausing must freeze the current shape"
            );
            if let Some(bounds) = model.animation_bounds() {
                for p in moving {
                    for (i, v) in p.into_iter().enumerate() {
                        assert!(v >= bounds.min()[i] && v <= bounds.max()[i]);
                    }
                }
            }
            model.reset_motion();
            assert_eq!(initial, vertices(&model.scene));
        }
    }
    #[test]
    fn neural_training_is_explicit_and_inspection_never_trains() {
        let mut example = Science::new(8);
        let initial = vertices(&example.scene);
        let truth: Vec<_> = example.scene.dots[..2]
            .iter()
            .map(|s| s.data.as_slice().to_vec())
            .collect();
        assert!(!example.animate_values);
        example.update(1.0);
        assert_eq!(example.neural.as_ref().unwrap().steps, 0);
        assert_eq!(vertices(&example.scene), initial);
        example.animate_values = true;
        example.update(1.0);
        let steps = example.neural.as_ref().unwrap().steps;
        assert_eq!(steps, 60);
        assert_ne!(vertices(&example.scene), initial);
        for (series, expected) in example.scene.dots[..2].iter().zip(&truth) {
            assert_eq!(series.data.as_slice(), expected);
        }
        example.animate_values = false;
        example.update(1.0);
        example.select_neural_input([-0.7, 0.4, 0.0]);
        assert_eq!(example.neural.as_ref().unwrap().steps, steps);
        example.neural.as_mut().unwrap().step_once();
        assert_eq!(example.neural.as_ref().unwrap().steps, steps + 1);
        // Learning-rate edits preserve learned weights; dataset edits reset and pause.
        example.params[0] = 0.01;
        example.rebuild();
        assert_eq!(example.neural.as_ref().unwrap().steps, steps + 1);
        example.animate_values = true;
        example.params[1] = 0.30;
        example.rebuild();
        assert_eq!(example.neural.as_ref().unwrap().steps, 0);
        assert!(!example.animate_values);
        example.params = Science::defaults(8);
        example.neural_input = [0.3, 0.2];
        example.reset_motion();
        assert_eq!(vertices(&example.scene), initial);
    }
    #[test]
    fn spiral_animation_is_live_and_resettable() {
        let mut model = Science::new(6);
        let initial = vertices(&model.scene);
        for _ in 0..120 {
            model.update(1.0 / 60.0);
        }
        let moving = vertices(&model.scene);
        assert!(
            initial
                .iter()
                .zip(&moving)
                .filter(|(a, b)| (a[2] - b[2]).abs() > 0.05)
                .count()
                > 100
        );
        model.animate_values = false;
        model.update(1.0);
        assert_eq!(moving, vertices(&model.scene));
        model.reset_motion();
        assert_eq!(initial, vertices(&model.scene));
    }
    #[test]
    fn morphing_harmonics_remain_closed() {
        for time in [0.0, 0.7, 3.0, 20.0] {
            for f in [[2.0, 3.0, 5.0], [8.0, 1.0, 7.0]] {
                let a = crate::harmonics::animated_position(0.0, f, time, 1.0);
                let b = crate::harmonics::animated_position(std::f64::consts::TAU, f, time, 1.0);
                for i in 0..3 {
                    assert!((a[i] - b[i]).abs() < 1e-12);
                }
            }
        }
    }
}
