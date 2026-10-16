use egui::{Context, Pos2, RawInput, Rect, Vec2};
use egui_plot3d::{Camera, Line, Plot, PointBuffer, Points};
use std::{hint::black_box, time::Instant};
fn main() {
    println!("CPU frame preparation + egui tessellation, rotating camera, no GPU or presentation");
    for count in [20_000, 100_000] {
        let data = PointBuffer::from(
            (0..count)
                .map(|i| {
                    let t = i as f64 * 0.013;
                    [t.sin(), (t * 1.618).cos(), (t * 0.37).sin()]
                })
                .collect::<Vec<_>>(),
        );
        for lines in [false, true] {
            let ctx = Context::default();
            let mut samples = Vec::new();
            for frame in 0..70 {
                let now = Instant::now();
                let mut output = ctx.run_ui(
                    RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            Vec2::new(1000.0, 800.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        Plot::new("bench")
                            .camera(Camera {
                                yaw: frame as f32 * 0.01,
                                ..Default::default()
                            })
                            .show(ui, |p| {
                                if lines {
                                    p.line(Line::new("data", &data));
                                } else {
                                    p.points(Points::new("data", &data));
                                }
                            });
                    },
                );
                output.textures_delta.clear();
                black_box(ctx.tessellate(output.shapes, output.pixels_per_point));
                if frame >= 10 {
                    samples.push(now.elapsed().as_secs_f64() * 1000.0);
                }
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "{count:>6} {}: median {:.2} ms, p95 {:.2} ms",
                if lines {
                    "line samples"
                } else {
                    "points      "
                },
                samples[30],
                samples[57]
            );
        }
    }
}
