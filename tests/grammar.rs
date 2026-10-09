use egui::Color32;
use egui_plot3d::*;
use std::sync::Arc;

#[test]
fn color_scales_handle_reversal_missing_log_and_extreme_domains() {
    let palette = ColorMap::custom([Color32::BLACK, Color32::WHITE]);
    let linear = ColorScale::new(-1.0..=1.0, palette.clone());
    assert_eq!(linear.normalize(0.0), Some(0.5));
    assert_eq!(linear.color(-20.0), Color32::BLACK);
    assert_eq!(linear.color(20.0), Color32::WHITE);
    assert_eq!(linear.color(f64::NAN), Color32::TRANSPARENT);
    let reverse = ColorScale::new(1.0..=-1.0, palette.clone());
    assert_eq!(reverse.color(1.0), Color32::BLACK);
    assert_eq!(reverse.color(-1.0), Color32::WHITE);
    let extreme = ColorScale::new(-f64::MAX..=f64::MAX, palette.clone());
    assert_eq!(extreme.normalize(0.0), Some(0.5));
    assert_eq!(extreme.value_at(0.5), Some(0.0));
    assert_eq!(extreme.value_at(1.0), Some(f64::MAX));
    let log = ColorScale::new(1e-12..=1e12, palette.clone()).logarithmic();
    assert_eq!(log.normalize(1.0), Some(0.5));
    for t in [0.0, 0.2, 0.5, 0.9, 1.0] {
        assert!((log.normalize(log.value_at(t).unwrap()).unwrap() - t).abs() < 1e-12);
    }
    assert_eq!(log.normalize(0.0), None);
    assert_eq!(log.normalize(-1.0), None);
    assert_eq!(
        ColorScale::new(2.0..=2.0, palette.clone()).normalize(2.0),
        Some(0.5)
    );
    assert_eq!(
        ColorScale::new(f64::NAN..=1.0, palette)
            .missing_color(Color32::RED)
            .color(0.5),
        Color32::RED
    );
    assert_eq!(ColorMap::custom([]).color(0.5), Color32::TRANSPARENT);
    assert_eq!(ColorMap::custom([Color32::RED]).color(0.9), Color32::RED);
    let transparent = ColorMap::custom([Color32::TRANSPARENT, Color32::WHITE]).color(0.5);
    assert_eq!(transparent.a(), 128);
    assert!(transparent.r() <= transparent.a());
    for map in [
        ColorMap::Viridis,
        ColorMap::Inferno,
        ColorMap::Mako,
        ColorMap::Plasma,
    ] {
        assert_ne!(map.color(0.0), map.color(1.0));
        assert_eq!(map.color(f64::NAN), Color32::TRANSPARENT);
    }
}

#[test]
fn grids_preserve_layout_share_snapshots_and_resample_safely() {
    let mut grid = SurfaceGrid::from_fn(-1.0..=1.0, -2.0..=2.0, [3, 2], |x, y| x + 2.0 * y);
    assert_eq!(
        grid.vertices().as_slice(),
        &[
            [-1.0, -2.0, -5.0],
            [0.0, -2.0, -4.0],
            [1.0, -2.0, -3.0],
            [-1.0, 2.0, 3.0],
            [0.0, 2.0, 4.0],
            [1.0, 2.0, 5.0]
        ]
    );
    let before = grid.clone();
    let item = grid.surface("snapshot");
    grid.resample(|u, v| [u, v, 10.0]);
    assert!(Arc::ptr_eq(grid.indices(), before.indices()));
    assert_eq!(before.vertices().bounds().max()[2], 5.0);
    assert_eq!(item.bounds().max()[2], 5.0);
    assert_eq!(grid.vertices().bounds().max()[2], 10.0);
    assert_eq!(
        grid.indices().as_ref(),
        &[[0, 1, 3], [1, 4, 3], [1, 2, 4], [2, 5, 4]]
    );
    let extreme = SurfaceGrid::from_fn(-f64::MAX..=f64::MAX, 0.0..=1.0, [3, 2], |_, _| 0.0);
    assert_eq!(extreme.vertices().as_slice()[1][0], 0.0);
    assert!(
        extreme
            .vertices()
            .as_slice()
            .iter()
            .flatten()
            .all(|x| x.is_finite())
    );
    let measured = SurfaceGrid::from_heights(0.0..=1.0, 0.0..=1.0, [2, 2], &[1.0, 2.0, 3.0, 4.0]);
    assert_eq!(measured.vertices().as_slice()[3], [1.0, 1.0, 4.0]);
    assert!(std::panic::catch_unwind(|| SurfaceGrid::triangle_indices([0, 5])).is_err());
    assert!(std::panic::catch_unwind(|| SurfaceGrid::triangle_indices([usize::MAX, 2])).is_err());
    assert!(
        std::panic::catch_unwind(|| SurfaceGrid::from_heights(
            0.0..=1.0,
            0.0..=1.0,
            [2, 2],
            &[1.0]
        ))
        .is_err()
    );
}

#[test]
fn contours_match_mesh_plane_intersections_and_skip_holes() {
    let grid = SurfaceGrid::from_fn(-1.0..=1.0, -1.0..=1.0, [9, 9], |x, y| x + y);
    let contour = grid.contour(0.125);
    assert!(!contour.is_empty());
    for point in contour.as_slice().iter().filter(|p| p[0].is_finite()) {
        assert_eq!(point[2], 0.125);
        assert!((point[0] + point[1] - 0.125).abs() < 1e-12);
    }
    // A shared edge at exactly the level is emitted only once.
    let edge = SurfaceGrid::from_heights(0.0..=1.0, 0.0..=1.0, [2, 2], &[-1.0, 0.0, 0.0, -1.0]);
    let points = edge.contour(0.0);
    assert_eq!(points.len(), 3);
    assert_eq!(points.as_slice()[0][2], 0.0);
    let flat = SurfaceGrid::from_fn(0.0..=1.0, 0.0..=1.0, [2, 2], |_, _| 0.0);
    assert!(flat.contour(0.0).is_empty());
    assert!(grid.contour(f64::NAN).is_empty());
    let hole = SurfaceGrid::from_heights(
        0.0..=1.0,
        0.0..=1.0,
        [2, 2],
        &[0.0, f64::NAN, f64::NAN, 1.0],
    );
    assert!(hole.contour(0.5).is_empty());
    // A saddle is split along the exact same diagonal as the displayed mesh.
    let saddle = SurfaceGrid::from_heights(0.0..=1.0, 0.0..=1.0, [2, 2], &[1.0, -1.0, -1.0, 1.0]);
    assert_eq!(saddle.contour(0.0).len(), 6);
}

#[test]
fn style_columns_can_be_borrowed_owned_or_shared() {
    let shared: Arc<[f32]> = vec![1.0, 2.0, 3.0].into();
    let values = PlotValues::from(&shared);
    assert_eq!(values.as_slice().as_ptr(), shared.as_ptr());
    assert_eq!(values.clone().as_slice().as_ptr(), shared.as_ptr());
    let slice = [1.0, 2.0];
    assert_eq!(PlotValues::from(&slice).as_slice().as_ptr(), slice.as_ptr());
    let colors: Arc<[Color32]> = vec![Color32::RED; 6].into();
    let data = [
        [0.0, 0.0, 0.0],
        [0.4, 0.0, 0.0],
        [f64::NAN; 3],
        [0.0, 0.4, 0.0],
        [0.4, 0.4, 0.0],
        [0.8, 0.4, 0.0],
    ];
    let points = Points::new("sized", data.as_slice())
        .radii(vec![3.0, f32::NAN, 4.0, 0.0, -1.0, f32::INFINITY])
        .colors(&colors);
    let lines = Line::new("styled", data.as_slice())
        .widths([2.0, 2.0, 2.0, f32::NAN, 3.0])
        .colors(vec![Color32::BLUE; 6]);
    let grid = SurfaceGrid::from_fn(0.0..=1.0, 0.0..=1.0, [2, 2], |x, y| x * y);
    let scale = ColorScale::new(0.0..=1.0, ColorMap::Viridis);
    let surface = grid.surface("mapped").color_by(|p| p.z, &scale);
    let layers: [&dyn PlotItem; 3] = [&points, &lines, &surface];
    egui::__run_test_ui(|ui| {
        let result = Plot::new("styles")
            .width(400.0)
            .height(300.0)
            .depth_cue(false)
            .show(ui, |plot| plot.extend(layers));
        assert_eq!(result.rendered_primitives, 5); // 1 point, 2 segments, 2 faces
        ui.add(ColorBar::new("Height", &scale));
    });
}
