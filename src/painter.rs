//! Canvas backend for egui-sdl2.
//!
//! This module provides [`Painter`], which integrates egui rendering with
//! SDL2’s [`Canvas<Window>`].

use egui::epaint::{ImageDelta, Primitive};
use egui::{ClippedPrimitive, TexturesDelta};
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::sys::{SDL_Color, SDL_FPoint, SDL_Vertex};
use sdl2::video::{Window, WindowContext};
use sdl2_sys::{SDL_RenderGeometry, SDL_Renderer, SDL_Texture};
use std::collections::HashMap;
use std::os::raw::c_int;

#[cfg(target_endian = "little")]
const SDL_EGUI_FORMAT: PixelFormatEnum = PixelFormatEnum::ABGR8888;
#[cfg(target_endian = "big")]
const SDL_EGUI_FORMAT: PixelFormatEnum = PixelFormatEnum::RGBA8888;

/// An Canvas painter using [`sdl2`].
///
/// This is responsible for painting egui and managing egui textures.
/// You can access the underlying [`sdl2::Window`] with [`Self::Window`].
///
/// This struct must be destroyed with [`Painter::destroy`] before dropping, to ensure
/// objects have been properly deleted and are not leaked.
///
/// NOTE: all egui viewports share the same painter.
pub struct Painter {
    textures: HashMap<egui::TextureId, Texture>,
    texture_creator: TextureCreator<WindowContext>,

    pub canvas: Canvas<Window>,
}

impl Painter {
    pub fn new(window: sdl2::video::Window) -> Self {
        let canvas: Canvas<Window> = window.into_canvas().build().unwrap();
        let texture_creator = canvas.texture_creator();

        Self {
            textures: HashMap::new(),
            canvas,
            texture_creator,
        }
    }

    /// This function must be called before [`Painter`] is dropped, as [`Painter`] has some objects
    /// that should be deleted.
    pub fn destroy(&mut self) {
        let textures = std::mem::replace(&mut self.textures, HashMap::with_capacity(0));
        for (_id, tex) in textures {
            unsafe {
                tex.destroy();
            }
        }
    }

    /// You are expected to have cleared the color buffer before calling this.
    pub fn paint_and_update_textures(
        &mut self,
        pixels_per_point: f32,
        textures_delta: &TexturesDelta,
        paint_jobs: Vec<ClippedPrimitive>,
    ) -> Result<(), String> {
        for (id, delta) in &textures_delta.set {
            self.set_texture(*id, &delta);
        }

        self.paint_primitives(pixels_per_point, paint_jobs);

        for &id in &textures_delta.free {
            self.free_texture(&id);
        }

        Ok(())
    }

    /// Main entry-point for painting a frame.
    pub fn paint_primitives(&mut self, pixels_per_point: f32, paint_jobs: Vec<ClippedPrimitive>) {
        for job in paint_jobs.into_iter() {
            match job.primitive {
                Primitive::Mesh(mesh) => self.paint_mesh(pixels_per_point, job.clip_rect, mesh),
                Primitive::Callback(_callback) => {
                    log::warn!("PaintCallbacks are not supported")
                }
            }
        }
    }

    pub fn set_texture(&mut self, id: egui::TextureId, delta: &ImageDelta) {
        let mut _buf: Option<Vec<u8>> = None;

        let (bytes, w, h): (&[u8], u32, u32) = match &delta.image {
            egui::ImageData::Color(img) => {
                let bytes: &[u8] = bytemuck::cast_slice(img.pixels.as_ref());
                (bytes, img.width() as u32, img.height() as u32)
            }
        };

        const BYTES_PER_PIXEL: usize = 4;
        let pitch = (w as usize) * BYTES_PER_PIXEL;

        let tex = self.textures.entry(id).or_insert_with(|| {
            let mut tex = self
                .texture_creator
                .create_texture_streaming(SDL_EGUI_FORMAT, w, h) // ABGR8888 on Little-Endian
                .expect("Failed to create egui/sdl texture");
            tex.set_blend_mode(BlendMode::Blend);

            tex
        });

        if let Some([x, y]) = delta.pos {
            let rect = Rect::new(x as i32, y as i32, w, h);
            tex.update(rect, &bytes, pitch).unwrap();
        } else {
            tex.update(None, &bytes, pitch).unwrap();
        }
    }

    pub fn free_texture(&mut self, id: &egui::TextureId) {
        if let Some(tex) = self.textures.remove(id) {
            unsafe {
                tex.destroy();
            }
        }
    }

    fn paint_mesh(&mut self, pixels_per_point: f32, clip_rect: egui::Rect, mesh: egui::Mesh) {
        let texture_ptr = self
            .textures
            .get(&mesh.texture_id)
            .map(|t| t.raw() as *mut SDL_Texture)
            .unwrap_or(std::ptr::null_mut()); // egui may draw untextured shape (nullptr in SDL_RenderGeometry)

        let clip = sdl2::rect::Rect::new(
            (clip_rect.min.x * pixels_per_point) as i32,
            (clip_rect.min.y * pixels_per_point) as i32,
            ((clip_rect.max.x - clip_rect.min.x) * pixels_per_point) as u32,
            ((clip_rect.max.y - clip_rect.min.y) * pixels_per_point) as u32,
        );
        self.canvas.set_clip_rect(clip);

        let sdl_vertices: Vec<SDL_Vertex> = mesh
            .vertices
            .iter()
            .map(|v| into_sdl_vertex(v, pixels_per_point))
            .collect();
        let verts_len = sdl_vertices.len() as c_int;
        let verts_ptr = if verts_len == 0 {
            std::ptr::null()
        } else {
            sdl_vertices.as_ptr()
        };

        let idxs_len = mesh.indices.len() as c_int;
        let idxs_ptr = if idxs_len == 0 {
            std::ptr::null()
        } else {
            mesh.indices.as_ptr() as *const c_int
        };

        let rv = unsafe {
            SDL_RenderGeometry(
                self.canvas.raw() as *mut SDL_Renderer,
                texture_ptr,
                verts_ptr,
                verts_len,
                idxs_ptr,
                idxs_len,
            )
        };

        if rv != 0 {
            log::error!("SDL_RenderGeometry failed with error {}", rv);
        }

        self.canvas.set_clip_rect(None);
    }
}

#[inline]
fn into_sdl_vertex(vertex: &egui::epaint::Vertex, pixels_per_point: f32) -> SDL_Vertex {
    SDL_Vertex {
        position: SDL_FPoint {
            x: vertex.pos.x * pixels_per_point,
            y: vertex.pos.y * pixels_per_point,
        },
        color: SDL_Color {
            r: vertex.color.r(),
            g: vertex.color.g(),
            b: vertex.color.b(),
            a: vertex.color.a(),
        },
        tex_coord: SDL_FPoint {
            x: vertex.uv.x,
            y: vertex.uv.y,
        },
    }
}
