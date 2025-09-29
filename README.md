[![Dependencies](https://deps.rs/repo/github/mxmgorin/egui-sdl2/status.svg)](https://deps.rs/repo/github/mxmgorin/egui-sdl2)

# egui-sdl2

This crate provides integration between [`egui`](https://github.com/emilk/egui) and [`sdl2`](https://github.com/Rust-SDL2/rust-sdl2). It also includes optional [`glow`](https://crates.io/crates/glow) support for rendering.

Features include:

- Translation sdl2 events to egui.
- Handling egui's PlatformOutput (clipboard, cursor updates, opening links — optional via `links` feature).
- Rendering egui via glow (optional via `glow-backend`)

Both egui and sdl2 are re-exported for convenience, sdl2 is re-exported with all of its feature flags available for use.

Implementation follows the design of the official egui-winit and egui_glow crates, using them as references.
