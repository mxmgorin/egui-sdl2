[![CI](https://github.com/mxmgorin/egui-sdl2/actions/workflows/ci.yml/badge.svg)](https://github.com/mxmgorin/egui-sdl2/actions)
[![Dependencies](https://deps.rs/repo/github/mxmgorin/egui-sdl2/status.svg)](https://deps.rs/repo/github/mxmgorin/egui-sdl2)
[![crates.io](https://img.shields.io/crates/v/egui-sdl2.svg)](https://crates.io/crates/egui-sdl2)
[![Documentation](https://docs.rs/egui-sdl2/badge.svg)](https://docs.rs/egui-sdl2)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

# egui-sdl2
This crate provides integration between [`egui`](https://github.com/emilk/egui) and [`sdl2`](https://github.com/Rust-SDL2/rust-sdl2), including event handling and multiple rendering backends with a consistent API. It supports optional rendering backends:

 - Software via [`Canvas`](https://docs.rs/sdl2/latest/sdl2/render/struct.Canvas.html) (`canvas-backend` feature)
 - OpengGL via [`glow`](https://crates.io/crates/glow) (`glow-backend` feature)
 - WebgGPU via [`wgpu`](https://github.com/gfx-rs/wgpu) (`wgpu-backend` feature)

The implementation is based on the design of the official egui-winit, egui_glow, egui-wgpu crates, aming to make it easy to use SDL2 with egui.

Both `egui` and `sdl2` are re-exported for convenience. The `sdl2` re-export includes all feature flags available to use.

## Usage

To get started, create an [`EguiGlow`](https://docs.rs/egui-sdl2/latest/egui_sdl2/glow/index.html) or [`EguiCanvas`](https://docs.rs/egui-sdl2/latest/egui_sdl2/canvas/index.html) or [`EguiWgpu`](https://docs.rs/egui-sdl2/latest/egui_sdl2/wgpu/index.html) instance to manage rendering. Pass SDL2 events to `on_event`, then call `run` and `paint` each frame. For event handling only, you can use the [`State`](https://docs.rs/egui-sdl2/latest/egui_sdl2/state/index.html) type.
Examples are available in the [examples/](https://github.com/mxmgorin/egui-sdl2/tree/main/examples/) directory. To run the `canvas` example:

```sh
cargo run --example canvas
```
