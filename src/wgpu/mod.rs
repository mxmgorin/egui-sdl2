//! Integration between [`egui`] and [`wgpu`](https://docs.rs/wgpu) API.
//!
//! This module provides [`EguiWgpu`], a convenience wrapper that bundles
//! together:
//! - [`egui::Context`] for running your UI
//! - [`crate::State`] for event and input handling
//! - [`Painter`] for rendering using [`wgpu`](https://docs.rs/wgpu)

//! # Usage
//! Typical usage is to:
//! 1. Create an [`EguiWgpu`] for your SDL2 window
//! 2. Pass SDL2 events to [`EguiWgpu::on_event`]
//! 3. Call [`EguiWgpu::run`] providing our UI function
//! 4. Paint egui output via [`EguiWgpu::paint`]
//!

use std::num::NonZeroU32;
pub mod painter;
pub use painter::*;

/// Integration between [`egui`] and [`wgpu`](https://docs.rs/wgpu) for app based on [`sdl2`].
pub struct EguiWgpu {
    run_output: crate::EguiRunOutput,
    viewport_id: egui::ViewportId,
    pub ctx: egui::Context,
    pub state: crate::State,
    pub painter: painter::Painter,
    pub window: sdl2::video::Window,
}

impl EguiWgpu {
    pub async fn new(window: sdl2::video::Window) -> Self {
        let ctx = egui::Context::default();
        let viewport_id = egui::ViewportId::ROOT;
        let state = crate::State::new(&window, ctx.clone(), viewport_id);
        let run_output = crate::EguiRunOutput::default();
        let config = egui_wgpu::WgpuConfiguration::default();
        let mut painter = painter::Painter::new(ctx.clone(), config, 1, None, true, false).await;
        // SAFETY:
        // Window lives as long as self
        unsafe {
            painter.set_window(viewport_id, &window).await.unwrap();
        }

        Self {
            window,
            ctx,
            painter,
            state,
            run_output,
            viewport_id,
        }
    }

    pub fn on_event(&mut self, event: &sdl2::event::Event) -> crate::EventResponse {
        match event {
            sdl2::event::Event::Window {
                window_id,
                win_event:
                    sdl2::event::WindowEvent::Resized(w, h)
                    | sdl2::event::WindowEvent::SizeChanged(w, h),
                ..
            } if *window_id == self.window.id() && *w > 0 && *h > 0 => {
                let w = NonZeroU32::new(*w as u32).unwrap();
                let h = NonZeroU32::new(*h as u32).unwrap();
                self.painter.on_window_resized(self.viewport_id, w, h);
            }
            _ => {}
        }

        self.state.on_event(&self.window, event)
    }

    /// Call [`Self::paint`] later to paint.
    pub fn run(&mut self, run_ui: impl FnMut(&egui::Context)) {
        self.run_output.update(&self.ctx, &mut self.state, run_ui);
    }

    /// Paint the results of the last call to [`Self::run`].
    pub fn paint(&mut self, clear_color: [f32; 4]) {
        let pixels_per_point = self.run_output.pixels_per_point;
        let (textures_delta, shapes) = self.run_output.take();
        let clipped_primitives = self.ctx.tessellate(shapes, pixels_per_point);
        self.painter.paint_and_update_textures(
            self.viewport_id,
            pixels_per_point,
            clear_color,
            &clipped_primitives,
            &textures_delta,
            Vec::with_capacity(0),
        );
    }
}
