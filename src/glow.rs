use crate::{EguiBackend, PainterTrait, State};
use std::sync::Arc;

/// Integration between [`egui`] and [`glow`] for app based on [`sdl2`].
pub struct EguiGlow {
    backend: EguiBackend,
    pub ctx: egui::Context,
    pub state: State,
    pub painter: egui_glow::Painter,
}

impl EguiGlow {
    /// For automatic shader version detection set `shader_version` to `None`.
    pub fn new(
        window: &sdl2::video::Window,
        glow_ctx: Arc<glow::Context>,
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
        let backend = EguiBackend::new(ctx.clone());

        Self {
            painter,
            backend,
            state,
            ctx,
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

impl PainterTrait for egui_glow::Painter {
    fn paint_primitives(
        &mut self,
        screen_size_px: [u32; 2],
        pixels_per_point: f32,
        clipped_primitives: Vec<egui::ClippedPrimitive>,
    ) {
        self.paint_primitives(screen_size_px, pixels_per_point, &clipped_primitives);
    }

    fn set_texture(&mut self, tex_id: egui::TextureId, delta: &egui::epaint::ImageDelta) {
        self.set_texture(tex_id, delta);
    }

    fn free_texture(&mut self, tex_id: egui::TextureId) {
        self.free_texture(tex_id);
    }
}
