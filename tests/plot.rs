use egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use egui_plot3d::*;

fn input(events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
        events,
        ..Default::default()
    }
}
fn frame(
    ctx: &Context,
    events: Vec<Event>,
    build: impl Fn(&mut egui::Ui) -> PlotResponse<()>,
) -> (PlotResponse<()>, egui::FullOutput) {
    let mut result = None;
    let mut output = ctx.run_ui(input(events), |ui| {
        result = Some(build(ui));
    });
    // These tests inspect CPU geometry without uploading textures to a GPU.
    output.textures_delta.clear();
    (result.unwrap(), output)
}
fn basic(ui: &mut egui::Ui) -> PlotResponse<()> {
    Plot::new("test").width(600.0).height(400.0).show(ui, |p| {
        p.points(Points::new(
            "points",
            vec![[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
        ))
    })
}
fn button(pos: Pos2, pressed: bool, button: PointerButton) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn borrowing_shared_bounds_and_nonfinite_data() {
    let points = [[1.0, 2.0, 3.0], [-1.0, 4.0, -5.0], [f64::NAN, 1.0, 2.0]];
    let borrowed = PlotPoints::from(&points);
    assert_eq!(borrowed.as_slice().as_ptr(), points.as_ptr());
    let shared = PointBuffer::from(points.to_vec());
    assert_eq!(shared.bounds().min(), [-1.0, 2.0, -5.0]);
    assert_eq!(shared.bounds().max(), [1.0, 4.0, 3.0]);
    assert_eq!(
        shared.as_slice().as_ptr(),
        shared.clone().as_slice().as_ptr()
    );
}
#[test]
fn projection_roundtrip_and_independent_z() {
    for projection in [Projection::Perspective, Projection::Orthographic] {
        let t = PlotTransform::new(
            Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)),
            PlotBounds::from_min_max([-10.0; 3], [10.0; 3]),
            Camera {
                projection,
                ..Default::default()
            },
            true,
        );
        let point = PlotPoint::new(1.25, -3.5, 6.0);
        let (s, d) = t.project(point).unwrap();
        let p = t.point_from_position(s, d).unwrap();
        for (a, b) in point.to_array().into_iter().zip(p.to_array()) {
            assert!((a - b).abs() < 1e-5, "{a} != {b}");
        }
        assert_ne!(
            t.position_from_point([1.0, 0.0, 0.0]),
            t.position_from_point([1.0, 0.0, 1.0])
        );
    }
}
#[test]
fn near_plane_and_invalid_camera_are_safe() {
    let t = PlotTransform::new(
        Rect::from_min_size(Pos2::ZERO, Vec2::splat(400.0)),
        PlotBounds::from_min_max([-1.0; 3], [1.0; 3]),
        Camera {
            yaw: 0.0,
            pitch: 0.0,
            ..Default::default()
        },
        true,
    );
    assert!(t.position_from_point([0.0, 5.0, 0.0]).is_none());
    assert!(t.position_from_point([f64::NAN, 0.0, 0.0]).is_none());
    let t = PlotTransform::new(
        t.frame(),
        PlotBounds::NOTHING,
        Camera {
            zoom: f32::NAN,
            pitch: f32::NAN,
            yaw: f32::NAN,
            pan: Vec2::splat(f32::NAN),
            ..Default::default()
        },
        true,
    );
    assert!(t.position_from_point([0.0; 3]).unwrap().is_finite());
}
#[test]
fn degenerate_and_large_bounds_fit() {
    for bounds in [
        PlotBounds::NOTHING,
        PlotBounds::from_min_max([7.0; 3], [7.0; 3]),
        PlotBounds::from_min_max([-1e308; 3], [1e308; 3]),
    ] {
        let t = PlotTransform::new(
            Rect::from_min_size(Pos2::ZERO, Vec2::splat(400.0)),
            bounds,
            Camera::default(),
            true,
        );
        assert!(
            t.position_from_point(t.bounds().center())
                .unwrap()
                .is_finite()
        );
    }
}
#[test]
fn mesh_batches_cloud_and_breaks_nan_lines() {
    let ctx = Context::default();
    let (r, out) = frame(&ctx, vec![], |ui| {
        Plot::new("batch")
            .show_grid(false)
            .show_frame(false)
            .show_axes([false; 3])
            .show(ui, |p| {
                p.points(Points::new("cloud", vec![[0.0; 3]; 1000]));
                p.line(Line::new(
                    "line",
                    vec![[0.0; 3], [1.0; 3], [f64::NAN; 3], [2.0; 3], [3.0; 3]],
                ));
            })
    });
    assert_eq!(r.rendered_primitives, 1002);
    let meshes: Vec<_> = out
        .shapes
        .iter()
        .filter_map(|s| {
            if let egui::Shape::Mesh(m) = &s.shape {
                Some(m)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(meshes.len(), 1);
    assert_eq!(meshes[0].vertices.len(), 4008);
    assert!(meshes[0].is_valid());
}
#[test]
fn hovering_and_clicking_returns_original_index() {
    let ctx = Context::default();
    let (r, _) = frame(&ctx, vec![], basic);
    let pos = r.transform.position_from_point([1.0; 3]).unwrap();
    let (r, _) = frame(&ctx, vec![Event::PointerMoved(pos)], basic);
    assert_eq!(r.hovered_point.unwrap().index, 1);
    frame(&ctx, vec![button(pos, true, PointerButton::Primary)], basic);
    let (r, _) = frame(
        &ctx,
        vec![button(pos, false, PointerButton::Primary)],
        basic,
    );
    assert_eq!(
        r.clicked_point.unwrap().position,
        PlotPoint::new(1.0, 1.0, 1.0)
    );
}
#[test]
fn nearest_depth_wins_at_overlapping_points() {
    let ctx = Context::default();
    let build = |ui: &mut egui::Ui| {
        Plot::new("overlap")
            .camera(Camera {
                yaw: 0.0,
                pitch: 0.0,
                ..Default::default()
            })
            .default_bounds(PlotBounds::from_min_max([-1.0; 3], [1.0; 3]))
            .show(ui, |p| {
                p.points(Points::new(
                    "points",
                    vec![[0.0, -0.5, 0.0], [0.0, 0.5, 0.0]],
                ))
            })
    };
    let (r, _) = frame(&ctx, vec![], build);
    let pos = r.transform.position_from_point([0.0; 3]).unwrap();
    let (r, _) = frame(&ctx, vec![Event::PointerMoved(pos)], build);
    assert_eq!(r.hovered_point.unwrap().index, 1);
}
#[test]
fn orbit_pan_zoom_and_plot_state_are_persistent() {
    let ctx = Context::default();
    let (r, _) = frame(&ctx, vec![], basic);
    let pos = r.response.rect.center();
    frame(
        &ctx,
        vec![
            Event::PointerMoved(pos),
            button(pos, true, PointerButton::Primary),
        ],
        basic,
    );
    let (r, _) = frame(
        &ctx,
        vec![Event::PointerMoved(pos + Vec2::new(40.0, 20.0))],
        basic,
    );
    assert_ne!(r.transform.camera().yaw, Camera::default().yaw);
    let rotated = r.transform.camera();
    frame(
        &ctx,
        vec![button(pos, false, PointerButton::Primary)],
        basic,
    );
    let (r, _) = frame(&ctx, vec![], basic);
    assert_eq!(r.transform.camera(), rotated);
    frame(
        &ctx,
        vec![button(pos, true, PointerButton::Secondary)],
        basic,
    );
    let (r, _) = frame(
        &ctx,
        vec![Event::PointerMoved(pos + Vec2::new(20.0, 10.0))],
        basic,
    );
    assert_ne!(r.transform.camera().pan, Vec2::ZERO);
    frame(
        &ctx,
        vec![button(pos, false, PointerButton::Secondary)],
        basic,
    );
    let (r, _) = frame(
        &ctx,
        vec![Event::MouseWheel {
            phase: egui::TouchPhase::Move,
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(0.0, 100.0),
            modifiers: Modifiers::NONE,
        }],
        basic,
    );
    assert!(r.transform.camera().zoom > 1.0);
    let (r, _) = frame(&ctx, vec![Event::PointerGone], |ui| {
        Plot::new("other").allow_zoom(false).show(ui, |_| ())
    });
    assert_eq!(r.transform.camera(), Camera::default());
}
#[test]
fn legend_click_hides_series() {
    let ctx = Context::default();
    let build = |ui: &mut egui::Ui| {
        Plot::new("legend")
            .width(600.0)
            .height(400.0)
            .legend(Legend::default())
            .show(ui, |p| p.points(Points::new("A", vec![[0.0; 3]])))
    };
    let (r, _) = frame(&ctx, vec![], build);
    let pos = r.response.rect.right_top() + Vec2::new(-28.0, 23.0);
    frame(&ctx, vec![Event::PointerMoved(pos)], build);
    frame(&ctx, vec![button(pos, true, PointerButton::Primary)], build);
    frame(
        &ctx,
        vec![button(pos, false, PointerButton::Primary)],
        build,
    );
    let (r, _) = frame(&ctx, vec![], build);
    assert_eq!(r.rendered_primitives, 0);
    assert!(r.hovered_point.is_none());
}
#[test]
fn custom_items_and_all_builtin_items_render() {
    struct Custom;
    impl PlotItem for Custom {
        fn name(&self) -> &str {
            "custom"
        }
        fn bounds(&self) -> PlotBounds {
            PlotBounds::from_min_max([0.0; 3], [1.0; 3])
        }
        fn render(&self, ctx: &mut PlotItemContext<'_>) {
            ctx.point([0.5; 3].into(), 3.0, MarkerShape::Diamond, ctx.color(), 0);
        }
    }
    let ctx = Context::default();
    let (r, out) = frame(&ctx, vec![], |ui| {
        Plot::new("all").show(ui, |p| {
            p.add(Custom);
            p.arrows(Arrows::new("vector", vec![[0.0; 3]], vec![[1.0; 3]]));
            p.polygon(Polygon::new(
                "plane",
                vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            ));
            p.text(Text::new("label", [0.0; 3], "Origin"));
        })
    });
    assert_eq!(r.rendered_primitives, 5);
    assert!(!ctx.tessellate(out.shapes, out.pixels_per_point).is_empty());
}

#[test]
fn manual_bounds_persist_until_auto_bounds_or_reset() {
    let ctx = Context::default();
    let manual = PlotBounds::from_min_max([-20.0; 3], [20.0; 3]);
    let draw = |ui: &mut egui::Ui, set: bool, auto: bool, reset: bool| {
        let plot = Plot::new("bounds");
        let plot = if reset { plot.reset() } else { plot };
        plot.show(ui, |p| {
            p.points(Points::new("points", vec![[0.0; 3], [1.0; 3]]));
            if set {
                p.set_plot_bounds(manual);
            }
            if auto {
                p.set_auto_bounds(true);
            }
        })
    };
    frame(&ctx, vec![], |ui| draw(ui, true, false, false));
    let (r, _) = frame(&ctx, vec![], |ui| draw(ui, false, false, false));
    assert_eq!(r.transform.bounds(), manual);
    let (r, _) = frame(&ctx, vec![], |ui| draw(ui, false, true, false));
    assert_eq!(r.transform.bounds().max(), [1.0; 3]);
    frame(&ctx, vec![], |ui| draw(ui, true, false, false));
    let (r, _) = frame(&ctx, vec![], |ui| draw(ui, false, false, true));
    assert_eq!(r.transform.bounds().max(), [1.0; 3]);
}

#[test]
fn reset_takes_precedence_over_a_supplied_camera() {
    let ctx = Context::default();
    let (r, _) = frame(&ctx, vec![], |ui| {
        Plot::new("reset")
            .camera(Camera {
                yaw: 2.0,
                zoom: 7.0,
                pan: Vec2::splat(0.2),
                projection: Projection::Orthographic,
                ..Default::default()
            })
            .reset()
            .show(ui, |_| ())
    });
    assert_eq!(
        r.transform.camera(),
        Camera {
            projection: Projection::Orthographic,
            ..Default::default()
        }
    );
}

#[test]
fn axis_aspect_and_bounds_match_the_builder_contract() {
    let ctx = Context::default();
    let (r, _) = frame(&ctx, vec![], |ui| {
        Plot::new("aspect")
            .camera(Camera {
                yaw: 0.0,
                pitch: 0.0,
                projection: Projection::Orthographic,
                ..Default::default()
            })
            .data_aspect(2.0)
            .include_x(-5.0)
            .include_z(8.0)
            .default_y_bounds(-10.0, 10.0)
            .show(ui, |p| {
                p.points(Points::new("data", vec![[0.0; 3], [1.0; 3]]))
            })
    });
    assert_eq!(r.transform.bounds().min(), [-5.0, -10.0, 0.0]);
    assert_eq!(r.transform.bounds().max(), [1.0, 10.0, 8.0]);
    let t = r.transform;
    let origin = t.position_from_point([0.0; 3]).unwrap();
    let x = t
        .position_from_point([1.0, 0.0, 0.0])
        .unwrap()
        .distance(origin);
    let z = t
        .position_from_point([0.0, 0.0, 1.0])
        .unwrap()
        .distance(origin);
    assert!((x / z - 2.0).abs() < 1e-5);
}

#[test]
fn near_plane_clips_a_triangle_instead_of_dropping_it() {
    let ctx = Context::default();
    let (r, out) = frame(&ctx, vec![], |ui| {
        Plot::new("clip")
            .camera(Camera {
                yaw: 0.0,
                pitch: 0.0,
                ..Default::default()
            })
            .default_bounds(PlotBounds::from_min_max([-1.0; 3], [1.0; 3]))
            .show_axes([false; 3])
            .show_grid(false)
            .show_frame(false)
            .show(ui, |p| {
                p.polygon(Polygon::new(
                    "triangle",
                    vec![[-0.1, 0.0, 0.0], [0.1, 0.0, 0.0], [0.0, 5.0, 0.1]],
                ))
            })
    });
    assert_eq!(r.rendered_primitives, 2);
    for shape in out.shapes {
        if let egui::Shape::Mesh(mesh) = shape.shape {
            assert!(mesh.is_valid());
            assert!(mesh.vertices.iter().all(|v| v.pos.is_finite()));
        }
    }
}

#[test]
fn extreme_single_point_bounds_stay_finite() {
    for value in [-f64::MAX, f64::MAX] {
        let bounds = PlotBounds::from_min_max([value; 3], [value; 3]);
        let transform = PlotTransform::new(
            Rect::from_min_size(Pos2::ZERO, Vec2::splat(400.0)),
            bounds,
            Camera::default(),
            true,
        );
        assert!(transform.bounds().is_valid());
        assert!(
            transform
                .position_from_point([value; 3])
                .unwrap()
                .is_finite()
        );
    }
}

#[test]
fn arrows_reject_invalid_width_and_do_not_invent_clipped_tips() {
    for (width, tip, a, b, expected) in [
        (0.0, 8.0, [0.0; 3], [1.0; 3], 0),
        (f32::NAN, 8.0, [0.0; 3], [1.0; 3], 0),
        (1.0, 0.0, [0.0; 3], [1.0; 3], 1),
        (1.0, f32::NAN, [0.0; 3], [1.0; 3], 1),
        (1.0, 8.0, [0.0; 3], [0.0; 3], 0),
        (1.0, 8.0, [0.1, 0.0, 0.0], [0.0, 5.0, 0.0], 1),
    ] {
        let (r, out) = frame(&Context::default(), vec![], |ui| {
            Plot::new("arrow")
                .default_bounds(PlotBounds::from_min_max([-1.0; 3], [1.0; 3]))
                .camera(Camera {
                    yaw: 0.0,
                    pitch: 0.0,
                    ..Default::default()
                })
                .show(ui, |p| {
                    p.arrows(
                        Arrows::new("vector", vec![a], vec![b])
                            .width(width)
                            .tip_length(tip),
                    )
                })
        });
        assert_eq!(
            r.rendered_primitives, expected,
            "width {width}, tip {tip}, endpoints {a:?}, {b:?}"
        );
        for s in out.shapes {
            if let egui::Shape::Mesh(m) = s.shape {
                assert!(m.vertices.iter().all(|v| v.pos.is_finite()));
            }
        }
    }
}

#[test]
fn unbounded_layouts_and_invalid_sizes_produce_finite_widgets() {
    let ctx = Context::default();
    frame(&ctx, vec![], |ui| {
        let r = egui::ScrollArea::vertical()
            .show(ui, |ui| Plot::new("scroll").show(ui, |_| ()))
            .inner;
        assert!(r.response.rect.is_finite());
        assert!(r.response.rect.height() >= 100.0);
        r
    });
    for size in [f32::INFINITY, f32::NAN, -1.0, 0.0] {
        let (r, _) = frame(&ctx, vec![], |ui| {
            Plot::new("invalid size")
                .width(size)
                .height(size)
                .show(ui, |_| ())
        });
        assert!(r.response.rect.is_finite());
        assert!(r.response.rect.is_positive());
    }
}

#[test]
fn explicit_plot_id_is_global_and_view_aspect_works_from_height() {
    let ctx = Context::default();
    let camera = Camera {
        yaw: 1.2,
        pitch: 0.6,
        ..Default::default()
    };
    let draw = |ui: &mut egui::Ui, parent, initial| {
        ui.push_id(parent, |ui| {
            let plot = Plot::new("local salt")
                .id(egui::Id::new("global plot"))
                .height(100.0)
                .view_aspect(2.0);
            let plot = if initial { plot.camera(camera) } else { plot };
            plot.show(ui, |_| ())
        })
        .inner
    };
    frame(&ctx, vec![], |ui| draw(ui, "first parent", true));
    let (r, _) = frame(&ctx, vec![], |ui| draw(ui, "other parent", false));
    assert_eq!(r.transform.camera(), camera);
    assert_eq!(r.response.rect.size(), Vec2::new(200.0, 100.0));
}

#[test]
fn background_visibility_last_setting_wins_and_keeps_color() {
    let color = egui::Color32::from_rgb(12, 34, 56);
    for visible in [false, true] {
        let (_, out) = frame(&Context::default(), vec![], |ui| {
            Plot::new("background")
                .background_color(color)
                .show_background(false)
                .show_background(visible)
                .show_axes([false; 3])
                .show_grid(false)
                .show_frame(false)
                .show(ui, |_| ())
        });
        let present = out
            .shapes
            .iter()
            .any(|s| matches!(&s.shape,egui::Shape::Rect(r) if r.fill==color));
        assert_eq!(present, visible);
    }
}
