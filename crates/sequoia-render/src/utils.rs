//! Shared helpers for image decoding, colors, rectangles, and buffer limits.

#[allow(unused_imports)]
use std::any::Any;

use crate::types::*;
use sequoia_core::{Color, ImageSource, Rect, RenderItem};

pub(crate) fn find_image_source(items: &[RenderItem], key: u64) -> Option<ImageSource> {
    items.iter().find_map(|item| match item {
        RenderItem::Image(d) if d.source.cache_key() == key => Some(d.source.clone()),
        _ => None,
    })
}

/// Decodes an image into RGBA8, rejecting sources outside the upload limits.
pub(crate) fn decode_image(source: &ImageSource) -> Option<(u32, u32, Vec<u8>)> {
    match source {
        ImageSource::File(path) => {
            if let Ok((w, h)) = image::image_dimensions(path)
                && !image_dims_allowed(w, h)
            {
                log::warn!(
                    "sequoia-render: image {path} past the limit ({w}x{h}, max {MAX_IMAGE_DIM}px / {} MiB) — refused",
                    MAX_IMAGE_BYTES / (1 << 20)
                );
                return None;
            }
            let img = image::open(path)
                .inspect_err(|e| {
                    log::warn!("sequoia-render: cannot load image `{path}`: {e}");
                })
                .ok()?
                .to_rgba8();
            let (width, height) = img.dimensions();
            if !image_dims_allowed(width, height) {
                log::warn!(
                    "sequoia-render: image {path} after decoding past the limit ({width}x{height}) — refused"
                );
                return None;
            }
            if width == 0 || height == 0 {
                return None;
            }
            Some((width, height, img.into_raw()))
        }
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
            let (width, height) = (*width, *height);
            if !image_dims_allowed(width, height) {
                log::warn!(
                    "sequoia-render: RGBA buffer past the limit ({width}x{height}) — refused"
                );
                return None;
            }
            if width == 0 || height == 0 {
                return None;
            }

            let want = (width as usize)
                .checked_mul(height as usize)?
                .checked_mul(4)?;
            if data.len() != want {
                return None;
            }
            Some((width, height, data.to_vec()))
        }
        _ => None,
    }
}

/// Returns the number of mip levels for an image of `width` x `height`.
pub(crate) fn mip_level_count(width: u32, height: u32) -> u32 {
    let max = width.max(height).max(1);
    u32::BITS - max.leading_zeros()
}

fn srgb_u8_to_linear(v: u8) -> f32 {
    let c = v as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb_u8(v: f32) -> u8 {
    let c = v.clamp(0.0, 1.0);
    let s = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}

/// Box-filters RGBA8 by half in linear light with premultiplied alpha.
///
/// Straight-alpha averaging pulls edge colors toward transparent black; using
/// alpha-weighted linear averages keeps glyph and sprite edges from darkening
/// as they shrink.
pub(crate) fn downsample_rgba(width: u32, height: u32, data: &[u8]) -> (u32, u32, Vec<u8>) {
    let nw = (width / 2).max(1);
    let nh = (height / 2).max(1);
    let mut out = Vec::with_capacity((nw as usize) * (nh as usize) * 4);
    for y in 0..nh {
        let y0 = y * 2;
        let y1 = (y0 + 1).min(height.saturating_sub(1));
        for x in 0..nw {
            let x0 = x * 2;
            let x1 = (x0 + 1).min(width.saturating_sub(1));
            let mut r = 0.0;
            let mut g = 0.0;
            let mut b = 0.0;
            let mut a = 0.0;
            for (sx, sy) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
                let i = ((sy as usize) * (width as usize) + (sx as usize)) * 4;
                let alpha = data[i + 3] as f32 / 255.0;
                r += srgb_u8_to_linear(data[i]) * alpha;
                g += srgb_u8_to_linear(data[i + 1]) * alpha;
                b += srgb_u8_to_linear(data[i + 2]) * alpha;
                a += alpha;
            }
            let a_out = a / 4.0;
            let (r_out, g_out, b_out) = if a > 0.0 {
                (r / a, g / a, b / a)
            } else {
                (0.0, 0.0, 0.0)
            };
            out.push(linear_to_srgb_u8(r_out));
            out.push(linear_to_srgb_u8(g_out));
            out.push(linear_to_srgb_u8(b_out));
            out.push((a_out * 255.0 + 0.5).clamp(0.0, 255.0) as u8);
        }
    }
    (nw, nh, out)
}

/// Uploads RGBA8 data and its complete mip chain.
pub(crate) fn upload_rgba_levels(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    data: &[u8],
) {
    let levels = mip_level_count(width, height);
    let mut w = width;
    let mut h = height;
    let mut current: Option<Vec<u8>> = None;

    for level in 0..levels {
        let src: &[u8] = if level == 0 {
            data
        } else {
            current.as_deref().unwrap_or(data)
        };
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: level,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            src,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        if level + 1 < levels {
            let (nw, nh, next) = downsample_rgba(w, h, src);
            current = Some(next);
            w = nw;
            h = nh;
        }
    }
}

pub(crate) fn image_dims_allowed(width: u32, height: u32) -> bool {
    if width == 0 || height == 0 || width > MAX_IMAGE_DIM || height > MAX_IMAGE_DIM {
        return false;
    }
    match (width as usize)
        .checked_mul(height as usize)
        .and_then(|px| px.checked_mul(4))
    {
        Some(bytes) => bytes <= MAX_IMAGE_BYTES,
        None => false,
    }
}

pub(crate) fn pick_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .or_else(|| formats.first().copied())
}

fn is_linear_float(format: wgpu::TextureFormat) -> bool {
    matches!(
        format,
        wgpu::TextureFormat::Rgba16Float
            | wgpu::TextureFormat::Rgba32Float
            | wgpu::TextureFormat::Rgb9e5Ufloat
    )
}

/// Chooses the swapchain configuration and the format used for rendering.
///
/// Non-sRGB 8-bit formats get an sRGB view so the scene keeps blending in
/// linear light while the swapchain stores the requested format. Formats
/// without an sRGB counterpart are only accepted when they are already linear
/// (float targets); everything else falls back to the automatic sRGB pick.
pub(crate) fn resolve_surface_format(
    formats: &[wgpu::TextureFormat],
    requested: Option<wgpu::TextureFormat>,
) -> Option<ResolvedSurfaceFormat> {
    let automatic = || {
        pick_format(formats).map(|format| ResolvedSurfaceFormat {
            config: format,
            render: format,
            view_formats: Vec::new(),
        })
    };

    let Some(want) = requested else {
        return automatic();
    };
    if !formats.contains(&want) {
        log::warn!(
            "sequoia-render: surface format {:?} is unavailable on this surface - using the default",
            want
        );
        return automatic();
    }
    if want.is_srgb() {
        return Some(ResolvedSurfaceFormat {
            config: want,
            render: want,
            view_formats: Vec::new(),
        });
    }
    let srgb = want.add_srgb_suffix();
    if srgb != want {
        return Some(ResolvedSurfaceFormat {
            config: want,
            render: srgb,
            view_formats: vec![srgb],
        });
    }
    if is_linear_float(want) {
        return Some(ResolvedSurfaceFormat {
            config: want,
            render: want,
            view_formats: Vec::new(),
        });
    }
    log::warn!(
        "sequoia-render: surface format {:?} has no safe sRGB view - using the default",
        want
    );
    automatic()
}

/// Returns whether `format` can be used as a sampled render target.
pub(crate) fn texture_format_usable(device: &wgpu::Device, format: wgpu::TextureFormat) -> bool {
    if !format.has_color_aspect() || format.is_depth_stencil_format() || format.is_compressed() {
        return false;
    }
    let required = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
    if !format
        .guaranteed_format_features(device.features())
        .allowed_usages
        .contains(required)
    {
        return false;
    }
    matches!(
        format.sample_type(None, Some(device.features())),
        Some(wgpu::TextureSampleType::Float { filterable: true })
    )
}

/// Returns the first usable offscreen surface format for `device`.
pub(crate) fn default_surface_format(device: &wgpu::Device) -> wgpu::TextureFormat {
    [
        DEFAULT_SURFACE_FORMAT,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Bgra8Unorm,
    ]
    .into_iter()
    .find(|format| texture_format_usable(device, *format))
    .unwrap_or(DEFAULT_SURFACE_FORMAT)
}

/// Clamps a requested MSAA sample count to what `format` supports.
pub(crate) fn supported_samples(format: wgpu::TextureFormat, samples: u32) -> u32 {
    let samples = crate::surface::sanitize_samples(samples);
    if format
        .guaranteed_format_features(wgpu::Features::empty())
        .flags
        .sample_count_supported(samples)
    {
        samples
    } else {
        1
    }
}

pub(crate) fn instance_ceiling(device: &wgpu::Device, item_bytes: usize) -> usize {
    ceiling_for(device.limits().max_buffer_size, item_bytes)
}

pub(crate) fn ceiling_for(max_buffer_size: u64, item_bytes: usize) -> usize {
    (max_buffer_size as usize / item_bytes.max(1)).max(1)
}

pub(crate) fn clear_color_authored(color: Color) -> wgpu::Color {
    wgpu::Color {
        r: color.r as f64,
        g: color.g as f64,
        b: color.b as f64,
        a: color.a as f64,
    }
}

pub(crate) fn clear_color_linear(color: Color) -> wgpu::Color {
    clear_color_authored(color.to_linear())
}

/// Converts a logical rectangle to physical pixels, clipped to the framebuffer.
pub(crate) fn phys_rect(rect: Rect, scale: f32, fb_w: u32, fb_h: u32) -> Option<Rect> {
    let x0 = (rect.left() * scale).max(0.0);
    let y0 = (rect.top() * scale).max(0.0);
    let x1 = (rect.right() * scale).min(fb_w as f32);
    let y1 = (rect.bottom() * scale).min(fb_h as f32);
    let r = Rect::from_xywh(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0));
    if r.is_empty() { None } else { Some(r) }
}

/// Converts a physical damage rect into a wgpu scissor rectangle.
pub(crate) fn scissor_rect(rect: Rect, fb: (u32, u32)) -> Option<(u32, u32, u32, u32)> {
    let x0 = rect.left().floor().clamp(0.0, fb.0 as f32) as u32;
    let y0 = rect.top().floor().clamp(0.0, fb.1 as f32) as u32;
    let x1 = rect.right().ceil().clamp(0.0, fb.0 as f32) as u32;
    let y1 = rect.bottom().ceil().clamp(0.0, fb.1 as f32) as u32;
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1 - x0, y1 - y0))
}

/// Maximum number of disjoint scissor regions recorded for one frame.
pub(crate) const MAX_DAMAGE_REGIONS: usize = 8;

/// Clips logical damage regions to the framebuffer and converts them to
/// scissors, inflating each by `margin` logical pixels first.
///
/// More than [`MAX_DAMAGE_REGIONS`] inputs collapse into one bounding box.
/// Regions that clip away completely are dropped.
pub(crate) fn damage_scissors(
    regions: &[Rect],
    margin: f32,
    scale: f32,
    fb: (u32, u32),
) -> Vec<(Rect, (u32, u32, u32, u32))> {
    let margin = if margin.is_finite() {
        margin.max(0.0)
    } else {
        0.0
    };
    let mut out = Vec::new();
    if regions.len() > MAX_DAMAGE_REGIONS {
        let all = regions.iter().copied().fold(Rect::ZERO, Rect::union);
        push_damage_scissor(&mut out, all, margin, scale, fb);
    } else {
        for &region in regions {
            push_damage_scissor(&mut out, region, margin, scale, fb);
        }
    }
    out
}

fn push_damage_scissor(
    out: &mut Vec<(Rect, (u32, u32, u32, u32))>,
    region: Rect,
    margin: f32,
    scale: f32,
    fb: (u32, u32),
) {
    let Some(phys) = phys_rect(region.inflate(margin, margin), scale, fb.0, fb.1) else {
        return;
    };
    if let Some(scissor) = scissor_rect(phys, fb) {
        out.push((phys, scissor));
    }
}

pub(crate) fn inset_rect(r: Rect, pad: [f32; 4]) -> Rect {
    Rect::from_xywh(
        r.left() + pad[0],
        r.top() + pad[1],
        (r.size.width - pad[0] - pad[2]).max(0.0),
        (r.size.height - pad[1] - pad[3]).max(0.0),
    )
}

pub(crate) fn linear_rgba(color: Color) -> [f32; 4] {
    let c = color.to_linear();
    [c.r, c.g, c.b, c.a]
}

pub(crate) fn scale_inset(pad: [f32; 4], scale: f32) -> [f32; 4] {
    [
        pad[0] * scale,
        pad[1] * scale,
        pad[2] * scale,
        pad[3] * scale,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scissor_rounds_outward_and_clamps() {
        let rect = Rect::from_xywh(10.2, 20.7, 30.2, 40.1);
        assert_eq!(scissor_rect(rect, (100, 100)), Some((10, 20, 31, 41)));
    }

    #[test]
    fn scissor_clips_to_framebuffer() {
        let rect = Rect::from_xywh(-5.0, -5.0, 120.0, 120.0);
        assert_eq!(scissor_rect(rect, (50, 40)), Some((0, 0, 50, 40)));
    }

    #[test]
    fn scissor_rejects_empty_and_offscreen() {
        let empty = Rect::from_xywh(0.0, 0.0, 0.0, 10.0);
        assert_eq!(scissor_rect(empty, (50, 50)), None);
        let outside = Rect::from_xywh(60.0, 0.0, 10.0, 10.0);
        assert_eq!(scissor_rect(outside, (50, 50)), None);
    }
}
