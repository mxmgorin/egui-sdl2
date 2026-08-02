//! A window plus the best renderer it can get: [`EguiWindow`] walks a list of
//! [`Renderer`]s and keeps the first that comes up, so an app on a device with
//! missing or broken GL drivers still shows a UI instead of exiting.
//!
//! The window is owned here because the attempts need different ones — GL sets
//! flags on the [`WindowBuilder`], wgpu takes the window by value — and a
//! builder cannot be reused. Pass the title and size; `configure` runs on a
//! fresh builder per attempt, for whatever else the window needs.
//!
//! ```no_run
//! let sdl = sdl2::init().unwrap();
//! let video = sdl.video().unwrap();
//! let mut egui = egui_sdl2::EguiWindow::new(
//!     &video,
//!     "Egui SDL2",
//!     (800, 600),
//!     |builder| {
//!         builder.resizable();
//!     },
//!     &egui_sdl2::Renderer::FALLBACK_CHAIN,
//! )
//! .unwrap();
//! let mut event_pump = sdl.event_pump().unwrap();
//! loop {
//!     for event in event_pump.poll_iter() {
//!         egui.on_event(&event);
//!     }
//!     egui.run(|ctx: &egui::Context| {});
//!     egui.paint([0.1, 0.1, 0.1, 1.0]);
//! }
//! ```

use sdl2::event::Event;
use sdl2::video::{Window, WindowBuilder};
use sdl2::VideoSubsystem;

/// A way to put egui on screen. Apps list the ones they accept, best first.
#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Renderer {
    /// OpenGL ES 3.0 through [`glow`](https://crates.io/crates/glow) — what
    /// mobile and handheld GPU blobs expose.
    Gles3,
    /// Desktop OpenGL 3.2 core through glow.
    Gl32,
    /// SDL's own 2D renderer: an accelerated driver when SDL finds one, its
    /// software rasterizer otherwise. The one that needs no GL at all.
    Canvas,
    /// wgpu (`wgpu-backend` feature).
    Wgpu,
}

impl Renderer {
    /// GL first, SDL's renderer as the safety net.
    pub const FALLBACK_CHAIN: [Renderer; 3] = [Renderer::Gles3, Renderer::Gl32, Renderer::Canvas];

    fn name(self) -> &'static str {
        match self {
            Renderer::Gles3 => "GLES 3.0",
            Renderer::Gl32 => "GL 3.2 core",
            Renderer::Canvas => "SDL renderer",
            Renderer::Wgpu => "wgpu",
        }
    }
}

enum Backend {
    #[cfg(feature = "glow-backend")]
    Glow {
        window: Window,
        // Kept alive for the lifetime of the window; dropping it destroys the context.
        _gl_context: sdl2::video::GLContext,
        egui: crate::EguiGlow,
    },
    #[cfg(feature = "canvas-backend")]
    Canvas {
        canvas: sdl2::render::WindowCanvas,
        egui: crate::EguiCanvas,
    },
    // Boxed: an EguiWgpu is twice the size of the other variants.
    #[cfg(feature = "wgpu-backend")]
    Wgpu { egui: Box<crate::EguiWgpu> },
}

/// egui and the window it draws into, over whichever renderer was available.
/// Vsync is on; every backend clears, paints and presents in [`Self::paint`].
pub struct EguiWindow {
    backend: Backend,
    renderer: Renderer,
}

impl EguiWindow {
    /// Try `order` in sequence, returning the first renderer that comes up.
    /// The error is the last attempt's, since that is the one that decided it.
    pub fn new(
        video: &VideoSubsystem,
        title: &str,
        size: (u32, u32),
        configure: impl Fn(&mut WindowBuilder),
        order: &[Renderer],
    ) -> Result<Self, String> {
        let make_window = |video: &VideoSubsystem, gl: bool| {
            let mut builder = video.window(title, size.0, size.1);
            if gl {
                builder.opengl();
            }
            configure(&mut builder);
            builder.build().map_err(|e| e.to_string())
        };
        let mut last = "no renderer requested".to_string();
        for &renderer in order {
            match build(video, &make_window, renderer) {
                Ok(backend) => {
                    log::info!("egui renderer: {}", renderer.name());
                    return Ok(Self { backend, renderer });
                }
                Err(e) => {
                    log::warn!("{} unavailable: {e}", renderer.name());
                    last = e;
                }
            }
        }
        Err(last)
    }

    /// Which one won, for the app's own logging and about screens.
    pub fn renderer(&self) -> Renderer {
        self.renderer
    }

    pub fn ctx(&self) -> &egui::Context {
        match &self.backend {
            #[cfg(feature = "glow-backend")]
            Backend::Glow { egui, .. } => &egui.ctx,
            #[cfg(feature = "canvas-backend")]
            Backend::Canvas { egui, .. } => &egui.ctx,
            #[cfg(feature = "wgpu-backend")]
            Backend::Wgpu { egui } => &egui.ctx,
        }
    }

    pub fn window(&self) -> &Window {
        match &self.backend {
            #[cfg(feature = "glow-backend")]
            Backend::Glow { window, .. } => window,
            #[cfg(feature = "canvas-backend")]
            Backend::Canvas { canvas, .. } => canvas.window(),
            #[cfg(feature = "wgpu-backend")]
            Backend::Wgpu { egui } => &egui.window,
        }
    }

    /// For the things SDL only does through the window itself, like
    /// [`Window::set_icon`].
    pub fn window_mut(&mut self) -> &mut Window {
        match &mut self.backend {
            #[cfg(feature = "glow-backend")]
            Backend::Glow { window, .. } => window,
            #[cfg(feature = "canvas-backend")]
            Backend::Canvas { canvas, .. } => canvas.window_mut(),
            #[cfg(feature = "wgpu-backend")]
            Backend::Wgpu { egui } => &mut egui.window,
        }
    }

    /// Feed an SDL event to egui; wgpu also resizes its surface here.
    pub fn on_event(&mut self, event: &Event) -> crate::EventResponse {
        match &mut self.backend {
            #[cfg(feature = "glow-backend")]
            Backend::Glow { window, egui, .. } => egui.state.on_event(window, event),
            #[cfg(feature = "canvas-backend")]
            Backend::Canvas { canvas, egui } => egui.state.on_event(canvas.window(), event),
            #[cfg(feature = "wgpu-backend")]
            Backend::Wgpu { egui } => egui.on_event(event),
        }
    }

    /// Run the UI; [`Self::paint`] puts the result on screen.
    pub fn run(&mut self, run_ui: impl FnMut(&egui::Context)) {
        match &mut self.backend {
            #[cfg(feature = "glow-backend")]
            Backend::Glow { egui, .. } => egui.run(run_ui),
            #[cfg(feature = "canvas-backend")]
            Backend::Canvas { egui, .. } => egui.run(run_ui),
            #[cfg(feature = "wgpu-backend")]
            Backend::Wgpu { egui } => egui.run(run_ui),
        }
    }

    /// Like [`Self::run`], but hands the closure egui's root [`egui::Ui`].
    pub fn run_ui(&mut self, run_ui: impl FnMut(&mut egui::Ui)) {
        match &mut self.backend {
            #[cfg(feature = "glow-backend")]
            Backend::Glow { egui, .. } => egui.run_ui(run_ui),
            #[cfg(feature = "canvas-backend")]
            Backend::Canvas { egui, .. } => egui.run_ui(run_ui),
            #[cfg(feature = "wgpu-backend")]
            Backend::Wgpu { egui } => egui.run_ui(run_ui),
        }
    }

    /// How long until egui wants another frame, from the last [`Self::run`].
    pub fn repaint_delay(&self) -> std::time::Duration {
        match &self.backend {
            #[cfg(feature = "glow-backend")]
            Backend::Glow { egui, .. } => egui.repaint_delay(),
            #[cfg(feature = "canvas-backend")]
            Backend::Canvas { egui, .. } => egui.repaint_delay(),
            #[cfg(feature = "wgpu-backend")]
            Backend::Wgpu { egui } => egui.repaint_delay(),
        }
    }

    /// Clear to `clear_color`, paint the last [`Self::run`], present.
    pub fn paint(&mut self, clear_color: [f32; 4]) {
        match &mut self.backend {
            #[cfg(feature = "glow-backend")]
            Backend::Glow { window, egui, .. } => {
                egui.clear(clear_color);
                egui.paint();
                window.gl_swap_window();
            }
            #[cfg(feature = "canvas-backend")]
            Backend::Canvas { canvas, egui } => {
                canvas.set_draw_color(rgb(clear_color));
                canvas.clear();
                egui.paint(canvas);
                canvas.present();
            }
            #[cfg(feature = "wgpu-backend")]
            Backend::Wgpu { egui } => egui.paint(clear_color),
        }
    }

    /// Release the renderer's graphics resources.
    pub fn destroy(&mut self) {
        match &mut self.backend {
            #[cfg(feature = "glow-backend")]
            Backend::Glow { egui, .. } => egui.destroy(),
            #[cfg(feature = "canvas-backend")]
            Backend::Canvas { egui, .. } => egui.destroy(),
            #[cfg(feature = "wgpu-backend")]
            Backend::Wgpu { .. } => {}
        }
    }
}

fn build(
    video: &VideoSubsystem,
    make_window: &impl Fn(&VideoSubsystem, bool) -> Result<Window, String>,
    renderer: Renderer,
) -> Result<Backend, String> {
    match renderer {
        Renderer::Gles3 => build_glow(video, make_window, sdl2::video::GLProfile::GLES, 3, 0),
        Renderer::Gl32 => build_glow(video, make_window, sdl2::video::GLProfile::Core, 3, 2),
        Renderer::Canvas => build_canvas(video, make_window),
        Renderer::Wgpu => build_wgpu(video, make_window),
    }
}

#[cfg(feature = "glow-backend")]
fn build_glow(
    video: &VideoSubsystem,
    make_window: &impl Fn(&VideoSubsystem, bool) -> Result<Window, String>,
    profile: sdl2::video::GLProfile,
    major: u8,
    minor: u8,
) -> Result<Backend, String> {
    {
        let gl_attr = video.gl_attr();
        gl_attr.set_context_profile(profile);
        gl_attr.set_context_version(major, minor);
        gl_attr.set_double_buffer(true);
    }
    let window = make_window(video, true)?;
    let gl_context = window.gl_create_context()?;
    window.gl_make_current(&gl_context)?;
    let _ = video.gl_set_swap_interval(sdl2::video::SwapInterval::VSync);

    let glow_ctx = std::sync::Arc::new(unsafe {
        glow::Context::from_loader_function(|name| {
            video.gl_get_proc_address(name) as *const std::os::raw::c_void
        })
    });
    let egui = crate::EguiGlow::new(&window, glow_ctx, None, false);
    Ok(Backend::Glow {
        window,
        _gl_context: gl_context,
        egui,
    })
}

#[cfg(not(feature = "glow-backend"))]
fn build_glow(
    _video: &VideoSubsystem,
    _make_window: &impl Fn(&VideoSubsystem, bool) -> Result<Window, String>,
    _profile: sdl2::video::GLProfile,
    _major: u8,
    _minor: u8,
) -> Result<Backend, String> {
    Err("built without the glow-backend feature".to_string())
}

#[cfg(feature = "canvas-backend")]
fn build_canvas(
    video: &VideoSubsystem,
    make_window: &impl Fn(&VideoSubsystem, bool) -> Result<Window, String>,
) -> Result<Backend, String> {
    let window = make_window(video, false)?;
    let canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;
    log::debug!("SDL renderer driver: {}", canvas.info().name);
    let egui = crate::EguiCanvas::new(&canvas);
    Ok(Backend::Canvas { canvas, egui })
}

#[cfg(not(feature = "canvas-backend"))]
fn build_canvas(
    _video: &VideoSubsystem,
    _make_window: &impl Fn(&VideoSubsystem, bool) -> Result<Window, String>,
) -> Result<Backend, String> {
    Err("built without the canvas-backend feature".to_string())
}

#[cfg(feature = "wgpu-backend")]
fn build_wgpu(
    video: &VideoSubsystem,
    make_window: &impl Fn(&VideoSubsystem, bool) -> Result<Window, String>,
) -> Result<Backend, String> {
    let window = make_window(video, false)?;
    // wgpu's setup is async; this is startup, so blocking on it is the whole
    // ceremony an app would otherwise write itself.
    let egui = pollster::block_on(crate::EguiWgpu::new(window));
    Ok(Backend::Wgpu {
        egui: Box::new(egui),
    })
}

#[cfg(not(feature = "wgpu-backend"))]
fn build_wgpu(
    _video: &VideoSubsystem,
    _make_window: &impl Fn(&VideoSubsystem, bool) -> Result<Window, String>,
) -> Result<Backend, String> {
    Err("built without the wgpu-backend feature".to_string())
}

/// egui and GL take linear floats; SDL clears in 8-bit channels.
#[cfg(feature = "canvas-backend")]
fn rgb(color: [f32; 4]) -> sdl2::pixels::Color {
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    sdl2::pixels::Color::RGB(byte(color[0]), byte(color[1]), byte(color[2]))
}
