[![Dependencies](https://deps.rs/repo/github/mxmgorin/egui-sdl2/status.svg)](https://deps.rs/repo/github/mxmgorin/egui-sdl2)
[![crates.io](https://img.shields.io/crates/v/egui-sdl2.svg)](https://crates.io/crates/egui-sdl2)
[![Documentation](https://docs.rs/egui-sdl2/badge.svg)](https://docs.rs/egui-sdl2)

# egui-sdl2

This crate provides integration between [`egui`](https://github.com/emilk/egui) and [`sdl2`](https://github.com/Rust-SDL2/rust-sdl2). It also includes optional [`glow`](https://crates.io/crates/glow) support for rendering. Implementation follows the design of the official egui-winit and egui_glow crates, using them as references.

Features include:

- Translation of sdl2 events to egui.
- Handling egui's PlatformOutput (clipboard, cursor updates, opening links — optional via `links` feature).
- Rendering egui via glow (optional via `glow-backend`)

Both `egui` and `sdl2` are re-exported for convenience. The `sdl2` re-export includes all feature flags available to use.

Examples can be found [here](https://github.com/mxmgorin/egui-sdl2/tree/main/examples/). Run the example with:
``` sh
cargo run --example hello_world
```
