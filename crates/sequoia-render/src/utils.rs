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

pub(crate) fn phys_rect(rect: Rect, scale: f32, fb_w: u32, fb_h: u32) -> Option<Rect> {
    let x0 = (rect.left() * scale).max(0.0);
    let y0 = (rect.top() * scale).max(0.0);
    let x1 = (rect.right() * scale).min(fb_w as f32);
    let y1 = (rect.bottom() * scale).min(fb_h as f32);
    let r = Rect::from_xywh(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0));
    if r.is_empty() { None } else { Some(r) }
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
