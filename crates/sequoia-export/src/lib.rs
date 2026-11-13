#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use image::RgbaImage;
use sequoia_assets::AssetRegistry;
use sequoia_core::{
    BlendMode, Color, GradientDraw, GradientKind, ImageDraw, ImageSource, OpacityNode, Point, Quad,
    Rect, RenderItem, RenderList, RenderTargetId, RoundedClip, Shadow, ShaderDraw, ShaderId, TexInput,
    TextDraw, Transform,
};
use sequoia_reactive::Environment;
use sequoia_style::Theme;
use sequoia_text::{GlyphAtlas, TextSystem};
use sequoia_view::{Runtime, View};

mod video;

pub const SCALE_1X: f32 = 1.0;

pub const SCALE_2X: f32 = 2.0;

pub const SCALE_3X: f32 = 3.0;

pub const SCALE_4X: f32 = 4.0;

pub const SCALE_8X: f32 = 8.0;

pub const MAX_EXPORT_SCALE: f32 = 8.0;

pub const MAX_EXPORT_DIMENSION: u32 = 16384;

pub const MAX_EXPORT_PIXELS: u64 = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub struct ExportOptions {
    pub width: f32,

    pub height: f32,

    pub scale: f32,

    pub background: Option<Color>,

    pub theme: Theme,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            scale: SCALE_2X,
            background: None,
            theme: Theme::dark(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AnimationOptions {
    pub fps: u32,
    pub duration: f32,
}

impl Default for AnimationOptions {
    fn default() -> Self {
        Self {
            fps: 30,
            duration: 1.0,
        }
    }
}

impl AnimationOptions {
    pub fn new(fps: u32, duration: f32) -> Self {
        Self { fps, duration }
    }

    pub fn fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }

    pub fn duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    pub fn total_frames(&self) -> usize {
        let f = (self.fps as f64 * self.duration as f64).round();
        f.max(1.0) as usize
    }

    pub fn frame_duration(&self) -> std::time::Duration {
        if self.fps == 0 {
            std::time::Duration::from_millis(33)
        } else {
            std::time::Duration::from_secs_f32(1.0 / self.fps as f32)
        }
    }
}

impl ExportOptions {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            ..Self::default()
        }
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn transparent(mut self) -> Self {
        self.background = Some(Color::TRANSPARENT);
        self
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    fn sanitized_scale(&self) -> f32 {
        if self.scale.is_finite() {
            self.scale.clamp(1.0, MAX_EXPORT_SCALE)
        } else {
            SCALE_1X
        }
    }

    fn physical_size(&self) -> Result<(u32, u32), ExportError> {
        if !self.width.is_finite()
            || !self.height.is_finite()
            || self.width <= 0.0
            || self.height <= 0.0
        {
            return Err(ExportError::InvalidSize {
                width: self.width,
                height: self.height,
            });
        }
        let s = self.sanitized_scale();
        let w = (self.width * s).round().max(1.0) as u32;
        let h = (self.height * s).round().max(1.0) as u32;
        if w == 0 || h == 0 || w > MAX_EXPORT_DIMENSION || h > MAX_EXPORT_DIMENSION {
            return Err(ExportError::TooLarge { w, h });
        }
        if w as u64 * h as u64 > MAX_EXPORT_PIXELS {
            return Err(ExportError::TooLarge { w, h });
        }
        Ok((w, h))
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ExportError {
    InvalidSize { width: f32, height: f32 },

    TooLarge { w: u32, h: u32 },

    Io(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSize { width, height } => {
                write!(f, "sequoia-export: invalid size {width}x{height}")
            }
            Self::TooLarge { w, h } => {
                write!(f, "sequoia-export: {w}x{h}px exceeds export limits")
            }
            Self::Io(e) => write!(f, "sequoia-export: {e}"),
        }
    }
}

impl std::error::Error for ExportError {}

pub fn render_to_image(
    root: impl View + 'static,
    opts: &ExportOptions,
) -> Result<RgbaImage, ExportError> {
    render_to_image_with_assets(root, opts, AssetRegistry::with_builtin_icons())
}

/// Renders `root` with a custom asset registry.
///
/// Use this when the exported view relies on icons or fonts registered at
/// runtime, for example an editor that loads its own icon directory.
pub fn render_to_image_with_assets(
    root: impl View + 'static,
    opts: &ExportOptions,
    assets: AssetRegistry,
) -> Result<RgbaImage, ExportError> {
    Exporter::with_assets(assets).render_to_image(root, opts)
}

fn install_fonts(assets: &AssetRegistry, text: &mut TextSystem) {
    let mut families: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (alias, bytes) in assets.fonts() {
        if let Some(family) = text.load_font(bytes.to_vec()) {
            text.register_alias(alias.clone(), family.clone());
            families.insert(alias, family);
        }
    }
    if let Some(alias) = assets.default_font() {
        if let Some(family) = families.get(&alias) {
            text.set_default_family(Some(family.clone()));
        } else if text.font_families().iter().any(|family| family == &alias) {
            text.set_default_family(Some(alias));
        }
    }
}

/// A CPU implementation of a shader used during export.
///
/// Export runs headlessly on the CPU, so every shader a scene draws needs a CPU
/// counterpart registered on the [`Exporter`]. The implementation receives the
/// same UV window, uniforms, and texture inputs the GPU pipeline binds.
pub trait CpuShader {
    /// Shader registration name, matching `SequoiShader::NAME`.
    fn name(&self) -> &'static str;

    /// Returns straight-alpha RGBA for one fragment.
    fn shade(&self, ctx: &ShaderCtx<'_>) -> [f32; 4];
}

struct ShaderLayer<'a> {
    width: u32,
    height: u32,
    data: &'a [u8],
}

/// Texture inputs bound to a CPU shader draw.
pub struct ShaderInputs<'a> {
    entries: Vec<(u32, ShaderLayer<'a>)>,
}

impl<'a> ShaderInputs<'a> {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn bind(&mut self, slot: u32, width: u32, height: u32, data: &'a [u8]) {
        self.entries.push((
            slot,
            ShaderLayer {
                width,
                height,
                data,
            },
        ));
    }

    /// Bilinear sample of input `slot`, in straight-alpha RGBA.
    ///
    /// Unbound slots sample transparent white, mirroring the GPU renderer.
    #[must_use]
    pub fn sample(&self, slot: u32, uv: [f32; 2]) -> [f32; 4] {
        let Some((_, layer)) = self.entries.iter().find(|(bound, _)| *bound == slot) else {
            return [1.0, 1.0, 1.0, 0.0];
        };
        sample_rgba(layer.data, layer.width, layer.height, uv)
    }
}

/// Per-fragment state passed to [`CpuShader::shade`].
pub struct ShaderCtx<'a> {
    /// Fragment position inside the draw rectangle, in logical pixels.
    pub local: Point,

    /// Draw rectangle in logical pixels.
    pub rect: Rect,

    /// Texture coordinates after applying the draw's UV window.
    pub uv: [f32; 2],

    /// Raw uniform payload, as encoded by the shader type.
    pub uniforms: &'a [u8],

    inputs: &'a ShaderInputs<'a>,
}

impl ShaderCtx<'_> {
    /// Samples a bound texture input at `uv`.
    #[must_use]
    pub fn sample(&self, slot: u32, uv: [f32; 2]) -> [f32; 4] {
        self.inputs.sample(slot, uv)
    }
}

/// Owns the assets and CPU shaders used by an export.
///
/// The crate-level helpers build a default `Exporter` for you. Build one
/// explicitly when the scene draws shaders or needs a custom asset registry.
#[derive(Clone)]
pub struct Exporter {
    assets: AssetRegistry,
    shaders: Vec<Rc<dyn CpuShader>>,
}

impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}

struct Prepared {
    rt: Runtime,
    text: TextSystem,
    shaders: HashMap<ShaderId, Rc<dyn CpuShader>>,
}

impl Exporter {
    /// Creates an exporter with the built-in icon set and no CPU shaders.
    #[must_use]
    pub fn new() -> Self {
        Self::with_assets(AssetRegistry::with_builtin_icons())
    }

    /// Creates an exporter with a custom asset registry.
    #[must_use]
    pub fn with_assets(assets: AssetRegistry) -> Self {
        Self {
            assets,
            shaders: Vec::new(),
        }
    }

    /// Returns the asset registry used by this exporter.
    #[must_use]
    pub fn assets(&self) -> &AssetRegistry {
        &self.assets
    }

    /// Registers a CPU shader.
    ///
    /// The name must match the shader the scene draws, otherwise the draw is
    /// skipped with a warning.
    #[must_use]
    pub fn shader(mut self, shader: impl CpuShader + 'static) -> Self {
        self.shaders.push(Rc::new(shader));
        self
    }

    fn prepare(
        &self,
        root: impl View + 'static,
        opts: &ExportOptions,
    ) -> Result<Prepared, ExportError> {
        opts.physical_size()?;
        let scale = opts.sanitized_scale();

        let env = Environment::new();
        env.provide(opts.theme);

        let mut text = TextSystem::new();
        text.set_scale_factor(scale);
        install_fonts(&self.assets, &mut text);
        env.provide(self.assets.clone());

        let rt = Runtime::new();
        rt.set_environment(env);
        let mut shaders = HashMap::new();
        for shader in &self.shaders {
            let id = rt.register_shader_id(shader.name());
            shaders.insert(id, shader.clone());
        }
        rt.mount(root);

        Ok(Prepared { rt, text, shaders })
    }

    fn paint_frame(
        &self,
        prepared: &mut Prepared,
        opts: &ExportOptions,
    ) -> Result<RgbaImage, ExportError> {
        let (pw, ph) = opts.physical_size()?;
        let scale = opts.sanitized_scale();
        prepared.rt.layout(
            Rect::from_xywh(0.0, 0.0, opts.width, opts.height),
            &mut prepared.text,
        );

        let mut list = RenderList::new();
        for _ in 0..8 {
            let epoch = prepared.text.atlas().epoch();
            list.clear();
            prepared.rt.paint(&mut list, &mut prepared.text);
            if prepared.text.atlas().epoch() == epoch {
                break;
            }
        }

        Ok(rasterize(
            &list,
            prepared.text.atlas(),
            pw,
            ph,
            scale,
            opts,
            &prepared.shaders,
        ))
    }

    /// Renders one frame.
    pub fn render_to_image(
        &self,
        root: impl View + 'static,
        opts: &ExportOptions,
    ) -> Result<RgbaImage, ExportError> {
        let mut prepared = self.prepare(root, opts)?;
        self.paint_frame(&mut prepared, opts)
    }

    /// Renders `anim_opts.total_frames()` frames, advancing the frame clock.
    pub fn render_animation(
        &self,
        root: impl View + 'static,
        opts: &ExportOptions,
        anim_opts: &AnimationOptions,
    ) -> Result<Vec<RgbaImage>, ExportError> {
        let mut prepared = self.prepare(root, opts)?;
        let total_frames = anim_opts.total_frames();
        let dt = anim_opts.frame_duration();
        let mut frames = Vec::with_capacity(total_frames);
        for i in 0..total_frames {
            if i > 0 {
                prepared.rt.advance_for_test(dt);
                prepared.rt.tick();
            }
            frames.push(self.paint_frame(&mut prepared, opts)?);
        }
        Ok(frames)
    }

    /// Renders one frame and saves it as PNG.
    pub fn export_png(
        &self,
        root: impl View + 'static,
        opts: &ExportOptions,
        path: impl AsRef<Path>,
    ) -> Result<PathBuf, ExportError> {
        let path = path.as_ref().to_path_buf();
        ensure_parent(&path)?;
        let img = self.render_to_image(root, opts)?;
        img.save(&path)
            .map_err(|e| ExportError::Io(format!("save {}: {e}", path.display())))?;
        Ok(path)
    }

    /// Renders an animation and saves it as an animated GIF.
    pub fn export_gif(
        &self,
        root: impl View + 'static,
        opts: &ExportOptions,
        anim_opts: &AnimationOptions,
        path: impl AsRef<Path>,
    ) -> Result<PathBuf, ExportError> {
        let path = path.as_ref().to_path_buf();
        ensure_parent(&path)?;
        let frames = self.render_animation(root, opts, anim_opts)?;
        write_gif(frames, anim_opts, &path)?;
        Ok(path)
    }

    /// Renders an animation and saves it as a Motion JPEG AVI.
    ///
    /// The container is plain AVI with one JPEG frame per animation frame, so
    /// it plays in common video players without external tooling. Video codecs
    /// such as H.264 are intentionally not bundled.
    pub fn export_video(
        &self,
        root: impl View + 'static,
        opts: &ExportOptions,
        anim_opts: &AnimationOptions,
        path: impl AsRef<Path>,
    ) -> Result<PathBuf, ExportError> {
        let path = path.as_ref().to_path_buf();
        ensure_parent(&path)?;
        let frames = self.render_animation(root, opts, anim_opts)?;
        video::write_mjpeg_avi(&frames, anim_opts.fps.max(1), &path)?;
        Ok(path)
    }

    /// Renders an animation and saves every frame as a numbered PNG.
    pub fn export_frames(
        &self,
        root: impl View + 'static,
        opts: &ExportOptions,
        anim_opts: &AnimationOptions,
        dir_path: impl AsRef<Path>,
    ) -> Result<Vec<PathBuf>, ExportError> {
        let dir_path = dir_path.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir_path)
            .map_err(|e| ExportError::Io(format!("mkdir {}: {e}", dir_path.display())))?;
        let frames = self.render_animation(root, opts, anim_opts)?;
        write_frames(frames, &dir_path)
    }
}

fn ensure_parent(path: &Path) -> Result<(), ExportError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| ExportError::Io(format!("mkdir {}: {e}", parent.display())))?;
    }
    Ok(())
}

fn write_gif(
    frames: Vec<RgbaImage>,
    anim_opts: &AnimationOptions,
    path: &Path,
) -> Result<(), ExportError> {
    let file = std::fs::File::create(path)
        .map_err(|e| ExportError::Io(format!("create gif {}: {e}", path.display())))?;
    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    encoder
        .set_repeat(image::codecs::gif::Repeat::Infinite)
        .map_err(|e| ExportError::Io(format!("gif encoder repeat: {e}")))?;
    let delay = image::Delay::from_saturating_duration(anim_opts.frame_duration());
    for img in frames {
        let frame = image::Frame::from_parts(img, 0, 0, delay);
        encoder
            .encode_frame(frame)
            .map_err(|e| ExportError::Io(format!("encode gif frame: {e}")))?;
    }
    Ok(())
}

fn write_frames(frames: Vec<RgbaImage>, dir_path: &Path) -> Result<Vec<PathBuf>, ExportError> {
    let mut paths = Vec::with_capacity(frames.len());
    let width_digits = frames.len().to_string().len().max(4);
    for (i, img) in frames.into_iter().enumerate() {
        let filename = format!("frame_{i:0width_digits$}.png");
        let path = dir_path.join(filename);
        img.save(&path)
            .map_err(|e| ExportError::Io(format!("save frame {}: {e}", path.display())))?;
        paths.push(path);
    }
    Ok(paths)
}

pub fn export_png(
    root: impl View + 'static,
    opts: &ExportOptions,
    path: impl AsRef<Path>,
) -> Result<PathBuf, ExportError> {
    export_png_with_assets(root, opts, AssetRegistry::with_builtin_icons(), path)
}

/// Saves a PNG rendered with a custom asset registry.
pub fn export_png_with_assets(
    root: impl View + 'static,
    opts: &ExportOptions,
    assets: AssetRegistry,
    path: impl AsRef<Path>,
) -> Result<PathBuf, ExportError> {
    Exporter::with_assets(assets).export_png(root, opts, path)
}

pub fn render_animation(
    root: impl View + 'static,
    opts: &ExportOptions,
    anim_opts: &AnimationOptions,
) -> Result<Vec<RgbaImage>, ExportError> {
    render_animation_with_assets(root, opts, anim_opts, AssetRegistry::with_builtin_icons())
}

pub fn render_animation_with_assets(
    root: impl View + 'static,
    opts: &ExportOptions,
    anim_opts: &AnimationOptions,
    assets: AssetRegistry,
) -> Result<Vec<RgbaImage>, ExportError> {
    Exporter::with_assets(assets).render_animation(root, opts, anim_opts)
}

pub fn export_gif(
    root: impl View + 'static,
    opts: &ExportOptions,
    anim_opts: &AnimationOptions,
    path: impl AsRef<Path>,
) -> Result<PathBuf, ExportError> {
    export_gif_with_assets(root, opts, anim_opts, AssetRegistry::with_builtin_icons(), path)
}

pub fn export_gif_with_assets(
    root: impl View + 'static,
    opts: &ExportOptions,
    anim_opts: &AnimationOptions,
    assets: AssetRegistry,
    path: impl AsRef<Path>,
) -> Result<PathBuf, ExportError> {
    Exporter::with_assets(assets).export_gif(root, opts, anim_opts, path)
}

pub fn export_frames(
    root: impl View + 'static,
    opts: &ExportOptions,
    anim_opts: &AnimationOptions,
    dir_path: impl AsRef<Path>,
) -> Result<Vec<PathBuf>, ExportError> {
    export_frames_with_assets(root, opts, anim_opts, AssetRegistry::with_builtin_icons(), dir_path)
}

pub fn export_frames_with_assets(
    root: impl View + 'static,
    opts: &ExportOptions,
    anim_opts: &AnimationOptions,
    assets: AssetRegistry,
    dir_path: impl AsRef<Path>,
) -> Result<Vec<PathBuf>, ExportError> {
    Exporter::with_assets(assets).export_frames(root, opts, anim_opts, dir_path)
}

/// Renders an animation and saves it as a Motion JPEG AVI.
///
/// See [`Exporter::export_video`] for the container and codec constraints.
pub fn export_video(
    root: impl View + 'static,
    opts: &ExportOptions,
    anim_opts: &AnimationOptions,
    path: impl AsRef<Path>,
) -> Result<PathBuf, ExportError> {
    Exporter::new().export_video(root, opts, anim_opts, path)
}

/// Saves a video rendered with a custom asset registry.
pub fn export_video_with_assets(
    root: impl View + 'static,
    opts: &ExportOptions,
    anim_opts: &AnimationOptions,
    assets: AssetRegistry,
    path: impl AsRef<Path>,
) -> Result<PathBuf, ExportError> {
    Exporter::with_assets(assets).export_video(root, opts, anim_opts, path)
}

fn rasterize(
    list: &RenderList,
    atlas: &GlyphAtlas,
    pw: u32,
    ph: u32,
    scale: f32,
    opts: &ExportOptions,
    shaders: &HashMap<ShaderId, Rc<dyn CpuShader>>,
) -> RgbaImage {
    let bg = opts.background.unwrap_or(opts.theme.background);
    let mut img = RgbaImage::new(pw, ph);
    fill_solid(&mut img, bg);

    let mut groups = Vec::new();
    collect_opacity_groups(&list.opacity_plan(), &mut groups);

    let mut raster = Rasterizer {
        list,
        atlas,
        scale,
        shaders,
        targets: HashMap::new(),
        groups,
    };
    let end = list.items().len();
    raster.render_range(&mut img, 0, end, &[]);
    img
}

/// Collects every opacity group as `(start, end, opacity)` item ranges.
fn collect_opacity_groups(nodes: &[OpacityNode], out: &mut Vec<(usize, usize, f32)>) {
    for node in nodes {
        if let OpacityNode::Group { opacity, children } = node {
            if let Some((start, end)) = opacity_range(children) {
                out.push((start, end, *opacity));
                collect_opacity_groups(children, out);
            }
        }
    }
}

fn opacity_range(nodes: &[OpacityNode]) -> Option<(usize, usize)> {
    let mut start = usize::MAX;
    let mut end = 0usize;
    for node in nodes {
        match node {
            OpacityNode::Segment { start: s, end: e } => {
                start = start.min(*s);
                end = end.max(*e);
            }
            OpacityNode::Group { children, .. } => {
                if let Some((s, e)) = opacity_range(children) {
                    start = start.min(s);
                    end = end.max(e);
                }
            }
            _ => {}
        }
    }
    (start != usize::MAX && start < end).then_some((start, end))
}

struct Rasterizer<'a> {
    list: &'a RenderList,
    atlas: &'a GlyphAtlas,
    scale: f32,
    shaders: &'a HashMap<ShaderId, Rc<dyn CpuShader>>,
    targets: HashMap<RenderTargetId, RgbaImage>,
    groups: Vec<(usize, usize, f32)>,
}

impl Rasterizer<'_> {
    fn group_at(
        &self,
        index: usize,
        end: usize,
        applied: &[usize],
    ) -> Option<(usize, usize, f32)> {
        self.groups
            .iter()
            .enumerate()
            .filter(|(slot, (start, stop, _))| {
                *start == index && *stop <= end && *stop > index && !applied.contains(slot)
            })
            .max_by_key(|(_, (_, stop, _))| *stop)
            .map(|(slot, (_, stop, opacity))| (slot, *stop, *opacity))
    }

    fn render_range(
        &mut self,
        dst: &mut RgbaImage,
        start: usize,
        end: usize,
        applied: &[usize],
    ) {
        let mut index = start;
        while index < end {
            if let Some((group_slot, group_end, opacity)) = self.group_at(index, end, applied) {
                let mut layer = RgbaImage::new(dst.width(), dst.height());
                let mut chain = applied.to_vec();
                chain.push(group_slot);
                self.render_range(&mut layer, index, group_end, &chain);
                composite_layer(dst, &layer, opacity);
                index = group_end;
                continue;
            }
            let Some(item) = self.list.items().get(index) else {
                break;
            };
            match item {
                RenderItem::TargetBegin { id, size, .. } => {
                    let target_end = self.target_end(index, end, *id);
                    let tw = ((size.width * self.scale).ceil().max(1.0) as u32).max(1);
                    let th = ((size.height * self.scale).ceil().max(1.0) as u32).max(1);
                    let mut layer = RgbaImage::new(tw, th);
                    self.render_range(&mut layer, index + 1, target_end, applied);
                    self.targets.insert(*id, layer);
                    index = target_end.saturating_add(1);
                }
                RenderItem::TargetEnd { .. } => index += 1,
                _ => {
                    self.draw_item(dst, index);
                    index += 1;
                }
            }
        }
    }

    fn target_end(&self, begin: usize, end: usize, id: RenderTargetId) -> usize {
        let mut depth = 0usize;
        for (offset, item) in self
            .list
            .items()
            .iter()
            .enumerate()
            .take(end)
            .skip(begin + 1)
        {
            match item {
                RenderItem::TargetBegin { .. } => depth += 1,
                RenderItem::TargetEnd { id: close, .. } => {
                    if depth == 0 && *close == id {
                        return offset;
                    }
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
        }
        end
    }

    fn draw_item(&self, img: &mut RgbaImage, index: usize) {
        let items = self.list.items();
        let Some(item) = items.get(index) else {
            return;
        };
        let clip = self.list.item_clips().get(index).copied().flatten();
        let transform = self
            .list
            .item_transforms()
            .get(index)
            .copied()
            .unwrap_or_default();
        match item {
            RenderItem::Quad(q) => draw_quad(img, *q, clip, self.scale),
            RenderItem::Text(t) => draw_text(img, t, self.atlas, clip, self.scale, transform),
            RenderItem::Image(d) => draw_image(img, d, clip, self.scale),
            RenderItem::Shadow(s) => draw_shadow(img, *s, clip, self.scale),
            RenderItem::Waveform(w) => {
                for bar in &w.bars {
                    draw_quad(
                        img,
                        Quad {
                            rect: *bar,
                            color: w.color,
                            radius: 0.0,
                            border_color: Color::TRANSPARENT,
                            border_width: 0.0,
                        },
                        clip,
                        self.scale,
                    );
                }
            }
            RenderItem::Gradient(g) => draw_gradient(img, g, clip, self.scale),
            RenderItem::Shader(s) => self.draw_shader(img, s, clip),
            RenderItem::Blit {
                rect,
                uv,
                target,
                blend,
            } => self.draw_blit(img, *rect, *uv, *target, *blend, clip),
            _ => {}
        }
    }

    fn draw_shader(&self, img: &mut RgbaImage, draw: &ShaderDraw, clip: Option<RoundedClip>) {
        let Some(shader) = self.shaders.get(&draw.shader) else {
            log::warn!(
                "sequoia-export: shader {:?} has no CPU implementation - skipped",
                draw.shader
            );
            return;
        };
        if draw.rect.is_empty() {
            return;
        }
        let Some((x0, y0, x1, y1)) = int_bounds(img, draw.rect, self.scale, 1.0) else {
            return;
        };

        let mut inputs = ShaderInputs::new();
        for (slot, input) in draw.textures.iter() {
            match input {
                TexInput::Target(target) => {
                    if let Some(layer) = self.targets.get(target) {
                        inputs.bind(*slot, layer.width(), layer.height(), layer.as_raw());
                    }
                }
                TexInput::Image(ImageSource::Rgba {
                    width,
                    height,
                    data,
                    ..
                })
                | TexInput::Image(ImageSource::Dynamic {
                    width,
                    height,
                    data,
                    ..
                }) => {
                    inputs.bind(*slot, *width, *height, data.as_slice());
                }
                TexInput::Image(ImageSource::File(_)) => {
                    log::warn!("sequoia-export: file textures are not supported by CPU shaders");
                }
                TexInput::Image(ImageSource::Cube(_)) => {
                    log::warn!("sequoia-export: 3D LUT inputs are not supported by CPU shaders");
                }
                _ => {}
            }
        }

        let left = draw.rect.left() * self.scale;
        let top = draw.rect.top() * self.scale;
        let rw = (draw.rect.size.width * self.scale).max(1.0);
        let rh = (draw.rect.size.height * self.scale).max(1.0);
        let [u0, v0, u1, v1] = draw.uv;
        let opacity = if draw.opacity.is_finite() {
            draw.opacity.clamp(0.0, 1.0)
        } else {
            1.0
        };

        for y in y0..y1 {
            for x in x0..x1 {
                let fx = (x as f32 + 0.5 - left) / rw;
                let fy = (y as f32 + 0.5 - top) / rh;
                let uv = [
                    u0 + fx.clamp(0.0, 1.0) * (u1 - u0),
                    v0 + fy.clamp(0.0, 1.0) * (v1 - v0),
                ];
                let ctx = ShaderCtx {
                    local: Point::new(
                        (x as f32 + 0.5) / self.scale - draw.rect.left(),
                        (y as f32 + 0.5) / self.scale - draw.rect.top(),
                    ),
                    rect: draw.rect,
                    uv,
                    uniforms: &draw.uniforms,
                    inputs: &inputs,
                };
                let color = shader.shade(&ctx);
                let cov = rect_coverage(x, y, (left, top, left + rw, top + rh))
                    * clip_coverage(img, x, y, clip, self.scale);
                if cov <= 0.0 {
                    continue;
                }
                let alpha = color[3].clamp(0.0, 1.0) * opacity * cov;
                blend_pixel_mode(img, x, y, color[0], color[1], color[2], alpha, draw.blend);
            }
        }
    }

    fn draw_blit(
        &self,
        img: &mut RgbaImage,
        rect: Rect,
        uv: [f32; 4],
        target: RenderTargetId,
        blend: BlendMode,
        clip: Option<RoundedClip>,
    ) {
        let Some(layer) = self.targets.get(&target) else {
            return;
        };
        let Some((x0, y0, x1, y1)) = int_bounds(img, rect, self.scale, 1.0) else {
            return;
        };
        let left = rect.left() * self.scale;
        let top = rect.top() * self.scale;
        let rw = (rect.size.width * self.scale).max(1.0);
        let rh = (rect.size.height * self.scale).max(1.0);
        let (sw, sh) = (layer.width(), layer.height());
        let data = layer.as_raw();
        for y in y0..y1 {
            for x in x0..x1 {
                let fx = (x as f32 + 0.5 - left) / rw;
                let fy = (y as f32 + 0.5 - top) / rh;
                let sample = sample_rgba(
                    data,
                    sw,
                    sh,
                    [
                        uv[0] + fx.clamp(0.0, 1.0) * (uv[2] - uv[0]),
                        uv[1] + fy.clamp(0.0, 1.0) * (uv[3] - uv[1]),
                    ],
                );
                let cov = rect_coverage(x, y, (left, top, left + rw, top + rh))
                    * clip_coverage(img, x, y, clip, self.scale);
                if cov <= 0.0 {
                    continue;
                }
                blend_pixel_mode(
                    img,
                    x,
                    y,
                    sample[0],
                    sample[1],
                    sample[2],
                    sample[3] * cov,
                    blend,
                );
            }
        }
    }
}

/// Bilinear sample of straight-alpha RGBA data at normalized `uv`.
fn sample_rgba(data: &[u8], width: u32, height: u32, uv: [f32; 2]) -> [f32; 4] {
    if width == 0 || height == 0 || data.len() < width as usize * height as usize * 4 {
        return [1.0, 1.0, 1.0, 0.0];
    }
    let x = uv[0].clamp(0.0, 1.0) * (width as f32 - 1.0);
    let y = uv[1].clamp(0.0, 1.0) * (height as f32 - 1.0);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let texel = |tx: u32, ty: u32| -> [f32; 4] {
        let off = (ty as usize * width as usize + tx as usize) * 4;
        [
            data[off] as f32 / 255.0,
            data[off + 1] as f32 / 255.0,
            data[off + 2] as f32 / 255.0,
            data[off + 3] as f32 / 255.0,
        ]
    };
    let a = texel(x0, y0);
    let b = texel(x1, y0);
    let c = texel(x0, y1);
    let d = texel(x1, y1);
    let mix = |p: f32, q: f32, r: f32, s: f32| {
        p * (1.0 - fx) * (1.0 - fy) + q * fx * (1.0 - fy) + r * (1.0 - fx) * fy + s * fx * fy
    };
    [
        mix(a[0], b[0], c[0], d[0]),
        mix(a[1], b[1], c[1], d[1]),
        mix(a[2], b[2], c[2], d[2]),
        mix(a[3], b[3], c[3], d[3]),
    ]
}

/// Composites a straight-alpha layer over a straight-alpha destination.
fn composite_layer(dst: &mut RgbaImage, src: &RgbaImage, opacity: f32) {
    let o = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if o <= 0.0 {
        return;
    }
    for (dp, sp) in dst.pixels_mut().zip(src.pixels()) {
        let s = sp.0;
        let sa = s[3] as f32 / 255.0;
        if sa <= 0.0 {
            continue;
        }
        let a = sa * o;
        let d = dp.0;
        let da = d[3] as f32 / 255.0;
        let out_a = a + da * (1.0 - a);
        if out_a <= 0.0 {
            continue;
        }
        let w_dst = da * (1.0 - a);
        dp.0 = [
            q((s[0] as f32 / 255.0 * a + d[0] as f32 / 255.0 * w_dst) / out_a),
            q((s[1] as f32 / 255.0 * a + d[1] as f32 / 255.0 * w_dst) / out_a),
            q((s[2] as f32 / 255.0 * a + d[2] as f32 / 255.0 * w_dst) / out_a),
            q(out_a),
        ];
    }
}

fn blend_pixel(img: &mut RgbaImage, x: u32, y: u32, r: f32, g: f32, b: f32, alpha: f32) {
    blend_pixel_mode(img, x, y, r, g, b, alpha, BlendMode::Normal);
}

fn overlay_channel(base: f32, blend: f32) -> f32 {
    if base > 0.5 {
        1.0 - 2.0 * (1.0 - base) * (1.0 - blend)
    } else {
        2.0 * base * blend
    }
}

/// Composites straight-alpha source over `img` using `mode`.
fn blend_pixel_mode(
    img: &mut RgbaImage,
    x: u32,
    y: u32,
    r: f32,
    g: f32,
    b: f32,
    alpha: f32,
    mode: BlendMode,
) {
    if alpha <= 0.0 {
        return;
    }
    let a = alpha.clamp(0.0, 1.0);
    let d = img.get_pixel(x, y).0;
    let da = d[3] as f32 / 255.0;
    let out_a = a + da * (1.0 - a);
    if out_a <= 0.0 {
        img.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
        return;
    }
    let dst = [
        d[0] as f32 / 255.0,
        d[1] as f32 / 255.0,
        d[2] as f32 / 255.0,
    ];
    let (sr, sg, sb) = match mode {
        BlendMode::Add => (
            (dst[0] + r).min(1.0),
            (dst[1] + g).min(1.0),
            (dst[2] + b).min(1.0),
        ),
        BlendMode::Multiply => (dst[0] * r, dst[1] * g, dst[2] * b),
        BlendMode::Screen => (
            1.0 - (1.0 - dst[0]) * (1.0 - r),
            1.0 - (1.0 - dst[1]) * (1.0 - g),
            1.0 - (1.0 - dst[2]) * (1.0 - b),
        ),
        BlendMode::Subtract => (
            (dst[0] - r).max(0.0),
            (dst[1] - g).max(0.0),
            (dst[2] - b).max(0.0),
        ),
        BlendMode::Darken => (dst[0].min(r), dst[1].min(g), dst[2].min(b)),
        BlendMode::Lighten => (dst[0].max(r), dst[1].max(g), dst[2].max(b)),
        BlendMode::Overlay => (
            overlay_channel(dst[0], r),
            overlay_channel(dst[1], g),
            overlay_channel(dst[2], b),
        ),
        BlendMode::HardLight => (
            overlay_channel(r, dst[0]),
            overlay_channel(g, dst[1]),
            overlay_channel(b, dst[2]),
        ),
        _ => (r, g, b),
    };
    let w_dst = da * (1.0 - a);
    let mix = |s: f32, d_: f32| (s * a + d_ * w_dst) / out_a;
    img.put_pixel(
        x,
        y,
        image::Rgba([
            q(mix(sr, dst[0])),
            q(mix(sg, dst[1])),
            q(mix(sb, dst[2])),
            q(out_a),
        ]),
    );
}

fn q(c: f32) -> u8 {
    (c.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn fill_solid(img: &mut RgbaImage, c: Color) {
    if c.a <= 0.0 {
        return;
    }
    let (r, g, b) = (q(c.r), q(c.g), q(c.b));
    if c.a >= 1.0 {
        for px in img.pixels_mut() {
            *px = image::Rgba([r, g, b, 255]);
        }
        return;
    }
    let (w, h) = (img.width(), img.height());
    for y in 0..h {
        for x in 0..w {
            blend_pixel(img, x, y, c.r, c.g, c.b, c.a);
        }
    }
}

fn rect_coverage(x: u32, y: u32, r: (f32, f32, f32, f32)) -> f32 {
    let (x0, y0, x1, y1) = r;
    let cx = ((x as f32 + 1.0).min(x1) - (x as f32).max(x0)).clamp(0.0, 1.0);
    let cy = ((y as f32 + 1.0).min(y1) - (y as f32).max(y0)).clamp(0.0, 1.0);
    cx * cy
}

fn rounded_coverage(x: u32, y: u32, r: (f32, f32, f32, f32), radius: f32) -> f32 {
    let (x0, y0, x1, y1) = r;
    if x1 <= x0 || y1 <= y0 {
        return 0.0;
    }
    let rad = radius.clamp(0.0, ((x1 - x0).min(y1 - y0) * 0.5).max(0.0));
    if rad <= 0.0 {
        return rect_coverage(x, y, r);
    }
    let px = x as f32 + 0.5;
    let py = y as f32 + 0.5;
    let cx = (x0 + x1) * 0.5;
    let cy = (y0 + y1) * 0.5;
    let hx = (x1 - x0) * 0.5 - rad;
    let hy = (y1 - y0) * 0.5 - rad;
    let qx = (px - cx).abs() - hx;
    let qy = (py - cy).abs() - hy;
    let ax = qx.max(0.0);
    let ay = qy.max(0.0);
    let outside = (ax * ax + ay * ay).sqrt() + qx.max(qy).min(0.0) - rad;
    (0.5 - outside).clamp(0.0, 1.0)
}

fn clip_coverage(img: &RgbaImage, x: u32, y: u32, clip: Option<RoundedClip>, scale: f32) -> f32 {
    let Some(c) = clip else { return 1.0 };
    if c.rect.is_empty() {
        return 0.0;
    }
    let _ = img;
    rounded_coverage(
        x,
        y,
        (
            c.rect.left() * scale,
            c.rect.top() * scale,
            c.rect.right() * scale,
            c.rect.bottom() * scale,
        ),
        c.radius * scale,
    )
}

fn int_bounds(
    img: &RgbaImage,
    rect: Rect,
    scale: f32,
    inflate: f32,
) -> Option<(u32, u32, u32, u32)> {
    let x0 = ((rect.left() * scale - inflate).floor().max(0.0) as u32).min(img.width());
    let y0 = ((rect.top() * scale - inflate).floor().max(0.0) as u32).min(img.height());
    let x1 = ((rect.right() * scale + inflate).ceil().max(0.0) as u32).min(img.width());
    let y1 = ((rect.bottom() * scale + inflate).ceil().max(0.0) as u32).min(img.height());
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x0, y0, x1, y1))
}

fn draw_quad(img: &mut RgbaImage, quad: Quad, clip: Option<RoundedClip>, scale: f32) {
    let has_border = quad.border_width > 0.0 && quad.border_color.a > 0.0;
    if quad.color.a <= 0.0 && !has_border {
        return;
    }
    if has_border {
        fill_rounded(
            img,
            quad.rect,
            quad.radius,
            quad.border_color,
            quad.border_color.a,
            clip,
            scale,
        );
        let bw = quad.border_width.min(quad.rect.size.width * 0.5);
        let bw = bw.min(quad.rect.size.height * 0.5).max(0.0);
        let inner = Rect::from_xywh(
            quad.rect.left() + bw,
            quad.rect.top() + bw,
            (quad.rect.size.width - bw * 2.0).max(0.0),
            (quad.rect.size.height - bw * 2.0).max(0.0),
        );
        fill_rounded(
            img,
            inner,
            (quad.radius - bw).max(0.0),
            quad.color,
            quad.color.a,
            clip,
            scale,
        );
    } else {
        fill_rounded(
            img,
            quad.rect,
            quad.radius,
            quad.color,
            quad.color.a,
            clip,
            scale,
        );
    }
}

fn fill_rounded(
    img: &mut RgbaImage,
    rect: Rect,
    radius: f32,
    color: Color,
    alpha: f32,
    clip: Option<RoundedClip>,
    scale: f32,
) {
    if rect.is_empty() || alpha <= 0.0 {
        return;
    }
    let Some((x0, y0, x1, y1)) = int_bounds(img, rect, scale, 1.5) else {
        return;
    };
    let r = (
        rect.left() * scale,
        rect.top() * scale,
        rect.right() * scale,
        rect.bottom() * scale,
    );
    let rad = radius * scale;
    for y in y0..y1 {
        for x in x0..x1 {
            let cov = rounded_coverage(x, y, r, rad) * clip_coverage(img, x, y, clip, scale);
            if cov > 0.0 {
                blend_pixel(img, x, y, color.r, color.g, color.b, alpha * cov);
            }
        }
    }
}

fn draw_gradient(img: &mut RgbaImage, g: &GradientDraw, clip: Option<RoundedClip>, scale: f32) {
    if !g.is_visible() {
        return;
    }
    let stops = g.stops.stops();
    if stops.is_empty() {
        return;
    }

    let lin: Vec<([f32; 4], f32)> = stops
        .iter()
        .map(|s| {
            let c = s.color.to_linear();
            ([c.r, c.g, c.b, c.a], s.at)
        })
        .collect();
    let sample = |t: f32| -> [f32; 4] {
        let tc = t.clamp(0.0, 1.0);
        if lin.len() == 1 {
            return lin[0].0;
        }
        let mut seg = lin.len() - 2;
        for i in 0..lin.len() - 1 {
            if tc <= lin[i + 1].1 {
                seg = i;
                break;
            }
        }
        let (c0, p0) = lin[seg];
        let (c1, p1) = lin[seg + 1];
        let span = (p1 - p0).max(0.0001);
        let k = ((tc - p0) / span).clamp(0.0, 1.0);
        [
            c0[0] + (c1[0] - c0[0]) * k,
            c0[1] + (c1[1] - c0[1]) * k,
            c0[2] + (c1[2] - c0[2]) * k,
            c0[3] + (c1[3] - c0[3]) * k,
        ]
    };
    let Some((x0, y0, x1, y1)) = int_bounds(img, g.rect, scale, 1.5) else {
        return;
    };
    let (rx, ry, rw, rh) = (
        g.rect.left() * scale,
        g.rect.top() * scale,
        g.rect.size.width * scale,
        g.rect.size.height * scale,
    );
    let rad = g.radius * scale;
    for y in y0..y1 {
        for x in x0..x1 {
            let cov = rounded_coverage(x, y, (rx, ry, rx + rw, ry + rh), rad)
                * clip_coverage(img, x, y, clip, scale);
            if cov <= 0.0 {
                continue;
            }

            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let t = match g.kind {
                GradientKind::Linear { angle_deg } => {
                    let a = if angle_deg.is_finite() {
                        angle_deg.to_radians()
                    } else {
                        0.0
                    };
                    let (dx, dy) = (a.cos(), a.sin());
                    let span = (rw * dx.abs() + rh * dy.abs()).max(0.0001);
                    (((px - rx) * dx + (py - ry) * dy) / span).clamp(0.0, 1.0)
                }
                GradientKind::Radial { radius } => {
                    let (cx, cy) = (rx + rw * 0.5, ry + rh * 0.5);
                    let mut extent = (rw * 0.5).hypot(rh * 0.5);
                    let r = if radius.is_finite() {
                        radius.max(0.0)
                    } else {
                        0.0
                    } * scale;
                    if r > 0.0001 {
                        extent = r;
                    }
                    if extent <= 0.0001 {
                        0.0
                    } else {
                        ((px - cx).hypot(py - cy) / extent).clamp(0.0, 1.0)
                    }
                }
            };
            let c = sample(t);
            let col = Color {
                r: c[0],
                g: c[1],
                b: c[2],
                a: c[3],
            }
            .from_linear();
            blend_pixel(img, x, y, col.r, col.g, col.b, col.a * cov);
        }
    }
}

fn draw_shadow(img: &mut RgbaImage, shadow: Shadow, clip: Option<RoundedClip>, scale: f32) {
    if !shadow.is_visible() {
        return;
    }
    if shadow.inset {
        draw_inset_shadow(img, shadow, clip, scale);
        return;
    }

    const STEPS: i32 = 6;
    let base = Rect::from_xywh(
        shadow.rect.left() + shadow.offset.x,
        shadow.rect.top() + shadow.offset.y,
        shadow.rect.size.width,
        shadow.rect.size.height,
    );
    for i in (1..=STEPS).rev() {
        let t = i as f32 / STEPS as f32;
        let inflate = shadow.blur * 0.5 * t + shadow.spread;
        let layer = Rect::from_xywh(
            base.left() - inflate,
            base.top() - inflate,
            (base.size.width + inflate * 2.0).max(0.0),
            (base.size.height + inflate * 2.0).max(0.0),
        );
        let a = shadow.color.a * 0.22 * (1.0 - t).powi(2) + shadow.color.a * 0.02;
        fill_rounded(
            img,
            layer,
            shadow.radius + shadow.blur * 0.5 * t,
            shadow.color,
            a,
            clip,
            scale,
        );
    }
}

fn sd_rounded_box(ax: f32, ay: f32, hx: f32, hy: f32, r: f32) -> f32 {
    let rr = r.clamp(0.0, hx.min(hy).max(0.0));
    let bx = hx - rr;
    let by = hy - rr;
    let dx = ax - bx;
    let dy = ay - by;
    let ox = dx.max(0.0);
    let oy = dy.max(0.0);
    (ox * ox + oy * oy).sqrt() + dx.max(dy).min(0.0) - rr
}

fn draw_inset_shadow(img: &mut RgbaImage, shadow: Shadow, clip: Option<RoundedClip>, scale: f32) {
    if !(shadow.blur > 0.0) {
        return;
    }
    let r = shadow.rect;
    if r.is_empty() {
        return;
    }
    let Some((x0, y0, x1, y1)) = int_bounds(img, r, scale, 1.5) else {
        return;
    };

    let cx = r.left() + r.size.width * 0.5 + shadow.offset.x;
    let cy = r.top() + r.size.height * 0.5 + shadow.offset.y;
    let hx = r.size.width * 0.5 - shadow.spread;
    let hy = r.size.height * 0.5 - shadow.spread;
    let mask_r = (
        r.left() * scale,
        r.top() * scale,
        r.right() * scale,
        r.bottom() * scale,
    );
    let mask_radius = (shadow.radius + shadow.spread).max(0.0) * scale;
    for y in y0..y1 {
        for x in x0..x1 {
            let lx = (x as f32 + 0.5) / scale;
            let ly = (y as f32 + 0.5) / scale;
            let d = sd_rounded_box((lx - cx).abs(), (ly - cy).abs(), hx, hy, shadow.radius);
            let t = (0.5 + d / shadow.blur).clamp(0.0, 1.0);
            if t <= 0.0 {
                continue;
            }
            let mask = rounded_coverage(x, y, mask_r, mask_radius);
            if mask <= 0.0 {
                continue;
            }
            let cov = clip_coverage(img, x, y, clip, scale);
            let a = t * shadow.color.a * mask * cov;
            if a > 0.0 {
                blend_pixel(img, x, y, shadow.color.r, shadow.color.g, shadow.color.b, a);
            }
        }
    }
}

fn draw_text(
    img: &mut RgbaImage,
    draw: &TextDraw,
    atlas: &GlyphAtlas,
    clip: Option<RoundedClip>,
    scale: f32,
    transform: Transform,
) {
    if draw.glyphs.is_empty() || draw.color.a <= 0.0 {
        return;
    }
    let (aw, ah) = atlas.size();
    if aw == 0 || ah == 0 {
        return;
    }
    let pixels = atlas.pixels();
    let stride = aw as usize * 4;
    if transform.is_identity() {
        for g in &draw.glyphs {
            if g.rect.is_empty() {
                continue;
            }
            let dx0 = (g.rect.left() * scale).floor() as i32;
            let dy0 = (g.rect.top() * scale).floor() as i32;
            let dx1 = (g.rect.right() * scale).ceil() as i32;
            let dy1 = (g.rect.bottom() * scale).ceil() as i32;
            let dw = (dx1 - dx0).max(1) as f32;
            let dh = (dy1 - dy0).max(1) as f32;
            let (u0, v0, u1, v1) = (g.uv[0], g.uv[1], g.uv[2], g.uv[3]);
            for py in dy0.max(0)..dy1.min(img.height() as i32) {
                for px in dx0.max(0)..dx1.min(img.width() as i32) {
                    let fx = (px as f32 + 0.5 - dx0 as f32) / dw;
                    let fy = (py as f32 + 0.5 - dy0 as f32) / dh;
                    let tx = (((u0 + fx * (u1 - u0)) * aw as f32) as u32).min(aw - 1);
                    let ty = (((v0 + fy * (v1 - v0)) * ah as f32) as u32).min(ah - 1);
                    let off = (ty as usize) * stride + (tx as usize) * 4;
                    if off + 3 >= pixels.len() {
                        continue;
                    }
                    let ta = pixels[off + 3] as f32 / 255.0;
                    if ta <= 0.0 {
                        continue;
                    }
                    let (r, gg, b) = if g.color_glyph {
                        (
                            pixels[off] as f32 / 255.0,
                            pixels[off + 1] as f32 / 255.0,
                            pixels[off + 2] as f32 / 255.0,
                        )
                    } else {
                        (draw.color.r, draw.color.g, draw.color.b)
                    };
                    let cov = clip_coverage(img, px as u32, py as u32, clip, scale);
                    if cov > 0.0 {
                        blend_pixel(img, px as u32, py as u32, r, gg, b, draw.color.a * ta * cov);
                    }
                }
            }
        }
        return;
    }
    let Some(inverse) = transform.inverse() else {
        return;
    };
    for g in &draw.glyphs {
        if g.rect.is_empty() {
            continue;
        }
        let bounds = transform.apply_rect_aabb(g.rect);
        let dx0 = (bounds.left() * scale).floor() as i32;
        let dy0 = (bounds.top() * scale).floor() as i32;
        let dx1 = (bounds.right() * scale).ceil() as i32;
        let dy1 = (bounds.bottom() * scale).ceil() as i32;
        let (u0, v0, u1, v1) = (g.uv[0], g.uv[1], g.uv[2], g.uv[3]);
        for py in dy0.max(0)..dy1.min(img.height() as i32) {
            for px in dx0.max(0)..dx1.min(img.width() as i32) {
                let local = inverse.apply(Point::new(
                    (px as f32 + 0.5) / scale,
                    (py as f32 + 0.5) / scale,
                ));
                if !g.rect.contains(local) {
                    continue;
                }
                let fx = (local.x - g.rect.left()) / g.rect.size.width;
                let fy = (local.y - g.rect.top()) / g.rect.size.height;
                let tx = (((u0 + fx * (u1 - u0)) * aw as f32) as u32).min(aw - 1);
                let ty = (((v0 + fy * (v1 - v0)) * ah as f32) as u32).min(ah - 1);
                let off = (ty as usize) * stride + (tx as usize) * 4;
                if off + 3 >= pixels.len() {
                    continue;
                }
                let ta = pixels[off + 3] as f32 / 255.0;
                if ta <= 0.0 {
                    continue;
                }
                let (r, gg, b) = if g.color_glyph {
                    (
                        pixels[off] as f32 / 255.0,
                        pixels[off + 1] as f32 / 255.0,
                        pixels[off + 2] as f32 / 255.0,
                    )
                } else {
                    (draw.color.r, draw.color.g, draw.color.b)
                };
                let cov = clip_coverage(img, px as u32, py as u32, clip, scale);
                if cov > 0.0 {
                    blend_pixel(img, px as u32, py as u32, r, gg, b, draw.color.a * ta * cov);
                }
            }
        }
    }
}

fn image_file_allowed(path: &str) -> bool {
    match image::image_dimensions(path) {
        Ok((w, h)) => {
            w > 0
                && h > 0
                && w <= MAX_EXPORT_DIMENSION
                && h <= MAX_EXPORT_DIMENSION
                && u64::from(w) * u64::from(h) <= MAX_EXPORT_PIXELS
        }

        Err(_) => true,
    }
}

fn draw_image(img: &mut RgbaImage, draw: &ImageDraw, clip: Option<RoundedClip>, scale: f32) {
    match &draw.source {
        ImageSource::Rgba {
            width,
            height,
            data,
            ..
        }
        | ImageSource::Dynamic {
            width,
            height,
            data,
            ..
        } => {
            blit_rgba(img, draw, *width, *height, data, clip, scale);
        }
        ImageSource::File(path) => {
            if !image_file_allowed(path) {
                log::warn!("sequoia-export: image `{path}` past the size limits — skipped");
                return;
            }
            match image::open(path) {
                Ok(loaded) => {
                    let rgba = loaded.to_rgba8();
                    let (w, h) = (rgba.width(), rgba.height());
                    blit_rgba(img, draw, w, h, &rgba.into_raw(), clip, scale);
                }
                Err(e) => log::warn!("sequoia-export: cannot load image `{path}`: {e}"),
            }
        }

        _ => {}
    }
}

fn blit_rgba(
    img: &mut RgbaImage,
    draw: &ImageDraw,
    sw: u32,
    sh: u32,
    data: &[u8],
    clip: Option<RoundedClip>,
    scale: f32,
) {
    if sw == 0 || sh == 0 || data.len() < sw as usize * sh as usize * 4 {
        return;
    }
    let Some((x0, y0, x1, y1)) = int_bounds(img, draw.rect, scale, 1.0) else {
        return;
    };
    let [u0, v0, u1, v1] = draw.uv;
    let dw = ((draw.rect.right() - draw.rect.left()) * scale).max(1.0);
    let dh = ((draw.rect.bottom() - draw.rect.top()) * scale).max(1.0);
    for y in y0..y1 {
        for x in x0..x1 {
            let fx = (x as f32 + 0.5 - draw.rect.left() * scale) / dw;
            let fy = (y as f32 + 0.5 - draw.rect.top() * scale) / dh;

            let sx = (u0 + fx.clamp(0.0, 1.0) * (u1 - u0)) * sw as f32 - 0.5;
            let sy = (v0 + fy.clamp(0.0, 1.0) * (v1 - v0)) * sh as f32 - 0.5;
            let (r, g, b, a) = bilinear(data, sw, sh, sx, sy);
            let cov = rect_coverage(
                x,
                y,
                (
                    draw.rect.left() * scale,
                    draw.rect.top() * scale,
                    draw.rect.right() * scale,
                    draw.rect.bottom() * scale,
                ),
            ) * clip_coverage(img, x, y, clip, scale);
            if cov > 0.0 {
                blend_pixel(img, x, y, r, g, b, a * cov);
            }
        }
    }
}

fn texel(data: &[u8], sw: u32, sh: u32, x: u32, y: u32) -> (f32, f32, f32, f32) {
    let x = x.min(sw - 1) as usize;
    let y = y.min(sh - 1) as usize;
    let off = (y * sw as usize + x) * 4;
    if off + 3 >= data.len() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    (
        data[off] as f32 / 255.0,
        data[off + 1] as f32 / 255.0,
        data[off + 2] as f32 / 255.0,
        data[off + 3] as f32 / 255.0,
    )
}

fn bilinear(data: &[u8], sw: u32, sh: u32, x: f32, y: f32) -> (f32, f32, f32, f32) {
    let x0 = x.floor().clamp(0.0, sw as f32 - 1.0) as u32;
    let y0 = y.floor().clamp(0.0, sh as f32 - 1.0) as u32;
    let x1 = (x0 + 1).min(sw - 1);
    let y1 = (y0 + 1).min(sh - 1);
    let (fx, fy) = (
        (x - x0 as f32).clamp(0.0, 1.0),
        (y - y0 as f32).clamp(0.0, 1.0),
    );
    let (a, b, c, d) = (
        texel(data, sw, sh, x0, y0),
        texel(data, sw, sh, x1, y0),
        texel(data, sw, sh, x0, y1),
        texel(data, sw, sh, x1, y1),
    );
    let mix = |p: f32, q: f32, r: f32, s: f32| {
        p * (1.0 - fx) * (1.0 - fy) + q * fx * (1.0 - fy) + r * (1.0 - fx) * fy + s * fx * fy
    };
    (
        mix(a.0, b.0, c.0, d.0),
        mix(a.1, b.1, c.1, d.1),
        mix(a.2, b.2, c.2, d.2),
        mix(a.3, b.3, c.3, d.3),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_core::{BlendMode, BrushStroke, Fill, GradientStops, Length, Point};
    use sequoia_shader::{SequoiShader, shader};
    use sequoia_view::prelude::*;
    use sequoia_view::{Canvas, Painter};

    #[test]
    fn physical_size_follows_scale() {
        let opts = ExportOptions::new(200.0, 100.0).scale(SCALE_2X);
        assert_eq!(opts.physical_size().unwrap(), (400, 200));
        let opts = ExportOptions::new(200.0, 100.0).scale(SCALE_3X);
        assert_eq!(opts.physical_size().unwrap(), (600, 300));
    }

    #[test]
    fn scale_is_clamped_to_sane_range() {
        let opts = ExportOptions::new(100.0, 100.0).scale(99.0);
        let (w, _) = opts.physical_size().unwrap();
        assert_eq!(w, (100.0 * MAX_EXPORT_SCALE).round() as u32);
        assert!(ExportOptions::new(0.0, 0.0).physical_size().is_err());
    }

    #[test]
    fn red_panel_covers_canvas() {
        let opts = ExportOptions::new(64.0, 32.0)
            .scale(SCALE_1X)
            .background(Color::BLACK);
        let img = render_to_image(Panel::new().background(Color::RED).fill_size(), &opts).unwrap();
        assert_eq!((img.width(), img.height()), (64, 32));
        let center = img.get_pixel(32, 16).0;
        assert!(
            center[0] > 200 && center[1] < 60 && center[2] < 60,
            "{center:?}"
        );
    }

    #[test]
    fn opacity_group_composites_overlapping_children_once() {
        let opts = ExportOptions::new(40.0, 40.0)
            .scale(SCALE_1X)
            .background(Color::BLACK);
        let layered = Stack::new()
            .fill_size()
            .child(Panel::new().background(Color::RED).fill_size())
            .child(Panel::new().background(Color::RED).fill_size());
        let img = render_to_image(Opacity::new(layered, 0.5), &opts).unwrap();
        let center = img.get_pixel(20, 20).0;
        assert!(
            (110..=140).contains(&center[0]) && center[1] < 10 && center[2] < 10,
            "group alpha should be applied once, got {center:?}"
        );
    }

    #[test]
    fn nested_opacity_groups_multiply() {
        let opts = ExportOptions::new(40.0, 40.0)
            .scale(SCALE_1X)
            .background(Color::BLACK);
        let panel = Panel::new()
            .background(Color::RED)
            .width(Length::px(40.0))
            .height(Length::px(40.0));
        let img = render_to_image(Opacity::new(Opacity::new(panel, 0.5), 0.5), &opts).unwrap();
        let center = img.get_pixel(20, 20).0;
        assert!(
            (50..=80).contains(&center[0]) && center[1] < 10 && center[2] < 10,
            "nested opacity should multiply, got {center:?}"
        );
    }

    #[test]
    fn eight_x_output_has_correct_dimensions() {
        let opts = ExportOptions::new(480.0, 320.0).scale(SCALE_8X);
        let img = render_to_image(Panel::new().background(Color::BLUE).fill_size(), &opts).unwrap();
        assert_eq!((img.width(), img.height()), (3840, 2560));
    }

    #[test]
    fn export_png_roundtrips_through_temp_dir() {
        let dir = std::env::temp_dir().join("sequoia-export-test");
        let path = dir.join("roundtrip@2x.png");
        let opts = ExportOptions::new(64.0, 32.0).scale(SCALE_2X);
        let saved = export_png(
            Panel::new().background(Color::RED).fill_size(),
            &opts,
            &path,
        )
        .unwrap();
        assert_eq!(saved, path);
        let loaded = image::open(&path).unwrap().to_rgba8();
        assert_eq!((loaded.width(), loaded.height()), (128, 64));
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn three_x_output_has_nine_times_pixels() {
        let opts1 = ExportOptions::new(120.0, 80.0).scale(SCALE_1X);
        let opts3 = ExportOptions::new(120.0, 80.0).scale(SCALE_3X);
        let a = render_to_image(Panel::new().background(Color::BLUE).fill_size(), &opts1).unwrap();
        let b = render_to_image(Panel::new().background(Color::BLUE).fill_size(), &opts3).unwrap();
        assert_eq!((a.width(), a.height()), (120, 80));
        assert_eq!((b.width(), b.height()), (360, 240));
    }

    #[test]
    fn image_file_allowed_rejects_dimension_bomb() {
        fn crc32(data: &[u8]) -> u32 {
            let mut crc = 0xFFFF_FFFFu32;
            for &b in data {
                crc ^= u32::from(b);
                for _ in 0..8 {
                    let mask = (crc & 1).wrapping_neg();
                    crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
                }
            }
            !crc
        }
        use image::ImageEncoder as _;
        let pixels = [0u8; 16];
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(&pixels, 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();
        let dir = std::env::temp_dir().join("sequoia-export-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bomb.png");
        bytes[16..20].copy_from_slice(&40_000u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&40_000u32.to_be_bytes());
        let crc = crc32(&bytes[12..29]);
        bytes[29..33].copy_from_slice(&crc.to_be_bytes());
        std::fs::write(&path, &bytes).unwrap();
        assert!(!image_file_allowed(path.to_str().unwrap()));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn text_renders_glyphs_when_fonts_available() {
        let mut probe = TextSystem::new();
        if probe.shape("Sequoia", 20.0).glyph_count() == 0 {
            eprintln!("skip: no fonts in environment");
            return;
        }
        let opts = ExportOptions::new(200.0, 60.0).scale(SCALE_2X);
        let img = render_to_image(Text::new("Sequoia").size(20.0), &opts).unwrap();
        let bg = opts.theme.background.to_rgb8();
        let inked = img
            .pixels()
            .filter(|p| {
                (p.0[0] as i32 - bg.0 as i32).abs()
                    + (p.0[1] as i32 - bg.1 as i32).abs()
                    + (p.0[2] as i32 - bg.2 as i32).abs()
                    > 30
            })
            .count();
        assert!(inked > 20, "expected visible glyphs, got {inked}");
    }

    #[test]
    fn rotated_text_export_swaps_ink_axis() {
        let mut probe = TextSystem::new();
        if probe.shape("Sequoia", 28.0).glyph_count() == 0 {
            eprintln!("skip: no fonts in environment");
            return;
        }
        let opts = ExportOptions::new(320.0, 320.0).scale(SCALE_1X);
        let bg = opts.theme.background.to_rgb8();
        let ink_bbox = |img: &RgbaImage| -> (u32, u32) {
            let mut min_x = u32::MAX;
            let mut min_y = u32::MAX;
            let mut max_x = 0;
            let mut max_y = 0;
            for (x, y, p) in img.enumerate_pixels() {
                let d = (p.0[0] as i32 - bg.0 as i32).abs()
                    + (p.0[1] as i32 - bg.1 as i32).abs()
                    + (p.0[2] as i32 - bg.2 as i32).abs();
                if d > 30 {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
            assert!(
                min_x <= max_x && min_y <= max_y,
                "expected visible text ink"
            );
            (max_x - min_x + 1, max_y - min_y + 1)
        };
        let plain = render_to_image(
            Stack::new()
                .size(Length::Fill(1.0), Length::Fill(1.0))
                .overlay(Text::new("Sequoia").size(28.0), 120.0, 140.0),
            &opts,
        )
        .unwrap();
        let rotated = render_to_image(
            Stack::new()
                .size(Length::Fill(1.0), Length::Fill(1.0))
                .overlay(Text::new("Sequoia").size(28.0).rotate(90.0), 120.0, 140.0),
            &opts,
        )
        .unwrap();
        let (plain_w, plain_h) = ink_bbox(&plain);
        let (rot_w, rot_h) = ink_bbox(&rotated);
        assert!(
            plain_w > plain_h * 2,
            "plain ink should be wide: {plain_w}x{plain_h}"
        );
        assert!(
            rot_h > rot_w * 2,
            "rotated ink should be tall: {rot_w}x{rot_h}"
        );
    }

    #[test]
    fn animation_options_total_frames() {
        let anim = AnimationOptions::new(10, 2.0);
        assert_eq!(anim.total_frames(), 20);
        assert_eq!(anim.frame_duration().as_millis(), 100);
    }

    #[test]
    fn render_animation_and_export_gif_roundtrip() {
        let dir = std::env::temp_dir().join("sequoia-export-anim-test");
        let gif_path = dir.join("anim.gif");
        let frames_dir = dir.join("frames");
        let opts = ExportOptions::new(64.0, 32.0).scale(SCALE_1X);
        let anim_opts = AnimationOptions::new(10, 0.5); // 5 frames

        let frames = render_animation(
            Panel::new().background(Color::RED).fill_size(),
            &opts,
            &anim_opts,
        )
        .unwrap();
        assert_eq!(frames.len(), 5);
        assert_eq!((frames[0].width(), frames[0].height()), (64, 32));

        let saved_gif = export_gif(
            Panel::new().background(Color::RED).fill_size(),
            &opts,
            &anim_opts,
            &gif_path,
        )
        .unwrap();
        assert_eq!(saved_gif, gif_path);
        assert!(gif_path.exists());

        let saved_frames = export_frames(
            Panel::new().background(Color::RED).fill_size(),
            &opts,
            &anim_opts,
            &frames_dir,
        )
        .unwrap();
        assert_eq!(saved_frames.len(), 5);
        assert!(frames_dir.join("frame_0000.png").exists());

        std::fs::remove_file(&gif_path).ok();
        for p in saved_frames {
            std::fs::remove_file(&p).ok();
        }
        std::fs::remove_dir(&frames_dir).ok();
        std::fs::remove_dir(&dir).ok();
    }

    shader! {
        pub fn glass(uv: FragUv, content: Sampler2D, strength: f32, thickness: f32) -> Color {
            let base = textureSample(content, content_sampler, uv);

            let edge = 1.0 - uv.x;
            let band = 1.0 - sq_smoothstep(0.0, thickness, edge);
            let bend = strength * band * band;
            let refracted = textureSample(
                content,
                content_sampler,
                clamp(uv - vec2<f32>(bend, 0.0), vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0)),
            );
            let mixed = sq_mix3(base.rgb, refracted.rgb, 0.65);
            let rim = band * band * band;
            let glow = vec3<f32>(1.0, 0.90, 0.78) * (rim * 0.7);
            return vec4<f32>(mixed + glow, base.a);
        }
    }

    fn cpu_smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
        let t = ((x - edge0) / (edge1 - edge0).max(0.000_001)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    impl CpuShader for Glass {
        fn name(&self) -> &'static str {
            <Glass as SequoiShader>::NAME
        }

        fn shade(&self, ctx: &ShaderCtx<'_>) -> [f32; 4] {
            let base = ctx.sample(0, ctx.uv);
            let edge = 1.0 - ctx.uv[0];
            let band = 1.0 - cpu_smoothstep(0.0, self.thickness, edge);
            let bend = self.strength * band * band;
            let refracted = ctx.sample(0, [(ctx.uv[0] - bend).clamp(0.0, 1.0), ctx.uv[1]]);
            let rim = band * band * band;
            let glow = rim * 0.7;
            let mixed = |a: f32, b: f32| a + (b - a) * 0.65;
            [
                (mixed(base[0], refracted[0]) + glow).min(1.0),
                (mixed(base[1], refracted[1]) + glow * 0.90).min(1.0),
                (mixed(base[2], refracted[2]) + glow * 0.78).min(1.0),
                base[3],
            ]
        }
    }

    fn glass_scene(g: &mut Painter) {
        let rect = g.rect();
        let size = rect.size;
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }

        let card = rect.inflate(-20.0, -20.0);
        let radius = 40.0;
        let t = g.ctx().runtime().elapsed_secs();

        let (content, ()) = g.group(size, BlendMode::Normal, |inner| {
            inner.fill(
                card,
                Fill::vertical(GradientStops::two(
                    Color::hex(0x414141),
                    Color::hex(0x333333),
                )),
            );
            inner.rect_outline(
                card,
                radius,
                BrushStroke::new(1.0),
                Color::rgba(1.0, 1.0, 1.0, 0.10),
            );

            let phase = t * std::f32::consts::TAU / 1.5;
            let cx = card.left() + card.size.width * (0.66 + 0.12 * phase.sin());
            let cy = card.center().y;
            let r = card.size.height * 0.19;
            let center = Point::new(cx, cy);

            inner.circle(
                center,
                r * 1.35,
                Fill::radial_radius(
                    GradientStops::two(
                        Color::rgba(0.96, 0.96, 0.96, 0.22),
                        Color::rgba(0.96, 0.96, 0.96, 0.0),
                    ),
                    r * 1.35,
                ),
            );
            inner.circle(
                center,
                r,
                Fill::radial_radius(
                    GradientStops::two(Color::hex(0xF2F2F2), Color::hex(0xCFCFCF)),
                    r,
                ),
            );
        });

        let uv = Painter::target_uv(size, card);
        g.clipped_rounded(card, radius, |inner| {
            inner.shader_at(
                Glass {
                    strength: 0.35,
                    thickness: 0.16,
                },
                card,
                uv,
                &[("content", TexInput::Target(content))],
            );
        });
    }

    #[test]
    fn glass_shader_animates_a_moving_sphere_into_a_gif() {
        use image::AnimationDecoder as _;

        let dir = std::env::temp_dir().join("sequoia-export-glass-gif");
        let gif_path = dir.join("glass_sphere.gif");
        let opts = ExportOptions::new(240.0, 180.0).scale(SCALE_1X);
        let anim_opts = AnimationOptions::new(12, 1.5);
        let exporter = Exporter::new().shader(Glass {
            strength: 0.35,
            thickness: 0.16,
        });

        let saved = exporter
            .export_gif(Canvas::new().on_draw(glass_scene), &opts, &anim_opts, &gif_path)
            .unwrap();
        assert_eq!(saved, gif_path);

        let file = std::fs::File::open(&gif_path).unwrap();
        let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(file)).unwrap();
        let frames: Vec<RgbaImage> = decoder
            .into_frames()
            .map(|frame| frame.unwrap().into_buffer())
            .collect();
        assert_eq!(frames.len(), anim_opts.total_frames());
        assert_eq!((frames[0].width(), frames[0].height()), (240, 180));

        let brightest = frames[0]
            .pixels()
            .map(|p| p[0].max(p[1]).max(p[2]))
            .max()
            .unwrap();
        assert!(
            brightest > 120,
            "glass must reveal the captured sphere, got {brightest}"
        );

        let moved: u64 = frames[0]
            .pixels()
            .zip(frames[frames.len() / 4].pixels())
            .map(|(a, b)| u64::from((i32::from(a[0]) - i32::from(b[0])).unsigned_abs()))
            .sum();
        assert!(moved > 10_000, "sphere should move between frames: {moved}");

        std::fs::remove_file(&gif_path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn export_video_writes_a_mjpeg_avi() {
        let dir = std::env::temp_dir().join("sequoia-export-video");
        let path = dir.join("clip.avi");
        let opts = ExportOptions::new(48.0, 32.0).scale(SCALE_1X);
        let anim_opts = AnimationOptions::new(8, 0.5);
        let exporter = Exporter::new();
        let view = Canvas::new().on_draw(|g| {
            let t = g.ctx().runtime().elapsed_secs();
            g.fill(g.rect(), Fill::solid(Color::rgb(t.min(1.0), 0.2, 0.4)));
        });

        let saved = exporter
            .export_video(view, &opts, &anim_opts, &path)
            .unwrap();
        assert_eq!(saved, path);

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"AVI ");
        let riff_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        assert_eq!(riff_size + 8, bytes.len());

        let movi = bytes
            .windows(4)
            .position(|w| w == b"movi".as_slice())
            .unwrap();
        let idx1 = bytes
            .windows(4)
            .position(|w| w == b"idx1".as_slice())
            .unwrap();
        assert!(movi < idx1);

        let mut pos = movi + 4;
        let mut chunks = 0usize;
        while pos + 8 <= idx1 {
            assert_eq!(&bytes[pos..pos + 4], b"00dc");
            let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
            assert_eq!(&bytes[pos + 8..pos + 10], &[0xFF, 0xD8], "JPEG SOI");
            pos += 8 + size + (size % 2);
            chunks += 1;
        }
        assert_eq!(chunks, anim_opts.total_frames());

        let idx_size = u32::from_le_bytes(bytes[idx1 + 4..idx1 + 8].try_into().unwrap()) as usize;
        assert_eq!(idx_size, chunks * 16);
        let total_frames = u32::from_le_bytes(bytes[48..52].try_into().unwrap());
        assert_eq!(total_frames, anim_opts.total_frames() as u32);

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }
}
