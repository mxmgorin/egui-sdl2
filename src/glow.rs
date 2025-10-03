//! Integration between [`egui`] and [`glow`] for SDL2 applications.
//!
//! This module provides [`EguiGlow`], a convenience wrapper that bundles
//! together:
//! - [`egui::Context`] for running your UI
//! - [`State`] for event/input handling
//! - [`egui_glow::Painter`] for rendering with OpenGL (via [`glow`])
//!
//! # When to use
//! Use [`EguiGlow`] if you want to render egui using OpenGL through glow
//! in an SDL2 application. If you prefer SDL2’s `Canvas` renderer, see the
//! [`crate::canvas`] module instead.
//!
//! # Usage
//! Typical usage is to:
//! 1. Create an [`EguiGlow`] for your SDL2 window and GL context
//! 2. Pass SDL2 events to [`State::on_event`]
//! 3. Call [`egui::Context::run`] providing your UI fuction
//! 4. Paint egui output via [`EguiGlow::paint`]
//!

/// Integration between [`egui`] and [`glow`] for app based on [`sdl2`].
pub struct EguiGlow {
    run_output: crate::EguiRunOutput,
    pub ctx: egui::Context,
    pub state: crate::State,
    pub painter: egui_glow::Painter,
}

impl EguiGlow {
    /// For automatic shader version detection set `shader_version` to `None`.
    pub fn new(
        window: &sdl2::video::Window,
        glow_ctx: std::sync::Arc<glow::Context>,
        shader_version: Option<egui_glow::ShaderVersion>,
        dithering: bool,
    ) -> Self {
        let painter = egui_glow::Painter::new(glow_ctx, "", shader_version, dithering)
            .map_err(|err| {
                log::error!("error occurred in initializing painter:\n{err}");
            })
            .unwrap();
        let ctx = egui::Context::default();
        let state = crate::State::new(window, ctx.clone(), egui::ViewportId::ROOT);
        let run_output = crate::EguiRunOutput::default();

        Self {
            painter,
            run_output,
            state,
            ctx,
        }
    }

    /// Call [`Self::paint`] later to paint.
    pub fn run(&mut self, run_ui: impl FnMut(&egui::Context)) {
        self.run_output.update(&self.ctx, &mut self.state, run_ui);
    }

    /// Paint the results of the last call to [`Self::run`].
    pub fn paint(&mut self) {
        let pixels_per_point = self.run_output.pixels_per_point;
        let (textures_delta, shapes) = self.run_output.take();
        let clipped_primitives = self.ctx.tessellate(shapes, pixels_per_point);
        let screen_size = self.state.get_window_size();
        self.painter.paint_and_update_textures(
            screen_size.into(),
            pixels_per_point,
            &clipped_primitives,
            &textures_delta,
        );
    }

    /// Call to release the allocated graphics resources.
    pub fn destroy(&mut self) {
        self.painter.destroy();
    }
}
