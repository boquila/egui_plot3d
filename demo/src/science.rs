//! Shared gallery plumbing; each model and its explanation live in their own module.
use eframe::egui::{self, Color32};
use egui_plot3d::{
    Arrows, Line, PlotBounds, PlotItem, PlotItemContext, PlotUi, PointBuffer, Points,
};
use std::sync::Arc;

pub const BLUE: Color32 = Color32::from_rgb(70, 160, 215);
pub const ORANGE: Color32 = Color32::from_rgb(232, 145, 65);
pub const GREEN: Color32 = Color32::from_rgb(80, 185, 140);
pub type Triangle = ([[f64; 3]; 3], Color32);

#[derive(Clone)]
pub struct Surface {
    pub name: &'static str,
    pub triangles: Arc<Vec<Triangle>>,
    preview_triangles: Arc<Vec<Triangle>>,
    pub bounds: PlotBounds,
}
impl PlotItem for Surface {
    fn name(&self) -> &str {
        self.name
    }
    fn bounds(&self) -> PlotBounds {
        self.bounds
    }
    fn color(&self) -> Option<Color32> {
        Some(BLUE)
    }
    fn allow_hover(&self) -> bool {
        false
    }
    fn render(&self, ctx: &mut PlotItemContext<'_>) {
        ctx.reserve(self.triangles.len());
        for &(points, color) in self.triangles.iter() {
            ctx.triangle(points.map(Into::into), color);
        }
    }
}

pub struct Series {
    pub name: &'static str,
    pub data: PointBuffer,
    pub color: Color32,
}
impl Series {
    pub fn new(name: &'static str, data: Vec<[f64; 3]>, color: Color32) -> Self {
        Self {
            name,
            data: data.into(),
            color,
        }
    }
}
#[derive(Default)]
pub struct Scene {
    pub surface: Option<Surface>,
    pub lines: Vec<Series>,
    pub dots: Vec<Series>,
    pub arrows: Option<(PointBuffer, PointBuffer)>,
}
impl Scene {
    pub fn bounds(&self) -> PlotBounds {
        let mut bounds = self
            .surface
            .as_ref()
            .map_or(PlotBounds::NOTHING, |s| s.bounds);
        for s in self.lines.iter().chain(&self.dots) {
            bounds.merge(s.data.bounds());
        }
        bounds
    }
    pub fn plot(&self, ui: &mut PlotUi<'_>) {
        if let Some(surface) = &self.surface {
            ui.add(surface.clone());
        }
        for (i, s) in self.lines.iter().enumerate() {
            ui.line(
                Line::new(s.name, &s.data)
                    .id(("line", i))
                    .color(s.color)
                    .width(1.7),
            );
        }
        for (i, s) in self.dots.iter().enumerate() {
            ui.points(
                Points::new(s.name, &s.data)
                    .id(("dots", i))
                    .color(s.color)
                    .radius(4.0),
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
        if let Some(surface) = &self.surface {
            let mut triangles: Vec<_> = surface
                .preview_triangles
                .iter()
                .filter_map(|&(points, color)| {
                    let [Some(a), Some(b), Some(c)] = points.map(|p| t.project(p.into())) else {
                        return None;
                    };
                    Some(([a.0, b.0, c.0], color, a.1 + b.1 + c.1))
                })
                .collect();
            triangles.sort_unstable_by(|a, b| b.2.total_cmp(&a.2));
            let mut mesh = egui::Mesh::default();
            for (points, color, _) in triangles {
                let base = mesh.vertices.len() as u32;
                for pos in points {
                    mesh.colored_vertex(pos, color);
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
}
impl Science {
    pub const TITLES: [&'static str; 5] = [
        "Wave interference",
        "Kepler orbits",
        "Correlated Gaussian",
        "Harmonic motion",
        "Magnetic dipole",
    ];
    pub const NAMES: [&'static str; 5] = ["waves", "kepler", "gaussian", "harmonics", "dipole"];
    pub const TAGS: [&'static [&'static str]; 5] = [
        &["surface", "waves"],
        &["orbit", "astronomy"],
        &["surface", "statistics"],
        &["lines", "frequencies"],
        &["vectors", "magnetism"],
    ];
    pub const SOURCES: [&'static str; 5] = [
        include_str!("waves.rs"),
        include_str!("kepler.rs"),
        include_str!("gaussian.rs"),
        include_str!("harmonics.rs"),
        include_str!("dipole.rs"),
    ];
    pub const DESCRIPTIONS: [&'static str; 5] = [
        crate::waves::DESCRIPTION,
        crate::kepler::DESCRIPTION,
        crate::gaussian::DESCRIPTION,
        crate::harmonics::DESCRIPTION,
        crate::dipole::DESCRIPTION,
    ];
    pub const REFERENCES: [&'static str; 5] = [
        "https://openstax.org/books/university-physics-volume-1/pages/16-5-interference-of-waves",
        "https://science.nasa.gov/solar-system/orbits-and-keplers-laws/",
        "https://mathworld.wolfram.com/BivariateNormalDistribution.html",
        "https://mathworld.wolfram.com/LissajousCurve.html",
        "https://farside.ph.utexas.edu/teaching/jk1/Electromagnetism/node52.html",
    ];
    fn defaults(kind: usize) -> [f64; 3] {
        match kind {
            0 => [0.0, 1.5, 2.4],
            1 => [0.65, 30.0, 0.0],
            2 => [0.65, 1.0, 0.0],
            3 => [2.0, 3.0, 5.0],
            _ => [20.0, 8.0, 0.0],
        }
    }
    pub fn new(kind: usize) -> Self {
        let mut s = Self {
            kind,
            params: Self::defaults(kind),
            animate: false,
            animate_values: false,
            value_time: 0.0,
            motion_speed: 1.0,
            motion_strength: 1.0,
            time: 0.0,
            scene: Scene::default(),
            preview: None,
        };
        s.rebuild();
        s
    }
    pub fn aspect(&self) -> [f64; 3] {
        if self.kind == 2 {
            [1.0, 1.0, 8.0]
        } else {
            [1.0; 3]
        }
    }
    pub fn camera(kind: usize) -> egui_plot3d::Camera {
        egui_plot3d::Camera {
            pitch: if kind == 1 {
                1.25
            } else if kind == 0 || kind == 2 {
                0.75
            } else {
                0.45
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
                ("Inclination (degrees)", 0.0..=75.0),
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
            _ => [
                ("Tilt (degrees)", 0.0..=90.0),
                ("Meridian planes", 3.0..=12.0),
                ("Unused", 0.0..=1.0),
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
                .add(if self.kind == 3 || (self.kind == 4 && i == 1) {
                    slider.integer()
                } else {
                    slider
                })
                .changed();
        }
        if ui.button("Restore defaults").clicked() {
            self.params = Self::defaults(self.kind);
            self.time = 0.0;
            self.value_time = 0.0;
            self.animate = false;
            self.animate_values = false;
            self.motion_speed = 1.0;
            self.motion_strength = 1.0;
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
            ui.label(self.motion_description());
        }
        if changed {
            self.rebuild();
        }
        ui.separator();
        ui.set_max_width(330.0);
        ui.label(Self::DESCRIPTIONS[self.kind]);
        ui.hyperlink_to("Model reference", Self::REFERENCES[self.kind]);
    }
    pub fn toolbar(&mut self, ui: &mut egui::Ui) {
        if ui.button("Compare setting").clicked() {
            let alternatives = [1.0, 0.0, -0.65, 4.0, 75.0];
            let d = Self::defaults(self.kind)[0];
            self.params[0] = if (self.params[0] - d).abs() < 1e-6 {
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
            0 => format!(
                "Phase: {:.2} pi | blue negative, orange positive",
                self.params[0]
            ),
            1 => format!("e = {:.2} | 24 equal-time samples", self.params[0]),
            2 => format!("Correlation: {:.2} | Z scale: 8x", self.params[0]),
            3 => format!(
                "Frequency ratio {:.0}:{:.0}:{:.0}",
                self.params[0], self.params[1], self.params[2]
            ),
            _ => format!(
                "Tilt: {:.0} degrees | arrows show direction",
                self.params[0]
            ),
        });
    }
    fn has_value_animation(&self) -> bool {
        matches!(self.kind, 0 | 3 | 4)
    }
    fn motion_description(&self) -> &'static str {
        match self.kind {
            0 => {
                "Animate values varies source phase, wavelength, and separation with sine functions while the waves travel. Sliders set the base values."
            }
            3 => {
                "Animate values varies all three phases and amplitudes, and blends in higher harmonics. Every sample moves; the integer-frequency curve stays closed. Sliders set the base frequencies."
            }
            _ => {
                "Animate values precesses the dipole axis and varies the radii of the sampled field lines. This shows a sequence of ideal dipole shapes, not a time-dependent electromagnetic simulation."
            }
        }
    }
    pub fn motion_toolbar(&mut self, ui: &mut egui::Ui) {
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
            ui.label(egui::RichText::new("Speed & intensity in Parameters").weak());
        });
    }
    pub fn animation_bounds(&self) -> Option<PlotBounds> {
        match self.kind {
            3 => Some(PlotBounds::from_min_max([-1.6; 3], [1.6; 3])),
            4 => Some(PlotBounds::from_min_max([-4.4; 3], [4.4; 3])),
            _ => None,
        }
    }
    fn reset_motion(&mut self) {
        self.animate_values = false;
        self.value_time = 0.0;
        self.rebuild();
    }
    pub fn update(&mut self, dt: f32) {
        if self.animate {
            self.time = (self.time + dt as f64).rem_euclid(std::f64::consts::TAU * 100.0);
        }
        if self.animate_values {
            self.value_time += dt as f64 * self.motion_speed;
        }
        if self.animate_values || (self.kind == 0 && self.animate) {
            // Keep cached thumbnails while playing; regenerate the main data only.
            self.rebuild_scene();
        }
    }
    pub fn plot(&self, ui: &mut PlotUi<'_>) {
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
            _ if self.value_time == 0.0 => crate::dipole::build(self.params),
            _ => crate::dipole::build_animated(self.params, self.value_time, self.motion_strength),
        };
    }
}

pub fn surface(
    name: &'static str,
    extent: f64,
    n: usize,
    f: impl Fn(f64, f64) -> f64,
    color: impl Fn(f64) -> Color32,
) -> Surface {
    let vertices: Vec<_> = (0..n * n)
        .map(|i| {
            let x = extent * (2.0 * (i % n) as f64 / (n - 1) as f64 - 1.0);
            let y = extent * (2.0 * (i / n) as f64 / (n - 1) as f64 - 1.0);
            [x, y, f(x, y)]
        })
        .collect();
    let bounds = PointBuffer::from(vertices.clone()).bounds();
    let triangulate = |step: usize| {
        let mut triangles = Vec::new();
        for y in (0..n - 1).step_by(step) {
            for x in (0..n - 1).step_by(step) {
                let x1 = (x + step).min(n - 1);
                let y1 = (y + step).min(n - 1);
                for indices in [
                    [y * n + x, y * n + x1, y1 * n + x],
                    [y * n + x1, y1 * n + x1, y1 * n + x],
                ] {
                    let p = indices.map(|i| vertices[i]);
                    triangles.push((p, color(p.iter().map(|p| p[2]).sum::<f64>() / 3.0)));
                }
            }
        }
        Arc::new(triangles)
    };
    Surface {
        name,
        triangles: triangulate(1),
        preview_triangles: triangulate(2),
        bounds,
    }
}
pub fn gradient(t: f64) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (BLUE.r() as f64 * (1.0 - t) + ORANGE.r() as f64 * t) as u8,
        (BLUE.g() as f64 * (1.0 - t) + ORANGE.g() as f64 * t) as u8,
        (BLUE.b() as f64 * (1.0 - t) + ORANGE.b() as f64 * t) as u8,
    )
}

#[cfg(test)]
mod animation_tests {
    use super::*;
    fn vertices(scene: &Scene) -> Vec<[f64; 3]> {
        let mut result: Vec<[f64; 3]> = Vec::new();
        if let Some(surface) = &scene.surface {
            for (triangle, _) in surface.triangles.iter() {
                result.extend(triangle);
            }
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
        for kind in [0, 3, 4] {
            let mut model = Science::new(kind);
            let initial = vertices(&model.scene);
            model.animate_values = true;
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
