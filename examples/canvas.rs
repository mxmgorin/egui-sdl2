use crate::common::UiExample;
use sdl2::event::{Event, WindowEvent};
use std::time::Duration;
mod common;

fn main() {
    let sdl = sdl2::init().unwrap();
    let mut event_pump = sdl.event_pump().unwrap();
    let mut app = App::new(&sdl);
    const TARGET_FPS: f64 = 60.0;
    let sleep_dur = Duration::from_secs_f64(1.0 / TARGET_FPS);

    while !app.ui.quit {
        for event in event_pump.poll_iter() {
            app.handle_event(&event);
        }

        app.update();
        app.draw();
        std::thread::sleep(sleep_dur);
    }
}

struct App {
    egui: egui_sdl2::EguiCanvas,
    ui: UiExample,
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
            .window("Egui SDL2 Canvas", 800, 600)
            .resizable()
            .build()
            .unwrap();
        let egui = egui_sdl2::EguiCanvas::new(window);

        Self {
            egui,
            ui: UiExample::default(),
        }
    }

    pub fn handle_event(&mut self, event: &Event) {
        let resp = self.egui.on_event(event);

        if !resp.consumed {
            if let Event::Window {
                win_event: WindowEvent::Close,
                ..
            } = event
            {
                self.ui.quit = true;
            }
        }
    }

    pub fn update(&mut self) {
        self.egui.run(|ctx| self.ui.update(ctx));
    }

    pub fn draw(&mut self) {
        self.egui.clear(f32_to_u8_color(self.ui.color));
        self.egui.paint();
        self.egui.painter.canvas.present();
    }
}

fn f32_to_u8_color(c: [f32; 4]) -> [u8; 4] {
    [
        (c[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}
