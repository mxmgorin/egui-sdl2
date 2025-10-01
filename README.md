[![CI](https://github.com/mxmgorin/egui-sdl2/actions/workflows/ci.yml/badge.svg)](https://github.com/mxmgorin/egui-sdl2/actions)
[![Dependencies](https://deps.rs/repo/github/mxmgorin/egui-sdl2/status.svg)](https://deps.rs/repo/github/mxmgorin/egui-sdl2)
[![crates.io](https://img.shields.io/crates/v/egui-sdl2.svg)](https://crates.io/crates/egui-sdl2)
[![Documentation](https://docs.rs/egui-sdl2/badge.svg)](https://docs.rs/egui-sdl2)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

# egui-sdl2

This crate provides integration between [`egui`](https://github.com/emilk/egui) and [`sdl2`](https://github.com/Rust-SDL2/rust-sdl2). It also includes optional OpenGL rendering via [`glow`](https://crates.io/crates/glow) and software rendering via [`Canvas`](https://docs.rs/sdl2/latest/sdl2/render/struct.Canvas.html). The implementation is based on the design of the official egui-winit and egui_glow crates.

Features:

- Translate SDL2 events into [`egui`] events.
- Handle [`egui::PlatformOutput`] (clipboard, cursor updates, links).
- Render with OpenGL via [`glow`] (`glow-backend` feature).
- Render with the SDL2 software renderer via [`Canvas`] (`canvas-backend` feature).

Both `egui` and `sdl2` are re-exported for convenience. The `sdl2` re-export includes all feature flags available to use.

## Usage

To get started, create an [`EguiGlow`](https://docs.rs/egui-sdl2/latest/egui_sdl2/glow/index.html) or [`EguiCanvas`](https://docs.rs/egui-sdl2/latest/egui_sdl2/canvas/index.html) instance to manage rendering. Pass SDL2 events to it, then call `run` and `paint` each frame. For event handling only, you can use the [`State`](https://docs.rs/egui-sdl2/latest/egui_sdl2/state/index.html) type.
Examples are available in the [examples/](https://github.com/mxmgorin/egui-sdl2/tree/main/examples/) directory. To run the `canvas` example:

```sh
cargo run --example canvas
```
