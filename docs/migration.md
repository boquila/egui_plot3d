# Migrating an existing 3D renderer

Use the same egui version family throughout the application: this crate uses
egui 0.36. An application on egui 0.35 must update eframe and any widgets tied to
that version before its `Ui`, `Color32`, and `Response` types can interoperate.
The library renders ordinary egui meshes on both glow and wgpu; there is no
renderer initialization or GPU resource lifecycle for the application to manage.

The [native data explorer](../examples/demo/src/bin/explorer.rs) demonstrates a
million-point cloud, selection, cached hover, and interpolated surface readings.
Run it with `cargo run --release -p egui_plot3d_demo --bin explorer`.

## Preserve application data

Keep metadata and scalar attributes in the application, in input order.
Create `PointBuffer` once when positions change. `PointBuffer::from_points`
accepts an iterator of f32 arrays, f64 arrays, tuples, or `PlotPoint` values:

```rust
let positions = PointBuffer::from_points(records.iter().map(|record| record.position));
```

Pass `&positions` to `Points`, `Line`, or `Surface` each frame. This shares
coordinates and cached bounds. Borrowed f64 slices also work, but auto-fitting
must scan their coordinates. Use explicit bounds for continuously updated data
with a known range. Index and style columns accept borrowed slices, owned
vectors, and `Arc` slices. Avoid allocating those columns inside the plot closure
when they have not changed.

`UpAxis::Y` supports existing Y-up embeddings; the default is Z up. Coordinate
values, axis labels, and picked indices remain in the application's original
coordinate system. Use `axis_aspect([1.0, 1.0, exaggeration])` for display-only
vertical exaggeration, or `equal_axis_scale(false)` to fit axes independently.
`zoom_to_pointer(true)` anchors zoom under the pointer.

## Selection and custom tooltips

`response.hovered_point` and `response.clicked_point` include the stable item ID,
input index, world position, and screen position. Invalid or clipped samples do
not shift indices. Distinguish a click on empty space with
`response.response.clicked() && response.clicked_point.is_none()`.

Use `label_formatter` for application metadata or return `None` to suppress the
built-in text. Draw selection rings using the final `response.transform` and
`ui.painter_at(response.response.rect)`. Keeping these overlays outside the plot
closure avoids invalidating a large cached dataset when selection changes.

`Points::perspective_scaling(true)` scales radii with depth. `radius_limits`
can keep distant samples visible and prevent nearby samples becoming too large.

## Indexed surfaces and scalar fields

Pass original vertices and triangle triplets to `Surface::new`. `colors` supplies
per-face colors; `vertex_colors` supplies smooth colors and overrides face colors.
Missing colors use the item color. Invalid indices and nonfinite faces are skipped.
With shading and depth cues disabled, uniform and vertex-colored surfaces retain
shared vertices in the output mesh rather than emitting three copies per face.

Turn off plot depth cues and surface shading when colors encode quantitative
values. Apply `ColorScale` to application scalars once when values or the scale
change. `SurfaceGrid` offers regular measured grids and sampled functions, while
`Surface` supports arbitrary indexed meshes, disconnected regions, and holes.
Boundaries, contours and vectors use `Line` and `Arrows`; custom `PlotItem`
implementations submit geometry through `PlotItemContext`.

For a surface hit, `hit.index` identifies the original triangle and
`hit.barycentric` contains its three perspective-correct weights:

```rust
if let Some(hit) = response.hovered_point {
    if let Some(weights) = hit.barycentric {
        let indices = triangles[hit.index];
        let value: f64 = (0..3)
            .map(|i| values[indices[i] as usize] * weights[i])
            .sum();
    }
}
```

Clipping at the camera near plane preserves the original face index and weights.
The caller decides how to interpolate missing values, cyclic angles, categories,
and other attributes; the plotting crate does not impose domain-specific rules.

For annotations or particles over surfaces, request `capture_picking(true)`.
The returned `response.picking` snapshot supports `surface_at(screen_position)`
and `is_visible(world_point, depth_bias)`. It excludes hover-disabled items and
remains valid after subsequent frames. Visibility tests are point tests against
pickable triangles, including transparent faces. Sample or subdivide long overlay
segments when individual endpoints do not establish visibility along the segment.

## Performance choices

`cache_revision(revision)` reuses projected meshes and a spatial picking index
while geometry is unchanged. Increment the revision whenever positions, topology,
colors, radii, widths, text/font definitions, or custom item rendering changes.
Camera, plot bounds, viewport, DPI, item names/IDs/colors/visibility, and plot depth
cues invalidate automatically. Item-specific builder settings are the caller's
revision responsibility. The build closure still runs; do not regenerate large
buffers there. Rebuilding the cache has an extra indexing cost, so omit it for
data that changes every frame. The cache retains geometry and picking storage;
it does not eliminate egui tessellation or GPU uploads.

`Points::screen_space_sampling(4.0)` is an explicit approximation for dense clouds:
it draws and picks only the nearest sample in each four-logical-pixel screen cell.
Original input indices and full-data bounds are preserved. The sampling grid is
bounded to one million cells; extreme viewport sizes use coarser cells. Markers
whose centers lie just outside the viewport retain their exact edge geometry.
Sampling does not merge classes, average values, or modify the input. Leave it
disabled whenever every sample's footprint or transparency is significant.

Measure both camera motion and pointer-only frames using
`cargo bench --bench workloads -- --large`. These benchmarks include CPU frame
preparation and egui tessellation, not GPU uploads, draw time, or presentation.
Use release builds when evaluating interactive performance.

## Rendering boundaries

This is a CPU projection and depth-sorting renderer. Points sort by depth;
triangles and segments sort by average depth. Intersecting faces can therefore
have incorrect per-pixel occlusion, and perspective-correct picking can disagree
with their painted order. Smooth colors interpolate in screen space. A renderer
requiring exact depth-buffer visibility or full-resolution million-point GPU
throughput is not a drop-in replacement target for this backend. Keep those
requirements explicit when comparing the native example with an existing GPU
renderer; sampling improves throughput by drawing less geometry.
