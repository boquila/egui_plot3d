use egui::{Pos2, Rect, Vec2};
use egui_plot3d::{Camera, PlotBounds, PlotPoint, PlotPoints, PlotTransform, Projection};

fn transform(min: [f64; 3], max: [f64; 3]) -> PlotTransform {
    PlotTransform::new(
        Rect::from_min_size(Pos2::ZERO, Vec2::splat(800.0)),
        PlotBounds::from_min_max(min, max),
        Camera {
            yaw: 0.0,
            pitch: 0.0,
            projection: Projection::Orthographic,
            ..Default::default()
        },
        true,
    )
}

#[test]
fn scientific_scales_preserve_normalized_geometry() {
    let reference = transform([-1.0; 3], [1.0; 3])
        .position_from_point([0.5, 0.0, 0.5])
        .unwrap();
    for magnitude in [1e-300, 1e-100, 1e-15, 1e-6, 1.0, 1e100, 1e300] {
        let t = transform([-magnitude; 3], [magnitude; 3]);
        let point = [magnitude * 0.5, 0.0, magnitude * 0.5];
        let screen = t.position_from_point(point).unwrap();
        assert!(
            screen.distance(reference) < 0.001,
            "scale {magnitude:e}: {screen:?} != {reference:?}"
        );
    }
}

#[test]
fn constant_axes_do_not_swamp_small_planar_data() {
    let reference = transform([-1.0, -1.0, 0.0], [1.0, 1.0, 0.0])
        .position_from_point([0.5, 0.0, 0.0])
        .unwrap();
    for scale in [1e-100, 1e-15, 1.0, 1e100] {
        let t = transform([-scale, -scale, 0.0], [scale, scale, 0.0]);
        let p = t.position_from_point([0.5 * scale, 0.0, 0.0]).unwrap();
        assert!(p.distance(reference) < 0.001, "scale {scale:e}: {p:?}");
    }
}

#[test]
fn callback_ranges_support_tuples_points_and_exclusive_endpoints() {
    let p = PlotPoints::from_parametric_callback(|t| (t, t * t, 0.0), 0.0..1.0, 4);
    assert_eq!(
        p.as_slice().iter().map(|p| p[0]).collect::<Vec<_>>(),
        vec![0.0, 0.25, 0.5, 0.75]
    );
    let p = PlotPoints::from_parametric_callback(|t| PlotPoint::new(t, 0.0, 0.0), 2.0..=0.0, 3);
    assert_eq!(
        p.as_slice().iter().map(|p| p[0]).collect::<Vec<_>>(),
        vec![2.0, 1.0, 0.0]
    );
}

#[test]
fn large_offsets_do_not_change_small_local_differences() {
    let expected = transform([-0.375; 3], [0.375; 3])
        .position_from_point([0.125, 0.0, 0.125])
        .unwrap();
    let offset = 1e15;
    let t = transform([offset - 0.375; 3], [offset + 0.375; 3]);
    let actual = t
        .position_from_point([offset + 0.125, offset, offset + 0.125])
        .unwrap();
    assert!(
        actual.distance(expected) < 0.001,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn inverse_projection_never_returns_nonfinite_coordinates() {
    let t = transform([-f64::MAX; 3], [f64::MAX; 3]);
    let point = t.point_from_position(Pos2::new(f32::MAX, 0.0), 4.0);
    assert!(point.is_none_or(PlotPoint::is_finite));
}

#[test]
fn parametric_samples_preserve_finite_endpoints_at_extreme_ranges() {
    let points = PlotPoints::from_parametric_callback(|t| [t, 0.0, 0.0], -f64::MAX..=f64::MAX, 3);
    assert_eq!(
        points.as_slice(),
        &[[-f64::MAX, 0.0, 0.0], [0.0; 3], [f64::MAX, 0.0, 0.0]]
    );
    assert!(PlotPoints::from_parametric_callback(|t| [t; 3], 0.0..=1.0, 0).is_empty());
    assert_eq!(
        PlotPoints::from_parametric_callback(|t| [t; 3], 2.0..=3.0, 1).as_slice(),
        &[[2.0; 3]]
    );
}
