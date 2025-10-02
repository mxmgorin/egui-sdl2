use sdl2::event::{Event, WindowEvent};
use std::time::Duration;

fn main() {
    let sdl = sdl2::init().unwrap();
    let mut event_pump = sdl.event_pump().unwrap();
    let mut app = pollster::block_on(App::new(&sdl));
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
    egui: egui_sdl2::EguiWgpu,
    // state
    multiline_text: String,
    slider_value: f32,
    pub running: bool,
}

impl App {
    pub async fn new(sdl: &sdl2::Sdl) -> Self {
        let video = sdl.video().unwrap();
        let window = video
            .window("Egui SDL2 WGPU", 800, 600)
            .resizable()
            .build()
            .unwrap();
        let egui = egui_sdl2::EguiWgpu::new(window).await;

        Self {
            egui,
            running: true,
            multiline_text: "Cut, copy, paste here".to_string(),
            slider_value: 0.0,
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
        self.egui.paint([0.0, 0.0, 0.0, 1.0]);
    }
}
