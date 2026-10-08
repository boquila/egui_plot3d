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
mod data;
mod items;
mod plot;
mod render;
mod texture;

pub use camera::{Camera, PlotTransform, Projection};
pub use data::{PlotBounds, PlotPoint, PlotPoints, PointBuffer};
pub use items::{Arrows, Line, MarkerShape, PlotItem, Points, Polygon, Surface, Text};
pub use plot::{Corner, HoveredPoint, Legend, Plot, PlotResponse, PlotUi};
pub use render::PlotItemContext;
