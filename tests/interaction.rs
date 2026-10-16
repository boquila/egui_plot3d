use egui::{Color32, Context, Event, Pos2, RawInput, Rect, Vec2};
use egui_plot3d::*;
use std::cell::Cell;

fn frame(
    ctx: &Context,
    events: Vec<Event>,
    draw: impl Fn(&mut egui::Ui) -> PlotResponse<()>,
) -> (PlotResponse<()>, egui::FullOutput) {
    let mut response = None;
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            ..Default::default()
        },
        |ui| response = Some(draw(ui)),
    );
    output.textures_delta.clear();
    (response.unwrap(), output)
}
fn plot() -> Plot<'static> {
    Plot::new("interaction")
        .width(600.0)
        .height(400.0)
        .show_grid(false)
        .show_axes([false; 3])
        .show_frame(false)
        .show_background(false)
        .depth_cue(false)
        .default_bounds(PlotBounds::from_min_max([-1.0; 3], [1.0; 3]))
        .camera(Camera {
            yaw: 0.0,
            pitch: 0.0,
            ..Default::default()
        })
}
fn mesh_vertices(output: &egui::FullOutput) -> Vec<egui::epaint::Vertex> {
    output
        .shapes
        .iter()
        .flat_map(|s| match &s.shape {
            egui::Shape::Mesh(m) => m.vertices.clone(),
            _ => Vec::new(),
        })
        .collect()
}

#[test]
fn up_axes_preserve_original_coordinates_and_inverse_projection() {
    for up in [UpAxis::X, UpAxis::Y, UpAxis::Z] {
        for projection in [Projection::Perspective, Projection::Orthographic] {
            let transform = PlotTransform::new(
                Rect::from_min_size(Pos2::ZERO, Vec2::splat(600.0)),
                PlotBounds::from_min_max([-2.0; 3], [2.0; 3]),
                Camera {
                    yaw: 0.0,
                    pitch: 0.0,
                    projection,
                    ..Default::default()
                },
                true,
            )
            .with_up_axis(up)
            .with_axis_aspect([1.0, 2.0, 3.0]);
            let point = PlotPoint::from([0.3, -0.2, 0.6]);
            let (screen, depth) = transform.project(point).unwrap();
            let recovered = transform.point_from_position(screen, depth).unwrap();
            for (a, b) in point.to_array().into_iter().zip(recovered.to_array()) {
                assert!((a - b).abs() < 1e-6);
            }
            let vertical = match up {
                UpAxis::X => [1.0, 0.0, 0.0],
                UpAxis::Y => [0.0, 1.0, 0.0],
                UpAxis::Z => [0.0, 0.0, 1.0],
            };
            let origin = transform.position_from_point([0.0; 3]).unwrap();
            let top = transform.position_from_point(vertical).unwrap();
            assert!((top.x - origin.x).abs() < 1e-5 && top.y < origin.y);
        }
    }
}

#[test]
fn surface_picks_original_faces_and_perspective_correct_attributes() {
    let vertices = [[-1.0, -0.5, -1.0], [1.0, 0.5, -1.0], [0.0, 0.0, 1.0]];
    let indices = [[0, 0, 0], [0, 1, 2], [99, 1, 2]];
    let expected = [0.2, 0.3, 0.5];
    let point: [f64; 3] =
        std::array::from_fn(|axis| (0..3).map(|i| vertices[i][axis] * expected[i]).sum());
    for projection in [Projection::Perspective, Projection::Orthographic] {
        for cached in [false, true] {
            let ctx = Context::default();
            let draw = |ui: &mut egui::Ui| {
                let p = plot().projection(projection);
                let p = if cached { p.cache_revision(1) } else { p };
                p.show(ui, |p| {
                    p.surface(Surface::new(
                        "mesh",
                        vertices.as_slice(),
                        indices.as_slice(),
                    ))
                })
            };
            let (first, _) = frame(&ctx, vec![], draw);
            assert_eq!(first.rendered_primitives, 1);
            let pointer = first.transform.position_from_point(point).unwrap();
            let (hit, _) = frame(&ctx, vec![Event::PointerMoved(pointer)], draw);
            let hit = hit.hovered_point.unwrap();
            assert_eq!(hit.index, 1);
            for (a, b) in hit.barycentric.unwrap().into_iter().zip(expected) {
                assert!((a - b).abs() < 1e-6, "{a} != {b}");
            }
            for (a, b) in hit.position.to_array().into_iter().zip(point) {
                assert!((a - b).abs() < 1e-6);
            }
        }
    }
}

#[test]
fn clipped_surface_retains_original_barycentric_weights_and_colors() {
    let vertices = [[-1.0, 0.0, -1.0], [1.0, 0.0, -1.0], [0.0, 5.0, 1.0]];
    let ctx = Context::default();
    let draw = |ui: &mut egui::Ui| {
        plot().cache_revision(0).show(ui, |p| {
            p.surface(
                Surface::new("clip", vertices.as_slice(), [[0, 1, 2]]).vertex_colors([
                    Color32::RED,
                    Color32::GREEN,
                    Color32::BLUE,
                ]),
            )
        })
    };
    let (first, output) = frame(&ctx, vec![], draw);
    assert_eq!(first.rendered_primitives, 2);
    assert!(mesh_vertices(&output).iter().all(|v| v.pos.is_finite()));
    let pointer = first
        .transform
        .position_from_point([0.0, 1.0, -0.6])
        .unwrap();
    let (picked, _) = frame(&ctx, vec![Event::PointerMoved(pointer)], draw);
    let weights = picked.hovered_point.unwrap().barycentric.unwrap();
    for (a, b) in weights.into_iter().zip([0.4, 0.4, 0.2]) {
        assert!((a - b).abs() < 1e-5, "{weights:?}");
    }
}

struct Counted<'a> {
    calls: &'a Cell<usize>,
    color: Color32,
}
impl PlotItem for Counted<'_> {
    fn name(&self) -> &str {
        "counted"
    }
    fn color(&self) -> Option<Color32> {
        Some(self.color)
    }
    fn bounds(&self) -> PlotBounds {
        PlotBounds::from_min_max([-1.0; 3], [1.0; 3])
    }
    fn render(&self, ctx: &mut PlotItemContext<'_>) {
        self.calls.set(self.calls.get() + 1);
        ctx.point([0.0; 3].into(), 4.0, MarkerShape::Circle, ctx.color(), 42);
        ctx.polyline(&[[-1.0, 0.0, -0.5], [1.0, 0.0, -0.5]], 2.0, Color32::WHITE);
        ctx.text([0.0, 0.0, 0.8].into(), "Label".into(), Color32::WHITE);
    }
}

#[test]
fn cached_meshes_and_picking_survive_pointer_changes_and_invalidate_correctly() {
    let ctx = Context::default();
    let calls = Cell::new(0);
    let draw = |ui: &mut egui::Ui, revision, color, yaw| {
        plot()
            .camera(Camera {
                yaw,
                pitch: 0.0,
                ..Default::default()
            })
            .cache_revision(revision)
            .show(ui, |p| {
                p.add(Counted {
                    calls: &calls,
                    color,
                })
            })
    };
    let (first, saved) = frame(&ctx, vec![], |ui| draw(ui, 0, Color32::RED, 0.0));
    let old_vertices = mesh_vertices(&saved);
    assert_eq!(calls.get(), 1);
    let point = first.transform.position_from_point([0.0; 3]).unwrap();
    let (hover, repeat) = frame(&ctx, vec![Event::PointerMoved(point)], |ui| {
        draw(ui, 0, Color32::RED, 0.0)
    });
    assert_eq!(calls.get(), 1, "pointer-only frame must reuse geometry");
    assert_eq!(hover.hovered_point.unwrap().index, 42);
    assert_eq!(first.rendered_primitives, hover.rendered_primitives);
    let original_mesh = saved
        .shapes
        .iter()
        .find_map(|s| {
            if let egui::Shape::Mesh(m) = &s.shape {
                Some(m)
            } else {
                None
            }
        })
        .unwrap();
    let cached_mesh = repeat
        .shapes
        .iter()
        .find_map(|s| {
            if let egui::Shape::Mesh(m) = &s.shape {
                Some(m)
            } else {
                None
            }
        })
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(original_mesh, cached_mesh));
    let line = first
        .transform
        .position_from_point([0.1, 0.0, -0.5])
        .unwrap();
    let (hover, _) = frame(&ctx, vec![Event::PointerMoved(line)], |ui| {
        draw(ui, 0, Color32::RED, 0.0)
    });
    assert_eq!(calls.get(), 1);
    assert!(hover.hovered_point.unwrap().barycentric.is_none());
    frame(&ctx, vec![], |ui| draw(ui, 1, Color32::RED, 0.0));
    assert_eq!(calls.get(), 2);
    frame(&ctx, vec![], |ui| draw(ui, 1, Color32::GREEN, 0.0));
    assert_eq!(calls.get(), 3);
    frame(&ctx, vec![], |ui| draw(ui, 1, Color32::GREEN, 0.2));
    assert_eq!(calls.get(), 4);
    assert_eq!(
        mesh_vertices(&saved),
        old_vertices,
        "retained output must stay immutable"
    );
}

#[test]
fn sampling_is_opt_in_preserves_indices_and_selects_nearest_depth() {
    let points =
        PointBuffer::from_points([[0.0_f32, -0.8, 0.0], [0.0, 0.7, 0.0], [0.0, -0.2, 0.0]]);
    assert_eq!(points.as_slice()[1], [0.0, 0.7_f32 as f64, 0.0]);
    let ctx = Context::default();
    let draw = |ui: &mut egui::Ui, sampled| {
        plot().cache_revision(sampled).show(ui, |p| {
            let data = Points::new("cloud", &points);
            p.points(if sampled {
                data.screen_space_sampling(4.0)
            } else {
                data
            });
        })
    };
    let (full, _) = frame(&ctx, vec![], |ui| draw(ui, false));
    assert_eq!(full.rendered_primitives, 3);
    let (sampled, _) = frame(&ctx, vec![], |ui| draw(ui, true));
    assert_eq!(sampled.rendered_primitives, 1);
    assert_eq!(full.transform.bounds(), sampled.transform.bounds());
    let pointer = sampled
        .transform
        .position_from_point(points.as_slice()[1])
        .unwrap();
    let (picked, _) = frame(&ctx, vec![Event::PointerMoved(pointer)], |ui| {
        draw(ui, true)
    });
    assert_eq!(picked.hovered_point.unwrap().index, 1);
}

#[test]
fn vertex_colors_survive_rendering_and_point_radius_limits_apply_after_depth() {
    let ctx = Context::default();
    let colors = [Color32::RED, Color32::GREEN, Color32::BLUE];
    let (_, output) = frame(&ctx, vec![], |ui| {
        plot().show(ui, |p| {
            p.surface(
                Surface::new(
                    "colors",
                    [[-1.0, 0.0, -1.0], [1.0, 0.0, -1.0], [0.0, 0.0, 1.0]],
                    [[0, 1, 2]],
                )
                .vertex_colors(colors),
            )
        })
    });
    for color in colors {
        assert!(mesh_vertices(&output).iter().any(|v| v.color == color));
    }
    let (_, output) = frame(&ctx, vec![], |ui| {
        plot().show(ui, |p| {
            p.points(
                Points::new("radius", [[0.0, 3.0, 0.0]])
                    .radius(3.5)
                    .perspective_scaling(true)
                    .radius_limits(2.0..=9.0),
            )
        })
    });
    let vertices = mesh_vertices(&output);
    let min = vertices
        .iter()
        .map(|v| v.pos.x)
        .fold(f32::INFINITY, f32::min);
    let max = vertices
        .iter()
        .map(|v| v.pos.x)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        (max - min - 18.0).abs() < 2.0,
        "radius must clamp to nine pixels: {}",
        max - min
    );
}

#[test]
fn picking_snapshots_support_occluded_overlays_and_remain_immutable() {
    let ctx = Context::default();
    let vertices = [[-1.0, 0.0, -1.0], [1.0, 0.0, -1.0], [0.0, 0.0, 1.0]];
    let draw = |ui: &mut egui::Ui, revision, height| {
        plot()
            .allow_hover(false)
            .capture_picking(true)
            .cache_revision(revision)
            .show(ui, |p| {
                p.surface(Surface::new(
                    "surface",
                    vertices.map(|mut p| {
                        p[1] = height;
                        p
                    }),
                    [[0, 1, 2]],
                ));
                p.points(Points::new("overlay", [[0.0, 0.5, 0.0]]));
            })
    };
    let (first, _) = frame(&ctx, vec![], |ui| draw(ui, 0, 0.0));
    let position = first.transform.position_from_point([0.0; 3]).unwrap();
    assert!(first.hovered_point.is_none());
    let picking = first.picking.unwrap();
    assert_eq!(picking.hit_test(position).unwrap().name, "overlay");
    assert_eq!(picking.surface_at(position).unwrap().name, "surface");
    assert!(picking.is_visible([0.0, 0.5, 0.0], 0.0));
    assert!(picking.is_visible([0.0, 0.0, 0.0], 0.001));
    assert!(!picking.is_visible([0.0, -0.5, 0.0], 0.001));
    assert!(!picking.is_visible([f64::NAN; 3], 0.001));
    let (next, _) = frame(&ctx, vec![], |ui| draw(ui, 1, -0.8));
    assert!(next.picking.unwrap().is_visible([0.0, -0.5, 0.0], 0.001));
    assert!(
        !picking.is_visible([0.0, -0.5, 0.0], 0.001),
        "old snapshot must keep the old surface"
    );
}

#[test]
fn indexed_vertex_colors_share_mesh_vertices_across_faces() {
    let ctx = Context::default();
    let positions = [
        [-1.0, 0.0, -1.0],
        [1.0, 0.0, -1.0],
        [1.0, 0.0, 1.0],
        [-1.0, 0.0, 1.0],
    ];
    let (_, output) = frame(&ctx, vec![], |ui| {
        plot().show(ui, |p| {
            p.surface(
                Surface::new("quad", positions, [[0, 1, 2], [0, 2, 3]]).vertex_colors([
                    Color32::RED,
                    Color32::GREEN,
                    Color32::BLUE,
                    Color32::WHITE,
                ]),
            )
        })
    });
    let vertices = mesh_vertices(&output);
    assert_eq!(
        vertices.len(),
        4,
        "smooth surfaces should retain shared vertices"
    );
    for shape in &output.shapes {
        if let egui::Shape::Mesh(mesh) = &shape.shape {
            assert!(mesh.is_valid());
            assert_eq!(mesh.indices.len(), 6);
        }
    }
}

#[test]
fn cursor_zoom_keeps_a_world_point_fixed_for_all_projections_and_limits() {
    let rect = Rect::from_min_size(Pos2::new(40.0, 50.0), Vec2::new(800.0, 600.0));
    let bounds = PlotBounds::from_min_max([-1.0; 3], [1.0; 3]);
    for projection in [Projection::Perspective, Projection::Orthographic] {
        let mut camera = Camera {
            projection,
            ..Default::default()
        };
        for factor in [2.0, 0.5, 1e6, 1e-9, 1.0] {
            let before = PlotTransform::new(rect, bounds, camera, true);
            let point = [0.2, -0.4, 0.1];
            let position = before.position_from_point(point).unwrap();
            camera.zoom_at(factor, position, rect);
            let after = PlotTransform::new(rect, bounds, camera, true);
            assert!(after.position_from_point(point).unwrap().distance(position) < 0.003);
        }
    }
}

#[test]
fn shared_surface_vertices_remap_correctly_when_text_splits_meshes() {
    let ctx = Context::default();
    let positions = [
        [-1.0, -0.7, -1.0],
        [1.0, -0.7, -1.0],
        [0.0, 0.7, 1.0],
        [0.0, 0.7, -1.0],
    ];
    let (_, output) = frame(&ctx, vec![], |ui| {
        plot().show(ui, |p| {
            p.surface(
                Surface::new("surface", positions, [[0, 1, 2], [1, 2, 3]]).vertex_colors([
                    Color32::RED,
                    Color32::GREEN,
                    Color32::BLUE,
                    Color32::WHITE,
                ]),
            );
            p.text(Text::new("label", [0.0, 0.0, 0.0], "split"));
        })
    });
    let meshes: Vec<_> = output
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
    assert_eq!(meshes.len(), 2);
    assert!(meshes.iter().all(|m| m.is_valid()));
    assert_eq!(meshes.iter().map(|m| m.indices.len()).sum::<usize>(), 6);
}

#[test]
fn cached_picking_preserves_ties_between_large_and_small_faces() {
    let ctx = Context::default();
    let draw = |ui: &mut egui::Ui| {
        plot().cache_revision(0).show(ui, |p| {
            p.surface(Surface::new(
                "first",
                [[-2.0, 0.0, -2.0], [2.0, 0.0, -2.0], [0.0, 0.0, 2.0]],
                [[0, 1, 2]],
            ));
            p.surface(Surface::new(
                "second",
                [[-0.1, 0.0, -0.1], [0.1, 0.0, -0.1], [0.0, 0.0, 0.1]],
                [[0, 1, 2]],
            ));
        })
    };
    let (response, _) = frame(&ctx, vec![], draw);
    let at = response.transform.position_from_point([0.0; 3]).unwrap();
    let (response, _) = frame(&ctx, vec![Event::PointerMoved(at)], draw);
    assert_eq!(response.hovered_point.unwrap().name, "first");
}

#[test]
fn cached_geometry_invalidates_on_viewport_clip_and_hover_changes() {
    let ctx = Context::default();
    let calls = Cell::new(0);
    let draw = |ui: &mut egui::Ui, width, hover, clipped| {
        if clipped {
            ui.set_clip_rect(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0)));
        }
        plot()
            .width(width)
            .allow_hover(hover)
            .cache_revision(0)
            .show(ui, |p| {
                p.add(Counted {
                    calls: &calls,
                    color: Color32::WHITE,
                })
            })
    };
    frame(&ctx, vec![], |ui| draw(ui, 600.0, true, false));
    frame(&ctx, vec![], |ui| draw(ui, 600.0, true, false));
    assert_eq!(calls.get(), 1);
    frame(&ctx, vec![], |ui| draw(ui, 500.0, true, false));
    assert_eq!(calls.get(), 2);
    frame(&ctx, vec![], |ui| draw(ui, 500.0, false, false));
    assert_eq!(calls.get(), 3);
    frame(&ctx, vec![], |ui| draw(ui, 500.0, false, true));
    assert_eq!(calls.get(), 4);
}
