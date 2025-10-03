#[allow(dead_code)]
pub const TARGET_FPS: f64 = 60.0;

pub struct UiExample {
    pub multiline_text: String,
    pub slider_value: f32,
    pub color: [f32; 4],
    pub quit: bool,
}

impl Default for UiExample {
    fn default() -> Self {
        Self {
            multiline_text: String::new(),
            slider_value: 0.0,
            color: egui::Rgba::BLACK.to_array(),
            quit: false,
        }
    }
}

impl UiExample {
    pub fn update(&mut self, ctx: &egui::Context) {
        egui::Window::new("Hello, world!").show(ctx, |ui| {
            ui.label("Hello, world!");

            if ui.button("Greet").clicked() {
                self.multiline_text = "Hello, world!".to_string();
                println!("{}", &self.multiline_text);
            }

            ui.text_edit_multiline(&mut self.multiline_text);
            ui.add(egui::Slider::new(&mut self.slider_value, 0.0..=50.0).text("Slider"));

            ui.horizontal(|ui| {
                ui.label("Color: ");
                ui.color_edit_button_rgba_premultiplied(&mut self.color);
            });

            ui.separator();

            if ui.button("Quit?").clicked() {
                self.quit = false;
            }
        });
    }
}
