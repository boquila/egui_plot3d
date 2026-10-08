# egui_plot3d

[<img alt="github" src="https://img.shields.io/badge/github-boquila/egui_plot3d-8da0cb?logo=github" height="20">](https://github.com/boquila/egui_plot3d)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![Apache](https://img.shields.io/badge/license-Apache-blue.svg)](LICENSE-APACHE)

Immediate mode 3D plotting library for [`egui`](https://github.com/emilk/egui), with an API inspired by [`egui_plot`](https://github.com/emilk/egui_plot).

![Animated wave interference, spherical harmonics, and torus knots](media/egui_plot3d.gif)

[API examples](src/lib.rs) · [Gallery source](demo/src)

Not yet published to crates.io. Use a local path or Git dependency.

## Testing

- Locally: `cargo run --release -p egui_plot3d_demo`
- Web: `cd demo` then `trunk serve --release`
- Tests: `cargo test --workspace`
- Documentation: `cargo doc --no-deps --open`

The web demo requires [Trunk](https://trunkrs.dev/) and the `wasm32-unknown-unknown` Rust target.

## Plotting libraries in Rust

See egui_plot's [notes on the Rust plotting ecosystem](https://github.com/emilk/egui_plot/blob/main/ECOSYSTEM.md).

## Contributing

[Issues](https://github.com/boquila/egui_plot3d/issues) and [pull requests](https://github.com/boquila/egui_plot3d/pulls) are welcome.
