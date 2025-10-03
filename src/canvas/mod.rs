//! Integration between [`egui`] and SDL2’s [`sdl2::render::Canvas`] API.
//!
//! This module provides [`EguiCanvas`], a convenience wrapper that bundles
//! together:
//! - [`egui::Context`] for running your UI
//! - [`crate::State`] for event and input handling
//! - [`Painter`] for rendering using [`sdl2::render::Canvas`]
//!
//! # When to use
//! Use [`EguiCanvas`] if you want to render egui using SDL2’s 2D canvas API
//! instead of OpenGL.
//!
//! # Usage
//! Typical usage is to:
//! 1. Create an [`EguiCanvas`] for your SDL2 window and canvas
//! 2. Pass SDL2 events to [`EguiCanvas::on_event`]
//! 3. Call [`EguiCanvas::run`] providing our UI function
//! 4. Paint egui output via [`EguiCanvas::paint`]
//!
pub mod painter;
pub use painter::*;

/// Integration between [`egui`] and [`sdl2::render::Canvas`] for app based on [`sdl2`].
pub struct EguiCanvas {
    run_output: crate::EguiRunOutput,
    pub ctx: egui::Context,
    pub state: crate::State,
    pub painter: Painter,
}

impl EguiCanvas {
    pub fn new(window: sdl2::video::Window) -> Self {
        let ctx = egui::Context::default();
        let state = crate::State::new(&window, ctx.clone(), egui::ViewportId::ROOT);
        let run_output = crate::EguiRunOutput::default();
        let painter = Painter::new(window);

        Self {
            ctx,
            painter,
            state,
            run_output,
        }
    }

    #[inline]
    pub fn on_event(&mut self, event: &sdl2::event::Event) -> crate::EventResponse {
        self.state.on_event(self.painter.canvas.window(), event)
    }

    /// Call [`Self::paint`] later to paint.
    #[inline]
    pub fn run(&mut self, run_ui: impl FnMut(&egui::Context)) {
        self.run_output.update(&self.ctx, &mut self.state, run_ui);
    }

    /// Paint the results of the last call to [`Self::run`].
    pub fn paint(&mut self) {
        let pixels_per_point = self.run_output.pixels_per_point;
        let (textures_delta, shapes) = self.run_output.take();
        let clipped_primitives = self.ctx.tessellate(shapes, pixels_per_point);
        if let Err(e) = self.painter.paint_and_update_textures(
            pixels_per_point,
            &textures_delta,
            clipped_primitives,
        ) {
            log::error!("Failed to paint: {e}");
        }
    }

    #[inline]
    pub fn clear(&mut self, color: [u8; 4]) {
        let color = sdl2::pixels::Color::RGBA(color[0], color[1], color[2], color[3]);
        self.painter.canvas.set_draw_color(color);
        self.painter.canvas.clear();
    }

    /// Call to release the allocated graphics resources.
    pub fn destroy(&mut self) {
        self.painter.destroy();
    }
}
