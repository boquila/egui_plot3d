//! Immediate mode 3D plotting with an egui_plot-style API.
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use egui_plot3d::{Legend, Line, Plot, Points};
//! let cloud = vec![[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]];
//! Plot::new("space").legend(Legend::default()).show(ui, |plot_ui| {
//!     plot_ui.points(Points::new("Samples", cloud.as_slice()).radius(4.0));
//!     plot_ui.line(Line::new("Path", cloud.as_slice()).width(2.0));
//! });
//! # });
//! ```
//!
//! Drag to orbit, secondary-drag or shift-drag to pan, scroll to zoom,
//! and double-click to fit. State is stored in egui under the plot's ID.
//! Coordinates are right-handed with Z up. No renderer setup is required:
//! plots use batched egui meshes and work with both glow and wgpu.
//!
//! # Function plots
//!
//! Parametric functions can return tuples, arrays, or [`PlotPoint`] values:
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use egui_plot3d::{Line, Plot, PlotPoints};
//! let curve = PlotPoints::from_parametric_callback(
//!     |t| (t.cos(), t.sin(), t / 10.0),
//!     0.0..=std::f64::consts::TAU * 3.0,
//!     1_000,
//! );
//! Plot::new("helix").show(ui, |plot| plot.line(Line::new("Helix", curve)));
//! # });
//! ```
//!
//! # Shared data
//!
//! Store a [`PointBuffer`] in your application to share coordinates and cached
//! bounds across frames. Building a plot item then clones only an `Arc`:
//!
//! ```
//! use egui_plot3d::{Plot, PointBuffer, Points};
//! let samples: PointBuffer = (0..10_000)
//!     .map(|i| { let t = i as f64 * 0.01; [t.sin(), t.cos(), t / 100.0] })
//!     .collect();
//! # egui::__run_test_ui(|ui| {
//! Plot::new("samples").show(ui, |plot| {
//!     plot.points(Points::new("Data", &samples).radius(2.0));
//! });
//! # });
//! ```
//!
//! # Surfaces
//!
//! Share vertices between indexed triangles; face colors and directional shading
//! are optional. Shared vertices are projected once per surface:
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use egui_plot3d::{Plot, Surface};
//! let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 1.0]];
//! let triangles = vec![[0, 1, 2]];
//! Plot::new("surface").show(ui, |plot| {
//!     plot.surface(Surface::new("Mesh", vertices, triangles).shading(true));
//! });
//! # });
//! ```
//!
//! # Function surfaces, contours, scales, and reusable layers
//!
//! ```
//! use egui_plot3d::{ColorBar, ColorMap, ColorScale, Line, Plot, PlotItem, SurfaceGrid};
//! let grid = SurfaceGrid::from_fn(-3.0..=3.0, -3.0..=3.0, [65, 65],
//!     |x, y| x.sin() * y.cos());
//! let scale = ColorScale::new(-1.0..=1.0, ColorMap::Viridis);
//! let surface = grid.surface("Height").color_by(|p| p.z, &scale);
//! let contour = Line::new("z = 0.5", grid.contour(0.5)).width(2.0);
//! // Reuse these items without cloning their geometry.
//! let layers: [&dyn PlotItem; 2] = [&surface, &contour];
//! # egui::__run_test_ui(|ui| {
//! Plot::new("field").depth_cue(false).show(ui, |plot| plot.extend(layers));
//! ui.add(ColorBar::new("Height", &scale));
//! # });
//! ```
//!
//! [`SurfaceGrid::from_heights`] accepts measured grids, while
//! [`SurfaceGrid::parametric`] samples arbitrary `(u,v) -> (x,y,z)` functions.
//! [`SurfaceGrid::resample`] reuses triangle topology during animation.
//! [`Points::radii`], [`Line::widths`], and item color columns accept borrowed
//! slices, owned vectors, and shared `Arc` slices through [`PlotValues`].
//! Color scales support linear or logarithmic domains, custom palettes, reversed
//! domains, and explicit missing colors. Set [`Plot::depth_cue`] to false when
//! surface or point colors must retain an exact quantitative meaning.
//!
//! # Rendering limits
//!
//! Projection and depth sorting run on the CPU. Points are depth-sorted;
//! segments and triangles use average depth, so intersecting geometry is not
//! guaranteed to have correct per-pixel occlusion. There is no depth buffer.
//! Picking currently covers points and line/arrow shafts, not filled polygons
//! or text. World coordinates are `f64`; normalized camera/screen coordinates
//! use `f32`, matching egui's mesh representation.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod camera;
mod color;
mod colormaps;
mod data;
mod grid;
mod items;
mod plot;
mod render;
mod texture;
mod values;

pub use camera::{Camera, PlotTransform, Projection};
pub use color::{ColorBar, ColorMap, ColorScale};
pub use data::{PlotBounds, PlotPoint, PlotPoints, PointBuffer};
pub use grid::SurfaceGrid;
pub use items::{Arrows, Line, MarkerShape, PlotItem, Points, Polygon, Surface, Text};
pub use plot::{Corner, HoveredPoint, Legend, Plot, PlotResponse, PlotUi};
pub use render::PlotItemContext;
pub use values::PlotValues;
