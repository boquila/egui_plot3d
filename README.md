# egui_plot3d

[<img alt="github" src="https://img.shields.io/badge/github-boquila/egui_plot3d-8da0cb?logo=github" height="20">](https://github.com/boquila/egui_plot3d)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![Apache](https://img.shields.io/badge/license-Apache-blue.svg)](LICENSE-APACHE)

Immediate mode 3D plotting library for [`egui`](https://github.com/emilk/egui), with an API inspired by [`egui_plot`](https://github.com/emilk/egui_plot).

![Animated wave interference, spherical harmonics, and torus knots](https://raw.githubusercontent.com/boquila/egui_plot3d/main/assets/egui_plot3d.gif)

[Live demo](https://boquila.org/egui_plot3d/) · [API examples](https://github.com/boquila/egui_plot3d/blob/main/src/lib.rs) · [Gallery source](https://github.com/boquila/egui_plot3d/tree/main/examples/demo/src)

## Testing

- Locally: `cargo run --release -p egui_plot3d_demo`
- Data explorer: `cargo run --release -p egui_plot3d_demo --bin explorer`
- Web: `cd examples/demo` then `trunk serve --release`
- Tests: `cargo test --workspace`
- Benchmarks: `cargo bench --bench render` or `cargo bench --bench workloads`
- Documentation: `cargo doc --no-deps --open`
- As a library: point the repo at your clanker
