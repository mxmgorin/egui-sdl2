use crate::{painter::Painter, EguiBackend, State};
use egui::ViewportId;

/// Integration between [`egui`] and [`sdl2::canvas`] for app based on [`sdl2`].
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
