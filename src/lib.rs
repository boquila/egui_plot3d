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
#![forbid(unsafe_code)]

mod camera;
mod data;
mod items;
mod plot;
mod render;
mod texture;

pub use camera::{Camera, PlotTransform, Projection};
pub use data::{PlotBounds, PlotPoint, PlotPoints, PointBuffer};
pub use items::{Arrows, Line, MarkerShape, PlotItem, Points, Polygon, Text};
pub use plot::{Corner, HoveredPoint, Legend, Plot, PlotResponse, PlotUi};
pub use render::PlotItemContext;
