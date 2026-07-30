//! Canvas backend for egui-sdl2.
//!
//! This module provides [`Painter`], which integrates egui rendering with
//! SDL2’s [`Canvas<Window>`].

use egui::epaint::{ImageDelta, Primitive};
use egui::{ClippedPrimitive, ImageData, TexturesDelta};
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::sys::{SDL_Color, SDL_FPoint, SDL_Vertex};
use sdl2::video::{Window, WindowContext};
use std::collections::HashMap;
use std::os::raw::c_int;

#[cfg(target_endian = "little")]
const PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::ABGR8888;
#[cfg(target_endian = "big")]
const PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::RGBA8888;

const BYTES_PER_PIXEL: usize = 4;

/// An Canvas painter using [`sdl2`].
///
/// This is responsible for painting egui and managing egui textures. The
/// [`Canvas`] stays owned by the caller and is passed in per paint call, so egui
/// can draw over content the application already rendered.
///
/// This struct must be destroyed with [`Painter::destroy`] before dropping, to ensure
/// objects have been properly deleted and are not leaked.
///
/// NOTE: all egui viewports share the same painter.
pub struct Painter {
    textures: HashMap<egui::TextureId, Texture>,
    texture_creator: TextureCreator<WindowContext>,
    /// Reused across meshes and frames so `paint_mesh` repacks egui vertices
    /// into SDL's layout without allocating a fresh `Vec` per mesh.
    vertex_scratch: Vec<SDL_Vertex>,
    /// Clip rect currently applied to the canvas within a `paint_primitives`
    /// run, so meshes sharing a clip skip a redundant `SDL_RenderSetClipRect`.
    /// Reset to `None` at the start of each run because the caller draws to the
    /// same canvas between runs.
    last_clip: Option<Rect>,
}

impl Painter {
    /// Textures are created from `canvas`'s renderer, so pass the same canvas to
    /// the paint calls.
    pub fn new(canvas: &Canvas<Window>) -> Self {
        Self {
            textures: HashMap::new(),
            texture_creator: canvas.texture_creator(),
            vertex_scratch: Vec::new(),
            last_clip: None,
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
        canvas: &mut Canvas<Window>,
        pixels_per_point: f32,
        textures_delta: &TexturesDelta,
        paint_jobs: Vec<ClippedPrimitive>,
    ) -> Result<(), String> {
        for (id, delta) in &textures_delta.set {
            self.set_texture(*id, delta);
        }

        self.paint_primitives(canvas, pixels_per_point, paint_jobs);

        for &id in &textures_delta.free {
            self.free_texture(&id);
        }

        Ok(())
    }

    /// Main entry-point for painting a frame.
    pub fn paint_primitives(
        &mut self,
        canvas: &mut Canvas<Window>,
        pixels_per_point: f32,
        paint_jobs: Vec<ClippedPrimitive>,
    ) {
        // The caller may have drawn to the canvas (and changed its clip) since
        // the last run, so don't assume any clip is still applied.
        self.last_clip = None;
        for job in paint_jobs.into_iter() {
            match job.primitive {
                Primitive::Mesh(mesh) => {
                    self.paint_mesh(canvas, pixels_per_point, job.clip_rect, mesh)
                }
                Primitive::Callback(_callback) => {
                    // TODO
                    log::warn!("PaintCallbacks are not supported")
                }
            }
        }
        // Clear the clip once, after all meshes, so content the caller draws
        // after `paint()` isn't clipped to the last mesh's rect. Guard on
        // `last_clip`: a frame that drew no meshes never set a clip, so leave
        // the caller's own clip untouched.
        if self.last_clip.is_some() {
            canvas.set_clip_rect(None);
        }
    }

    pub fn set_texture(&mut self, id: egui::TextureId, delta: &ImageDelta) {
        let ImageData::Color(img) = &delta.image;
        let bytes: &[u8] = bytemuck::cast_slice(img.pixels.as_ref());
        let w = img.width() as u32;
        let h = img.height() as u32;
        let pitch = (w as usize) * BYTES_PER_PIXEL;

        if delta.pos.is_none() {
            if let Some(tex) = self.textures.get(&id) {
                let q = tex.query();
                if q.width != w || q.height != h {
                    self.free_texture(&id);
                }
            }
        }

        let tex = self
            .textures
            .entry(id)
            .or_insert_with(|| create_texture(&self.texture_creator, w, h));
        let rect = delta.pos.map(|[x, y]| Rect::new(x as i32, y as i32, w, h));
        tex.update(rect, bytes, pitch).unwrap();
    }

    #[inline]
    pub fn free_texture(&mut self, id: &egui::TextureId) {
        if let Some(tex) = self.textures.remove(id) {
            unsafe {
                tex.destroy();
            }
        }
    }

    #[inline]
    fn paint_mesh(
        &mut self,
        canvas: &mut Canvas<Window>,
        pixels_per_point: f32,
        clip_rect: egui::Rect,
        mesh: egui::Mesh,
    ) {
        let texture_ptr = self
            .textures
            .get(&mesh.texture_id)
            .map(|t| t.raw())
            .unwrap_or(std::ptr::null_mut()); // egui may draw untextured shape (nullptr in SDL_RenderGeometry)

        let min = clip_rect.min * pixels_per_point;
        let max = clip_rect.max * pixels_per_point;
        let clip_rect = sdl2::rect::Rect::new(
            min.x as i32,
            min.y as i32,
            (max.x - min.x) as u32,
            (max.y - min.y) as u32,
        );
        // Adjacent meshes (e.g. all glyphs in one panel) usually share a clip;
        // only hit `SDL_RenderSetClipRect` when it actually changes.
        if self.last_clip != Some(clip_rect) {
            canvas.set_clip_rect(clip_rect);
            self.last_clip = Some(clip_rect);
        }

        // Repack egui vertices into SDL's layout in a reused buffer. A zero-copy
        // cast is impossible (SDL_Vertex is {position, color, tex_coord} vs egui
        // {pos, uv, color}, and position is scaled by ppp), but reusing the
        // allocation across meshes/frames avoids a malloc+free per mesh.
        self.vertex_scratch.clear();
        self.vertex_scratch.reserve(mesh.vertices.len());
        self.vertex_scratch.extend(
            mesh.vertices
                .iter()
                .map(|v| into_sdl_vertex(v, pixels_per_point)),
        );
        let verts_ptr = self.vertex_scratch.as_ptr();
        let verts_len = self.vertex_scratch.len() as c_int;
        let indcs_ptr = mesh.indices.as_ptr() as *const c_int;
        let indcs_len = mesh.indices.len() as c_int;

        let result = unsafe {
            sdl2_sys::SDL_RenderGeometry(
                canvas.raw(),
                texture_ptr,
                if verts_len == 0 {
                    std::ptr::null()
                } else {
                    verts_ptr
                },
                verts_len,
                if indcs_len == 0 {
                    std::ptr::null()
                } else {
                    indcs_ptr
                },
                indcs_len,
            )
        };

        if result != 0 {
            log::error!("SDL_RenderGeometry failed: {}", result);
        }
    }
}

#[inline]
fn create_texture(texture_creator: &TextureCreator<WindowContext>, w: u32, h: u32) -> Texture {
    let mut tex = texture_creator
        .create_texture_streaming(PIXEL_FORMAT, w, h) // ABGR8888 on Little-Endian
        .expect("Failed to create sdl2 texture");
    tex.set_blend_mode(BlendMode::Blend);

    tex
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
