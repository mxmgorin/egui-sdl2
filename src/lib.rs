//! # egui-sdl2
//!
//! Integration between [`egui`](https://github.com/emilk/egui) and
//! [`sdl2`](https://github.com/Rust-SDL2/rust-sdl2).
//!
//! ## Features
//! - Translate SDL2 events into [`egui`] events.
//! - Handle [`egui::PlatformOutput`] (clipboard, cursor updates, links).
//! - Render with OpenGL via [`glow`] (`glow-backend` feature).
//! - Render with the SDL2 software renderer via [`sdl2::render::Canvas`] (`canvas-backend` feature).
//!
//! ## Usage
//! ```no_run
//! // Create SDL2 window:
//! let sdl = sdl2::init().unwrap();
//! let video = sdl.video().unwrap();
//! let window = video.window("Egui SDL2 Canvas", 800, 600).build().unwrap();
//! let mut event_pump = sdl.event_pump().unwrap();
//! // Create egui renderer:
//! let mut egui = egui_sdl2::EguiCanvas::new(window);
//! // Feed SDL2 events into egui:
//! let window = egui.painter.canvas.window();
//! let event = event_pump.wait_event();
//! egui.state.on_event(window, &event);
//! // Call `run` + `paint` each frame:
//! egui.run(|ctx: &egui::Context| {});
//! egui.paint();
//! ```

pub use egui;
#[cfg(feature = "glow-backend")]
pub use egui_glow;
pub use sdl2;

#[cfg(feature = "canvas-backend")]
pub mod canvas;
#[cfg(feature = "glow-backend")]
pub mod glow;
pub mod painter;
pub mod state;

#[cfg(feature = "canvas-backend")]
pub use canvas::*;
#[cfg(feature = "glow-backend")]
pub use glow::*;
pub use state::*;

struct EguiBackend {
    pub ctx: egui::Context,

    // output from the last run:
    shapes: Vec<egui::epaint::ClippedShape>,
    pixels_per_point: f32,
    textures_delta: egui::TexturesDelta,
}

impl EguiBackend {
    pub fn new(ctx: egui::Context) -> Self {
        Self {
            ctx,
            shapes: Default::default(),
            pixels_per_point: 1.0,
            textures_delta: Default::default(),
        }
    }

    #[inline]
    pub fn run(&mut self, state: &mut State, run_ui: impl FnMut(&egui::Context)) {
        let raw_input = state.take_egui_input();
        let egui::FullOutput {
            platform_output,
            viewport_output: _,
            textures_delta,
            shapes,
            pixels_per_point,
        } = self.ctx.run(raw_input, run_ui);
        state.handle_platform_output(platform_output);

        self.shapes = shapes;
        self.textures_delta.append(textures_delta);
        self.pixels_per_point = pixels_per_point;
    }

    #[inline]
    pub fn paint(&mut self, state: &State, painter: &mut impl PainterTrait) {
        let mut textures_delta = std::mem::take(&mut self.textures_delta);

        for (id, image_delta) in textures_delta.set {
            painter.set_texture(id, &image_delta);
        }

        let pixels_per_point = self.pixels_per_point;
        let shapes = std::mem::take(&mut self.shapes);
        let clipped_primitives = self.ctx.tessellate(shapes, pixels_per_point);
        let size = state.get_window_size();
        painter.paint_primitives(size.into(), pixels_per_point, clipped_primitives);

        for id in textures_delta.free.drain(..) {
            painter.free_texture(id);
        }
    }
}

trait PainterTrait {
    fn paint_primitives(
        &mut self,
        screen_size_px: [u32; 2],
        pixels_per_point: f32,
        clipped_primitives: Vec<egui::ClippedPrimitive>,
    );
    fn set_texture(&mut self, tex_id: egui::TextureId, delta: &egui::epaint::ImageDelta);
    fn free_texture(&mut self, tex_id: egui::TextureId);
}
