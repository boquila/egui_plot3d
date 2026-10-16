//! Native example: shared data, cache revisions, selection, and surface attributes.
//! cargo run --release -p egui_plot3d_demo --bin explorer
use eframe::egui::{self, Color32};
use egui_plot3d::{
    ColorMap, ColorScale, Plot, PointBuffer, Points, Projection, Surface, SurfaceGrid, UpAxis,
};

struct Explorer {
    points: PointBuffer,
    colors: Vec<Color32>,
    grid: SurfaceGrid,
    surface_colors: Vec<Color32>,
    values: Vec<f64>,
    selected: Option<usize>,
    sampled: bool,
    surface: bool,
    reading: Option<f64>,
}
impl Default for Explorer {
    fn default() -> Self {
        let points = PointBuffer::from_points((0..1_000_000).map(|i| {
            let t = i as f64 * 0.013;
            [t.sin(), (t * 1.618).cos(), (t * 0.37).sin()]
        }));
        let scale = ColorScale::new(-1.0..=1.0, ColorMap::Viridis);
        let colors = scale.colors(points.as_slice().iter().map(|p| p[2]));
        let grid =
            SurfaceGrid::from_fn(-3.0..=3.0, -3.0..=3.0, [129, 129], |x, y| x.sin() * y.cos());
        let values: Vec<_> = grid.vertices().as_slice().iter().map(|p| p[2]).collect();
        let surface_colors = scale.colors(values.iter().copied());
        Self {
            points,
            colors,
            grid,
            surface_colors,
            values,
            selected: None,
            sampled: true,
            surface: false,
            reading: None,
        }
    }
}
impl eframe::App for Explorer {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.surface, false, "Point cloud");
                ui.selectable_value(&mut self.surface, true, "Surface");
                if !self.surface {
                    ui.checkbox(&mut self.sampled, "Sample nearest point per 4 px cell");
                }
            });
            if self.surface {
                ui.label(self.reading.map_or_else(
                    || "Hover the surface to interpolate its vertex values.".into(),
                    |v| format!("Interpolated value: {v:.4}"),
                ));
            } else {
                ui.label(self.selected.map_or_else(
                    || {
                        "1,000,000 samples. Click a point to select it; click empty space to clear."
                            .into()
                    },
                    |i| {
                        format!(
                            "Selected original sample {i}: {:?}",
                            self.points.as_slice()[i]
                        )
                    },
                ));
            }
            let response = Plot::new(("explorer", self.surface))
                .cache_revision((self.surface, self.sampled))
                .up_axis(if self.surface { UpAxis::Z } else { UpAxis::Y })
                .projection(if self.surface {
                    Projection::Orthographic
                } else {
                    Projection::Perspective
                })
                .zoom_to_pointer(true)
                .depth_cue(false)
                .height(ui.available_height())
                .show(ui, |plot| {
                    if self.surface {
                        plot.surface(
                            Surface::new("Field", self.grid.vertices(), self.grid.indices())
                                .vertex_colors(self.surface_colors.as_slice()),
                        );
                    } else {
                        let points = Points::new("Samples", &self.points)
                            .colors(self.colors.as_slice())
                            .radius(3.5)
                            .perspective_scaling(true)
                            .radius_limits(2.0..=9.0);
                        plot.points(if self.sampled {
                            points.screen_space_sampling(4.0)
                        } else {
                            points
                        });
                    }
                });
            if self.surface {
                self.reading = response.hovered_point.and_then(|hit| {
                    let weights = hit.barycentric?;
                    let indices = self.grid.indices().get(hit.index)?;
                    Some(
                        (0..3)
                            .map(|i| self.values[indices[i] as usize] * weights[i])
                            .sum(),
                    )
                });
            } else {
                if response.response.clicked() {
                    self.selected = response.clicked_point.map(|h| h.index);
                }
                // Selection is an overlay: it does not invalidate the large cached cloud.
                if let Some(index) = self.selected
                    && let Some(position) = response
                        .transform
                        .position_from_point(self.points.as_slice()[index])
                {
                    ui.painter_at(response.response.rect).circle_stroke(
                        position,
                        10.0,
                        egui::Stroke::new(2.0, Color32::WHITE),
                    );
                }
            }
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    eframe::run_native(
        "Data explorer",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<Explorer>::default())),
    )
}
// The main gallery supplies the browser entry point; this additional example is native.
#[cfg(target_arch = "wasm32")]
fn main() {
    let _ = Explorer::default;
}
