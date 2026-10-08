//! Additional workloads: hovering, borrowed and changing data, surfaces, clipping,
//! and (with `--large`) million-sample inputs. Includes CPU tessellation, not GPU work.
use egui::{Color32, Context, Event, Pos2, RawInput, Rect, Vec2};
use egui_plot3d::{
    Camera, Line, MarkerShape, Plot, PlotBounds, PlotItem, PlotItemContext, PointBuffer, Points,
};
use std::{hint::black_box, time::Instant};

fn measure(label: &str, hover: bool, mut draw: impl FnMut(&mut egui::Ui, usize)) {
    let context = Context::default();
    let mut durations = Vec::new();
    let mut geometry = (0, 0);
    for frame in 0..70 {
        let started = Instant::now();
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 800.0))),
                events: if hover {
                    vec![Event::PointerMoved(Pos2::new(500.0 + frame as f32, 400.0))]
                } else {
                    vec![]
                },
                ..Default::default()
            },
            |ui| draw(ui, frame),
        );
        output.textures_delta.clear();
        let meshes = context.tessellate(output.shapes, output.pixels_per_point);
        geometry = meshes.iter().fold((0, 0), |(v, i), p| match &p.primitive {
            egui::epaint::Primitive::Mesh(m) => (v + m.vertices.len(), i + m.indices.len()),
            _ => (v, i),
        });
        black_box(meshes);
        if frame >= 10 {
            durations.push(started.elapsed().as_secs_f64() * 1000.0);
        }
    }
    durations.sort_by(f64::total_cmp);
    println!(
        "{label:38}: median {:6.2} ms, p95 {:6.2} ms, {} vertices / {} indices",
        durations[30], durations[57], geometry.0, geometry.1
    );
}
fn data(count: usize, time: f64) -> Vec<[f64; 3]> {
    (0..count)
        .map(|i| {
            let t = i as f64 * 0.013;
            [t.sin(), (t * 1.618 + time).cos(), (t * 0.37).sin()]
        })
        .collect()
}
fn plot(frame: usize) -> Plot<'static> {
    Plot::new("workload")
        .show_axes([false; 3])
        .show_grid(false)
        .show_frame(false)
        .camera(Camera {
            yaw: frame as f32 * 0.01,
            ..Default::default()
        })
}
struct Grid<'a>(&'a PointBuffer);
impl PlotItem for Grid<'_> {
    fn name(&self) -> &str {
        "surface"
    }
    fn bounds(&self) -> PlotBounds {
        self.0.bounds()
    }
    fn render(&self, ctx: &mut PlotItemContext<'_>) {
        let points = self.0.as_slice();
        ctx.reserve(128 * 128 * 2);
        for y in 0..128 {
            for x in 0..128 {
                let a = y * 129 + x;
                for indices in [[a, a + 1, a + 129], [a + 1, a + 130, a + 129]] {
                    ctx.triangle(indices.map(|i| points[i].into()), Color32::LIGHT_BLUE);
                }
            }
        }
    }
}
fn main() {
    println!("CPU frame preparation + tessellation; no GPU, uploads, presentation, or guides");
    let shared = PointBuffer::from(data(100_000, 0.0));
    for hover in [false, true] {
        measure(
            if hover {
                "100k shared points, hover"
            } else {
                "100k shared points"
            },
            hover,
            |ui, frame| {
                plot(frame).show(ui, |p| p.points(Points::new("data", &shared)));
            },
        );
        measure(
            if hover {
                "100k shared line, hover"
            } else {
                "100k shared line"
            },
            hover,
            |ui, frame| {
                plot(frame).show(ui, |p| p.line(Line::new("data", &shared)));
            },
        );
    }
    measure("100k borrowed points (bounds scan)", false, |ui, frame| {
        plot(frame).show(ui, |p| p.points(Points::new("data", shared.as_slice())));
    });
    measure(
        "100k dynamic points (generation included)",
        true,
        |ui, frame| {
            let samples = data(100_000, frame as f64 * 0.03);
            plot(frame).show(ui, |p| p.points(Points::new("data", samples)));
        },
    );
    measure("100k cross markers", false, |ui, frame| {
        plot(frame).show(ui, |p| {
            p.points(Points::new("data", &shared).shape(MarkerShape::Cross))
        });
    });
    measure("100k points, zoomed clipping", true, |ui, frame| {
        plot(frame)
            .camera(Camera {
                zoom: 8.0,
                yaw: frame as f32 * 0.01,
                ..Default::default()
            })
            .show(ui, |p| p.points(Points::new("data", &shared)));
    });
    let grid = PointBuffer::from(
        (0..129 * 129)
            .map(|i| {
                let x = (i % 129) as f64 / 128.0 * 8.0 - 4.0;
                let y = (i / 129) as f64 / 128.0 * 8.0 - 4.0;
                [x, y, x.sin() * y.cos()]
            })
            .collect::<Vec<_>>(),
    );
    measure("32,768 surface triangles", true, |ui, frame| {
        plot(frame).show(ui, |p| p.add(Grid(&grid)));
    });
    let indices: Vec<[u32; 3]> = (0..128 * 128)
        .flat_map(|i| {
            let a = (i / 128 * 129 + i % 128) as u32;
            [[a, a + 1, a + 129], [a + 1, a + 130, a + 129]]
        })
        .collect();
    measure("32,768 indexed surface triangles", true, |ui, frame| {
        plot(frame).show(ui, |p| {
            p.surface(
                egui_plot3d::Surface::new("surface", &grid, indices.as_slice())
                    .color(Color32::LIGHT_BLUE),
            )
        });
    });
    if std::env::args().any(|s| s == "--large") {
        let large = PointBuffer::from(data(1_000_000, 0.0));
        measure("1M points, hover", true, |ui, frame| {
            plot(frame).show(ui, |p| p.points(Points::new("data", &large)));
        });
        measure("1M line samples, hover", true, |ui, frame| {
            plot(frame).show(ui, |p| p.line(Line::new("data", &large)));
        });
    }
}
