# egui_plot3d

Immediate mode 3D plotting for **egui 0.36**, with the builder and closure API of `egui_plot`.

```rust
use egui_plot3d::{Legend, Line, Plot, Points};

let samples = vec![[0.0, 0.0, 0.0], [1.0, 2.0, 3.0], [2.0, 1.0, 1.0]];
Plot::new("space")
    .data_aspect(1.0)
    .legend(Legend::default())
    .show(ui, |plot_ui| {
        plot_ui.points(Points::new("Samples", samples.as_slice()).radius(4.0));
        plot_ui.line(Line::new("Path", samples.as_slice()).width(2.0));
    });
```

While working locally, depend on this repository with
`egui_plot3d = { path = "../egui_plot3d" }`. The crate has not been published to crates.io.

## Gallery

The gallery follows the reference `ref/egui_plot/demo`: default egui light/dark themes, a resizable example thumbnail panel, the interactive plot, and a source panel with Rust syntax highlighting. The small HTML/CSS shell hosts the canvas; egui draws the application UI.

It contains ten interactive examples:

1. **Points in space** — 100–100,000 deterministic samples in five clusters, point inspection, adjustable density/radius, and cluster visibility. Inspired by the interaction and camera in `../boquilahub/src/gui/embedding_plot_3d.rs`.
2. **Lorenz attractor** — a 3D trajectory integrated with fourth-order Runge–Kutta, adjustable system parameters, and a second trajectory starting at a nearby initial condition. **Draw trajectory** starts empty and integrates forward from (1, 1, 1), leaving a trail behind the ball. Pause/resume, restart, and drawing speed controls let you watch the attractor form. **Animate values** varies sigma, rho, and beta smoothly while integrating new samples; the existing trail preserves the system's history. This mode illustrates a driven Lorenz system with time-varying parameters. The camera bounds remain fixed during drawing.

3. **Wave interference** — two coherent radial sources; adjust phase, wavelength, and separation to reveal reinforcement and cancellation. A high-contrast Inferno palette encodes amplitude on a fixed -2 to +2 scale, with a visible color key. Depth shading is disabled for this plot to preserve the mapping. The palette is the [CC0 Inferno lookup table by Nathaniel J. Smith and Stefan van der Walt](https://github.com/BIDS/colormap/blob/master/colormaps.py).
4. **Kepler orbits** — equal-time samples along an inclined ellipse show why a planet moves faster near its star. Adjust eccentricity or animate the planet.
5. **Correlated Gaussian** — a normalized probability-density surface with probability contours. Flip correlation to rotate the ridge; adjust the Y standard deviation.
6. **Harmonic motion** — three sinusoidal coordinates form a closed 3D Lissajous curve. Change frequency ratios and compare its coordinate projections.
7. **Magnetic dipole** — field lines and direction arrows around a tilted ideal dipole. Explore the polar and equatorial field directions.

8. **Spherical harmonics** — an oscillating sphere reveals the lobes and nodal patterns of real [spherical harmonics](https://dlmf.nist.gov/14.30). Change degree and order; Inferno colors encode signed displacement.
9. **Spiral waves** — a live [Barkley reaction–diffusion model](https://www.scholarpedia.org/article/Barkley_model) develops curling excitation fronts. Height and Inferno color show activation; change excitability and recovery to reshape the waves.
10. **Torus knots** — animated tubes reveal how winding numbers produce [knots](https://mathworld.wolfram.com/TorusKnot.html) and links. Compare a trefoil with linked loops, or choose your own winding numbers.

Each science example has a **Compare setting** button, parameter controls, Rust source, and a model reference. The Gaussian uses an 8x vertical scale; spiral activation uses 2.4x. The new surfaces use the library's indexed `Surface` item, sharing vertices across triangles. Thumbnails are cached.

Value animations are enabled by default on Lorenz, Wave interference, Harmonic motion, Magnetic dipole, and all three new plots, and advance when that example is selected. Lorenz draws from its initial condition; use **Pause** to inspect the growing trail or uncheck **Draw trajectory** to show the complete classic trajectory. The other animated examples freeze when **Animate values** is unchecked; **Reset motion** restores their initial state and pauses them.

**Animate values** on Wave interference, Harmonic motion, and Magnetic dipole continuously changes the plotted geometry. Waves vary phase, wavelength, and source separation; harmonic curves morph through changing phases, amplitudes, and higher harmonics; dipole snapshots precess and vary the radii of sampled field lines. **Parameters** contains speed and intensity controls. Uncheck the toggle to freeze the current shape, or use **Reset motion** to restore the base values. Harmonic and dipole bounds stay fixed so the animation does not move the camera. The dipole animation illustrates changing orientation and field-line sampling, not a time-dependent electromagnetic simulation. Particle motion and wave travel remain separate controls.

Model references: [OpenStax wave interference](https://openstax.org/books/university-physics-volume-1/pages/16-5-interference-of-waves), [NASA Kepler orbits](https://science.nasa.gov/solar-system/orbits-and-keplers-laws/), [Wolfram bivariate normal](https://mathworld.wolfram.com/BivariateNormalDistribution.html), [Wolfram Lissajous curves](https://mathworld.wolfram.com/LissajousCurve.html), and [UT Austin magnetic dipoles](https://farside.ph.utexas.edu/teaching/jk1/Electromagnetism/node52.html).

Run natively:

```sh
cargo run --release -p egui_plot3d_demo
```

Append `-- points`, `-- lorenz`, `-- waves`, `-- kepler`, `-- gaussian`, `-- harmonics`, `-- dipole`, `-- spherical`, `-- spirals`, or `-- knots` to open an example directly.

Run in a browser (install [Trunk](https://trunkrs.dev/) first):

```sh
rustup target add wasm32-unknown-unknown
cd demo
trunk serve --release
```

Open `http://localhost:8080` and select a thumbnail. If your environment sets `NO_COLOR=1`, unset it or set it to `true` for Trunk 0.21.14.

## Interaction

| Action | Control |
| --- | --- |
| Orbit | Primary drag / one-finger drag |
| Pan | Secondary drag, middle drag, or Shift + primary drag |
| Zoom | Scroll wheel / pinch |
| Fit and reset camera | Double-click, `Plot::reset()`, or gallery Reset view |
| Inspect a sample | Hover or click a point; use `PlotResponse::hovered_point` / `clicked_point` |
| Hide a series | Click its legend entry |

Coordinates are right-handed, with **Z up**. `Camera` contains yaw, pitch, zoom, normalized screen pan, and perspective/orthographic projection. Plot state lives in the egui context under the plot ID; give each plot and each named series a stable, distinct ID. `.id(...)` can separate a series' identity from its displayed name.

## API and data

`Plot`, `PlotUi`, `PlotResponse`, `PlotPoint`, `PlotPoints`, `PlotBounds`, `Line`, `Points`, `Arrows`, `Polygon`, `Text`, and `Legend` follow the reference's naming and usage. Common builders include `width`, `height`, `view_aspect`, `data_aspect`, `include_x/y/z`, `default_x/y/z_bounds`, `show_axes`, `show_grid`, `allow_drag`, `allow_zoom`, `label_formatter`, and `reset`.

- Data consists of `[f64; 3]` coordinates. `PlotPoint::new(x, y, z)` provides named fields for interaction results and custom geometry.
- `Points` offers circle, square, diamond, and cross markers, per-item or per-point colors, radius, highlighting, and hover control.
- `Line` is a polyline. Non-finite samples break the path; invalid points do not affect fitting.
- `Arrows` takes origins and tips. `Polygon` triangulates a **convex planar** polygon. `Text` places a label in data space.
- `Surface` draws indexed triangle meshes with shared vertex projection, per-face colors, and optional directional shading. Vertices can be borrowed, owned, or shared through `PointBuffer`; indices can be borrowed or owned. Invalid faces are skipped and faces crossing the camera near plane are clipped. Surface picking is not yet implemented.
- `.data_aspect(1.0)` preserves equal units; `.axis_aspect([x, y, z])` supplies arbitrary unit-length ratios before camera rotation. `.equal_axis_scale(false)` stretches each data axis to fit independently.
- `PlotUi::set_plot_bounds` freezes bounds until reset or `set_auto_bounds(true)`. `.default_bounds(...)` and `.default_x/y/z_bounds(...)` supply starting/reset bounds. Explicit manual bounds or auto-fit requests override these defaults across frames. Bounds include hidden legend series, keeping the camera stable when toggling visibility.
- `.camera(camera)` and `PlotResponse::transform.camera()` let applications control or link views. The closure sees the previous fitted transform; the response contains the final transform after collecting that frame's data.
- `pointer_coordinate()` unprojects onto the plane through the plot center parallel to the screen. A screen coordinate does not identify a unique 3D point; use `hovered_point` for data selection or `PlotTransform::point_from_position(position, depth)` with an explicit depth.
- Implement `PlotItem` and submit points, segments, triangles, and text through `PlotItemContext` to extend the library while retaining shared projection and depth ordering.

For large static datasets, calculate bounds once and share the allocation:

```rust
use egui_plot3d::{Plot, PointBuffer, Points};

let data = PointBuffer::from(vec![[0.0, 1.0, 2.0]; 100_000]);
// Store `data` in your application, outside the UI callback.
Plot::new("large cloud").show(ui, |plot_ui| {
    plot_ui.points(Points::new("Samples", &data));
});
```

`PointBuffer` clones only an `Arc` and has cached bounds. Borrowing `&[[f64; 3]]` also avoids copying coordinates, but bounds are scanned each frame. Owned vectors and iterator collection are supported as well.

Function plots accept arrays, tuples, or `PlotPoint` values, and inclusive or exclusive finite ranges:

```rust
use egui_plot3d::{Line, Plot, PlotPoints};
let curve = PlotPoints::from_parametric_callback(
    |t| (t.cos(), t.sin(), t / 10.0),
    0.0..=std::f64::consts::TAU * 3.0,
    1_000,
);
Plot::new("helix").show(ui, |plot| plot.line(Line::new("Helix", curve)));
```

`Points::new` and `Line::new` also accept fixed arrays directly. A single `[x, y, z]` or `PlotPoint` creates a one-point series. Use `.id(egui::Id::new(...))` on `Plot` for a global ID independent of the parent UI; the constructor's ID salt is scoped to the parent, matching `egui_plot`. Item `.name(...)` changes only the displayed name, so legend state survives dynamic labels.

## Rendering and scope

The library depends only on `egui`; it needs no renderer initialization and works with both egui's glow and wgpu integrations. Projection, clipping, and sorting happen on the CPU. Geometry is submitted as batched egui meshes; circular markers and common-width line segments use four vertices and a shared antialiased texture. Wide strokes or a full stroke atlas use geometric antialiasing. The implementation forbids unsafe Rust. Depth shading changes RGB brightness while preserving the specified opacity.

This is a 3D plotting library, not full API parity with the 2D crate. Specialized 2D items such as bar charts, box plots, heatmaps, linked axis cursors, and boxed zoom are not implemented. Axis visibility uses `[bool; 3]`; the ground grid is a single boolean. The camera replaces the 2D view transform.

Depth uses the painter's algorithm: points are sorted by depth, while segments and triangles are sorted by average depth. This works well for point clouds and densely sampled curves, including the demos. Intersecting large triangles or long crossing segments can be ordered incorrectly; there is no per-pixel depth buffer. Subdivide geometry when necessary. Grid and axis guides are drawn behind the data. Camera state persists across frames, not across application restarts.

## Performance and checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo bench --bench render
cargo bench --bench workloads -- --large
```

The benchmark measures rotating-camera CPU frame preparation plus egui tessellation at 20,000 and 100,000 points/line samples. It excludes GPU rendering, uploads, and presentation; it is not an FPS claim. Run it on your target hardware. The application requests frames for interaction and animation, and otherwise idles.

Before/after runs on the same Windows machine, Ryzen 9 9950X3D, Rust 1.98.1, release profile (2026-10-08), using the same benchmark and full sample counts:

| Data | Before median | After median | After p95 | CPU time reduction |
| --- | ---: | ---: | ---: | ---: |
| 20,000 points | 1.48 ms | 0.74 ms | 0.77 ms | 50% |
| 100,000 points | 7.36 ms | 3.60 ms | 3.74 ms | 51% |
| 20,000 line samples | 2.94 ms | 1.11 ms | 1.16 ms | 62% |
| 100,000 line samples | 15.10 ms | 5.53 ms | 5.69 ms | 63% |

The renderer sorts compact depth keys with an exact stable radix sort, reuses projection/sort storage and uniquely owned mesh allocations between frames, projects shared polyline vertices once, and reserves the mesh's actual vertex/index requirements. Lines retain antialiasing and all original samples. Picking rejects distant candidates before expensive unprojection, and plots fully outside the UI clip rectangle skip geometry generation. No extra threading dependency or renderer-specific integration is required. The `render` benchmark excludes hovering and the gallery UI as well as GPU work; `workloads` additionally measures hovering, borrowed data, data generation, clipping, surfaces, and million-sample inputs. Browser performance depends on the browser and graphics backend.

The broader workload run measured 100,000 animated samples including generation and hovering at 6.95 ms median, and 32,768 surface triangles at 2.30 ms. One million points with hovering took 71.33 ms (90.13 ms p95); one million line samples took 99.13 ms (112.45 ms p95). These workloads exceed a 60 Hz frame budget before GPU rendering. Mesh construction and egui tessellation/copying dominate at that scale; this CPU renderer does not promise smooth million-sample animation. Measurements vary with system load and input geometry.

A paired run of 32,768 identical surface faces measured **1.68 ms** median with individual triangle submission versus **0.90 ms** with indexed `Surface` (about 46% less CPU time; 0.95 ms p95). Both produced the same tessellated vertex and index counts. This measures CPU preparation and egui tessellation, excluding GPU rendering.

To preview the release gallery locally:

```sh
cd demo
trunk serve --release --locked
```

The library and gallery checks are Rust tests, run with `cargo test --workspace`.

## GitHub Pages

`.github/workflows/pages.yml` builds the same Rust gallery with Trunk and deploys it using GitHub Pages. Enable **Settings → Pages → Source → GitHub Actions**, then push to `main` or dispatch the workflow. The build uses the repository name as its public URL prefix, so assets resolve under `/egui_plot3d/`.

To reproduce that deployment layout locally:

```sh
cd demo
trunk build --release --locked --public-url /egui_plot3d/
```

No remote deployment is implied by a successful local build. A public URL requires a successful Pages workflow in your repository.

## License and references

MIT OR Apache-2.0. The gallery layout is adapted from [egui_plot](https://github.com/emilk/egui_plot), under the same dual license. Its source is available locally in `ref/egui_plot`. BoquilaHub's 3D embedding plot informed the orbit/pan/picking interaction; this crate uses its own renderer.
