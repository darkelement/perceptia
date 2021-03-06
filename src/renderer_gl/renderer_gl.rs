// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/

//! This module contains GL renderer which allows drawing frame scenes with GL.

// -------------------------------------------------------------------------------------------------

use std;
use gl;
use egl;

use qualia::{Coordinator, SurfaceContext, Illusion, Size, Pixmap};

use gl_tools;
use egl_tools;

// -------------------------------------------------------------------------------------------------

const MAX_TEXTURES: u32 = 32;

/// Vertex shader source code for OpenGL ES 2.0 (GLSL ES 100)
const VERTEX_SHADER_100: &'static str = include_str!("vertex.100.glsl");

/// Fragment shader source code for OpenGL ES 2.0 (GLSL ES 100)
const FRAGMENT_SHADER_100: &'static str = include_str!("fragment.100.glsl");

/// Vertex shader source code for OpenGL ES 3.0 (GLSL ES 300)
const VERTEX_SHADER_300: &'static str = include_str!("vertex.300.glsl");

/// Fragment shader source code for OpenGL ES 3.0 (GLSL ES 300)
const FRAGMENT_SHADER_300: &'static str = include_str!("fragment.300.glsl");

// -------------------------------------------------------------------------------------------------

/// GL renderer.
pub struct RendererGl {
    egl: egl_tools::EglBucket,
    size: Size,

    // GL rendering
    program: gl::types::GLuint,
    loc_vertices: gl::types::GLint,
    loc_texcoords: gl::types::GLint,
    loc_texture: gl::types::GLint,
    loc_screen_size: gl::types::GLint,
    vbo_vertices: gl::types::GLuint,
    vbo_texcoords: gl::types::GLuint,
    vbo_texture: [gl::types::GLuint; MAX_TEXTURES as usize],
}

// -------------------------------------------------------------------------------------------------

impl RendererGl {
    /// `RendererGl` constructor.
    pub fn new(egl: egl_tools::EglBucket, size: Size) -> Self {
        RendererGl {
            egl: egl,
            size: size,
            program: gl::types::GLuint::default(),
            loc_vertices: gl::types::GLint::default(),
            loc_texcoords: gl::types::GLint::default(),
            loc_texture: gl::types::GLint::default(),
            loc_screen_size: gl::types::GLint::default(),
            vbo_vertices: gl::types::GLuint::default(),
            vbo_texcoords: gl::types::GLuint::default(),
            vbo_texture: [0; MAX_TEXTURES as usize],
        }
    }

    /// Initialize renderer.
    ///  - prepare shaders and program,
    ///  - bind locations,
    ///  - generate buffers,
    ///  - configure textures,
    pub fn initialize(&mut self) -> Result<(), Illusion> {
        gl::load_with(|s| egl::get_proc_address(s) as *const std::os::raw::c_void);

        let _context = self.egl.make_current()?;

        // Get GLSL version
        let (vshader_src, fshader_src) = match gl_tools::get_shading_lang_version() {
            gl_tools::GlslVersion::Glsl100 => {
                (VERTEX_SHADER_100.to_owned(), FRAGMENT_SHADER_100.to_owned())
            }
            gl_tools::GlslVersion::Glsl300 => {
                (VERTEX_SHADER_300.to_owned(), FRAGMENT_SHADER_300.to_owned())
            }
            gl_tools::GlslVersion::Unknown => {
                return Err(Illusion::General(format!("Could not figure out GLSL version")));
            }
        };

        // Compile shades, link program and get locations
        self.program = gl_tools::prepare_shader_program(vshader_src, fshader_src)?;
        self.loc_vertices = gl_tools::get_attrib_location(self.program, "vertices".to_owned())?;
        self.loc_texcoords = gl_tools::get_attrib_location(self.program, "texcoords".to_owned())?;
        self.loc_texture = gl_tools::get_uniform_location(self.program, "texture".to_owned())?;
        self.loc_screen_size = gl_tools::get_uniform_location(self.program,
                                                              "screen_size".to_owned())?;

        // Generate vertex buffer object
        unsafe {
            gl::GenBuffers(1, &mut self.vbo_vertices);
            gl::GenBuffers(1, &mut self.vbo_texcoords);
        }

        // Create texture buffer
        // FIXME: Implement support for more textures.
        unsafe {
            gl::GenTextures(MAX_TEXTURES as i32, (&mut self.vbo_texture).as_mut_ptr());
            for i in 0..MAX_TEXTURES {
                gl::ActiveTexture(gl::TEXTURE0 + 1);
                gl::BindTexture(gl::TEXTURE_2D, self.vbo_texture[i as usize]);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            }
        }

        Ok(())
    }

    /// Draw passed frame scene.
    pub fn draw(&mut self,
                surfaces: &Vec<SurfaceContext>,
                pointer: SurfaceContext,
                coordinator: &Coordinator)
                -> Result<(), Illusion> {
        let _context = self.egl.make_current()?;
        self.prepare_view();
        self.draw_bg_image();
        self.draw_surfaces(surfaces, coordinator);
        self.draw_pointer(pointer, coordinator);
        self.release_view();
        Ok(())
    }

    /// Swap buffers.
    pub fn swap_buffers(&mut self) -> Result<(), Illusion> {
        let context = self.egl.make_current()?;
        context.swap_buffers()
    }
}

// -------------------------------------------------------------------------------------------------

/// Drawing helpers.
impl RendererGl {
    /// Prepare view for drawing.
    fn prepare_view(&self) {
        unsafe {
            gl::ClearColor(0.0, 0.3, 0.5, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            gl::UseProgram(self.program);
            gl::Uniform2i(self.loc_screen_size, self.size.width as i32, self.size.height as i32);
        }
    }

    /// Draw background image.
    fn draw_bg_image(&self) {}

    /// Load textures and prepare vertices.
    fn load_texture_and_prepare_vertices(&self,
                                         coordinator: &Coordinator,
                                         context: &SurfaceContext,
                                         vertices: &mut [gl::types::GLfloat],
                                         texcoords: &mut [gl::types::GLfloat],
                                         index: usize) {
        if let Some(ref surface) = coordinator.get_surface(context.id) {
            if let Some(ref buffer) = surface.buffer {
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE0 + index as u32);
                    gl::BindTexture(gl::TEXTURE_2D, self.vbo_texture[index]);
                    gl::TexImage2D(gl::TEXTURE_2D, // target
                                   0, // level, 0 = no mipmap
                                   gl::RGBA as gl::types::GLint, // internal format
                                   (*buffer).get_width() as gl::types::GLint, // width
                                   (*buffer).get_height() as gl::types::GLint, // height
                                   0, // always 0 in OpenGL ES
                                   gl::RGBA, // format
                                   gl::UNSIGNED_BYTE, // type
                                   (*buffer).as_ptr() as *const _);
                }

                let left = (context.pos.x - surface.offset.x) as gl::types::GLfloat;
                let top = (context.pos.y - surface.offset.y) as gl::types::GLfloat;
                let right = left + (*buffer).get_width() as gl::types::GLfloat;
                let bottom = top + (*buffer).get_height() as gl::types::GLfloat;

                vertices[0] = left;
                vertices[1] = top;
                vertices[2] = right;
                vertices[3] = top;
                vertices[4] = left;
                vertices[5] = bottom;
                vertices[6] = right;
                vertices[7] = top;
                vertices[8] = right;
                vertices[9] = bottom;
                vertices[10] = left;
                vertices[11] = bottom;

                texcoords[0] = 0.0;
                texcoords[1] = 0.0;
                texcoords[2] = 1.0;
                texcoords[3] = 0.0;
                texcoords[4] = 0.0;
                texcoords[5] = 1.0;
                texcoords[6] = 1.0;
                texcoords[7] = 0.0;
                texcoords[8] = 1.0;
                texcoords[9] = 1.0;
                texcoords[10] = 0.0;
                texcoords[11] = 1.0;
            } else {
                log_error!("Renderer: No buffer for surface {}", context.id);
            }
        } else {
            log_error!("Renderer: No info for surface {}", context.id);
        }
    }

    /// Draw surfaces.
    fn draw_surfaces(&self, surfaces: &Vec<SurfaceContext>, coordinator: &Coordinator) {
        if surfaces.len() == 0 {
            return;
        }

        // Prepare vertices positions and upload textures
        let vertices_len = 12 * surfaces.len();
        let vertices_size = vertices_len * std::mem::size_of::<gl::types::GLfloat>();
        let mut vertices = vec![0.0; vertices_len];
        let mut texcoords = vec![0.0; vertices_len];

        for i in 0..surfaces.len() {
            self.load_texture_and_prepare_vertices(coordinator,
                                                   &surfaces[i],
                                                   &mut vertices[12 * i..12 * i + 12],
                                                   &mut texcoords[12 * i..12 * i + 12],
                                                   i);
        }

        unsafe {
            // Upload positions to vertex buffer object
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_vertices);
            gl::EnableVertexAttribArray(self.loc_vertices as gl::types::GLuint);
            gl::VertexAttribPointer(self.loc_vertices as gl::types::GLuint,
                                    2,
                                    gl::FLOAT,
                                    gl::FALSE,
                                    2 *
                                    std::mem::size_of::<gl::types::GLfloat>() as gl::types::GLint,
                                    std::ptr::null());
            gl::BufferData(gl::ARRAY_BUFFER,
                           vertices_size as isize,
                           vertices.as_ptr() as *const _,
                           gl::DYNAMIC_DRAW);

            // Upload positions to vertex buffer object
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_texcoords);
            gl::EnableVertexAttribArray(self.loc_texcoords as gl::types::GLuint);
            gl::VertexAttribPointer(self.loc_texcoords as gl::types::GLuint,
                                    2,
                                    gl::FLOAT,
                                    gl::FALSE,
                                    2 *
                                    std::mem::size_of::<gl::types::GLfloat>() as gl::types::GLint,
                                    std::ptr::null());
            gl::BufferData(gl::ARRAY_BUFFER,
                           vertices_size as isize,
                           texcoords.as_ptr() as *const _,
                           gl::DYNAMIC_DRAW);

            // Redraw everything
            for i in 0..surfaces.len() as i32 {
                gl::Uniform1i(self.loc_texture, i);
                gl::DrawArrays(gl::TRIANGLES, 6 * i, 6);
            }

            // Release resources
            gl::DisableVertexAttribArray(self.loc_texcoords as gl::types::GLuint);
            gl::DisableVertexAttribArray(self.loc_vertices as gl::types::GLuint);
        }
    }

    /// Draw pointer.
    fn draw_pointer(&self, pointer: SurfaceContext, coordinator: &Coordinator) {
        let surfaces = vec![pointer];
        self.draw_surfaces(&surfaces, coordinator);
    }

    /// Unbind framebuffer and program.
    fn release_view(&self) {
        unsafe {
            gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
            gl::UseProgram(0);
        }
    }
}

// -------------------------------------------------------------------------------------------------
