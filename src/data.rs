use std::sync::Arc;

/// A point in a right-handed coordinate system (Z up).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlotPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl PlotPoint {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub fn to_array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
    pub fn is_finite(self) -> bool {
        self.to_array().iter().all(|v| v.is_finite())
    }
}
impl From<[f64; 3]> for PlotPoint {
    fn from(p: [f64; 3]) -> Self {
        Self::new(p[0], p[1], p[2])
    }
}
impl From<(f64, f64, f64)> for PlotPoint {
    fn from((x, y, z): (f64, f64, f64)) -> Self {
        Self::new(x, y, z)
    }
}
impl From<PlotPoint> for [f64; 3] {
    fn from(p: PlotPoint) -> Self {
        p.to_array()
    }
}

/// Finite data bounds. Invalid/empty bounds are fitted to [-1, 1].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlotBounds {
    min: [f64; 3],
    max: [f64; 3],
}
impl Default for PlotBounds {
    fn default() -> Self {
        Self::NOTHING
    }
}
impl PlotBounds {
    pub const NOTHING: Self = Self {
        min: [f64::INFINITY; 3],
        max: [f64::NEG_INFINITY; 3],
    };
    pub const fn from_min_max(min: [f64; 3], max: [f64; 3]) -> Self {
        Self { min, max }
    }
    pub fn min(&self) -> [f64; 3] {
        self.min
    }
    pub fn max(&self) -> [f64; 3] {
        self.max
    }
    pub fn is_valid(&self) -> bool {
        (0..3).all(|i| {
            self.min[i].is_finite() && self.max[i].is_finite() && self.min[i] <= self.max[i]
        })
    }
    pub fn extend_with(&mut self, p: impl Into<PlotPoint>) {
        let p = p.into();
        if p.is_finite() {
            for (i, v) in p.to_array().into_iter().enumerate() {
                self.min[i] = self.min[i].min(v);
                self.max[i] = self.max[i].max(v);
            }
        }
    }
    pub fn merge(&mut self, other: Self) {
        if other.is_valid() {
            self.extend_with(other.min);
            self.extend_with(other.max);
        }
    }
    pub fn extend_with_x(&mut self, x: f64) {
        self.extend_axis(0, x);
    }
    pub fn extend_with_y(&mut self, y: f64) {
        self.extend_axis(1, y);
    }
    pub fn extend_with_z(&mut self, z: f64) {
        self.extend_axis(2, z);
    }
    fn extend_axis(&mut self, axis: usize, value: f64) {
        if value.is_finite() {
            self.min[axis] = self.min[axis].min(value);
            self.max[axis] = self.max[axis].max(value);
        }
    }
    pub(crate) fn include_partial(&mut self, other: Self) {
        for i in 0..3 {
            self.extend_axis(i, other.min[i]);
            self.extend_axis(i, other.max[i]);
        }
    }
    pub(crate) fn set_axis(&mut self, axis: usize, min: f64, max: f64) {
        if min.is_finite() && max.is_finite() && min <= max {
            self.min[axis] = min;
            self.max[axis] = max;
        }
    }
    pub fn center(&self) -> [f64; 3] {
        std::array::from_fn(|i| {
            let span = self.max[i] - self.min[i];
            if span.is_finite() {
                self.min[i] + span * 0.5
            } else {
                self.min[i] * 0.5 + self.max[i] * 0.5
            }
        })
    }
    pub fn size(&self) -> [f64; 3] {
        std::array::from_fn(|i| self.max[i] - self.min[i])
    }
    pub(crate) fn fitted(self) -> Self {
        if !self.is_valid() {
            return Self::from_min_max([-1.0; 3], [1.0; 3]);
        }
        let mut b = self;
        let data_half_extent = (0..3)
            .map(|i| {
                let span = self.max[i] - self.min[i];
                if span.is_finite() {
                    span * 0.5
                } else {
                    self.max[i] * 0.5 - self.min[i] * 0.5
                }
            })
            .fold(0.0, f64::max);
        for i in 0..3 {
            if b.max[i] == b.min[i] {
                let value = b.min[i];
                let pad = if data_half_extent > 0.0 {
                    data_half_extent
                } else if value == 0.0 {
                    0.5
                } else {
                    (value.abs() * 0.05).max(f64::from_bits(1))
                };
                b.min[i] = (value - pad).max(-f64::MAX);
                b.max[i] = (value + pad).min(f64::MAX);
                if b.min[i] == b.max[i] {
                    b.min[i] = value.next_down().max(-f64::MAX);
                    b.max[i] = value.next_up().min(f64::MAX);
                }
            }
        }
        b
    }
}

/// Immutable, cheaply cloned data with precomputed bounds. Build once for large static plots.
#[derive(Clone, Debug)]
pub struct PointBuffer {
    points: Arc<[[f64; 3]]>,
    bounds: PlotBounds,
}
impl PointBuffer {
    pub fn new(points: impl Into<Arc<[[f64; 3]]>>) -> Self {
        let points = points.into();
        let mut bounds = PlotBounds::NOTHING;
        for &p in points.iter() {
            bounds.extend_with(p);
        }
        Self { points, bounds }
    }
    pub fn as_slice(&self) -> &[[f64; 3]] {
        &self.points
    }
    pub fn bounds(&self) -> PlotBounds {
        self.bounds
    }
    pub fn len(&self) -> usize {
        self.points.len()
    }
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}
impl From<Vec<[f64; 3]>> for PointBuffer {
    fn from(p: Vec<[f64; 3]>) -> Self {
        Self::new(p)
    }
}

/// Borrowed, owned, or shared coordinates. Borrowing never copies the data.
#[derive(Clone, Debug)]
pub enum PlotPoints<'a> {
    Borrowed(&'a [[f64; 3]]),
    Owned(Vec<[f64; 3]>),
    Shared(PointBuffer),
}
impl Default for PlotPoints<'_> {
    fn default() -> Self {
        Self::Borrowed(&[])
    }
}
impl<'a> PlotPoints<'a> {
    pub fn new(points: Vec<[f64; 3]>) -> Self {
        Self::Owned(points)
    }
    pub fn as_slice(&self) -> &[[f64; 3]] {
        match self {
            Self::Borrowed(p) => p,
            Self::Owned(p) => p,
            Self::Shared(p) => p.as_slice(),
        }
    }
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }
    pub fn bounds(&self) -> PlotBounds {
        if let Self::Shared(p) = self {
            return p.bounds();
        }
        let mut bounds = PlotBounds::NOTHING;
        for &p in self.as_slice() {
            bounds.extend_with(p);
        }
        bounds
    }
    /// Sample `(x, y, z) = f(t)` over a bounded range.
    ///
    /// Accepts arrays, tuples, or [`PlotPoint`] return values. `start..end`
    /// excludes the endpoint; `start..=end` includes it. Zero samples produce
    /// an empty series, and one sample evaluates only the start.
    ///
    /// # Panics
    /// Panics if the range is unbounded, non-finite, or excludes its start.
    pub fn from_parametric_callback<P: Into<PlotPoint>>(
        f: impl Fn(f64) -> P,
        range: impl std::ops::RangeBounds<f64>,
        points: usize,
    ) -> Self {
        use std::ops::Bound;
        let start = match range.start_bound() {
            Bound::Included(&value) => value,
            _ => panic!("Parametric ranges must have an included, finite start"),
        };
        let (end, inclusive) = match range.end_bound() {
            Bound::Included(&value) => (value, true),
            Bound::Excluded(&value) => (value, false),
            Bound::Unbounded => panic!("Parametric ranges must be bounded"),
        };
        assert!(
            start.is_finite() && end.is_finite(),
            "Parametric endpoints must be finite"
        );
        let intervals = if inclusive {
            points.saturating_sub(1)
        } else {
            points
        }
        .max(1);
        Self::Owned(
            (0..points)
                .map(|i| {
                    let t = i as f64 / intervals as f64;
                    let value = if i == 0 {
                        start
                    } else if inclusive && i + 1 == points {
                        end
                    } else {
                        let span = end - start;
                        if span.is_finite() {
                            start + span * t
                        } else {
                            start * (1.0 - t) + end * t
                        }
                    };
                    f(value).into().to_array()
                })
                .collect(),
        )
    }
}
impl From<[f64; 3]> for PlotPoints<'_> {
    fn from(point: [f64; 3]) -> Self {
        Self::Owned(vec![point])
    }
}
impl From<PlotPoint> for PlotPoints<'_> {
    fn from(point: PlotPoint) -> Self {
        Self::from(point.to_array())
    }
}
impl<const N: usize> From<[[f64; 3]; N]> for PlotPoints<'_> {
    fn from(points: [[f64; 3]; N]) -> Self {
        Self::Owned(points.into())
    }
}
impl From<Vec<PlotPoint>> for PlotPoints<'_> {
    fn from(points: Vec<PlotPoint>) -> Self {
        Self::Owned(points.into_iter().map(PlotPoint::to_array).collect())
    }
}
impl<'a> From<&'a [[f64; 3]]> for PlotPoints<'a> {
    fn from(p: &'a [[f64; 3]]) -> Self {
        Self::Borrowed(p)
    }
}
impl<'a, const N: usize> From<&'a [[f64; 3]; N]> for PlotPoints<'a> {
    fn from(p: &'a [[f64; 3]; N]) -> Self {
        Self::Borrowed(p)
    }
}
impl<'a> From<&'a Vec<[f64; 3]>> for PlotPoints<'a> {
    fn from(p: &'a Vec<[f64; 3]>) -> Self {
        Self::Borrowed(p)
    }
}
impl From<Vec<[f64; 3]>> for PlotPoints<'_> {
    fn from(p: Vec<[f64; 3]>) -> Self {
        Self::Owned(p)
    }
}
impl From<PointBuffer> for PlotPoints<'_> {
    fn from(p: PointBuffer) -> Self {
        Self::Shared(p)
    }
}
impl From<&PointBuffer> for PlotPoints<'_> {
    fn from(p: &PointBuffer) -> Self {
        Self::Shared(p.clone())
    }
}
impl FromIterator<[f64; 3]> for PlotPoints<'_> {
    fn from_iter<T: IntoIterator<Item = [f64; 3]>>(iter: T) -> Self {
        Self::Owned(iter.into_iter().collect())
    }
}
