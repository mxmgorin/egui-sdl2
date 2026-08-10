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
    /// [`Self::Canvas`] rasterized offscreen, presented as one texture copy per
    /// frame — for drivers that show nothing else, like the Miyoo Mini's
    /// `mmiyoo`. Costs an upload per frame.
    CanvasBlit,
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
            Renderer::CanvasBlit => "SDL renderer (offscreen blit)",
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
    #[cfg(feature = "canvas-backend")]
    CanvasBlit {
        canvas: sdl2::render::WindowCanvas,
        /// egui is drawn here, by SDL's software renderer.
        offscreen: sdl2::render::Canvas<sdl2::surface::Surface<'static>>,
        /// The offscreen pixels, uploaded and copied once per frame.
        present: sdl2::render::Texture,
        /// Size `offscreen` and `present` were built for.
        size: (u32, u32),
        egui: crate::EguiCanvas<sdl2::surface::SurfaceContext<'static>>,
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
            #[cfg(feature = "canvas-backend")]
            Backend::CanvasBlit { egui, .. } => &egui.ctx,
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
            #[cfg(feature = "canvas-backend")]
            Backend::CanvasBlit { canvas, .. } => canvas.window(),
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
            #[cfg(feature = "canvas-backend")]
            Backend::CanvasBlit { canvas, .. } => canvas.window_mut(),
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
            #[cfg(feature = "canvas-backend")]
            Backend::CanvasBlit { canvas, egui, .. } => egui.state.on_event(canvas.window(), event),
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
            #[cfg(feature = "canvas-backend")]
            Backend::CanvasBlit { egui, .. } => egui.run(run_ui),
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
            #[cfg(feature = "canvas-backend")]
            Backend::CanvasBlit { egui, .. } => egui.run_ui(run_ui),
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
            #[cfg(feature = "canvas-backend")]
            Backend::CanvasBlit { egui, .. } => egui.repaint_delay(),
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
            #[cfg(feature = "canvas-backend")]
            Backend::CanvasBlit {
                canvas,
                offscreen,
                present,
                size,
                egui,
            } => {
                // Rebuild on resize so surface, texture and window stay 1:1.
                if canvas.output_size().is_ok_and(|s| s != *size) {
                    match rebuild_blit_targets(canvas) {
                        Ok((new_offscreen, new_present, new_size)) => {
                            egui.destroy();
                            *egui = crate::EguiCanvas::for_surface(canvas.window(), &new_offscreen);
                            *offscreen = new_offscreen;
                            *present = new_present;
                            *size = new_size;
                        }
                        Err(e) => log::error!("could not resize the offscreen target: {e}"),
                    }
                }

                offscreen.set_draw_color(rgb(clear_color));
                offscreen.clear();
                egui.paint(offscreen);
                let surface = offscreen.surface();
                let pitch = surface.pitch() as usize;
                match surface.without_lock() {
                    Some(pixels) => {
                        if let Err(e) = present.update(None, pixels, pitch) {
                            log::error!("could not upload the offscreen frame: {e}");
                        }
                    }
                    None => log::error!("offscreen surface has no readable pixels"),
                }
                if let Err(e) = canvas.copy(present, None, None) {
                    log::error!("could not blit the offscreen frame: {e}");
                }
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
            #[cfg(feature = "canvas-backend")]
            Backend::CanvasBlit { egui, .. } => egui.destroy(),
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
        Renderer::CanvasBlit => build_canvas_blit(video, make_window),
        Renderer::Wgpu => build_wgpu(video, make_window),
    }
}

/// `SDL_GL_SetAttribute`, reported rather than asserted.
#[cfg(feature = "glow-backend")]
fn set_gl_attr(name: &str, attr: sdl2::sys::SDL_GLattr, value: i32) -> Result<(), String> {
    if unsafe { sdl2::sys::SDL_GL_SetAttribute(attr, value) } == 0 {
        return Ok(());
    }
    Err(format!("{name}={value} rejected: {}", sdl2::get_error()))
}

/// The values `SDL_GL_CONTEXT_PROFILE_MASK` takes.
#[cfg(feature = "glow-backend")]
fn gl_profile_value(profile: sdl2::video::GLProfile) -> i32 {
    use sdl2::video::GLProfile;
    match profile {
        GLProfile::Core => 1,
        GLProfile::Compatibility => 2,
        GLProfile::GLES => 4,
        GLProfile::Unknown(i) => i,
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
    // Not `video.gl_attr()`: its setters panic on rejection, which would kill
    // the fallthrough on a device without GL.
    use sdl2::sys::SDL_GLattr::*;
    set_gl_attr(
        "profile_mask",
        SDL_GL_CONTEXT_PROFILE_MASK,
        gl_profile_value(profile),
    )?;
    set_gl_attr("major_version", SDL_GL_CONTEXT_MAJOR_VERSION, major as i32)?;
    set_gl_attr("minor_version", SDL_GL_CONTEXT_MINOR_VERSION, minor as i32)?;
    set_gl_attr("doublebuffer", SDL_GL_DOUBLEBUFFER, 1)?;

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

/// The offscreen surface, its presentation texture, and the size both cover.
#[cfg(feature = "canvas-backend")]
type BlitTargets = (
    sdl2::render::Canvas<sdl2::surface::Surface<'static>>,
    sdl2::render::Texture,
    (u32, u32),
);

#[cfg(feature = "canvas-backend")]
fn rebuild_blit_targets(canvas: &sdl2::render::WindowCanvas) -> Result<BlitTargets, String> {
    let size = canvas.output_size()?;
    let surface =
        sdl2::surface::Surface::new(size.0, size.1, crate::canvas::painter::PIXEL_FORMAT)?;
    let offscreen = sdl2::render::Canvas::from_surface(surface)?;
    let mut present = canvas
        .texture_creator()
        .create_texture_streaming(crate::canvas::painter::PIXEL_FORMAT, size.0, size.1)
        .map_err(|e| e.to_string())?;
    // A whole frame replaces rather than blends. SDL gives a format with alpha
    // `BLEND` by default, which would dim any pixel the offscreen renderer left
    // short of opaque against whatever the window happened to hold, and there is
    // nothing under a full frame worth mixing in.
    present.set_blend_mode(sdl2::render::BlendMode::None);
    Ok((offscreen, present, size))
}

#[cfg(feature = "canvas-backend")]
fn build_canvas_blit(
    video: &VideoSubsystem,
    make_window: &impl Fn(&VideoSubsystem, bool) -> Result<Window, String>,
) -> Result<Backend, String> {
    let window = make_window(video, false)?;
    // No vsync request: not every driver this mode serves advertises it, and
    // asking excludes those that don't.
    let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    log::debug!("SDL renderer driver: {} (blit)", canvas.info().name);
    let (offscreen, present, size) = rebuild_blit_targets(&canvas)?;
    let egui = crate::EguiCanvas::for_surface(canvas.window(), &offscreen);
    Ok(Backend::CanvasBlit {
        canvas,
        offscreen,
        present,
        size,
        egui,
    })
}

#[cfg(not(feature = "canvas-backend"))]
fn build_canvas_blit(
    _video: &VideoSubsystem,
    _make_window: &impl Fn(&VideoSubsystem, bool) -> Result<Window, String>,
) -> Result<Backend, String> {
    Err("built without the canvas-backend feature".to_string())
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
