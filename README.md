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

It contains seven interactive examples:

1. **Points in space** — 100–100,000 deterministic samples in five clusters, point inspection, adjustable density/radius, and cluster visibility. Inspired by the interaction and camera in `../boquilahub/src/gui/embedding_plot_3d.rs`.
2. **Lorenz attractor** — a 3D trajectory integrated with fourth-order Runge–Kutta, adjustable system parameters, a moving particle, and a second trajectory starting at a nearby initial condition.

3. **Wave interference** — two coherent radial sources; adjust phase, wavelength, and separation to reveal reinforcement and cancellation.
4. **Kepler orbits** — equal-time samples along an inclined ellipse show why a planet moves faster near its star. Adjust eccentricity or animate the planet.
5. **Correlated Gaussian** — a normalized probability-density surface with probability contours. Flip correlation to rotate the ridge; adjust the Y standard deviation.
6. **Harmonic motion** — three sinusoidal coordinates form a closed 3D Lissajous curve. Change frequency ratios and compare its coordinate projections.
7. **Magnetic dipole** — field lines and direction arrows around a tilted ideal dipole. Explore the polar and equatorial field directions.

Each new example has a **Compare setting** button, parameter controls, source code, a model reference, and an explanation of units and simplifications. The Gaussian uses an explicitly labeled 8x vertical scale. Surface examples implement `PlotItem` with small triangles; the main plots retain full resolution while thumbnails use cached, smaller meshes.

**Animate values** on Wave interference, Harmonic motion, and Magnetic dipole continuously changes the plotted geometry. Waves vary phase, wavelength, and source separation; harmonic curves morph through changing phases, amplitudes, and higher harmonics; dipole snapshots precess and vary the radii of sampled field lines. **Parameters** contains speed and intensity controls. Uncheck the toggle to freeze the current shape, or use **Reset motion** to restore the base values. Harmonic and dipole bounds stay fixed so the animation does not move the camera. The dipole animation illustrates changing orientation and field-line sampling, not a time-dependent electromagnetic simulation. Particle motion and wave travel remain separate controls.

Model references: [OpenStax wave interference](https://openstax.org/books/university-physics-volume-1/pages/16-5-interference-of-waves), [NASA Kepler orbits](https://science.nasa.gov/solar-system/orbits-and-keplers-laws/), [Wolfram bivariate normal](https://mathworld.wolfram.com/BivariateNormalDistribution.html), [Wolfram Lissajous curves](https://mathworld.wolfram.com/LissajousCurve.html), and [UT Austin magnetic dipoles](https://farside.ph.utexas.edu/teaching/jk1/Electromagnetism/node52.html).

Run natively:

```sh
cargo run --release -p egui_plot3d_demo
```

Append `-- points`, `-- lorenz`, `-- waves`, `-- kepler`, `-- gaussian`, `-- harmonics`, or `-- dipole` to open an example directly.

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
- `.data_aspect(1.0)` preserves equal units; `.axis_aspect([x, y, z])` supplies arbitrary unit-length ratios before camera rotation. `.equal_axis_scale(false)` stretches each data axis to fit independently.
- `PlotUi::set_plot_bounds` freezes bounds until reset or `set_auto_bounds(true)`. `.default_bounds(...)` supplies explicit bounds each frame. Bounds include hidden legend series, keeping the camera stable when toggling visibility.
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

## Rendering and scope

The library depends only on `egui`; it needs no renderer initialization and works with both egui's glow and wgpu integrations. Projection, clipping, and sorting happen on the CPU. Geometry is submitted as batched egui meshes; circular markers use four vertices and a shared antialiased texture. The implementation forbids unsafe Rust.

This is a 3D plotting library, not full API parity with the 2D crate. Specialized 2D items such as bar charts, box plots, heatmaps, linked axis cursors, and boxed zoom are not implemented. Axis visibility uses `[bool; 3]`; the ground grid is a single boolean. The camera replaces the 2D view transform.

Depth uses the painter's algorithm: points are sorted by depth, while segments and triangles are sorted by average depth. This works well for point clouds and densely sampled curves, including the demos. Intersecting large triangles or long crossing segments can be ordered incorrectly; there is no per-pixel depth buffer or general surface renderer. Subdivide geometry when necessary. Grid and axis guides are drawn behind the data. Camera state persists across frames, not across application restarts.

## Performance and checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo bench --bench render
```

The benchmark measures rotating-camera CPU frame preparation plus egui tessellation at 20,000 and 100,000 points/line samples. It excludes GPU rendering, uploads, and presentation; it is not an FPS claim. Run it on your target hardware. The application requests frames for interaction and animation, and otherwise idles.

Before/after runs on the same Windows machine, Ryzen 9 9950X3D, Rust 1.98.1, release profile (2026-10-08), using the same benchmark and full sample counts:

| Data | Before median | After median | After p95 | CPU time reduction |
| --- | ---: | ---: | ---: | ---: |
| 20,000 points | 1.48 ms | 1.00 ms | 1.02 ms | 32% |
| 100,000 points | 7.36 ms | 4.62 ms | 4.83 ms | 37% |
| 20,000 line samples | 2.94 ms | 1.65 ms | 1.69 ms | 44% |
| 100,000 line samples | 15.10 ms | 8.56 ms | 9.14 ms | 43% |

The renderer sorts compact depth keys with an exact stable radix sort, reuses projection/sort storage between frames, projects shared polyline vertices once, and reserves the mesh's actual vertex/index requirements. Lines retain their antialiasing and all original samples. Picking also rejects distant candidates before expensive unprojection. No extra threading dependency or renderer-specific integration is required. The benchmark excludes hovering and the gallery UI as well as GPU work; browser performance depends on the browser and graphics backend.

For a reproducible browser check:

```sh
python -m pip install -r scripts/requirements.txt
python -m playwright install chromium
cd demo
trunk build --release --locked
cd ..
python scripts/browser_smoke.py --serve-dir demo/dist
```

The browser check exercises all seven demos, all five new model comparisons, wave/orbit/harmonic animation, orbit/pan/zoom/reset, legend toggling, theme changes, and resizing. Screenshots and a JSON report are written to `artifacts/`. Use `--browser-path` to select an installed Chromium-based browser.

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
