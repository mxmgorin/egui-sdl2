//! Integration between [`egui`] and SDL2’s [`sdl2::render::Canvas`] API.
//!
//! This module provides [`EguiCanvas`], a convenience wrapper that bundles
//! together:
//! - [`egui::Context`] for running your UI
//! - [`State`] for event and input handling
//! - [`Painter`] for rendering using [`sdl2::render::Canvas`]
//!
//! # When to use
//! Use [`EguiCanvas`] if you want to render egui using SDL2’s 2D canvas API
//! instead of OpenGL.
//!
//! # Usage
//! Typical usage is to:
//! 1. Create an [`EguiCanvas`] for your SDL2 window and canvas
//! 2. Pass SDL2 events to [`State::on_event`]
//! 3. Call [`egui::Context::run`] providing our UI function
//! 4. Paint egui output via [`EguiCanvas::paint`]
//!
use crate::{painter::Painter, EguiBackend, State};
use egui::ViewportId;

/// Integration between [`egui`] and [`sdl2::render::Canvas`] for app based on [`sdl2`].
pub struct EguiCanvas {
    backend: EguiBackend,
    pub ctx: egui::Context,
    pub state: State,
    pub painter: Painter,
}

impl EguiCanvas {
    pub fn new(window: sdl2::video::Window) -> Self {
        let ctx = egui::Context::default();
        let state = crate::State::new(&window, ctx.clone(), ViewportId::ROOT);
        let backend = EguiBackend::new(ctx.clone());
        let painter = Painter::new(window);

        Self {
            ctx,
            painter,
            state,
            backend,
        }
    }

    /// Call [`Self::paint`] later to paint.
    pub fn run(&mut self, run_ui: impl FnMut(&egui::Context)) {
        self.backend.run(&mut self.state, run_ui);
    }

    /// Paint the results of the last call to [`Self::run`].
    pub fn paint(&mut self) {
        self.backend.paint(&self.state, &mut self.painter);
    }

    /// Call to release the allocated graphics resources.
    pub fn destroy(&mut self) {
        self.painter.destroy();
    }
}

impl crate::PainterTrait for Painter {
    fn paint_primitives(
        &mut self,
        _screen_size_px: [u32; 2],
        pixels_per_point: f32,
        clipped_primitives: Vec<egui::ClippedPrimitive>,
    ) {
        self.paint_primitives(pixels_per_point, clipped_primitives);
    }

    fn set_texture(&mut self, tex_id: egui::TextureId, delta: &egui::epaint::ImageDelta) {
        self.set_texture(tex_id, delta);
    }

    fn free_texture(&mut self, tex_id: egui::TextureId) {
        self.free_texture(&tex_id);
    }
}
