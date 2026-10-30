#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use image::RgbaImage;
use sequoia_assets::AssetRegistry;
use sequoia_core::{
    Color, GradientDraw, GradientKind, ImageDraw, ImageSource, OpacityNode, Point, Quad, Rect,
    RenderItem, RenderList, RoundedClip, Shadow, TextDraw, Transform,
};
use sequoia_reactive::Environment;
use sequoia_style::Theme;
use sequoia_text::{GlyphAtlas, TextSystem};
use sequoia_view::{Runtime, View};

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
    let (pw, ph) = opts.physical_size()?;
    let scale = opts.sanitized_scale();

    let env = Environment::new();
    env.provide(opts.theme);
    env.provide(AssetRegistry::with_builtin_icons());

    let rt = Runtime::new();
    rt.set_environment(env);
    rt.mount(root);

    let mut text = TextSystem::new();
    text.set_scale_factor(scale);
    rt.layout(
        Rect::from_xywh(0.0, 0.0, opts.width, opts.height),
        &mut text,
    );

    let mut list = RenderList::new();
    for _ in 0..8 {
        let epoch = text.atlas().epoch();
        list.clear();
        rt.paint(&mut list, &mut text);
        if text.atlas().epoch() == epoch {
            break;
        }
    }

    Ok(rasterize(&list, text.atlas(), pw, ph, scale, opts))
}

pub fn export_png(
    root: impl View + 'static,
    opts: &ExportOptions,
    path: impl AsRef<Path>,
) -> Result<PathBuf, ExportError> {
    let path = path.as_ref().to_path_buf();
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| ExportError::Io(format!("mkdir {}: {e}", parent.display())))?;
    }
    let img = render_to_image(root, opts)?;
    img.save(&path)
        .map_err(|e| ExportError::Io(format!("save {}: {e}", path.display())))?;
    Ok(path)
}

fn rasterize(
    list: &RenderList,
    atlas: &GlyphAtlas,
    pw: u32,
    ph: u32,
    scale: f32,
    opts: &ExportOptions,
) -> RgbaImage {
    let bg = opts.background.unwrap_or(opts.theme.background);
    let mut img = RgbaImage::new(pw, ph);
    fill_solid(&mut img, bg);

    let plan = list.opacity_plan();
    draw_opacity_nodes(&mut img, &plan, list, atlas, scale);
    let _ = (pw, ph);
    img
}

/// Draws an opacity plan, rendering each group into a transparent layer and
/// compositing it once at the group opacity.
fn draw_opacity_nodes(
    img: &mut RgbaImage,
    nodes: &[OpacityNode],
    list: &RenderList,
    atlas: &GlyphAtlas,
    scale: f32,
) {
    for node in nodes {
        match node {
            OpacityNode::Segment { start, end } => {
                draw_item_range(img, list, *start, *end, atlas, scale);
            }
            OpacityNode::Group { opacity, children } => {
                let mut layer = RgbaImage::new(img.width(), img.height());
                draw_opacity_nodes(&mut layer, children, list, atlas, scale);
                composite_layer(img, &layer, *opacity);
            }
            _ => {}
        }
    }
}

fn draw_item_range(
    img: &mut RgbaImage,
    list: &RenderList,
    start: usize,
    end: usize,
    atlas: &GlyphAtlas,
    scale: f32,
) {
    let clips = list.item_clips();
    let transforms = list.item_transforms();
    let items = list.items();
    let start = start.min(items.len());
    let end = end.min(items.len());
    for (i, item) in items.iter().enumerate().take(end).skip(start) {
        let clip = clips.get(i).copied().flatten();
        let transform = transforms.get(i).copied().unwrap_or_default();
        match item {
            RenderItem::Quad(q) => draw_quad(img, *q, clip, scale),
            RenderItem::Text(t) => draw_text(img, t, atlas, clip, scale, transform),
            RenderItem::Image(d) => draw_image(img, d, clip, scale),
            RenderItem::Shadow(s) => draw_shadow(img, *s, clip, scale),
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
                        scale,
                    );
                }
            }
            RenderItem::Gradient(g) => draw_gradient(img, g, clip, scale),
            _ => {}
        }
    }
}

/// Composites a premultiplied layer over a premultiplied destination.
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
        let dr = d[0] as f32 / 255.0;
        let dg = d[1] as f32 / 255.0;
        let db = d[2] as f32 / 255.0;
        let da = d[3] as f32 / 255.0;
        dp.0 = [
            q(s[0] as f32 / 255.0 * o + dr * (1.0 - a)),
            q(s[1] as f32 / 255.0 * o + dg * (1.0 - a)),
            q(s[2] as f32 / 255.0 * o + db * (1.0 - a)),
            q(a + da * (1.0 - a)),
        ];
    }
}

fn blend_pixel(img: &mut RgbaImage, x: u32, y: u32, r: f32, g: f32, b: f32, alpha: f32) {
    if alpha <= 0.0 {
        return;
    }
    let a = alpha.clamp(0.0, 1.0);
    if a >= 1.0 {
        img.put_pixel(x, y, image::Rgba([q(r), q(g), q(b), 255]));
        return;
    }
    let d = img.get_pixel(x, y).0;
    let (dr, dg, db, da) = (
        d[0] as f32 / 255.0,
        d[1] as f32 / 255.0,
        d[2] as f32 / 255.0,
        d[3] as f32 / 255.0,
    );
    img.put_pixel(
        x,
        y,
        image::Rgba([
            q(r * a + dr * (1.0 - a)),
            q(g * a + dg * (1.0 - a)),
            q(b * a + db * (1.0 - a)),
            q((a + da * (1.0 - a)) * 255.0 / 255.0 * 255.0),
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
    use sequoia_core::Length;
    use sequoia_view::prelude::*;

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
}
