// Gallery layout adapted from https://github.com/emilk/egui_plot (MIT OR Apache-2.0).
use crate::{
    cloud::{COLORS, Cloud},
    lorenz::Lorenz,
    science::Science,
};
use eframe::egui::{self, RichText, ScrollArea, Vec2, special_emojis::GITHUB};
use egui_plot3d::{Camera, Legend, Plot, PlotBounds, PlotTransform, Projection};

const GITHUB_URL: &str = "https://github.com/boquila/egui_plot3d";

pub struct Gallery {
    cloud: Cloud,
    lorenz: Lorenz,
    science: Vec<Science>,
    selected: Option<usize>,
    grid: bool,
    axes: bool,
    frame: bool,
    equal: bool,
    depth: bool,
    orthographic: bool,
    spin: bool,
    reset: bool,
    camera: [Camera; 7],
}

impl Gallery {
    const COL_WIDTH: f32 = 128.0;

    pub fn new(_ctx: &egui::Context) -> Self {
        Self {
            cloud: Cloud::default(),
            lorenz: Lorenz::default(),
            science: (0..5).map(Science::new).collect(),
            selected: None,
            grid: true,
            axes: true,
            frame: true,
            equal: true,
            depth: true,
            orthographic: false,
            spin: false,
            reset: false,
            camera: std::array::from_fn(|i| {
                if i < 2 {
                    Camera::default()
                } else {
                    Science::camera(i - 2)
                }
            }),
        }
    }

    fn title(index: usize) -> &'static str {
        if index == 0 {
            "Points in space"
        } else if index == 1 {
            "Lorenz attractor"
        } else {
            Science::TITLES[index - 2]
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_example(mut self, name: &str) -> Self {
        self.selected = match name {
            "points" => Some(0),
            "lorenz" => Some(1),
            _ => Science::NAMES
                .iter()
                .position(|&n| n == name)
                .map(|i| i + 2),
        };
        self
    }

    fn tags(index: usize) -> &'static [&'static str] {
        if index == 0 {
            &["3D", "points", "interaction"]
        } else if index == 1 {
            &["3D", "lines", "dynamics"]
        } else {
            Science::TAGS[index - 2]
        }
    }

    fn top_bar(ui: &mut egui::Ui) {
        egui::Panel::top("top_panel").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                egui::widgets::global_theme_preference_buttons(ui);
                ui.add_space(16.0);
                ui.hyperlink_to(format!("{GITHUB} egui_plot3d on GitHub"), GITHUB_URL);
            });
        });
    }

    fn thumbnails_panel(&mut self, ui: &mut egui::Ui, default_width: f32) {
        let screen_width = ui.ctx().content_rect().width();
        let max_width = if screen_width < 1024.0 {
            (screen_width * 0.4).max(100.0)
        } else {
            Self::COL_WIDTH * 3.0 + 30.0
        };
        egui::Panel::left("examples_panel")
            .default_size(default_width)
            .min_size(100.0)
            .max_size(max_width)
            .resizable(true)
            .show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    let available_width = ui.available_width();
                    let columns = 1.max((available_width / Self::COL_WIDTH).floor() as usize);
                    let scale = 1.0_f32.min(available_width / (Self::COL_WIDTH * columns as f32));
                    ui.heading("Examples");
                    ui.separator();
                    egui::Grid::new("examples_grid").show(ui, |ui| {
                        for index in 0..7 {
                            self.make_cell(index, ui, scale);
                            if (index + 1) % columns == 0 {
                                ui.end_row();
                            }
                        }
                    });
                });
            });
    }

    fn make_cell(&mut self, index: usize, ui: &mut egui::Ui, scale: f32) {
        let mut button = egui::Button::new("").min_size(Vec2::splat(110.0 * scale + 8.0));
        if self.selected == Some(index) {
            button = button.fill(ui.visuals().selection.bg_fill);
        }
        let response = ui.add(button).on_hover_text_at_pointer(format!(
            "Click to select `{}`\nTags: {}",
            Self::title(index),
            Self::tags(index).join(", ")
        ));
        // Data previews use the reference gallery's thumbnail button styling.
        let rect = response.rect.shrink(4.0);
        let painter = ui.painter().with_clip_rect(rect);
        let mut bounds = PlotBounds::NOTHING;
        if index == 0 {
            for group in &self.cloud.groups {
                bounds.merge(group.bounds());
            }
        } else if index == 1 {
            bounds = self.lorenz.trajectory.bounds();
        }
        let transform = PlotTransform::new(rect, bounds, Camera::default(), true);
        if index == 0 {
            for (i, group) in self.cloud.groups.iter().enumerate() {
                for &point in group.as_slice().iter().step_by((group.len() / 100).max(1)) {
                    if let Some(pos) = transform.position_from_point(point) {
                        painter.circle_filled(pos, 1.1 * scale, COLORS[i]);
                    }
                }
            }
        } else if index == 1 {
            let points: Vec<_> = self
                .lorenz
                .trajectory
                .as_slice()
                .iter()
                .step_by(8)
                .filter_map(|&p| transform.position_from_point(p))
                .collect();
            painter.add(egui::Shape::line(points, egui::Stroke::new(0.7, COLORS[0])));
        } else {
            self.science[index - 2].preview(&painter, rect);
        }
        if response.clicked() {
            self.selected = Some(index);
        }
    }

    fn info_panel(&self, ui: &mut egui::Ui, index: usize) {
        egui::Panel::right("info_panel")
            .resizable(true)
            .default_size(600.0)
            .show(ui, |ui| {
                ui.label(RichText::new(Self::title(index)).heading());
                ui.separator();
                ui.label(RichText::new(if index == 0 {
                    "A 3D point cloud with five clusters, inspired by BoquilaHub's embedding plot. Drag to orbit, right-drag to pan, and scroll to zoom. Click a point to inspect its coordinates."
                } else if index == 1 {
                    "A 3D Lorenz attractor integrated with fourth-order Runge-Kutta. Change the system parameters or compare nearby initial conditions to explore sensitivity to initial conditions."
                } else {
                    Science::DESCRIPTIONS[index-2]
                }).line_height(Some(20.0)));
                if index>=2 { ui.hyperlink_to("Model reference",Science::REFERENCES[index-2]); }
                ui.horizontal_wrapped(|ui| {
                    ui.label("Tags:");
                    for &tag in Self::tags(index) {
                        ui.label(RichText::new(tag).background_color(ui.visuals().widgets.inactive.bg_fill).small());
                    }
                });
                ui.separator();
                let name = if index == 0 { "cloud" } else if index == 1 { "lorenz" } else {Science::NAMES[index-2]};
                ui.horizontal(|ui| {
                    ui.label(format!("Code of `{name}` see also on "));
                    ui.hyperlink_to(format!("{GITHUB} Github"), format!("{GITHUB_URL}/blob/main/demo/src/{name}.rs"));
                });
                let source = if index == 0 { include_str!("cloud.rs") } else if index == 1 { include_str!("lorenz.rs") } else {Science::SOURCES[index-2]};
                ScrollArea::both().show(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    let theme = egui_extras::syntax_highlighting::CodeTheme::from_style(ui.style());
                    egui_extras::syntax_highlighting::code_view_ui(ui, &theme, source, "rs");
                });
            });
    }

    fn controls(&mut self, ui: &mut egui::Ui, index: usize) {
        ui.horizontal_wrapped(|ui| {
            ui.menu_button("Parameters", |ui| {
                if index == 0 {
                    self.cloud.controls(ui);
                } else if index == 1 {
                    self.lorenz.controls(ui);
                } else {
                    self.science[index - 2].controls(ui);
                }
            });
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.grid, "Ground grid");
                ui.checkbox(&mut self.axes, "Axes and ticks");
                ui.checkbox(&mut self.frame, "Bounding box");
                ui.checkbox(&mut self.equal, "Equal axis scale");
                ui.checkbox(&mut self.depth, "Depth shading");
                ui.checkbox(&mut self.orthographic, "Orthographic projection");
            });
            ui.checkbox(&mut self.spin, "Auto orbit");
            if ui.button("Reset view").clicked() {
                self.reset = true;
            }
        });
        ui.horizontal_wrapped(|ui| {
            if index == 0 {
                ui.checkbox(&mut self.cloud.colored, "Color by cluster");
                ui.label(format!("{} points", self.cloud.count));
            } else if index == 1 {
                ui.checkbox(&mut self.lorenz.animate, "Animate particle");
                ui.checkbox(&mut self.lorenz.show_neighbor, "Nearby initial condition");
            } else {
                self.science[index - 2].toolbar(ui);
            }
        });
        if index >= 2 {
            self.science[index - 2].motion_toolbar(ui);
        }
        if index == 0
            && let Some((name, sample, p)) = &self.cloud.selected
        {
            ui.label(format!(
                "{name} [{sample}]: ({:.4}, {:.4}, {:.4})",
                p[0], p[1], p[2]
            ));
        }
    }

    fn demo_panel(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            let Some(index) = self.selected else {
                ui.vertical_centered(|ui| {
                    ui.heading("Select an example from the left panel");
                });
                return;
            };
            self.controls(ui, index);
            ui.separator();
            let dt = ui.input(|i| i.stable_dt).min(0.05);
            if index >= 2 {
                self.science[index - 2].update(dt);
            }
            let mut camera = self.camera[index];
            if self.spin {
                camera.yaw += dt * 0.18;
                ui.ctx().request_repaint();
            }
            if self.reset {
                camera = if index < 2 {
                    Camera::default()
                } else {
                    Science::camera(index - 2)
                };
                self.reset = false;
            }
            camera.projection = if self.orthographic {
                Projection::Orthographic
            } else {
                Projection::Perspective
            };
            let mut plot = Plot::new(("main", index))
                .height(ui.available_height().max(100.0))
                .camera(camera)
                .show_grid(self.grid)
                .show_axes([self.axes; 3])
                .show_frame(self.frame)
                .equal_axis_scale(self.equal)
                .depth_cue(self.depth)
                .legend(Legend::default());
            if index >= 2 {
                let example = &self.science[index - 2];
                let labels = example.axes();
                plot = plot
                    .x_axis_label(labels[0])
                    .y_axis_label(labels[1])
                    .z_axis_label(labels[2]);
                if example.kind == 2 {
                    plot = plot.axis_aspect(example.aspect());
                }
                if let Some(bounds) = example.animation_bounds() {
                    plot = plot.default_bounds(bounds);
                }
            }
            let response = plot.show(ui, |p| {
                if index == 0 {
                    self.cloud.plot(p);
                } else if index == 1 {
                    self.lorenz.plot(p, dt);
                } else {
                    self.science[index - 2].plot(p);
                }
            });
            self.camera[index] = response.transform.camera();
            if index == 0
                && let Some(h) = response.clicked_point
            {
                self.cloud.selected = Some((h.name, h.index, h.position.to_array()));
            }
            if (index == 1 && self.lorenz.animate)
                || (index >= 2
                    && (self.science[index - 2].animate || self.science[index - 2].animate_values))
            {
                ui.ctx().request_repaint();
            }
        });
    }
}

impl eframe::App for Gallery {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let screen_rect = ui.max_rect();
        Self::top_bar(ui);
        if screen_rect.width() >= 1024.0
            && let Some(index) = self.selected
        {
            self.info_panel(ui, index);
        }
        self.thumbnails_panel(ui, screen_rect.width() / 3.0);
        self.demo_panel(ui);
    }
}
