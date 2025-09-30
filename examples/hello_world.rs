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
    const TARGET_FPS: f64 = 60.0;
    let sleep_dur = Duration::from_secs_f64(1.0 / TARGET_FPS);

    while app.running {
        for event in event_pump.poll_iter() {
            app.handle_event(&event);
        }

        app.update();
        app.draw();
        std::thread::sleep(sleep_dur);
    }
}

struct App {
    _gl_ctx: GLContext,
    window: sdl2::video::Window,
    egui: egui_sdl2::EguiGlow,
    // state
    multiline_text: String,
    slider_value: f32,
    pub running: bool,
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
            .resizable()
            .build()
            .unwrap();
        let gl_ctx = window
            .gl_create_context()
            .expect("Failed to create GL context");
        window.gl_make_current(&gl_ctx).unwrap();
        let glow_ctx = Arc::new(unsafe {
            glow::Context::from_loader_function(|name| video.gl_get_proc_address(name) as *const _)
        });
        let egui = egui_sdl2::EguiGlow::new(&window, glow_ctx, None, false);

        Self {
            _gl_ctx: gl_ctx,
            window,
            egui,
            running: true,
            multiline_text: "Cut, copy, paste here".to_string(),
            slider_value: 0.0,
        }
    }

    pub fn handle_event(&mut self, event: &Event) {
        let resp = self.egui.state.on_event(&self.window, event);

        if !resp.consumed {
            if let Event::Window {
                win_event: WindowEvent::Close,
                ..
            } = event
            {
                self.running = false;
            }
        }
    }

    pub fn update(&mut self) {
        self.egui.run(|ctx| {
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
                    self.running = false;
                }
            });
        });
    }

    pub fn draw(&mut self) {
        unsafe {
            self.egui.painter.gl().clear_color(0.0, 0.0, 0.0, 1.0);
            self.egui.painter.gl().clear(glow::COLOR_BUFFER_BIT);
        }
        self.egui.paint();
        self.window.gl_swap_window();
    }
}
