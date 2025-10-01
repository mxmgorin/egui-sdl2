[![CI](https://github.com/mxmgorin/egui-sdl2/actions/workflows/ci.yml/badge.svg)](https://github.com/mxmgorin/egui-sdl2/actions)
[![Dependencies](https://deps.rs/repo/github/mxmgorin/egui-sdl2/status.svg)](https://deps.rs/repo/github/mxmgorin/egui-sdl2)
[![crates.io](https://img.shields.io/crates/v/egui-sdl2.svg)](https://crates.io/crates/egui-sdl2)
[![Documentation](https://docs.rs/egui-sdl2/badge.svg)](https://docs.rs/egui-sdl2)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

# egui-sdl2

This crate provides integration between [`egui`](https://github.com/emilk/egui) and [`sdl2`](https://github.com/Rust-SDL2/rust-sdl2). It also includes optional [`glow`](https://crates.io/crates/glow) and `canvas` support for rendering. Implementation follows the design of the official egui-winit and egui_glow crates, using them as references.

Features include:

- Translation of SDL2 events to egui.
- Handling egui's PlatformOutput (clipboard, cursor updates, opening links — optional via `links` feature).
- Rendering egui via glow (optional via `glow-backend`)
- Rendering egui via SDL2 canvas (optional via `canvas-backend`)

Both `egui` and `sdl2` are re-exported for convenience. The `sdl2` re-export includes all feature flags available to use.

## Usage

To get started, create an `EguiGlow` or `EguiCanvas` instance to manage rendering. Pass SDL2 events to it, then call `run` and `paint` each frame. If you only need event handling, you can use the only `State` type.
Examples are available in the [examples/](https://github.com/mxmgorin/egui-sdl2/tree/main/examples/) directory. To run the `canvas` example:

```sh
cargo run --example canvas
```
