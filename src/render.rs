use crate::texture::{Atlas, CIRCLE_UV, SOLID_UV};
use crate::{HoveredPoint, MarkerShape, PlotPoint, PlotTransform};
use egui::{Align2, Color32, FontId, Id, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};

#[derive(Clone)]
pub(crate) enum Geometry {
    Point {
        pos: Pos2,
        radius: f32,
        shape: MarkerShape,
    },
    Line {
        a: Pos2,
        b: Pos2,
        width: f32,
    },
    Triangle([Pos2; 3]),
    Text(Pos2, String),
}
#[derive(Clone)]
pub(crate) struct Primitive {
    depth: f32,
    color: Color32,
    geometry: Geometry,
}

#[derive(Clone, Default)]
pub(crate) struct RenderScratch {
    pub primitives: Vec<Primitive>,
    keys: Vec<u64>,
    temporary: Vec<u64>,
}
impl RenderScratch {
    fn sort(&mut self) {
        self.keys.clear();
        self.keys
            .extend(self.primitives.iter().enumerate().map(|(i, p)| {
                // Positive camera depths have the same IEEE bit and numeric order.
                ((!p.depth.to_bits() as u64) << 32) | i as u64
            }));
        if self.keys.len() < 1024 {
            self.keys.sort_unstable();
            return;
        }
        self.temporary.resize(self.keys.len(), 0);
        // Exact, stable depth ordering in three linear passes over compact keys.
        for shift in [32, 43, 54] {
            let mut offsets = [0_usize; 2048];
            for &key in &self.keys {
                offsets[((key >> shift) & 2047) as usize] += 1;
            }
            let mut total = 0;
            for count in &mut offsets {
                let next = total + *count;
                *count = total;
                total = next;
            }
            for &key in &self.keys {
                let bucket = ((key >> shift) & 2047) as usize;
                self.temporary[offsets[bucket]] = key;
                offsets[bucket] += 1;
            }
            std::mem::swap(&mut self.keys, &mut self.temporary);
        }
    }
}

/// Geometry sink for custom [`crate::PlotItem`] implementations. Coordinates are in data space.
pub struct PlotItemContext<'a> {
    pub(crate) transform: &'a PlotTransform,
    pub(crate) primitives: &'a mut Vec<Primitive>,
    pub(crate) pointer: Option<Pos2>,
    pub(crate) hovered: &'a mut Option<HoveredPoint>,
    pub(crate) item_id: Id,
    pub(crate) name: &'a str,
    pub(crate) color: Color32,
    pub(crate) hover: bool,
    pub(crate) depth_cue: bool,
}
impl PlotItemContext<'_> {
    /// Reserve projected geometry storage for a batch.
    pub fn reserve(&mut self, count: usize) {
        self.primitives.reserve(count);
    }
    pub fn color(&self) -> Color32 {
        self.color
    }
    pub fn transform(&self) -> &PlotTransform {
        self.transform
    }
    fn push(&mut self, depth: f32, color: Color32, geometry: Geometry) {
        let color = if self.depth_cue {
            color.gamma_multiply((1.3 - depth * 0.09).clamp(0.45, 1.0))
        } else {
            color
        };
        self.primitives.push(Primitive {
            depth,
            color,
            geometry,
        });
    }
    fn pick(&mut self, pos: Pos2, depth: f32, point: PlotPoint, index: usize, radius: f32) {
        if !self.hover {
            return;
        }
        let Some(pointer) = self.pointer else {
            return;
        };
        let distance_sq = pointer.distance_sq(pos);
        if distance_sq > radius.max(5.0).powi(2) {
            return;
        }
        let distance = distance_sq.sqrt();
        if distance <= radius.max(5.0)
            && self.hovered.as_ref().is_none_or(|h| {
                depth < h.depth - 1e-4 || ((depth - h.depth).abs() < 1e-4 && distance < h.distance)
            })
        {
            *self.hovered = Some(HoveredPoint {
                item_id: self.item_id,
                name: self.name.to_owned(),
                index,
                position: point,
                screen_position: pos,
                depth,
                distance,
            });
        }
    }
    pub fn point(
        &mut self,
        point: PlotPoint,
        radius: f32,
        shape: MarkerShape,
        color: Color32,
        index: usize,
    ) {
        if !radius.is_finite() || radius <= 0.0 {
            return;
        }
        let Some((pos, depth)) = self.transform.project(point) else {
            return;
        };
        if !self.transform.rect.expand(radius).contains(pos) {
            return;
        }
        self.pick(pos, depth, point, index, radius);
        self.push(depth, color, Geometry::Point { pos, radius, shape });
    }
    pub fn line(&mut self, a: PlotPoint, b: PlotPoint, width: f32, color: Color32, index: usize) {
        if !width.is_finite() || width <= 0.0 {
            return;
        }
        let Some([pa, pb]) = self.transform.segment(a, b) else {
            return;
        };
        self.projected_line(pa, pb, width, color, index);
    }
    /// Submit a connected line, projecting shared vertices only once.
    pub fn polyline(&mut self, points: &[[f64; 3]], width: f32, color: Color32) {
        if !width.is_finite() || width <= 0.0 {
            return;
        }
        let Some(&first) = points.first() else {
            return;
        };
        let mut previous = self.transform.project(first.into());
        self.primitives.reserve(points.len().saturating_sub(1));
        for (i, pair) in points.windows(2).enumerate() {
            let current = self.transform.project(pair[1].into());
            if let (Some(a), Some(b)) = (previous, current) {
                self.projected_line(a, b, width, color, i);
            } else if let Some([a, b]) = self.transform.segment(pair[0].into(), pair[1].into()) {
                self.projected_line(a, b, width, color, i);
            }
            previous = current;
        }
    }
    fn projected_line(
        &mut self,
        pa: (Pos2, f32),
        pb: (Pos2, f32),
        width: f32,
        color: Color32,
        index: usize,
    ) {
        let Some((start, end)) = clip_segment(pa.0, pb.0, self.transform.rect.expand(width)) else {
            return;
        };
        if let Some(pointer) = self.pointer.filter(|p| {
            self.hover
                && Rect::from_two_pos(start, end)
                    .expand((width * 0.5 + 3.0).max(5.0))
                    .contains(*p)
        }) {
            let delta = end - start;
            let t = ((pointer - start).dot(delta) / delta.length_sq().max(1e-12)).clamp(0.0, 1.0);
            let pos = start + delta * t;
            let original_delta = pb.0 - pa.0;
            let s = ((pos - pa.0).dot(original_delta) / original_delta.length_sq().max(1e-12))
                .clamp(0.0, 1.0);
            let depth = if self.transform.camera.projection == crate::Projection::Perspective {
                1.0 / ((1.0 - s) / pa.1 + s / pb.1)
            } else {
                pa.1 + (pb.1 - pa.1) * s
            };
            if let Some(point) = self.transform.point_from_position(pos, depth) {
                self.pick(pos, depth, point, index, width * 0.5 + 3.0);
            }
        }
        self.push(
            (pa.1 + pb.1) * 0.5,
            color,
            Geometry::Line {
                a: start,
                b: end,
                width,
            },
        );
    }
    pub fn arrow(
        &mut self,
        a: PlotPoint,
        b: PlotPoint,
        width: f32,
        tip_length: f32,
        color: Color32,
        index: usize,
    ) {
        if !width.is_finite() || width <= 0.0 || a == b {
            return;
        }
        self.line(a, b, width, color, index);
        if !tip_length.is_finite() || tip_length <= 0.0 || self.transform.project(b).is_none() {
            return;
        }
        let Some([a, b]) = self.transform.segment(a, b) else {
            return;
        };
        if !self.transform.rect.contains(b.0) {
            return;
        }
        if a.0.distance_sq(b.0) < 1e-12 {
            return;
        }
        let d = (b.0 - a.0).normalized() * tip_length.min(a.0.distance(b.0) * 0.4);
        let n = Vec2::new(-d.y, d.x) * 0.45;
        self.push(
            b.1,
            color,
            Geometry::Triangle([b.0, b.0 - d + n, b.0 - d - n]),
        );
    }
    pub fn triangle(&mut self, points: [PlotPoint; 3], color: Color32) {
        if points.iter().any(|p| !p.is_finite()) {
            return;
        }
        let input = points.map(|p| self.transform.view(p));
        if input.iter().flatten().any(|v| !v.is_finite()) {
            return;
        }
        // Clip against the camera near plane before perspective division.
        // A triangle can become a quad, but never needs a heap allocation.
        let mut clipped = [[0.0; 3]; 4];
        let mut len = 0;
        let mut previous = input[2];
        for current in input {
            if (previous[2] >= 0.05) != (current[2] >= 0.05) {
                let t = (0.05 - previous[2]) / (current[2] - previous[2]);
                clipped[len] =
                    std::array::from_fn(|i| previous[i] + t * (current[i] - previous[i]));
                clipped[len][2] = 0.05;
                len += 1;
            }
            if current[2] >= 0.05 {
                clipped[len] = current;
                len += 1;
            }
            previous = current;
        }
        for i in 1..len.saturating_sub(1) {
            let projected =
                [clipped[0], clipped[i], clipped[i + 1]].map(|v| self.transform.project_view(v));
            if let [Some(a), Some(b), Some(c)] = projected {
                self.push(
                    (a.1 + b.1 + c.1) / 3.0,
                    color,
                    Geometry::Triangle([a.0, b.0, c.0]),
                );
            }
        }
    }
    pub fn text(&mut self, position: PlotPoint, text: String, color: Color32) {
        if let Some((pos, depth)) = self.transform.project(position)
            && self.transform.rect.contains(pos)
        {
            self.push(depth, color, Geometry::Text(pos, text));
        }
    }
}

/// Liang-Barsky clipping avoids huge offscreen mesh coordinates when zoomed.
fn clip_segment(a: Pos2, b: Pos2, rect: Rect) -> Option<(Pos2, Pos2)> {
    if rect.contains(a) && rect.contains(b) {
        return Some((a, b));
    }
    let d = b - a;
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for (p, q) in [
        (-d.x, a.x - rect.left()),
        (d.x, rect.right() - a.x),
        (-d.y, a.y - rect.top()),
        (d.y, rect.bottom() - a.y),
    ] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                low = low.max(t);
            } else {
                high = high.min(t);
            }
        }
    }
    (low <= high).then_some((a + d * low, a + d * high))
}

pub(crate) fn paint(painter: &Painter, scratch: &mut RenderScratch) -> usize {
    scratch.sort();
    paint_sorted(painter, scratch)
}

fn paint_sorted(painter: &Painter, scratch: &mut RenderScratch) -> usize {
    let count = scratch.primitives.len();
    let mut atlas = Atlas::load(painter.ctx());
    let mut mesh = Mesh::with_texture(atlas.texture.id());
    let (vertices, indices) = scratch.primitives.iter().fold((0, 0), |(v, i), p| {
        let (dv, di) = match p.geometry {
            Geometry::Point {
                shape: MarkerShape::Circle,
                ..
            } => (4, 6),
            Geometry::Point {
                shape: MarkerShape::Cross,
                ..
            } => {
                if atlas.ensure_line(1.5) {
                    (8, 12)
                } else {
                    (16, 36)
                }
            }
            Geometry::Point { .. } => (9, 36),
            Geometry::Line { width, .. } => {
                if atlas.ensure_line(width) {
                    (4, 6)
                } else {
                    (8, 18)
                }
            }
            Geometry::Triangle(_) => (3, 3),
            Geometry::Text(..) => (0, 0),
        };
        (v + dv, i + di)
    });
    mesh.vertices.reserve_exact(vertices);
    mesh.indices.reserve_exact(indices);
    for &key in &scratch.keys {
        let p = &scratch.primitives[key as u32 as usize];
        match &p.geometry {
            Geometry::Point { pos, radius, shape } => {
                marker(&mut mesh, *pos, *radius, *shape, p.color, &atlas)
            }
            Geometry::Line { a, b, width } => {
                segment_mesh(&mut mesh, *a, *b, *width, p.color, atlas.line_uv(*width))
            }
            Geometry::Triangle(v) => triangle_mesh(&mut mesh, *v, p.color),
            Geometry::Text(pos, text) => {
                if !mesh.is_empty() {
                    painter.add(Shape::mesh(std::mem::replace(
                        &mut mesh,
                        Mesh::with_texture(atlas.texture.id()),
                    )));
                }
                painter.text(
                    *pos,
                    Align2::CENTER_CENTER,
                    text,
                    FontId::proportional(13.0),
                    p.color,
                );
            }
        }
    }
    if !mesh.is_empty() {
        painter.add(Shape::mesh(mesh));
    }
    scratch.primitives.clear();
    atlas.store(painter.ctx());
    count
}

fn vertex(mesh: &mut Mesh, pos: Pos2, color: Color32) {
    mesh.vertices.push(egui::epaint::Vertex {
        pos,
        uv: SOLID_UV,
        color,
    });
}
fn triangle_mesh(mesh: &mut Mesh, points: [Pos2; 3], color: Color32) {
    let base = mesh.vertices.len() as u32;
    for p in points {
        vertex(mesh, p, color);
    }
    mesh.add_triangle(base, base + 1, base + 2);
}
fn segment_mesh(
    mesh: &mut Mesh,
    a: Pos2,
    b: Pos2,
    width: f32,
    color: Color32,
    line_uv: Option<f32>,
) {
    let d = b - a;
    if d.length_sq() < 1e-12 {
        return;
    }
    let n = Vec2::new(-d.y, d.x).normalized();
    let base = mesh.vertices.len() as u32;
    if let Some(v) = line_uv {
        let fringe = n * (width * 0.5 + 0.7);
        let low = Pos2::new(0.5 / 256.0, v);
        let high = Pos2::new(255.5 / 256.0, v);
        mesh.vertices.extend_from_slice(&[
            egui::epaint::Vertex {
                pos: a - fringe,
                uv: low,
                color,
            },
            egui::epaint::Vertex {
                pos: b - fringe,
                uv: low,
                color,
            },
            egui::epaint::Vertex {
                pos: b + fringe,
                uv: high,
                color,
            },
            egui::epaint::Vertex {
                pos: a + fringe,
                uv: high,
                color,
            },
        ]);
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        return;
    }
    // Transparent fringes give antialiasing without per-segment egui tessellation.
    let vertices = [
        (-width * 0.5 - 0.7, Color32::TRANSPARENT),
        (-width * 0.5, color),
        (width * 0.5, color),
        (width * 0.5 + 0.7, Color32::TRANSPARENT),
    ]
    .map(|(offset, color)| {
        [
            egui::epaint::Vertex {
                pos: a + n * offset,
                uv: SOLID_UV,
                color,
            },
            egui::epaint::Vertex {
                pos: b + n * offset,
                uv: SOLID_UV,
                color,
            },
        ]
    });
    mesh.vertices.extend_from_slice(vertices.as_flattened());
    mesh.indices.extend_from_slice(&[
        base,
        base + 1,
        base + 2,
        base + 1,
        base + 3,
        base + 2,
        base + 2,
        base + 3,
        base + 4,
        base + 3,
        base + 5,
        base + 4,
        base + 4,
        base + 5,
        base + 6,
        base + 5,
        base + 7,
        base + 6,
    ]);
}
fn marker(
    mesh: &mut Mesh,
    pos: Pos2,
    radius: f32,
    shape: MarkerShape,
    color: Color32,
    atlas: &Atlas,
) {
    if shape == MarkerShape::Circle {
        mesh.add_rect_with_uv(
            Rect::from_center_size(pos, Vec2::splat(radius * 2.0 + 1.0)),
            CIRCLE_UV,
            color,
        );
        return;
    }
    if shape == MarkerShape::Cross {
        segment_mesh(
            mesh,
            pos - Vec2::splat(radius * 0.7),
            pos + Vec2::splat(radius * 0.7),
            1.5,
            color,
            atlas.line_uv(1.5),
        );
        let d = Vec2::new(radius * 0.7, -radius * 0.7);
        segment_mesh(mesh, pos - d, pos + d, 1.5, color, atlas.line_uv(1.5));
        return;
    }
    // Fixed unit circle avoids trigonometry per point.
    const CIRCLE: [Vec2; 12] = [
        Vec2::new(1.0, 0.0),
        Vec2::new(0.8660254, 0.5),
        Vec2::new(0.5, 0.8660254),
        Vec2::new(0.0, 1.0),
        Vec2::new(-0.5, 0.8660254),
        Vec2::new(-0.8660254, 0.5),
        Vec2::new(-1.0, 0.0),
        Vec2::new(-0.8660254, -0.5),
        Vec2::new(-0.5, -0.8660254),
        Vec2::new(0.0, -1.0),
        Vec2::new(0.5, -0.8660254),
        Vec2::new(0.8660254, -0.5),
    ];
    const SQUARE: [Vec2; 4] = [
        Vec2::new(-1.0, -1.0),
        Vec2::new(1.0, -1.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(-1.0, 1.0),
    ];
    const DIAMOND: [Vec2; 4] = [
        Vec2::new(0.0, -1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(-1.0, 0.0),
    ];
    let unit = match shape {
        MarkerShape::Circle => CIRCLE.as_slice(),
        MarkerShape::Square => &SQUARE,
        _ => &DIAMOND,
    };
    let base = mesh.vertices.len() as u32;
    vertex(mesh, pos, color);
    for &v in unit {
        vertex(mesh, pos + v * radius, color);
        vertex(mesh, pos + v * (radius + 0.7), Color32::TRANSPARENT);
    }
    for i in 0..unit.len() as u32 {
        let a = base + 1 + i * 2;
        let b = base + 1 + ((i + 1) % unit.len() as u32) * 2;
        mesh.add_triangle(base, a, b);
        mesh.add_triangle(a, a + 1, b);
        mesh.add_triangle(a + 1, b + 1, b);
    }
}

pub(crate) fn guides(
    painter: &Painter,
    transform: &PlotTransform,
    grid: bool,
    axes: [bool; 3],
    labels: &[String; 3],
    frame: bool,
) {
    let min = transform.bounds.min();
    let max = transform.bounds.max();
    let weak = painter
        .ctx()
        .style_of(painter.ctx().theme())
        .visuals
        .weak_text_color()
        .gamma_multiply(0.22);
    let draw = |a: [f64; 3], b: [f64; 3], color: Color32| {
        if let Some([a, b]) = transform.segment(a.into(), b.into())
            && let Some((a, b)) = clip_segment(a.0, b.0, transform.rect)
        {
            painter.line_segment([a, b], Stroke::new(1.0, color));
        }
    };
    if grid {
        for i in 0..=8 {
            let t = i as f64 / 8.0;
            let x = min[0] * (1.0 - t) + max[0] * t;
            let y = min[1] * (1.0 - t) + max[1] * t;
            draw([x, min[1], min[2]], [x, max[1], min[2]], weak);
            draw([min[0], y, min[2]], [max[0], y, min[2]], weak);
        }
    }
    if frame {
        for axis in 0..3 {
            for i in 0..4 {
                let mut a = min;
                let mut b = min;
                b[axis] = max[axis];
                for (j, k) in (0..3).filter(|&j| j != axis).enumerate() {
                    if i & (1 << j) != 0 {
                        a[k] = max[k];
                        b[k] = max[k];
                    }
                }
                draw(a, b, weak);
            }
        }
    }
    let colors = [
        Color32::from_rgb(225, 116, 125),
        Color32::from_rgb(128, 190, 134),
        Color32::from_rgb(115, 158, 231),
    ];
    for axis in 0..3 {
        if !axes[axis] {
            continue;
        }
        let mut end = min;
        end[axis] = max[axis];
        draw(min, end, colors[axis]);
        // The three axes share their lower corner. Repeating all three lower
        // bound labels there would make them unreadable.
        for step in 1..=4 {
            let t = step as f64 / 4.0;
            let mut p = min;
            p[axis] = min[axis] * (1.0 - t) + max[axis] * t;
            if let Some(pos) = transform.position_from_point(p) {
                painter.text(
                    pos + Vec2::new(4.0, 8.0),
                    Align2::LEFT_TOP,
                    format_number(p[axis]),
                    FontId::monospace(10.0),
                    colors[axis],
                );
            }
        }
        if let Some(pos) = transform.position_from_point(end) {
            painter.text(
                pos + Vec2::new(6.0, -12.0),
                Align2::LEFT_BOTTOM,
                &labels[axis],
                FontId::proportional(13.0),
                colors[axis],
            );
        }
    }
}
fn format_number(v: f64) -> String {
    if v != 0.0 && !(0.001..10000.0).contains(&v.abs()) {
        format!("{v:.1e}")
    } else {
        format!("{v:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn depth_keys_match_stable_numeric_order() {
        for count in [0, 1, 1023, 1024, 100_000] {
            let mut scratch = RenderScratch::default();
            for i in 0..count {
                let depth = match i % 7 {
                    0 => 0.05,
                    1 => f32::MAX,
                    2 => 4.0,
                    _ => 0.05 + ((i * 7919) % 10007) as f32 / 100.0,
                };
                scratch.primitives.push(Primitive {
                    depth,
                    color: Color32::WHITE,
                    geometry: Geometry::Text(Pos2::ZERO, String::new()),
                });
            }
            let mut expected: Vec<_> = (0..count).collect();
            expected.sort_by(|&a, &b| {
                scratch.primitives[b]
                    .depth
                    .total_cmp(&scratch.primitives[a].depth)
            });
            scratch.sort();
            assert_eq!(
                scratch
                    .keys
                    .iter()
                    .map(|key| *key as u32 as usize)
                    .collect::<Vec<_>>(),
                expected
            );
            scratch.sort(); // Reuse the scratch allocation without stale ordering.
            assert_eq!(
                scratch
                    .keys
                    .iter()
                    .map(|key| *key as u32 as usize)
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
    #[test]
    fn shared_vertex_projection_preserves_clipping_and_breaks() {
        let t = PlotTransform::new(
            Rect::from_min_size(Pos2::ZERO, Vec2::splat(500.0)),
            crate::PlotBounds::from_min_max([-1.0; 3], [1.0; 3]),
            crate::Camera::default(),
            true,
        );
        let points = [
            [0.0; 3],
            [0.3, 0.2, 0.1],
            [0.0, 10.0, 10.0],
            [0.2, 0.2, 0.2],
            [f64::NAN; 3],
            [0.5, 0.0, 0.0],
            [0.6, 0.0, 0.0],
        ];
        let mut batches = Vec::new();
        for shared in [false, true] {
            let mut primitives = Vec::new();
            let mut hovered = None;
            let mut context = PlotItemContext {
                transform: &t,
                primitives: &mut primitives,
                pointer: None,
                hovered: &mut hovered,
                item_id: Id::new("test"),
                name: "data",
                color: Color32::WHITE,
                hover: false,
                depth_cue: false,
            };
            if shared {
                context.polyline(&points, 1.5, Color32::WHITE);
            } else {
                for (i, p) in points.windows(2).enumerate() {
                    context.line(p[0].into(), p[1].into(), 1.5, Color32::WHITE, i);
                }
            }
            batches.push(primitives);
        }
        assert_eq!(batches[0].len(), batches[1].len());
        assert!(!batches[0].is_empty());
        for (a, b) in batches[0].iter().zip(&batches[1]) {
            assert_eq!(a.depth, b.depth);
            if let (Geometry::Line { a: a0, b: a1, .. }, Geometry::Line { a: b0, b: b1, .. }) =
                (&a.geometry, &b.geometry)
            {
                assert_eq!((a0, a1), (b0, b1));
            } else {
                panic!("expected segments");
            }
        }
    }
}
