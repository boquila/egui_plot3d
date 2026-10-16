use crate::{PlotBounds, PlotPoint};
use egui::{Pos2, Rect, Vec2, pos2, vec2};

/// World axis displayed vertically. Coordinates and sample indices stay unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UpAxis {
    /// X up, with Y/Z forming the floor.
    X,
    /// Y up, with X/Z forming the floor.
    Y,
    /// Z up, with X/Y forming the floor (default).
    #[default]
    Z,
}
impl UpAxis {
    pub(crate) fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
    fn orient(self, p: [f32; 3]) -> [f32; 3] {
        match self {
            Self::X => [p[1], p[2], p[0]],
            Self::Y => [p[0], -p[2], p[1]],
            Self::Z => p,
        }
    }
    fn unorient(self, p: [f32; 3]) -> [f32; 3] {
        match self {
            Self::X => [p[2], p[0], p[1]],
            Self::Y => [p[0], p[2], -p[1]],
            Self::Z => p,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Camera projection model.
pub enum Projection {
    #[default]
    /// Distant geometry appears smaller (default).
    Perspective,
    /// Parallel projection; distances do not change apparent size.
    Orthographic,
}

/// Orbit camera. Angles are radians, pan is in plot-size units, zoom is a multiplier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// Orbit azimuth in radians.
    pub yaw: f32,
    /// Orbit elevation in radians, clamped to -1.55..=1.55 when used.
    pub pitch: f32,
    /// Zoom multiplier, clamped to 0.02..=100 when used.
    pub zoom: f32,
    /// Screen displacement in fractions of the plot width and height.
    pub pan: Vec2,
    /// Perspective or orthographic projection.
    pub projection: Projection,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: -0.65,
            pitch: 0.45,
            zoom: 1.0,
            pan: Vec2::ZERO,
            projection: Projection::Perspective,
        }
    }
}
impl Camera {
    /// Apply a primary-drag delta in logical pixels; yaw wraps and pitch stays away from the poles.
    pub fn orbit(&mut self, delta: Vec2) {
        self.yaw = (self.yaw - delta.x * 0.008).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + delta.y * 0.008).clamp(-1.55, 1.55);
    }
    /// Multiply zoom by a positive finite factor, clamping the result to 0.02..=100.
    pub fn zoom_by(&mut self, factor: f32) {
        if factor.is_finite() && factor > 0.0 {
            self.zoom = (self.zoom * factor).clamp(0.02, 100.0);
        }
    }
    /// Zoom while keeping the world ray under a logical screen position fixed.
    /// `rect` is the plot frame; pan remains expressed in plot-size units.
    pub fn zoom_at(&mut self, factor: f32, position: Pos2, rect: Rect) {
        if !position.is_finite() || !rect.is_finite() || rect.width() <= 0.0 || rect.height() <= 0.0
        {
            return;
        }
        *self = self.sanitized();
        let before = self.zoom;
        self.zoom_by(factor);
        let center = rect.center() + self.pan * rect.size();
        let center = position + (center - position) * (self.zoom / before);
        self.pan = (center - rect.center()) / rect.size();
    }
    pub(crate) fn sanitized(mut self) -> Self {
        if !self.yaw.is_finite() {
            self.yaw = Self::default().yaw;
        }
        if !self.pitch.is_finite() {
            self.pitch = 0.45;
        }
        if !self.zoom.is_finite() {
            self.zoom = 1.0;
        }
        if !self.pan.is_finite() {
            self.pan = Vec2::ZERO;
        }
        self.pitch = self.pitch.clamp(-1.55, 1.55);
        self.zoom = self.zoom.clamp(0.02, 100.0);
        self
    }
}

/// The exact world-to-screen transform used for drawing and picking.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotTransform {
    pub(crate) rect: Rect,
    pub(crate) bounds: PlotBounds,
    pub(crate) camera: Camera,
    center: [f64; 3],
    scale: [f64; 3],
    extent: [f64; 3],
    screen_center: Pos2,
    screen_scale: f32,
    sy: f32,
    cy: f32,
    sp: f32,
    cp: f32,
    up_axis: UpAxis,
}
impl PlotTransform {
    /// Build a transform for a screen rectangle, fitted data bounds, and camera. Equal aspect preserves equal world units.
    pub fn new(rect: Rect, bounds: PlotBounds, camera: Camera, equal_aspect: bool) -> Self {
        let bounds = bounds.fitted();
        let camera = camera.sanitized();
        let center = bounds.center();
        // Half extents avoid overflow for very large but finite coordinates.
        let half = half_extents(bounds);
        let radius = half.into_iter().fold(f64::from_bits(1), f64::max);
        let extent = if equal_aspect { [radius; 3] } else { half };
        let scale = extent.map(|v| 1.0 / v);
        let (sy, cy) = camera.yaw.sin_cos();
        let (sp, cp) = camera.pitch.sin_cos();
        Self {
            rect,
            bounds,
            camera,
            center,
            scale,
            extent,
            screen_center: rect.center() + camera.pan * rect.size(),
            screen_scale: rect.width().min(rect.height()) * 0.28 * camera.zoom,
            sy,
            cy,
            sp,
            cp,
            up_axis: UpAxis::Z,
        }
    }
    /// Screen rectangle allocated to the plot, in logical pixels.
    pub fn frame(&self) -> Rect {
        self.rect
    }
    /// Choose the world axis displayed vertically without rewriting coordinates.
    pub fn with_up_axis(mut self, axis: UpAxis) -> Self {
        self.up_axis = axis;
        self
    }
    /// World axis displayed vertically.
    pub fn up_axis(&self) -> UpAxis {
        self.up_axis
    }
    /// Relative lengths of world units on X, Y, and Z, before camera rotation.
    /// Invalid ratios leave the transform unchanged.
    pub fn with_axis_aspect(mut self, aspect: [f64; 3]) -> Self {
        if aspect.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return self;
        }
        let largest = aspect.into_iter().fold(0.0, f64::max);
        let aspect = aspect.map(|v| (v / largest).max(f64::from_bits(1)));
        let half = half_extents(self.bounds);
        let radius = (0..3)
            .map(|i| half[i] * aspect[i])
            .fold(f64::from_bits(1), f64::max);
        self.scale = aspect.map(|v| v / radius);
        self.extent = aspect.map(|v| radius / v);
        self
    }
    /// Fitted world-space bounds used by this transform.
    pub fn bounds(&self) -> PlotBounds {
        self.bounds
    }
    /// Sanitized camera used by this transform.
    pub fn camera(&self) -> Camera {
        self.camera
    }
    pub(crate) fn view(&self, p: PlotPoint) -> [f32; 3] {
        let p = p.to_array();
        let v: [f32; 3] = std::array::from_fn(|i| {
            // Subtract before scaling to retain small differences at large offsets.
            // Exceptional ranges use division to avoid reciprocal or subtraction overflow.
            let delta = p[i] - self.center[i];
            let value = if !delta.is_finite() {
                p[i] / self.extent[i] - self.center[i] / self.extent[i]
            } else if self.scale[i].is_finite() {
                delta * self.scale[i]
            } else {
                delta / self.extent[i]
            };
            value as f32
        });
        let v = self.up_axis.orient(v);
        let x = v[0] * self.cy - v[1] * self.sy;
        let y = v[0] * self.sy + v[1] * self.cy;
        [
            x,
            v[2] * self.cp - y * self.sp,
            4.0 - (y * self.cp + v[2] * self.sp),
        ]
    }
    pub(crate) fn scaled_direction(&self, a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        std::array::from_fn(|i| (b[i] - a[i]) / self.extent[i])
    }
    pub(crate) fn project_view(&self, v: [f32; 3]) -> Option<(Pos2, f32)> {
        if !v.iter().all(|x| x.is_finite()) || v[2] < 0.05 {
            return None;
        }
        let scale = self.pixel_scale()
            * if self.camera.projection == Projection::Perspective {
                4.0 / v[2]
            } else {
                1.0
            };
        let c = self.screen_center;
        let pos = pos2(c.x + v[0] * scale, c.y - v[1] * scale);
        pos.is_finite().then_some((pos, v[2]))
    }
    fn pixel_scale(&self) -> f32 {
        self.screen_scale
    }
    /// Project a world point to logical screen pixels; returns `None` behind the near plane or for non-finite results.
    pub fn position_from_point(&self, point: impl Into<PlotPoint>) -> Option<Pos2> {
        self.project(point.into()).map(|p| p.0)
    }
    /// Project a world point, returning logical screen pixels and camera-space depth.
    pub fn project(&self, point: PlotPoint) -> Option<(Pos2, f32)> {
        if !point.is_finite() {
            return None;
        }
        self.project_view(self.view(point))
    }
    /// Unproject at camera-space depth. A screen point alone cannot specify a 3D point.
    pub fn point_from_position(&self, position: Pos2, depth: f32) -> Option<PlotPoint> {
        if !position.is_finite() || !depth.is_finite() || depth < 0.05 {
            return None;
        }
        let c = self.screen_center;
        let s = self.pixel_scale()
            * if self.camera.projection == Projection::Perspective {
                4.0 / depth
            } else {
                1.0
            };
        if s <= 0.0 {
            return None;
        }
        let x = (position.x - c.x) / s;
        let up = -(position.y - c.y) / s;
        let forward = 4.0 - depth;
        let y = forward * self.cp - up * self.sp;
        let z = forward * self.sp + up * self.cp;
        let v = [x * self.cy + y * self.sy, -x * self.sy + y * self.cy, z];
        let v = self.up_axis.unorient(v);
        let point: PlotPoint = std::array::from_fn::<_, 3, _>(|i| {
            (v[i] as f64).mul_add(self.extent[i], self.center[i])
        })
        .into();
        point.is_finite().then_some(point)
    }
    pub(crate) fn segment(&self, a: PlotPoint, b: PlotPoint) -> Option<[(Pos2, f32); 2]> {
        if !a.is_finite() || !b.is_finite() {
            return None;
        }
        let (mut a, mut b) = (self.view(a), self.view(b));
        if a[2] < 0.05 && b[2] < 0.05 {
            return None;
        }
        if a[2] < 0.05 || b[2] < 0.05 {
            let t = (0.05 - a[2]) / (b[2] - a[2]);
            let mut p = std::array::from_fn(|i| a[i] + t * (b[i] - a[i]));
            p[2] = 0.05;
            if a[2] < 0.05 {
                a = p;
            } else {
                b = p;
            }
        }
        Some([self.project_view(a)?, self.project_view(b)?])
    }
    /// Convert a logical pixel displacement to the normalized pan units used by [`Camera`].
    pub fn screen_delta_to_pan(&self, delta: Vec2) -> Vec2 {
        vec2(
            delta.x / self.rect.width().max(1.0),
            delta.y / self.rect.height().max(1.0),
        )
    }
}

fn half_extents(bounds: PlotBounds) -> [f64; 3] {
    std::array::from_fn(|i| {
        let span = bounds.max()[i] - bounds.min()[i];
        let half = if span.is_finite() {
            span * 0.5
        } else {
            bounds.max()[i] * 0.5 - bounds.min()[i] * 0.5
        };
        half.max(f64::from_bits(1))
    })
}
