use crate::painter::Painter;
use egui::ViewportId;

/// Integration between [`egui`] and [`sdl2 canvas`] for app based on [`sdl2`].
pub struct EguiCanvas {
    pub ctx: egui::Context,
    pub painter: Painter,
    pub state: crate::State,

    // output from the last run:
    shapes: Vec<egui::epaint::ClippedShape>,
    pixels_per_point: f32,
    textures_delta: egui::TexturesDelta,
}

impl EguiCanvas {
    pub fn new(window: sdl2::video::Window) -> Self {
        let ctx = egui::Context::default();
        let state = crate::State::new(&window, ctx.clone(), ViewportId::ROOT);
        let painter = Painter::new(window);

        Self {
            ctx,
            painter,
            state,
            shapes: Default::default(),
            pixels_per_point: 1.0,
            textures_delta: Default::default(),
        }
    }

    /// Call [`Self::paint`] later to paint.
    pub fn run(&mut self, run_ui: impl FnMut(&egui::Context)) {
        let raw_input = self.state.take_egui_input();
        let egui::FullOutput {
            platform_output,
            viewport_output: _,
            textures_delta,
            shapes,
            pixels_per_point,
        } = self.ctx.run(raw_input, run_ui);
        self.state.handle_platform_output(platform_output);

        self.shapes = shapes;
        self.textures_delta.append(textures_delta);
        self.pixels_per_point = pixels_per_point;
    }

    /// Paint the results of the last call to [`Self::run`].
    pub fn paint(&mut self) {
        let mut textures_delta = std::mem::take(&mut self.textures_delta);

        for (id, image_delta) in textures_delta.set {
            self.painter.set_texture(id, &image_delta);
        }

        let pixels_per_point = self.pixels_per_point;
        let shapes = std::mem::take(&mut self.shapes);
        let clipped_primitives = self.ctx.tessellate(shapes, pixels_per_point);
        self.painter
            .paint_primitives(pixels_per_point, clipped_primitives);

        for id in textures_delta.free.drain(..) {
            self.painter.free_texture(&id);
        }
    }

    /// Call to release the allocated graphics resources.
    pub fn destroy(&mut self) {
        self.painter.destroy();
    }
}
