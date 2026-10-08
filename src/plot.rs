use crate::{
    Arrows, Camera, Line, PlotBounds, PlotItem, PlotItemContext, PlotPoint, PlotTransform, Points,
    Polygon, Projection, Text, render,
};
use egui::{Color32, Id, PointerButton, Pos2, Rect, Response, Sense, Ui, Vec2};
use std::collections::HashSet;

type LabelFormatter<'a> = Box<dyn Fn(&HoveredPoint) -> Option<String> + 'a>;

#[derive(Clone, Copy, Debug, Default)]
/// Corner of the plot rectangle.
pub enum Corner {
    /// Upper left.
    LeftTop,
    #[default]
    /// Upper right (default).
    RightTop,
    /// Lower left.
    LeftBottom,
    /// Lower right.
    RightBottom,
}
#[derive(Clone, Debug, Default)]
/// Optional series legend with click-to-toggle visibility.
pub struct Legend {
    corner: Corner,
}
impl Legend {
    /// Choose the plot corner containing the legend (default top right).
    pub fn position(mut self, corner: Corner) -> Self {
        self.corner = corner;
        self
    }
}

#[derive(Clone, Debug)]
/// The nearest-depth pickable sample within the pointer hit area.
pub struct HoveredPoint {
    /// Stable ID of the picked item.
    pub item_id: Id,
    /// Displayed name of the picked item.
    pub name: String,
    /// Point index, or the first vertex of a line segment.
    pub index: usize,
    /// World-space sample or interpolated segment position.
    pub position: PlotPoint,
    /// Picked screen position in logical pixels.
    pub screen_position: Pos2,
    /// Positive camera-space depth; smaller values are closer.
    pub depth: f32,
    /// Distance from the pointer in logical pixels.
    pub distance: f32,
}

/// Result of building and interacting with one plot.
pub struct PlotResponse<R> {
    /// Return value from the plot-building closure.
    pub inner: R,
    /// Egui response for the plot rectangle.
    pub response: Response,
    /// Final transform after fitting this frame's data and applying input.
    pub transform: PlotTransform,
    /// Stable ID of the currently picked item.
    pub hovered_plot_item: Option<Id>,
    /// Picked sample, suppressed while dragging or hovering the legend.
    pub hovered_point: Option<HoveredPoint>,
    /// Picked sample on a primary click; otherwise `None`.
    pub clicked_point: Option<HoveredPoint>,
    /// Number of visible point/segment/triangle/text primitives after clipping.
    pub rendered_primitives: usize,
}

#[derive(Clone, Default)]
struct PlotMemory {
    camera: Camera,
    hidden: HashSet<Id>,
    bounds: PlotBounds,
    manual_bounds: Option<PlotBounds>,
    force_auto_bounds: bool,
}

/// Build a plot every frame; camera and legend state persist under its ID.
pub struct Plot<'label> {
    id: Id,
    explicit_id: Option<Id>,
    width: Option<f32>,
    height: Option<f32>,
    aspect: Option<f32>,
    legend: Option<Legend>,
    grid: bool,
    axes: [bool; 3],
    frame: bool,
    labels: [String; 3],
    equal_aspect: bool,
    axis_aspect: [f64; 3],
    camera: Option<Camera>,
    projection: Option<Projection>,
    reset: bool,
    drag: bool,
    zoom: bool,
    pan: bool,
    double_click: bool,
    hover: bool,
    background: Option<Color32>,
    show_background: bool,
    bounds: Option<PlotBounds>,
    axis_bounds: [Option<[f64; 2]>; 3],
    includes: PlotBounds,
    depth_cue: bool,
    label_formatter: Option<LabelFormatter<'label>>,
}
impl<'label> Plot<'label> {
    /// Create a plot with an ID salt scoped to its parent UI. Use distinct stable salts for separate plots.
    pub fn new(id_source: impl egui::AsId) -> Self {
        Self {
            id: Id::new(id_source),
            explicit_id: None,
            width: None,
            height: None,
            aspect: None,
            legend: None,
            grid: true,
            axes: [true; 3],
            frame: true,
            labels: ["X".into(), "Y".into(), "Z".into()],
            equal_aspect: true,
            axis_aspect: [1.0; 3],
            camera: None,
            projection: None,
            reset: false,
            drag: true,
            zoom: true,
            pan: true,
            double_click: true,
            hover: true,
            background: None,
            show_background: true,
            bounds: None,
            axis_bounds: [None; 3],
            includes: PlotBounds::NOTHING,
            depth_cue: true,
            label_formatter: None,
        }
    }
    /// Override the plot's ID with a globally unique ID, independent of its parent UI.
    pub fn id(mut self, id: Id) -> Self {
        self.explicit_id = Some(id);
        self
    }
    /// Set width in logical pixels. Otherwise use available width, or derive it from height and view aspect.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
    /// Set height in logical pixels. Unbounded layouts fall back to the visible clip rectangle.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }
    /// Set plot width / height. Derives the unspecified dimension; ignored when both dimensions are explicit.
    pub fn view_aspect(mut self, aspect: f32) -> Self {
        if aspect.is_finite() && aspect > 0.0 {
            self.aspect = Some(aspect);
        }
        self
    }
    /// X/Y unit-length ratio before rotation; Z uses the same scale as Y.
    /// `1.0` preserves equal units on all three axes, as in egui_plot.
    pub fn data_aspect(self, aspect: f32) -> Self {
        self.axis_aspect([aspect as f64, 1.0, 1.0])
    }
    /// Set relative X/Y/Z unit lengths before camera rotation.
    pub fn axis_aspect(mut self, aspect: [f64; 3]) -> Self {
        if aspect.iter().all(|v| v.is_finite() && *v > 0.0) {
            self.axis_aspect = aspect;
            self.equal_aspect = true;
        }
        self
    }
    /// Preserve equal world units. `false` fits each axis independently.
    pub fn equal_axis_scale(mut self, equal: bool) -> Self {
        self.equal_aspect = equal;
        self.axis_aspect = [1.0; 3];
        self
    }
    /// Display a legend. Clicking an entry toggles its series without changing fitted bounds.
    pub fn legend(mut self, legend: Legend) -> Self {
        self.legend = Some(legend);
        self
    }
    /// Show the XY grid at the minimum Z bound (default true).
    pub fn show_grid(mut self, show: bool) -> Self {
        self.grid = show;
        self
    }
    /// Show axis lines, ticks, and labels for X, Y, and Z (all enabled by default).
    pub fn show_axes(mut self, show: [bool; 3]) -> Self {
        self.axes = show;
        self
    }
    /// Show the plot background (default true). This does not change the configured background color.
    pub fn show_background(mut self, show: bool) -> Self {
        self.show_background = show;
        self
    }
    /// Override the theme background color.
    pub fn background_color(mut self, color: Color32) -> Self {
        self.background = Some(color);
        self
    }
    /// Show the twelve edges of the fitted bounding box (default true).
    pub fn show_frame(mut self, show: bool) -> Self {
        self.frame = show;
        self
    }
    /// Set the X-axis label (default "X").
    pub fn x_axis_label(mut self, label: impl Into<String>) -> Self {
        self.labels[0] = label.into();
        self
    }
    /// Set the Y-axis label (default "Y").
    pub fn y_axis_label(mut self, label: impl Into<String>) -> Self {
        self.labels[1] = label.into();
        self
    }
    /// Set the Z-axis label (default "Z").
    pub fn z_axis_label(mut self, label: impl Into<String>) -> Self {
        self.labels[2] = label.into();
        self
    }
    /// Override persisted camera for this frame (useful for linked views and animation).
    pub fn camera(mut self, camera: Camera) -> Self {
        self.camera = Some(camera);
        self
    }
    /// Override the camera projection for this frame.
    pub fn projection(mut self, projection: Projection) -> Self {
        self.projection = Some(projection);
        self
    }
    /// Reset camera and bounds on this frame, preserving projection and legend visibility.
    pub fn reset(mut self) -> Self {
        self.reset = true;
        self
    }
    /// Allow primary dragging to orbit (default true). Panning is controlled separately by [`Self::allow_pan`].
    pub fn allow_drag(mut self, allow: bool) -> Self {
        self.drag = allow;
        self
    }
    /// Allow wheel and pinch zoom while hovering (default true).
    pub fn allow_zoom(mut self, allow: bool) -> Self {
        self.zoom = allow;
        self
    }
    /// Alias for [`Self::allow_zoom`]; controls both wheel and pinch zoom.
    pub fn allow_scroll(self, allow: bool) -> Self {
        self.allow_zoom(allow)
    }
    /// Allow secondary, middle, or Shift-primary dragging to pan (default true).
    pub fn allow_pan(mut self, allow: bool) -> Self {
        self.pan = allow;
        self
    }
    /// Allow double-clicking the plot to reset its camera and bounds (default true).
    pub fn allow_double_click_reset(mut self, allow: bool) -> Self {
        self.double_click = allow;
        self
    }
    /// Enable sample picking, highlighting, and hover text (default true).
    pub fn allow_hover(mut self, allow: bool) -> Self {
        self.hover = allow;
        self
    }
    /// Dim distant geometry for a depth cue (default true). Does not alter depth order.
    pub fn depth_cue(mut self, enable: bool) -> Self {
        self.depth_cue = enable;
        self
    }
    /// Customize hover text, or return None to suppress the built-in tooltip.
    pub fn label_formatter(
        mut self,
        formatter: impl Fn(&HoveredPoint) -> Option<String> + 'label,
    ) -> Self {
        self.label_formatter = Some(Box::new(formatter));
        self
    }
    /// Starting and reset bounds. Explicit [`PlotUi`] bounds/auto-fit choices take precedence.
    pub fn default_bounds(mut self, bounds: PlotBounds) -> Self {
        if bounds.is_valid() {
            self.bounds = Some(bounds);
        }
        self
    }
    /// Starting/reset X range. Invalid ranges are ignored; explicit PlotUi bounds or auto-fit choices take precedence.
    pub fn default_x_bounds(mut self, min: f64, max: f64) -> Self {
        self.axis_bounds[0] = Some([min, max]);
        self
    }
    /// Starting/reset Y range. Invalid ranges are ignored; explicit PlotUi bounds or auto-fit choices take precedence.
    pub fn default_y_bounds(mut self, min: f64, max: f64) -> Self {
        self.axis_bounds[1] = Some([min, max]);
        self
    }
    /// Starting/reset Z range. Invalid ranges are ignored; explicit PlotUi bounds or auto-fit choices take precedence.
    pub fn default_z_bounds(mut self, min: f64, max: f64) -> Self {
        self.axis_bounds[2] = Some([min, max]);
        self
    }
    /// Expand automatically fitted/default bounds to include a finite X coordinate.
    pub fn include_x(mut self, value: impl Into<f64>) -> Self {
        self.includes.extend_with_x(value.into());
        self
    }
    /// Expand automatically fitted/default bounds to include a finite Y coordinate.
    pub fn include_y(mut self, value: impl Into<f64>) -> Self {
        self.includes.extend_with_y(value.into());
        self
    }
    /// Expand automatically fitted/default bounds to include a finite Z coordinate.
    pub fn include_z(mut self, value: impl Into<f64>) -> Self {
        self.includes.extend_with_z(value.into());
        self
    }
    /// Expand automatically fitted/default bounds to include a finite point.
    pub fn include_point(mut self, point: impl Into<PlotPoint>) -> Self {
        self.includes.extend_with(point);
        self
    }

    /// Build items and show the plot, returning the closure result, interaction response, and final transform.
    pub fn show<'a, R>(
        self,
        ui: &mut Ui,
        build: impl FnOnce(&mut PlotUi<'a>) -> R,
    ) -> PlotResponse<R> {
        let id = self
            .explicit_id
            .unwrap_or_else(|| ui.make_persistent_id(self.id));
        let mut memory = ui
            .ctx()
            .data_mut(|d| d.get_temp::<PlotMemory>(id))
            .unwrap_or_default();
        let finite_size = |value: f32, fallback: f32| {
            if value.is_finite() {
                value.max(1.0)
            } else {
                fallback.max(1.0)
            }
        };
        let available_width = finite_size(ui.available_width(), ui.clip_rect().width());
        let available_height =
            finite_size(ui.available_height(), ui.clip_rect().height()).max(150.0);
        let width = finite_size(
            self.width
                .unwrap_or_else(|| match (self.height, self.aspect) {
                    (Some(height), Some(aspect)) if height.is_finite() => height * aspect,
                    _ => available_width,
                }),
            available_width,
        );
        let height = finite_size(
            self.height.unwrap_or_else(|| {
                self.aspect
                    .map_or(available_height, |aspect| width / aspect)
            }),
            available_height,
        );
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());
        if let Some(camera) = self.camera {
            memory.camera = camera.sanitized();
        }
        if self.reset || (self.double_click && response.double_clicked()) {
            memory.camera = Camera {
                projection: memory.camera.projection,
                ..Camera::default()
            };
            memory.bounds = PlotBounds::NOTHING;
            memory.manual_bounds = None;
            memory.force_auto_bounds = false;
        }
        if let Some(projection) = self.projection {
            memory.camera.projection = projection;
        }
        let delta = ui.input(|i| i.pointer.delta());
        let shift = ui.input(|i| i.modifiers.shift);
        if self.pan
            && (response.dragged_by(PointerButton::Secondary)
                || response.dragged_by(PointerButton::Middle)
                || (shift && response.dragged_by(PointerButton::Primary)))
        {
            memory.camera.pan += Vec2::new(delta.x / width, delta.y / height);
        } else if self.drag && !shift && response.dragged_by(PointerButton::Primary) {
            memory.camera.orbit(delta);
        }
        if response.hovered() && self.zoom {
            let scroll = ui.input_mut(|i| std::mem::take(&mut i.smooth_scroll_delta.y));
            let pinch = ui.input(|i| i.zoom_delta());
            memory.camera.zoom_by((scroll * 0.003).exp() * pinch);
        }
        let make_transform = |bounds, camera| {
            let transform = PlotTransform::new(rect, bounds, camera, self.equal_aspect);
            if self.equal_aspect {
                transform.with_axis_aspect(self.axis_aspect)
            } else {
                transform
            }
        };
        let previous_transform = make_transform(memory.bounds, memory.camera);
        let mut plot_ui = PlotUi {
            items: Vec::new(),
            transform: previous_transform,
            pointer: response.hover_pos(),
            bounds: if memory.force_auto_bounds {
                None
            } else {
                memory.manual_bounds.or(self.bounds)
            },
            bounds_changed: false,
            camera: None,
            response: response.clone(),
        };
        let inner = build(&mut plot_ui);
        if plot_ui.bounds_changed {
            memory.manual_bounds = plot_ui.bounds;
            memory.force_auto_bounds = plot_ui.bounds.is_none();
        }
        if let Some(camera) = plot_ui.camera {
            memory.camera = camera.sanitized();
        }
        let mut bounds = plot_ui.bounds.unwrap_or(PlotBounds::NOTHING);
        if plot_ui.bounds.is_none() {
            for item in &plot_ui.items {
                bounds.merge(item.bounds());
            }
        }
        bounds = bounds.fitted();
        if memory.manual_bounds.is_none() {
            bounds.include_partial(self.includes);
        }
        if !plot_ui.bounds_changed && memory.manual_bounds.is_none() && !memory.force_auto_bounds {
            for (axis, range) in self.axis_bounds.iter().enumerate() {
                if let Some([min, max]) = range {
                    bounds.set_axis(axis, *min, *max);
                }
            }
        }
        memory.bounds = bounds.fitted();
        let transform = make_transform(memory.bounds, memory.camera);
        if !ui.is_rect_visible(rect) {
            ui.ctx().data_mut(|d| d.insert_temp(id, memory));
            return PlotResponse {
                inner,
                response,
                transform,
                hovered_plot_item: None,
                hovered_point: None,
                clicked_point: None,
                rendered_primitives: 0,
            };
        }
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        if self.show_background {
            painter.rect_filled(
                rect,
                4.0,
                self.background.unwrap_or(ui.visuals().extreme_bg_color),
            );
        }
        render::guides(
            &painter,
            &transform,
            self.grid,
            self.axes,
            &self.labels,
            self.frame,
        );
        let mut hovered = None;
        let scratch_id = id.with("render_scratch");
        let mut scratch = ui
            .ctx()
            .data_mut(|d| d.remove_temp::<render::RenderScratch>(scratch_id))
            .unwrap_or_default();
        for (index, item) in plot_ui.items.iter().enumerate() {
            if memory.hidden.contains(&item.id()) {
                continue;
            }
            item.render(&mut PlotItemContext {
                transform: &transform,
                primitives: &mut scratch.primitives,
                pointer: response.hover_pos(),
                hovered: &mut hovered,
                item_id: item.id(),
                name: item.name(),
                color: item.color().unwrap_or_else(|| palette(index)),
                hover: self.hover && item.allow_hover(),
                depth_cue: self.depth_cue,
            });
        }
        let rendered_primitives = render::paint(&painter, &mut scratch);
        ui.ctx().data_mut(|d| d.insert_temp(scratch_id, scratch));
        let mut over_legend = false;
        if let Some(legend) = self.legend {
            let entries: Vec<_> = plot_ui
                .items
                .iter()
                .enumerate()
                .filter(|(_, i)| !i.name().is_empty())
                .collect();
            if !entries.is_empty() {
                let legend_width = entries
                    .iter()
                    .map(|(_, i)| {
                        painter
                            .layout_no_wrap(
                                i.name().to_owned(),
                                egui::FontId::proportional(12.0),
                                Color32::WHITE,
                            )
                            .size()
                            .x
                    })
                    .fold(0.0, f32::max)
                    + 40.0;
                let size = Vec2::new(legend_width, entries.len() as f32 * 23.0 + 8.0);
                let origin = match legend.corner {
                    Corner::LeftTop => rect.left_top() + Vec2::splat(8.0),
                    Corner::RightTop => rect.right_top() + Vec2::new(-size.x - 8.0, 8.0),
                    Corner::LeftBottom => rect.left_bottom() + Vec2::new(8.0, -size.y - 8.0),
                    Corner::RightBottom => rect.right_bottom() - size - Vec2::splat(8.0),
                };
                let legend_rect = Rect::from_min_size(origin, size);
                over_legend = response
                    .hover_pos()
                    .is_some_and(|p| legend_rect.contains(p));
                painter.rect_filled(
                    legend_rect,
                    5.0,
                    ui.visuals().panel_fill.gamma_multiply(0.94),
                );
                for (row, (index, item)) in entries.into_iter().enumerate() {
                    let r = Rect::from_min_size(
                        origin + Vec2::new(4.0, 4.0 + row as f32 * 23.0),
                        Vec2::new(size.x - 8.0, 22.0),
                    );
                    let hit = ui.interact(r, id.with(item.id()), Sense::click());
                    let hidden = memory.hidden.contains(&item.id());
                    if hit.clicked() {
                        if hidden {
                            memory.hidden.remove(&item.id());
                        } else {
                            memory.hidden.insert(item.id());
                        }
                        ui.ctx().request_repaint();
                    }
                    let c = if hidden {
                        ui.visuals().weak_text_color()
                    } else {
                        item.color().unwrap_or_else(|| palette(index))
                    };
                    painter.circle_filled(r.left_center() + Vec2::new(8.0, 0.0), 3.5, c);
                    painter.text(
                        r.left_center() + Vec2::new(20.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        item.name(),
                        egui::FontId::proportional(12.0),
                        c,
                    );
                    hit.on_hover_text("Click to hide or show this series");
                }
            }
        }
        if over_legend || response.dragged() {
            hovered = None;
        }
        if let Some(h) = &hovered {
            painter.circle_stroke(
                h.screen_position,
                7.0,
                egui::Stroke::new(1.5, ui.visuals().text_color()),
            );
            let label = self.label_formatter.as_ref().map_or_else(
                || {
                    Some(format!(
                        "x  {:.5}\ny  {:.5}\nz  {:.5}\nindex  {}",
                        h.position.x, h.position.y, h.position.z, h.index
                    ))
                },
                |formatter| formatter(h),
            );
            if let Some(label) = label {
                response.clone().on_hover_ui(|ui| {
                    ui.strong(&h.name);
                    ui.monospace(label);
                });
            }
        }
        ui.ctx().data_mut(|d| d.insert_temp(id, memory));
        PlotResponse {
            inner,
            hovered_plot_item: hovered.as_ref().map(|h| h.item_id),
            clicked_point: if response.clicked() {
                hovered.clone()
            } else {
                None
            },
            hovered_point: hovered,
            response,
            transform,
            rendered_primitives,
        }
    }
}
pub(crate) fn palette(index: usize) -> Color32 {
    const COLORS: [[u8; 3]; 8] = [
        [89, 198, 214],
        [236, 157, 94],
        [174, 142, 235],
        [130, 201, 144],
        [225, 119, 153],
        [230, 204, 113],
        [129, 166, 240],
        [212, 161, 215],
    ];
    let c = COLORS[index % COLORS.len()];
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Submit plot items and control the camera/bounds inside [`Plot::show`].
pub struct PlotUi<'a> {
    items: Vec<Box<dyn PlotItem + 'a>>,
    transform: PlotTransform,
    pointer: Option<Pos2>,
    bounds: Option<PlotBounds>,
    bounds_changed: bool,
    camera: Option<Camera>,
    response: Response,
}
impl<'a> PlotUi<'a> {
    /// Submit a custom plot item. Geometry participates in the shared projection and depth ordering.
    pub fn add(&mut self, item: impl PlotItem + 'a) {
        self.items.push(Box::new(item));
    }
    /// Submit a marker series.
    pub fn points(&mut self, item: Points<'a>) {
        self.add(item);
    }
    /// Submit a polyline.
    pub fn line(&mut self, item: Line<'a>) {
        self.add(item);
    }
    /// Add a filled indexed triangle surface.
    pub fn surface(&mut self, item: crate::Surface<'a>) {
        self.add(item);
    }
    /// Submit a vector series.
    pub fn arrows(&mut self, item: Arrows<'a>) {
        self.add(item);
    }
    /// Submit a filled convex planar polygon.
    pub fn polygon(&mut self, item: Polygon<'a>) {
        self.add(item);
    }
    /// Submit a world-positioned text label.
    pub fn text(&mut self, item: Text) {
        self.add(item);
    }
    /// Previous frame's fitted bounds; set explicit bounds for immediate control.
    pub fn plot_bounds(&self) -> PlotBounds {
        self.transform.bounds()
    }
    /// Freeze finite bounds until reset or an explicit auto-fit request; overrides builder defaults.
    pub fn set_plot_bounds(&mut self, bounds: PlotBounds) {
        if bounds.is_valid() {
            self.bounds = Some(bounds);
            self.bounds_changed = true;
        }
    }
    /// Resume fitting all submitted data, or freeze the current bounds.
    pub fn set_auto_bounds(&mut self, auto: bool) {
        self.bounds = if auto { None } else { Some(self.plot_bounds()) };
        self.bounds_changed = true;
    }
    /// Current camera, including a camera set earlier in this closure.
    pub fn camera(&self) -> Camera {
        self.camera.unwrap_or(self.transform.camera())
    }
    /// Override the camera for this frame and persist it for following frames.
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = Some(camera);
    }
    /// Unproject the pointer onto the view-parallel plane through the plot center using the previous fitted bounds.
    pub fn pointer_coordinate(&self) -> Option<PlotPoint> {
        self.transform.point_from_position(self.pointer?, 4.0)
    }
    /// Response for the plot rectangle, for custom input handling.
    pub fn response(&self) -> &Response {
        &self.response
    }
    /// Previous frame's transform. The final transform is returned in PlotResponse.
    pub fn transform(&self) -> &PlotTransform {
        &self.transform
    }
}
