//! Custom imgui renderer targeting glow 0.17 (ported from imgui-glow-renderer 0.11).
//!
//! This renderer is intentionally minimal — it supports the common desktop
//! OpenGL path (3.3+) that is already required by egui_glow.

use std::{mem::size_of, sync::Arc};

use egui_glow::glow::{self, HasContext as _};
use imgui::{DrawCmd, DrawData, DrawVert};

// ── Type aliases ─────────────────────────────────────────────────────────────

type GlBuffer = glow::NativeBuffer;
type GlTexture = glow::NativeTexture;
type GlProgram = glow::NativeProgram;
type GlUniformLocation = glow::NativeUniformLocation;
type GlVertexArray = glow::NativeVertexArray;

// ── Error types ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum RendererError {
    Gl(String),
}

impl std::fmt::Display for RendererError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RendererError::Gl(s) => write!(f, "GL error: {s}"),
        }
    }
}

// ── Shader sources ───────────────────────────────────────────────────────────

const VERTEX_SRC: &str = r#"#version 330 core
layout (location = 0) in vec2 position;
layout (location = 1) in vec2 uv;
layout (location = 2) in vec4 color;

uniform mat4 matrix;
out vec2 fragment_uv;
out vec4 fragment_color;

void main() {
    fragment_uv = uv;
    fragment_color = color;
    gl_Position = matrix * vec4(position.xy, 0.0, 1.0);
}
"#;

const FRAGMENT_SRC: &str = r#"#version 330 core
in vec2 fragment_uv;
in vec4 fragment_color;

uniform sampler2D tex;
layout (location = 0) out vec4 out_color;

void main() {
    out_color = fragment_color * texture(tex, fragment_uv.st);
}
"#;

// ── Main renderer struct ──────────────────────────────────────────────────────

pub struct ImGuiRenderer {
    program: GlProgram,
    vao: GlVertexArray,
    vbo: GlBuffer,
    ebo: GlBuffer,
    // Holds the font atlas texture handle alive for the duration of the renderer.
    _font_texture: GlTexture,
    matrix_loc: GlUniformLocation,
    tex_loc: GlUniformLocation,
}

impl ImGuiRenderer {
    /// Compile shaders, create GPU buffers and upload the font atlas.
    pub fn new(
        gl: &Arc<glow::Context>,
        imgui: &mut imgui::Context,
    ) -> Result<Self, RendererError> {
        let program = compile_program(gl)?;
        let vao = unsafe { gl.create_vertex_array() }.map_err(RendererError::Gl)?;
        let vbo = unsafe { gl.create_buffer() }.map_err(RendererError::Gl)?;
        let ebo = unsafe { gl.create_buffer() }.map_err(RendererError::Gl)?;
        let (matrix_loc, tex_loc) = get_uniform_locations(gl, program)?;

        let font_texture = upload_font_atlas(gl, imgui)?;

        // Tell imgui what texture ID to use for the font atlas.
        imgui
            .fonts()
            .tex_id = imgui::TextureId::new(font_texture.0.get() as usize);

        Ok(Self {
            program,
            vao,
            vbo,
            ebo,
            _font_texture: font_texture,
            matrix_loc,
            tex_loc,
        })
    }

    /// Render imgui `DrawData` to the currently-bound framebuffer.
    pub fn render(
        &mut self,
        gl: &Arc<glow::Context>,
        draw_data: &DrawData,
    ) -> Result<(), RendererError> {        let fb_w = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        let fb_h = draw_data.display_size[1] * draw_data.framebuffer_scale[1];
        if fb_w <= 0.0 || fb_h <= 0.0 {
            return Ok(());
        }

        // Save / restore state so we don't stomp on egui or other renderers.
        let state = GlStateBackup::save(gl);

        self.setup_gl_state(gl, draw_data, fb_w, fb_h)?;

        for draw_list in draw_data.draw_lists() {
            unsafe {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    as_byte_slice(draw_list.vtx_buffer()),
                    glow::STREAM_DRAW,
                );
                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ebo));
                gl.buffer_data_u8_slice(
                    glow::ELEMENT_ARRAY_BUFFER,
                    as_byte_slice(draw_list.idx_buffer()),
                    glow::STREAM_DRAW,
                );
            }

            for cmd in draw_list.commands() {
                match cmd {
                    DrawCmd::Elements { count, cmd_params } => {
                        let clip_off = draw_data.display_pos;
                        let scale = draw_data.framebuffer_scale;

                        let clip_x1 =
                            (cmd_params.clip_rect[0] - clip_off[0]) * scale[0];
                        let clip_y1 =
                            (cmd_params.clip_rect[1] - clip_off[1]) * scale[1];
                        let clip_x2 =
                            (cmd_params.clip_rect[2] - clip_off[0]) * scale[0];
                        let clip_y2 =
                            (cmd_params.clip_rect[3] - clip_off[1]) * scale[1];

                        if clip_x2 <= clip_x1 || clip_y2 <= clip_y1 {
                            continue;
                        }

                        unsafe {
                            gl.scissor(
                                clip_x1 as i32,
                                (fb_h - clip_y2) as i32,
                                (clip_x2 - clip_x1) as i32,
                                (clip_y2 - clip_y1) as i32,
                            );

                            // Bind the correct texture (usually the font atlas).
                            let raw_id = cmd_params.texture_id.id() as u32;
                            gl.bind_texture(
                                glow::TEXTURE_2D,
                                Some(glow::NativeTexture(
                                    std::num::NonZeroU32::new(raw_id)
                                        .expect("imgui texture id 0 is invalid"),
                                )),
                            );

                            let idx_offset_bytes =
                                cmd_params.idx_offset * size_of::<imgui::DrawIdx>();
                            gl.draw_elements_base_vertex(
                                glow::TRIANGLES,
                                count as i32,
                                if size_of::<imgui::DrawIdx>() == 2 {
                                    glow::UNSIGNED_SHORT
                                } else {
                                    glow::UNSIGNED_INT
                                },
                                idx_offset_bytes as i32,
                                cmd_params.vtx_offset as i32,
                            );
                        }
                    }
                    DrawCmd::RawCallback { callback, raw_cmd } => unsafe {
                        use imgui::internal::RawWrapper as _;
                        callback(draw_list.raw() as *const _, raw_cmd)
                    },
                    DrawCmd::ResetRenderState => {
                        self.setup_gl_state(gl, draw_data, fb_w, fb_h)?;
                    }
                }
            }
        }

        state.restore(gl);
        Ok(())
    }

    #[allow(unused_unsafe)]
    fn setup_gl_state(
        &self,
        gl: &Arc<glow::Context>,
        draw_data: &DrawData,
        fb_w: f32,
        fb_h: f32,
    ) -> Result<(), RendererError> {
        unsafe {
            gl.enable(glow::BLEND);
            gl.blend_equation(glow::FUNC_ADD);
            gl.blend_func_separate(
                glow::SRC_ALPHA,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
            );
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::STENCIL_TEST);
            gl.enable(glow::SCISSOR_TEST);
            gl.active_texture(glow::TEXTURE0);
            gl.viewport(0, 0, fb_w as i32, fb_h as i32);
        }

        // Orthographic projection matrix.
        let l = draw_data.display_pos[0];
        let r = draw_data.display_pos[0] + draw_data.display_size[0];
        let t = draw_data.display_pos[1];
        let b = draw_data.display_pos[1] + draw_data.display_size[1];
        #[rustfmt::skip]
        let proj: [f32; 16] = [
            2.0 / (r - l),    0.0,              0.0,  0.0,
            0.0,              2.0 / (t - b),    0.0,  0.0,
            0.0,              0.0,             -1.0,  0.0,
            (r + l) / (l - r), (t + b) / (b - t), 0.0,  1.0,
        ];

        unsafe {
            gl.use_program(Some(self.program));
            gl.uniform_1_i32(Some(&self.tex_loc), 0);
            gl.uniform_matrix_4_f32_slice(Some(&self.matrix_loc), false, &proj);

            // Bind the VAO so all vertex attribute state is stored in it.
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ebo));

            // position (2×f32 @ offset 0)
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(
                0,
                2,
                glow::FLOAT,
                false,
                size_of::<DrawVert>() as i32,
                offset_of!(DrawVert, pos) as i32,
            );
            // uv (2×f32)
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(
                1,
                2,
                glow::FLOAT,
                false,
                size_of::<DrawVert>() as i32,
                offset_of!(DrawVert, uv) as i32,
            );
            // color (4×u8, normalized)
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(
                2,
                4,
                glow::UNSIGNED_BYTE,
                true,
                size_of::<DrawVert>() as i32,
                offset_of!(DrawVert, col) as i32,
            );
        }

        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn compile_program(gl: &Arc<glow::Context>) -> Result<GlProgram, RendererError> {
    unsafe {
        let vert = gl
            .create_shader(glow::VERTEX_SHADER)
            .map_err(RendererError::Gl)?;
        gl.shader_source(vert, VERTEX_SRC);
        gl.compile_shader(vert);
        if !gl.get_shader_compile_status(vert) {
            let log = gl.get_shader_info_log(vert);
            gl.delete_shader(vert);
            return Err(RendererError::Gl(format!("vertex shader: {log}")));
        }

        let frag = gl
            .create_shader(glow::FRAGMENT_SHADER)
            .map_err(RendererError::Gl)?;
        gl.shader_source(frag, FRAGMENT_SRC);
        gl.compile_shader(frag);
        if !gl.get_shader_compile_status(frag) {
            let log = gl.get_shader_info_log(frag);
            gl.delete_shader(frag);
            gl.delete_shader(vert);
            return Err(RendererError::Gl(format!("fragment shader: {log}")));
        }

        let prog = gl.create_program().map_err(RendererError::Gl)?;
        gl.attach_shader(prog, vert);
        gl.attach_shader(prog, frag);
        gl.link_program(prog);
        gl.delete_shader(vert);
        gl.delete_shader(frag);

        if !gl.get_program_link_status(prog) {
            let log = gl.get_program_info_log(prog);
            gl.delete_program(prog);
            return Err(RendererError::Gl(format!("link: {log}")));
        }

        Ok(prog)
    }
}

fn get_uniform_locations(
    gl: &Arc<glow::Context>,
    prog: GlProgram,
) -> Result<(GlUniformLocation, GlUniformLocation), RendererError> {
    let matrix_loc = unsafe { gl.get_uniform_location(prog, "matrix") }
        .ok_or_else(|| RendererError::Gl("uniform 'matrix' not found".into()))?;
    let tex_loc = unsafe { gl.get_uniform_location(prog, "tex") }
        .ok_or_else(|| RendererError::Gl("uniform 'tex' not found".into()))?;
    Ok((matrix_loc, tex_loc))
}

fn upload_font_atlas(
    gl: &Arc<glow::Context>,
    imgui: &mut imgui::Context,
) -> Result<GlTexture, RendererError> {
    let fonts = imgui.fonts();
    let atlas = fonts.build_rgba32_texture();

    unsafe {
        let tex = gl.create_texture().map_err(RendererError::Gl)?;
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            atlas.width as i32,
            atlas.height as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(atlas.data)),
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
        Ok(tex)
    }
}

/// Return the byte offset of a field within a `#[repr(C)]` struct.
macro_rules! offset_of {
    ($struct:ty, $field:ident) => {{
        let base = std::mem::MaybeUninit::<$struct>::uninit();
        let base_ptr = base.as_ptr();
        let field_ptr = unsafe { std::ptr::addr_of!((*base_ptr).$field) };
        (field_ptr as usize) - (base_ptr as usize)
    }};
}
use offset_of;

/// Cast a slice of T to a slice of bytes.
fn as_byte_slice<T>(slice: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            slice.as_ptr() as *const u8,
            slice.len() * size_of::<T>(),
        )
    }
}

// ── Minimal GL state backup / restore ────────────────────────────────────────

struct GlStateBackup {
    blend: bool,
    cull_face: bool,
    depth_test: bool,
    stencil_test: bool,
    scissor_test: bool,
    blend_src_rgb: i32,
    blend_dst_rgb: i32,
    blend_src_alpha: i32,
    blend_dst_alpha: i32,
    blend_equation_rgb: i32,
    blend_equation_alpha: i32,
    active_texture: i32,
    program: i32,
    texture: i32,
    vao: i32,
    viewport: [i32; 4],
    scissor: [i32; 4],
}

impl GlStateBackup {
    fn save(gl: &Arc<glow::Context>) -> Self {
        unsafe {
            Self {
                blend: gl.is_enabled(glow::BLEND),
                cull_face: gl.is_enabled(glow::CULL_FACE),
                depth_test: gl.is_enabled(glow::DEPTH_TEST),
                stencil_test: gl.is_enabled(glow::STENCIL_TEST),
                scissor_test: gl.is_enabled(glow::SCISSOR_TEST),
                blend_src_rgb: gl.get_parameter_i32(glow::BLEND_SRC_RGB),
                blend_dst_rgb: gl.get_parameter_i32(glow::BLEND_DST_RGB),
                blend_src_alpha: gl.get_parameter_i32(glow::BLEND_SRC_ALPHA),
                blend_dst_alpha: gl.get_parameter_i32(glow::BLEND_DST_ALPHA),
                blend_equation_rgb: gl.get_parameter_i32(glow::BLEND_EQUATION_RGB),
                blend_equation_alpha: gl.get_parameter_i32(glow::BLEND_EQUATION_ALPHA),
                active_texture: gl.get_parameter_i32(glow::ACTIVE_TEXTURE),
                program: gl.get_parameter_i32(glow::CURRENT_PROGRAM),
                texture: gl.get_parameter_i32(glow::TEXTURE_BINDING_2D),
                vao: gl.get_parameter_i32(glow::VERTEX_ARRAY_BINDING),
                viewport: {
                    let mut v = [0i32; 4];
                    gl.get_parameter_i32_slice(glow::VIEWPORT, &mut v);
                    v
                },
                scissor: {
                    let mut v = [0i32; 4];
                    gl.get_parameter_i32_slice(glow::SCISSOR_BOX, &mut v);
                    v
                },
            }
        }
    }

    fn restore(self, gl: &Arc<glow::Context>) {
        unsafe {
            set_capability(gl, glow::BLEND, self.blend);
            set_capability(gl, glow::CULL_FACE, self.cull_face);
            set_capability(gl, glow::DEPTH_TEST, self.depth_test);
            set_capability(gl, glow::STENCIL_TEST, self.stencil_test);
            set_capability(gl, glow::SCISSOR_TEST, self.scissor_test);
            gl.blend_func_separate(
                self.blend_src_rgb as u32,
                self.blend_dst_rgb as u32,
                self.blend_src_alpha as u32,
                self.blend_dst_alpha as u32,
            );
            gl.blend_equation_separate(
                self.blend_equation_rgb as u32,
                self.blend_equation_alpha as u32,
            );
            gl.active_texture(self.active_texture as u32);
            gl.use_program(if self.program == 0 {
                None
            } else {
                Some(glow::NativeProgram(
                    std::num::NonZeroU32::new(self.program as u32)
                        .expect("program id 0"),
                ))
            });
            gl.bind_texture(
                glow::TEXTURE_2D,
                if self.texture == 0 {
                    None
                } else {
                    Some(glow::NativeTexture(
                        std::num::NonZeroU32::new(self.texture as u32)
                            .expect("texture id 0"),
                    ))
                },
            );
            gl.bind_vertex_array(
                std::num::NonZeroU32::new(self.vao as u32)
                    .map(glow::NativeVertexArray),
            );
            gl.viewport(
                self.viewport[0],
                self.viewport[1],
                self.viewport[2],
                self.viewport[3],
            );
            gl.scissor(
                self.scissor[0],
                self.scissor[1],
                self.scissor[2],
                self.scissor[3],
            );
        }
    }
}

fn set_capability(gl: &Arc<glow::Context>, cap: u32, enable: bool) {
    unsafe {
        if enable {
            gl.enable(cap);
        } else {
            gl.disable(cap);
        }
    }
}
