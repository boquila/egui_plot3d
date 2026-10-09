use crate::{ColorScale, PointBuffer, Surface, data::interpolate};
use std::{collections::HashSet, ops::RangeInclusive, sync::Arc};

/// A sampled surface with shared coordinates, cached bounds, and reusable topology.
///
/// Build once for static functions. [`Self::resample`] replaces coordinates while
/// retaining topology, suitable for animations. Clones share both allocations.
#[derive(Clone, Debug)]
pub struct SurfaceGrid {
    dimensions: [usize; 2],
    ranges: [[f64; 2]; 2],
    vertices: PointBuffer,
    indices: Arc<[[u32; 3]]>,
}
impl SurfaceGrid {
    /// Build a regular grid from measured row-major heights. Coordinates are
    /// evenly spaced over the inclusive domains; NaNs represent missing data.
    /// Panics if the height count does not match the dimensions, or dimensions
    /// and endpoints fail the same validation as [`Self::from_fn`].
    pub fn from_heights(
        x: RangeInclusive<f64>,
        y: RangeInclusive<f64>,
        dimensions: [usize; 2],
        heights: &[f64],
    ) -> Self {
        assert_eq!(
            dimensions[0].checked_mul(dimensions[1]),
            Some(heights.len()),
            "Height count must match grid dimensions"
        );
        let mut values = heights.iter();
        Self::from_fn(x, y, dimensions, |_, _| *values.next().unwrap())
    }
    /// Sample z=f(x,y) on an inclusive rectangular grid, in row-major order.
    ///
    /// # Panics
    /// Panics for non-finite endpoints, dimensions below 2, or more than u32::MAX
    /// vertices. Non-finite function values are allowed and make holes.
    pub fn from_fn(
        x: RangeInclusive<f64>,
        y: RangeInclusive<f64>,
        dimensions: [usize; 2],
        mut f: impl FnMut(f64, f64) -> f64,
    ) -> Self {
        Self::parametric(x, y, dimensions, |x, y| [x, y, f(x, y)])
    }
    /// Sample an arbitrary parameterized surface on an inclusive (u,v) grid.
    /// Seams are not automatically welded. Validation is the same as `from_fn`.
    pub fn parametric(
        u: RangeInclusive<f64>,
        v: RangeInclusive<f64>,
        dimensions: [usize; 2],
        f: impl FnMut(f64, f64) -> [f64; 3],
    ) -> Self {
        let ranges = [[*u.start(), *u.end()], [*v.start(), *v.end()]];
        assert!(
            ranges.iter().flatten().all(|x| x.is_finite()),
            "Surface grid endpoints must be finite"
        );
        let indices = Self::triangle_indices(dimensions);
        let mut result = Self {
            dimensions,
            ranges,
            vertices: PointBuffer::default(),
            indices,
        };
        result.resample(f);
        result
    }
    /// Shared triangle indices for a row-major grid. Two consistently wound
    /// triangles per cell; the diagonal joins its top-right/bottom-left vertices.
    /// Panics for invalid dimensions or more than u32::MAX vertices.
    pub fn triangle_indices([columns, rows]: [usize; 2]) -> Arc<[[u32; 3]]> {
        assert!(
            columns >= 2 && rows >= 2,
            "Surface grids need at least 2 samples per axis"
        );
        let count = columns
            .checked_mul(rows)
            .expect("Surface grid size overflow");
        assert!(
            count <= u32::MAX as usize,
            "Surface grid indices exceed u32"
        );
        let mut indices = Vec::with_capacity(
            (columns - 1)
                .checked_mul(rows - 1)
                .and_then(|n| n.checked_mul(2))
                .expect("Surface grid face count overflow"),
        );
        for y in 0..rows - 1 {
            for x in 0..columns - 1 {
                let a = (y * columns + x) as u32;
                let c = a + columns as u32;
                indices.extend([[a, a + 1, c], [a + 1, c + 1, c]]);
            }
        }
        indices.into()
    }
    /// Replace coordinates without reallocating topology. Earlier clones and
    /// plot items keep their original data; returned invalid samples create holes.
    pub fn resample(&mut self, mut f: impl FnMut(f64, f64) -> [f64; 3]) {
        let [columns, rows] = self.dimensions;
        self.vertices = (0..rows)
            .flat_map(|j| (0..columns).map(move |i| (i, j)))
            .map(|(i, j)| {
                let u = interpolate(
                    self.ranges[0][0],
                    self.ranges[0][1],
                    i as f64 / (columns - 1) as f64,
                );
                let v = interpolate(
                    self.ranges[1][0],
                    self.ranges[1][1],
                    j as f64 / (rows - 1) as f64,
                );
                f(u, v)
            })
            .collect();
    }
    /// Grid width and height in samples.
    pub fn dimensions(&self) -> [usize; 2] {
        self.dimensions
    }
    /// Shared coordinates with cached finite bounds.
    pub fn vertices(&self) -> &PointBuffer {
        &self.vertices
    }
    /// Shared row-major triangle indices.
    pub fn indices(&self) -> &Arc<[[u32; 3]]> {
        &self.indices
    }
    /// Create an item sharing this snapshot's vertices and indices.
    pub fn surface(&self, name: impl Into<String>) -> Surface<'static> {
        Surface::new(name, &self.vertices, &self.indices)
    }
    /// Face colors obtained from the mean Z value per triangle and a common scale.
    pub fn colors(&self, scale: &ColorScale) -> Vec<egui::Color32> {
        self.indices
            .iter()
            .map(|f| {
                scale.color(
                    f.iter()
                        .map(|&i| self.vertices.as_slice()[i as usize][2] / 3.0)
                        .sum(),
                )
            })
            .collect()
    }
    /// Intersect the triangulated surface with z=level. Returns independent line
    /// segments separated by NaNs, ready for [`crate::Line`]. Shared edges are
    /// emitted once; non-finite faces and coplanar triangles are skipped.
    /// Contours follow the actual triangles, including their saddle diagonals.
    pub fn contour(&self, level: f64) -> PointBuffer {
        if !level.is_finite() {
            return PointBuffer::default();
        }
        let mut points = Vec::new();
        let mut seen = HashSet::new();
        let key = |p: [f64; 3]| p.map(|x| if x == 0.0 { 0 } else { x.to_bits() });
        let mut segment = |a: [f64; 3], b: [f64; 3]| {
            let a_key = key(a);
            let b_key = key(b);
            if a_key != b_key
                && seen.insert(if a_key < b_key {
                    (a_key, b_key)
                } else {
                    (b_key, a_key)
                })
            {
                points.extend([a, b, [f64::NAN; 3]]);
            }
        };
        for face in self.indices.iter() {
            let p = face.map(|i| self.vertices.as_slice()[i as usize]);
            if !p.iter().flatten().all(|x| x.is_finite()) || p.iter().all(|p| p[2] == level) {
                continue;
            }
            let mut crossings = Vec::with_capacity(3);
            for [i, j] in [[0, 1], [1, 2], [2, 0]] {
                let (a, b) = if face[i] < face[j] {
                    (p[i], p[j])
                } else {
                    (p[j], p[i])
                };
                if a[2] == level && b[2] == level {
                    segment(a, b);
                    continue;
                }
                if (a[2] < level) == (b[2] < level) {
                    continue;
                }
                let d = b[2] - a[2];
                let t = if d.is_finite() {
                    (level - a[2]) / d
                } else {
                    (level * 0.5 - a[2] * 0.5) / (b[2] * 0.5 - a[2] * 0.5)
                };
                let point = [
                    interpolate(a[0], b[0], t),
                    interpolate(a[1], b[1], t),
                    level,
                ];
                if !crossings.contains(&point) {
                    crossings.push(point);
                }
            }
            if crossings.len() == 2 {
                segment(crossings[0], crossings[1]);
            }
        }
        points.into()
    }
}
