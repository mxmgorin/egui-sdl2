//! Canvas backend for egui-sdl2.
//!
//! This module provides [`Painter`], which integrates egui rendering with an
//! SDL2 [`Canvas`] — a window's, or a surface's when the app draws offscreen.

use egui::epaint::{ImageDelta, Primitive};
use egui::{ClippedPrimitive, ImageData, TexturesDelta};
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, RenderTarget, Texture, TextureCreator};
use sdl2::surface::{Surface, SurfaceContext};
use sdl2::sys::{SDL_Color, SDL_FPoint, SDL_Vertex};
use sdl2::video::{Window, WindowContext};
use std::collections::HashMap;
use std::os::raw::c_int;

#[cfg(target_endian = "little")]
pub(crate) const PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::ABGR8888;
#[cfg(target_endian = "big")]
pub(crate) const PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::RGBA8888;

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
pub struct Painter<C = WindowContext> {
    textures: HashMap<egui::TextureId, Texture>,
    texture_creator: TextureCreator<C>,
    /// Reused across meshes and frames so `paint_mesh` repacks egui vertices
    /// into SDL's layout without allocating a fresh `Vec` per mesh.
    vertex_scratch: Vec<SDL_Vertex>,
    /// Clip rect currently applied to the canvas within a `paint_primitives`
    /// run, so meshes sharing a clip skip a redundant `SDL_RenderSetClipRect`.
    /// Reset to `None` at the start of each run because the caller draws to the
    /// same canvas between runs.
    last_clip: Option<Rect>,
    /// Triangles waiting to be drawn, so a run of them is still one SDL call.
    index_scratch: Vec<u32>,
    /// This renderer's texture size limit, for egui to lay its atlas out within.
    max_texture_side: Option<usize>,
}

impl Painter<WindowContext> {
    /// Textures are created from `canvas`'s renderer, so pass the same canvas to
    /// the paint calls.
    pub fn new(canvas: &Canvas<Window>) -> Self {
        Self::with_creator(canvas.texture_creator(), max_texture_side(canvas))
    }
}

impl<'s> Painter<SurfaceContext<'s>> {
    /// Paint into a surface instead of a window, for drivers that only present
    /// texture copies (see [`crate::Renderer::CanvasBlit`]).
    pub fn for_surface(canvas: &Canvas<Surface<'s>>) -> Self {
        Self::with_creator(canvas.texture_creator(), max_texture_side(canvas))
    }
}

impl<C> Painter<C> {
    fn with_creator(texture_creator: TextureCreator<C>, max_texture_side: Option<usize>) -> Self {
        Self {
            textures: HashMap::new(),
            texture_creator,
            vertex_scratch: Vec::new(),
            index_scratch: Vec::new(),
            last_clip: None,
            max_texture_side,
        }
    }

    /// The largest texture this renderer accepts, `None` if it reports no limit.
    /// Feed [`crate::State::set_max_texture_side`]: egui's atlas defaults to
    /// 2048, past what handheld drivers hold (Miyoo Mini: 1920x1080).
    pub fn max_texture_side(&self) -> Option<usize> {
        self.max_texture_side
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
    pub fn paint_and_update_textures<T: RenderTarget<Context = C>>(
        &mut self,
        canvas: &mut Canvas<T>,
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
    pub fn paint_primitives<T: RenderTarget<Context = C>>(
        &mut self,
        canvas: &mut Canvas<T>,
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
    fn paint_mesh<T: RenderTarget<Context = C>>(
        &mut self,
        canvas: &mut Canvas<T>,
        pixels_per_point: f32,
        clip_rect: egui::Rect,
        mesh: egui::Mesh,
    ) {
        // egui may draw untextured shapes (nullptr in SDL_RenderGeometry).
        let (texture_ptr, texture_size) = match self.textures.get(&mesh.texture_id) {
            Some(tex) => {
                let q = tex.query();
                (tex.raw(), Some((q.width as f32, q.height as f32)))
            }
            None => (std::ptr::null_mut(), None),
        };

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

        // Text and rectangles tessellate to axis-aligned quads: blit those, they
        // are exact and cheap. Rounded corners, circles and feathering stay on
        // the triangle path. Flushing before each blit keeps egui's draw order.
        self.index_scratch.clear();
        for corners in mesh.indices.chunks(6) {
            match as_axis_aligned_quad(&mesh.vertices, corners, pixels_per_point) {
                Some(quad) => {
                    self.flush_triangles(canvas, texture_ptr, &mesh, pixels_per_point);
                    quad.blit(canvas, texture_ptr, texture_size);
                }
                None => self.index_scratch.extend_from_slice(corners),
            }
        }
        self.flush_triangles(canvas, texture_ptr, &mesh, pixels_per_point);
    }

    /// Draw whatever indices have accumulated in `index_scratch` as triangles.
    fn flush_triangles<T: RenderTarget<Context = C>>(
        &mut self,
        canvas: &mut Canvas<T>,
        texture_ptr: *mut sdl2_sys::SDL_Texture,
        mesh: &egui::Mesh,
        pixels_per_point: f32,
    ) {
        if self.index_scratch.is_empty() {
            return;
        }
        // A blit may have left a colour mod; vertex colours carry their own tint.
        if !texture_ptr.is_null() {
            unsafe {
                sdl2_sys::SDL_SetTextureColorMod(texture_ptr, 255, 255, 255);
                sdl2_sys::SDL_SetTextureAlphaMod(texture_ptr, 255);
            }
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
        let verts_len = self.vertex_scratch.len() as c_int;
        let indcs_len = self.index_scratch.len() as c_int;

        let result = unsafe {
            sdl2_sys::SDL_RenderGeometry(
                canvas.raw(),
                texture_ptr,
                if verts_len == 0 {
                    std::ptr::null()
                } else {
                    self.vertex_scratch.as_ptr()
                },
                verts_len,
                self.index_scratch.as_ptr() as *const c_int,
                indcs_len,
            )
        };
        self.index_scratch.clear();

        if result != 0 {
            log::error!("SDL_RenderGeometry failed: {}", result);
        }
    }
}

/// An axis-aligned, single-colour quad: a glyph, or a plain rectangle.
struct Quad {
    dst: sdl2_sys::SDL_FRect,
    /// Normalized, as egui gives it; scaled to texels at blit time.
    uv: egui::Rect,
    color: egui::Color32,
    textured: bool,
}

impl Quad {
    fn blit<T: RenderTarget>(
        &self,
        canvas: &mut Canvas<T>,
        texture_ptr: *mut sdl2_sys::SDL_Texture,
        texture_size: Option<(f32, f32)>,
    ) {
        let (r, g, b, a) = self.color.to_tuple();
        let result = match (self.textured, texture_size) {
            (true, Some((tw, th))) => unsafe {
                // The atlas is premultiplied white coverage; modulating it matches
                // what the triangle path does per vertex.
                sdl2_sys::SDL_SetTextureColorMod(texture_ptr, r, g, b);
                sdl2_sys::SDL_SetTextureAlphaMod(texture_ptr, a);
                let src = sdl2_sys::SDL_Rect {
                    x: (self.uv.min.x * tw).round() as i32,
                    y: (self.uv.min.y * th).round() as i32,
                    w: (self.uv.width() * tw).round() as i32,
                    h: (self.uv.height() * th).round() as i32,
                };
                sdl2_sys::SDL_RenderCopyF(canvas.raw(), texture_ptr, &src, &self.dst)
            },
            _ => unsafe {
                sdl2_sys::SDL_SetRenderDrawColor(canvas.raw(), r, g, b, a);
                sdl2_sys::SDL_RenderFillRectF(canvas.raw(), &self.dst)
            },
        };
        if result != 0 {
            log::error!("blitting a quad failed: {result}");
        }
    }
}

/// The two triangles of an unrotated, single-colour quad, if that is what these
/// indices are; otherwise `None`, for [`Painter::flush_triangles`].
fn as_axis_aligned_quad(
    vertices: &[egui::epaint::Vertex],
    corners: &[u32],
    pixels_per_point: f32,
) -> Option<Quad> {
    if corners.len() != 6 {
        return None;
    }
    let mut uniq: Vec<&egui::epaint::Vertex> = Vec::with_capacity(4);
    for &i in corners {
        let v = vertices.get(i as usize)?;
        if !uniq.iter().any(|u| u.pos == v.pos && u.uv == v.uv) {
            uniq.push(v);
        }
    }
    if uniq.len() != 4 {
        return None;
    }
    let color = uniq[0].color;
    if uniq.iter().any(|v| v.color != color) {
        return None;
    }

    let rect = egui::Rect::from_points(&uniq.iter().map(|v| v.pos).collect::<Vec<_>>());
    let uv = egui::Rect::from_points(&uniq.iter().map(|v| v.uv).collect::<Vec<_>>());
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    // A degenerate uv means the mesh samples egui's single white texel: a fill.
    let textured = uv.width() > 0.0 && uv.height() > 0.0;

    for v in &uniq {
        let at_min_x = v.pos.x == rect.min.x;
        let at_min_y = v.pos.y == rect.min.y;
        if !(at_min_x || v.pos.x == rect.max.x) || !(at_min_y || v.pos.y == rect.max.y) {
            return None; // a vertex off the corners: not a rectangle
        }
        // Reject rotated and mirrored mappings; SDL_RenderCopy cannot express them.
        if textured && (at_min_x != (v.uv.x == uv.min.x) || at_min_y != (v.uv.y == uv.min.y)) {
            return None;
        }
    }

    Some(Quad {
        dst: sdl2_sys::SDL_FRect {
            x: rect.min.x * pixels_per_point,
            y: rect.min.y * pixels_per_point,
            w: rect.width() * pixels_per_point,
            h: rect.height() * pixels_per_point,
        },
        uv,
        color,
        textured,
    })
}

/// SDL leaves the fields at 0 for drivers with no limit, its software one included.
fn max_texture_side<T: RenderTarget>(canvas: &Canvas<T>) -> Option<usize> {
    let info = canvas.info();
    let side = match (info.max_texture_width, info.max_texture_height) {
        (0, 0) => return None,
        (0, h) => h,
        (w, 0) => w,
        (w, h) => w.min(h),
    };
    Some(side as usize)
}

#[inline]
fn create_texture<C>(texture_creator: &TextureCreator<C>, w: u32, h: u32) -> Texture {
    let mut tex = texture_creator
        .create_texture_streaming(PIXEL_FORMAT, w, h) // ABGR8888 on Little-Endian
        .unwrap_or_else(|e| {
            // Reached only if egui asked for more than the renderer's limit,
            // which `Painter::max_texture_side` exists to prevent — so the
            // integration failed to pass it on.
            panic!("failed to create a {w}x{h} sdl2 texture: {e}")
        });
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
