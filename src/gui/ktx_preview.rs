// GPU-side KTX previews. Block-compressed texels are uploaded as-is and decoded by
// the GPU's texture units, so a thumbnail costs ~1 byte/pixel of VRAM (vs 4 for
// RGBA) and no CPU decode. Textures are drawn through an egui_glow paint callback
// with our own shader, because egui's blending expects premultiplied alpha and
// compressed data can't be premultiplied before upload.

use crate::logic::ktx::{KtxImage, TextureFormat};
use eframe::{
    egui, egui_glow,
    glow::{self, HasContext},
};
use std::sync::{Arc, Mutex, OnceLock};

/// Which formats the GL context can sample directly. Unset means no GL context.
static GPU_FORMATS: OnceLock<GpuFormats> = OnceLock::new();

/// Textures whose last reference was dropped; deleted on the paint thread.
static DELETE_QUEUE: Mutex<Vec<glow::Texture>> = Mutex::new(Vec::new());

/// Shader program + VAO, created on first paint.
static RENDERER: Mutex<Option<Renderer>> = Mutex::new(None);

#[derive(Debug, Default)]
struct GpuFormats {
    etc2: bool,
    s3tc: bool,
    bptc: bool,
    rgba8: bool,
}

/// Probe the context's formats. `glow::Context` caches its version and extension
/// list at creation, so this makes no GL calls and is safe from any thread.
pub fn init(gl: Option<&glow::Context>) {
    let Some(gl) = gl else { return };
    if std::env::var_os("ROEXTRACT_KTX_CPU_DECODE").is_some() {
        log_info!("KTX previews: GPU decoding disabled by ROEXTRACT_KTX_CPU_DECODE");
        return;
    }

    let version = gl.version();
    let extensions = gl.supported_extensions();
    let at_least = |major: u32, minor: u32| (version.major, version.minor) >= (major, minor);
    let has = |name: &str| extensions.contains(name);

    // The preview shader needs GLSL 1.40 / ES 3.00 (gl_VertexID) and a VAO.
    let shader_ok = if version.is_embedded {
        at_least(3, 0)
    } else {
        at_least(3, 1)
    };
    if !shader_ok {
        log_info!("KTX previews: GL {:?} too old, using CPU decoding", version);
        return;
    }

    let formats = GpuFormats {
        etc2: version.is_embedded || at_least(4, 3) || has("GL_ARB_ES3_compatibility"),
        s3tc: has("GL_EXT_texture_compression_s3tc"),
        bptc: (!version.is_embedded && at_least(4, 2))
            || has("GL_ARB_texture_compression_bptc")
            || has("GL_EXT_texture_compression_bptc"),
        rgba8: true,
    };
    log_info!("KTX previews: GPU formats {:?}", formats);
    let _ = GPU_FORMATS.set(formats);
}

/// True if `format` can be uploaded compressed and sampled by the GPU.
pub fn gpu_supports(format: TextureFormat) -> bool {
    let Some(formats) = GPU_FORMATS.get() else {
        return false;
    };
    match format {
        TextureFormat::Etc2Rgb | TextureFormat::Etc2Rgba1 | TextureFormat::Etc2Rgba8 => {
            formats.etc2
        }
        TextureFormat::Bc1Rgb | TextureFormat::Bc1Rgba | TextureFormat::Bc3 => formats.s3tc,
        TextureFormat::Bc7 => formats.bptc,
        TextureFormat::Rgba8 => formats.rgba8,
    }
}

/// A preview texture. Data is uploaded lazily on the first paint (the only place
/// with a current GL context) and the GL texture is freed after the last drop.
pub struct KtxTexture {
    size: egui::Vec2,
    pending: Mutex<Option<KtxImage>>,
    texture: Mutex<Option<glow::Texture>>,
}

impl KtxTexture {
    pub fn new(image: KtxImage) -> Self {
        Self {
            size: egui::vec2(image.width as f32, image.height as f32),
            pending: Mutex::new(Some(image)),
            texture: Mutex::new(None),
        }
    }

    pub fn size(&self) -> egui::Vec2 {
        self.size
    }
}

impl Drop for KtxTexture {
    fn drop(&mut self) {
        if let Some(texture) = self.texture.get_mut().unwrap().take() {
            DELETE_QUEUE.lock().unwrap().push(texture);
        }
    }
}

/// Paint `texture` stretched over `rect`.
pub fn paint(ui: &egui::Ui, rect: egui::Rect, texture: Arc<KtxTexture>) {
    let callback = egui_glow::CallbackFn::new(move |_info, painter| unsafe {
        draw(painter.gl(), &texture);
    });
    ui.painter().add(egui::PaintCallback {
        rect,
        callback: Arc::new(callback),
    });
}

struct Renderer {
    program: glow::Program,
    vertex_array: glow::VertexArray,
    sampler: Option<glow::UniformLocation>,
}

const VERTEX_SHADER: &str = r#"
out vec2 v_uv;
void main() {
    // Triangle strip covering the viewport; uv (0,0) is the top-left texel row.
    vec2 corner = vec2(float(gl_VertexID & 1), float((gl_VertexID >> 1) & 1));
    v_uv = corner;
    gl_Position = vec4(corner.x * 2.0 - 1.0, 1.0 - corner.y * 2.0, 0.0, 1.0);
}
"#;

// Bilinear filtering done by hand: texels are straight alpha but egui blends
// premultiplied, and premultiplying *after* hardware filtering darkens edges
// (transparent texels bleed their RGB in). So premultiply each texel, then blend.
const FRAGMENT_SHADER: &str = r#"
uniform sampler2D u_sampler;
in vec2 v_uv;
out vec4 f_color;

vec4 fetch_premultiplied(ivec2 coord, ivec2 max_coord) {
    vec4 texel = texelFetch(u_sampler, clamp(coord, ivec2(0), max_coord), 0);
    return vec4(texel.rgb * texel.a, texel.a);
}

void main() {
    ivec2 size = textureSize(u_sampler, 0);
    ivec2 max_coord = size - 1;
    vec2 position = v_uv * vec2(size) - 0.5;
    ivec2 base = ivec2(floor(position));
    vec2 weight = fract(position);

    vec4 top = mix(
        fetch_premultiplied(base, max_coord),
        fetch_premultiplied(base + ivec2(1, 0), max_coord),
        weight.x);
    vec4 bottom = mix(
        fetch_premultiplied(base + ivec2(0, 1), max_coord),
        fetch_premultiplied(base + ivec2(1, 1), max_coord),
        weight.x);
    f_color = mix(top, bottom, weight.y);
}
"#;

unsafe fn create_renderer(gl: &glow::Context) -> Result<Renderer, String> {
    unsafe {
        let header = if gl.version().is_embedded {
            "#version 300 es\nprecision mediump float;\n"
        } else {
            "#version 140\n"
        };

        let program = gl.create_program()?;
        let mut shaders = Vec::new();
        for (kind, source) in [
            (glow::VERTEX_SHADER, VERTEX_SHADER),
            (glow::FRAGMENT_SHADER, FRAGMENT_SHADER),
        ] {
            let shader = gl.create_shader(kind)?;
            gl.shader_source(shader, &format!("{}{}", header, source));
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                let log = gl.get_shader_info_log(shader);
                gl.delete_shader(shader);
                gl.delete_program(program);
                return Err(log);
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }
        gl.link_program(program);
        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }
        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            return Err(log);
        }

        let vertex_array = gl.create_vertex_array()?;
        let sampler = gl.get_uniform_location(program, "u_sampler");
        Ok(Renderer {
            program,
            vertex_array,
            sampler,
        })
    }
}

unsafe fn upload(gl: &glow::Context, image: &KtxImage) -> Result<glow::Texture, String> {
    unsafe {
        // Clear errors left by earlier calls so the check below only sees ours.
        // Bounded: a lost context can report errors forever.
        for _ in 0..16 {
            if gl.get_error() == glow::NO_ERROR {
                break;
            }
        }

        let texture = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        // Single level: without these the texture is mip-incomplete and samples black.
        // The shader filters by hand with texelFetch, so NEAREST is all it needs.
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAX_LEVEL, 0);
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as i32,
        );

        let (width, height) = (image.width as i32, image.height as i32);
        if image.format.is_compressed() {
            gl.compressed_tex_image_2d(
                glow::TEXTURE_2D,
                0,
                image.format.gl_internal_format() as i32,
                width,
                height,
                0,
                image.data.len() as i32,
                &image.data,
            );
        } else {
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                width,
                height,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&image.data)),
            );
        }

        let error = gl.get_error();
        if error != glow::NO_ERROR {
            gl.delete_texture(texture);
            return Err(format!(
                "GL error {:#x} uploading {:?}",
                error, image.format
            ));
        }
        Ok(texture)
    }
}

unsafe fn draw(gl: &glow::Context, texture: &KtxTexture) {
    unsafe {
        for stale in DELETE_QUEUE.lock().unwrap().drain(..) {
            gl.delete_texture(stale);
        }

        let mut gl_texture = texture.texture.lock().unwrap();
        if gl_texture.is_none() {
            let Some(image) = texture.pending.lock().unwrap().take() else {
                return; // upload failed earlier
            };
            match upload(gl, &image) {
                Ok(uploaded) => *gl_texture = Some(uploaded),
                Err(e) => {
                    log_error!("KTX preview upload failed: {}", e);
                    return;
                }
            }
        }

        let mut renderer = RENDERER.lock().unwrap();
        if renderer.is_none() {
            match create_renderer(gl) {
                Ok(created) => *renderer = Some(created),
                Err(e) => {
                    log_error!("KTX preview shader failed: {}", e);
                    return;
                }
            }
        }
        let renderer = renderer.as_ref().unwrap();

        gl.use_program(Some(renderer.program));
        gl.uniform_1_i32(renderer.sampler.as_ref(), 0);
        gl.bind_vertex_array(Some(renderer.vertex_array));
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, *gl_texture);
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        gl.bind_vertex_array(None);
    }
}
