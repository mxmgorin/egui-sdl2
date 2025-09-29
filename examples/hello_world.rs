use glow::HasContext;
use sdl2::{
    event::{Event, WindowEvent},
    video::GLContext,
};
use std::{sync::Arc, time::Duration};

fn main() {
    let sdl = sdl2::init().unwrap();
    let mut event_pump = sdl.event_pump().unwrap();
    let mut app = App::new(&sdl);

    while !app.quit {
        app.update();
        app.draw();

        let event = if let Some(repaint_delay) = app.repaint_delay.take() {
            event_pump.wait_event_timeout(repaint_delay.as_millis() as u32)
        } else {
            Some(event_pump.wait_event())
        };

        if let Some(event) = event {
            app.handle_event(&event);
        }
    }
}

struct App {
    _gl_ctx: GLContext,
    glow_ctx: Arc<glow::Context>,
    window: sdl2::video::Window,
    egui: egui_sdl2::EguiGlow,
    repaint_delay: Option<Duration>,
    repaint_pending: bool,

    multiline_text: String,
    slider_value: f32,
    pub quit: bool,
}

impl Drop for App {
    fn drop(&mut self) {
        self.egui.destroy();
    }
}

impl App {
    pub fn new(sdl: &sdl2::Sdl) -> Self {
        let video = sdl.video().unwrap();
        let window = video
            .window("Egui-sdl2", 800, 600)
            .opengl()
            .build()
            .unwrap();
        let gl_ctx = window
            .gl_create_context()
            .expect("Failed to create GL context");
        window.gl_make_current(&gl_ctx).unwrap();
        let glow_ctx = Arc::new(unsafe {
            glow::Context::from_loader_function(|name| video.gl_get_proc_address(name) as *const _)
        });
        let egui = egui_sdl2::EguiGlow::new(&window, glow_ctx.clone(), None, false);

        Self {
            _gl_ctx: gl_ctx,
            glow_ctx,
            window,
            egui,
            repaint_delay: None,
            repaint_pending: false,
            quit: false,
            multiline_text: "Cut, copy, paste".to_string(),
            slider_value: 0.0,
        }
    }

    pub fn handle_event(&mut self, event: &Event) {
        let resp = self.egui.state.on_event(&self.window, event);
        self.repaint_pending = resp.repaint;

        if !resp.consumed {
            if let Event::Window {
                win_event: WindowEvent::Close,
                ..
            } = event
            {
                self.quit = true;
            }
        }
    }

    pub fn update(&mut self) {
        let repaint_delay = self.egui.run(|ctx| {
            egui::Window::new("Hello, world!").show(ctx, |ui| {
                ui.label("Hello, world!");

                if ui.button("Greet").clicked() {
                    self.multiline_text = "Hello, world!".to_string();
                    println!("{}", &self.multiline_text);
                }
                ui.text_edit_multiline(&mut self.multiline_text);
                ui.add(egui::Slider::new(&mut self.slider_value, 0.0..=50.0).text("Slider"));
                ui.separator();

                if ui.button("Quit?").clicked() {
                    self.quit = true;
                }
            });
        });
        self.repaint_delay.replace(repaint_delay);
    }

    pub fn draw(&mut self) {
        if self.repaint_pending || self.repaint_delay.is_some() {
            self.repaint_pending = false;
            unsafe {
                self.glow_ctx.clear_color(0.0, 0.0, 0.0, 1.0);
                self.glow_ctx.clear(glow::COLOR_BUFFER_BIT);
            }
            self.egui.paint();
            self.window.gl_swap_window();
        }
    }
}
