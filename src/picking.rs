use crate::{HoveredPoint, PlotPoint, PlotTransform, Projection};
use egui::{Id, Pos2, Rect};
use std::sync::Arc;

/// Immutable spatial picking snapshot for custom tooltips, annotations, or
/// visibility-tested overlays. Request with [`crate::Plot::capture_picking`].
/// Only hover-enabled items participate; text and arrowheads do not.
#[derive(Clone)]
pub struct PlotPicking {
    pub(crate) scene: Arc<PickScene>,
    pub(crate) transform: PlotTransform,
}
impl PlotPicking {
    /// Pick the nearest sample, segment or triangle at a logical screen position.
    pub fn hit_test(&self, position: Pos2) -> Option<HoveredPoint> {
        self.scene.pick(position, &self.transform)
    }
    /// Pick the nearest filled triangle, ignoring point and line overlays.
    pub fn surface_at(&self, position: Pos2) -> Option<HoveredPoint> {
        self.scene.pick_filtered(position, &self.transform, true)
    }
    /// Whether a world point projects inside the viewport and is not behind a
    /// pickable surface. `depth_bias` is a nonnegative camera-space tolerance for
    /// overlays lying on a surface. This is a point visibility test, not segment
    /// clipping or a pixel depth buffer. Transparent pickable faces also occlude.
    pub fn is_visible(&self, point: impl Into<PlotPoint>, depth_bias: f32) -> bool {
        let Some((screen, depth)) = self.transform.project(point.into()) else {
            return false;
        };
        if !self.scene.rect.contains(screen) {
            return false;
        }
        let bias = if depth_bias.is_finite() {
            depth_bias.max(0.0)
        } else {
            0.0
        };
        self.surface_at(screen)
            .is_none_or(|hit| depth <= hit.depth + bias)
    }
}

#[derive(Clone)]
pub(crate) enum PickShape {
    Point {
        position: PlotPoint,
        screen: Pos2,
        depth: f32,
        radius: f32,
    },
    Segment {
        ends: [(Pos2, f32); 2],
        clipped: [Pos2; 2],
        radius: f32,
    },
    Triangle {
        vertices: [(Pos2, f32); 3],
        weights: [[f64; 3]; 3],
    },
}

impl PickShape {
    fn bounds(&self) -> Rect {
        match self {
            Self::Point { screen, radius, .. } => {
                Rect::from_center_size(*screen, egui::Vec2::splat(radius.max(5.0) * 2.0))
            }
            Self::Segment {
                clipped, radius, ..
            } => Rect::from_two_pos(clipped[0], clipped[1]).expand(radius.max(5.0)),
            Self::Triangle { vertices, .. } => Rect::from_points(&vertices.map(|v| v.0)),
        }
    }

    #[inline]
    pub(crate) fn hit(
        &self,
        pointer: Pos2,
        transform: &PlotTransform,
        item_id: Id,
        name: &str,
        index: usize,
        best: Option<&HoveredPoint>,
    ) -> Option<HoveredPoint> {
        let (position, screen_position, depth, distance, barycentric) = match self {
            Self::Point {
                position,
                screen,
                depth,
                radius,
            } => {
                let distance_sq = pointer.distance_sq(*screen);
                if distance_sq > radius.max(5.0).powi(2) {
                    return None;
                }
                (*position, *screen, *depth, distance_sq.sqrt(), None)
            }
            Self::Segment {
                ends: [a, b],
                clipped: [start, end],
                radius,
            } => {
                if !self.bounds().contains(pointer) {
                    return None;
                }
                let delta = *end - *start;
                let t =
                    ((pointer - *start).dot(delta) / delta.length_sq().max(1e-12)).clamp(0.0, 1.0);
                let pos = *start + delta * t;
                let distance = pointer.distance(pos);
                if distance > radius.max(5.0) {
                    return None;
                }
                let dx = b.0.x as f64 - a.0.x as f64;
                let dy = b.0.y as f64 - a.0.y as f64;
                let s = if dx.abs() >= dy.abs() {
                    (pos.x as f64 - a.0.x as f64) / dx
                } else {
                    (pos.y as f64 - a.0.y as f64) / dy
                }
                .clamp(0.0, 1.0) as f32;
                let depth = if transform.camera().projection == Projection::Perspective {
                    1.0 / ((1.0 - s) / a.1 + s / b.1)
                } else {
                    a.1 + (b.1 - a.1) * s
                };
                (
                    transform.point_from_position(pos, depth)?,
                    pos,
                    depth,
                    distance,
                    None,
                )
            }
            Self::Triangle { vertices, weights } => {
                if !self.bounds().contains(pointer) {
                    return None;
                }
                // f64 screen arithmetic also handles very large clipped triangles.
                let [a, b, c] = vertices.map(|v| [v.0.x as f64, v.0.y as f64]);
                let ab = [b[0] - a[0], b[1] - a[1]];
                let ac = [c[0] - a[0], c[1] - a[1]];
                let q = [pointer.x as f64 - a[0], pointer.y as f64 - a[1]];
                let det = ab[0] * ac[1] - ab[1] * ac[0];
                if !det.is_finite() || det.abs() <= f64::EPSILON {
                    return None;
                }
                let v = (q[0] * ac[1] - q[1] * ac[0]) / det;
                let w = (ab[0] * q[1] - ab[1] * q[0]) / det;
                let mut screen_weights = [1.0 - v - w, v, w];
                if screen_weights.iter().any(|w| *w < -1e-7) {
                    return None;
                }
                let depth = if transform.camera().projection == Projection::Perspective {
                    for i in 0..3 {
                        screen_weights[i] /= vertices[i].1 as f64;
                    }
                    let inverse_depth: f64 = screen_weights.iter().sum();
                    for weight in &mut screen_weights {
                        *weight /= inverse_depth;
                    }
                    (1.0 / inverse_depth) as f32
                } else {
                    (0..3)
                        .map(|i| screen_weights[i] * vertices[i].1 as f64)
                        .sum::<f64>() as f32
                };
                let barycentric = std::array::from_fn(|axis| {
                    (0..3).map(|i| screen_weights[i] * weights[i][axis]).sum()
                });
                (
                    transform.point_from_position(pointer, depth)?,
                    pointer,
                    depth,
                    0.0,
                    Some(barycentric),
                )
            }
        };
        if best.is_some_and(|h| {
            !(depth < h.depth - 1e-4 || ((depth - h.depth).abs() < 1e-4 && distance < h.distance))
        }) {
            return None;
        }
        Some(HoveredPoint {
            item_id,
            name: name.to_owned(),
            index,
            position,
            screen_position,
            depth,
            distance,
            barycentric,
        })
    }
}

pub(crate) fn choose(best: &mut Option<HoveredPoint>, hit: HoveredPoint) {
    if best.as_ref().is_none_or(|h| {
        hit.depth < h.depth - 1e-4
            || ((hit.depth - h.depth).abs() < 1e-4 && hit.distance < h.distance)
    }) {
        *best = Some(hit);
    }
}

#[derive(Clone)]
pub(crate) struct PickScene {
    items: Vec<(Id, String)>,
    records: Vec<(usize, usize, PickShape)>,
    cells: Vec<Vec<usize>>,
    broad: Vec<usize>,
    rect: Rect,
    columns: usize,
    rows: usize,
}
impl Default for PickScene {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            records: Vec::new(),
            cells: Vec::new(),
            broad: Vec::new(),
            rect: Rect::NOTHING,
            columns: 0,
            rows: 0,
        }
    }
}
impl PickScene {
    pub fn clear(&mut self) {
        self.items.clear();
        self.records.clear();
        for cell in &mut self.cells {
            cell.clear();
        }
        self.broad.clear();
    }
    pub fn item(&mut self, id: Id, name: &str) -> usize {
        let index = self.items.len();
        self.items.push((id, name.to_owned()));
        index
    }
    pub fn push(&mut self, item: usize, index: usize, shape: PickShape) {
        self.records.push((item, index, shape));
    }
    // A bounded spatial index makes pointer-only redraws independent of total sample count
    // for locally distributed geometry. Large faces use a separate exact-test list.
    pub fn build(&mut self, rect: Rect) {
        self.rect = rect;
        self.columns = ((rect.width() / 32.0).ceil() as usize).clamp(1, 256);
        self.rows = ((rect.height() / 32.0).ceil() as usize).clamp(1, 256);
        self.cells.resize_with(self.columns * self.rows, Vec::new);
        for cell in &mut self.cells {
            cell.clear();
        }
        self.broad.clear();
        let mut entries = 0;
        for (index, (_, _, shape)) in self.records.iter().enumerate() {
            let bounds = shape.bounds().intersect(rect);
            if bounds.is_negative() {
                continue;
            }
            let [left, top] = self.cell(bounds.min);
            let [right, bottom] = self.cell(bounds.max);
            let count = (right - left + 1) * (bottom - top + 1);
            if count > 64 || entries + count > 1_048_576 {
                self.broad.push(index);
            } else {
                entries += count;
                for y in top..=bottom {
                    for x in left..=right {
                        self.cells[y * self.columns + x].push(index);
                    }
                }
            }
        }
    }
    fn cell(&self, p: Pos2) -> [usize; 2] {
        [
            (((p.x - self.rect.left()) / self.rect.width().max(1.0) * self.columns as f32).max(0.0)
                as usize)
                .min(self.columns - 1),
            (((p.y - self.rect.top()) / self.rect.height().max(1.0) * self.rows as f32).max(0.0)
                as usize)
                .min(self.rows - 1),
        ]
    }
    pub fn pick(&self, pointer: Pos2, transform: &PlotTransform) -> Option<HoveredPoint> {
        self.pick_filtered(pointer, transform, false)
    }
    fn pick_filtered(
        &self,
        pointer: Pos2,
        transform: &PlotTransform,
        surfaces_only: bool,
    ) -> Option<HoveredPoint> {
        if self.cells.is_empty() || !self.rect.contains(pointer) {
            return None;
        }
        let [x, y] = self.cell(pointer);
        let mut best = None;
        // Merge the sorted lists to retain submission-order ties, matching an
        // uncached render even when an earlier large face is in the broad list.
        let mut local = self.cells[y * self.columns + x].iter().peekable();
        let mut broad = self.broad.iter().peekable();
        loop {
            let record = match (local.peek(), broad.peek()) {
                (Some(&&a), Some(&&b)) if a <= b => *local.next().unwrap(),
                (Some(_), Some(_)) | (None, Some(_)) => *broad.next().unwrap(),
                (Some(_), None) => *local.next().unwrap(),
                (None, None) => break,
            };
            let (item, index, shape) = &self.records[record];
            if surfaces_only && !matches!(shape, PickShape::Triangle { .. }) {
                continue;
            }
            let (id, name) = &self.items[*item];
            if let Some(hit) = shape.hit(pointer, transform, *id, name, *index, best.as_ref()) {
                choose(&mut best, hit);
            }
        }
        best
    }
}
