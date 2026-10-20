use std::rc::Rc;

use crate::geometry::Rect;
use crate::paint::{Color, ImageSource};

const BPP: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pixels {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Default for Pixels {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl Pixels {
    pub fn new(width: u32, height: u32) -> Self {
        let Some(px) = checked_pixels(width, height) else {
            return Self {
                width: 0,
                height: 0,
                data: Vec::new(),
            };
        };
        Self {
            width,
            height,
            data: vec![0; px * BPP],
        }
    }

    pub fn filled(width: u32, height: u32, color: Color) -> Self {
        let mut px = Self::new(width, height);
        px.fill(color);
        px
    }

    pub fn from_rgba(width: u32, height: u32, data: impl Into<Vec<u8>>) -> Option<Self> {
        let data = data.into();
        if width > MAX_DIM || height > MAX_DIM {
            return None;
        }
        let expected = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(BPP)?;
        if data.len() != expected {
            return None;
        }
        Some(Self {
            width,
            height,
            data,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn pixel_count(&self) -> usize {
        self.data.len() / BPP
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some((y as usize * self.width as usize + x as usize) * BPP)
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.index(x, y).is_some()
    }

    pub fn get(&self, x: i32, y: i32) -> Option<Color> {
        let i = self.index(x, y)?;
        Some(unpack(&self.data[i..i + BPP]))
    }

    pub fn set(&mut self, x: i32, y: i32, color: Color) -> bool {
        let Some(i) = self.index(x, y) else {
            return false;
        };
        self.data[i..i + BPP].copy_from_slice(&pack(color));
        true
    }

    pub fn blend(&mut self, x: i32, y: i32, color: Color) -> bool {
        let Some(i) = self.index(x, y) else {
            return false;
        };
        let out = blend_over(unpack(&self.data[i..i + BPP]), color);
        self.data[i..i + BPP].copy_from_slice(&out);
        true
    }

    pub fn fill(&mut self, color: Color) {
        let px = pack(color);
        for chunk in self.data.chunks_exact_mut(BPP) {
            chunk.copy_from_slice(&px);
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) -> Rect {
        let Some((x0, y0, x1, y1)) = self.rect_bounds(rect) else {
            return Rect::ZERO;
        };
        let px = pack(color);
        for y in y0..y1 {
            let row = y as usize * self.width as usize;
            for x in x0..x1 {
                let i = (row + x as usize) * BPP;
                self.data[i..i + BPP].copy_from_slice(&px);
            }
        }
        bounds_rect(x0, y0, x1, y1)
    }

    pub fn blend_rect(&mut self, rect: Rect, color: Color) -> Rect {
        let Some((x0, y0, x1, y1)) = self.rect_bounds(rect) else {
            return Rect::ZERO;
        };
        for y in y0..y1 {
            let row = y as usize * self.width as usize;
            for x in x0..x1 {
                let i = (row + x as usize) * BPP;
                let out = blend_over(unpack(&self.data[i..i + BPP]), color);
                self.data[i..i + BPP].copy_from_slice(&out);
            }
        }
        bounds_rect(x0, y0, x1, y1)
    }

    pub fn blit(&mut self, src: &Pixels, dx: i32, dy: i32) -> Rect {
        self.blit_impl(src, dx, dy, false)
    }

    pub fn blend_from(&mut self, src: &Pixels, dx: i32, dy: i32) -> Rect {
        self.blit_impl(src, dx, dy, true)
    }

    fn blit_impl(&mut self, src: &Pixels, dx: i32, dy: i32, blend: bool) -> Rect {
        if src.is_empty() || self.is_empty() {
            return Rect::ZERO;
        }
        let sx0 = (-dx).max(0);
        let sy0 = (-dy).max(0);
        let sx1 = (self.width as i32 - dx).min(src.width as i32);
        let sy1 = (self.height as i32 - dy).min(src.height as i32);
        if sx0 >= sx1 || sy0 >= sy1 {
            return Rect::ZERO;
        }
        for sy in sy0..sy1 {
            let dy_row = (dy + sy) as usize * self.width as usize;
            let sx_row = sy as usize * src.width as usize;
            for sx in sx0..sx1 {
                let si = (sx_row + sx as usize) * BPP;
                let di = (dy_row + (dx + sx) as usize) * BPP;
                if blend {
                    let out = blend_over(
                        unpack(&self.data[di..di + BPP]),
                        unpack(&src.data[si..si + BPP]),
                    );
                    self.data[di..di + BPP].copy_from_slice(&out);
                } else {
                    self.data[di..di + BPP].copy_from_slice(&src.data[si..si + BPP]);
                }
            }
        }
        bounds_rect(dx + sx0, dy + sy0, dx + sx1, dy + sy1)
    }

    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    pub fn crop(&self, rect: Rect) -> Pixels {
        let Some((x0, y0, x1, y1)) = self.rect_bounds(rect) else {
            return Pixels::new(0, 0);
        };
        let w = (x1 - x0) as usize;
        let h = (y1 - y0) as usize;
        let mut out = Vec::with_capacity(w * h * BPP);
        for y in y0..y1 {
            let start = (y as usize * self.width as usize + x0 as usize) * BPP;
            out.extend_from_slice(&self.data[start..start + w * BPP]);
        }
        Pixels {
            width: (x1 - x0) as u32,
            height: (y1 - y0) as u32,
            data: out,
        }
    }

    pub fn flip_horizontal(&mut self) {
        if self.is_empty() {
            return;
        }
        let w = self.width as usize;
        let row_bytes = w * BPP;
        for y in 0..self.height as usize {
            let row = &mut self.data[y * row_bytes..(y + 1) * row_bytes];
            for x in 0..w / 2 {
                let a = x * BPP;
                let b = (w - 1 - x) * BPP;
                for k in 0..BPP {
                    row.swap(a + k, b + k);
                }
            }
        }
    }

    pub fn flip_vertical(&mut self) {
        if self.is_empty() {
            return;
        }
        let row_bytes = self.width as usize * BPP;
        let h = self.height as usize;
        for y in 0..h / 2 {
            let (a, b) = (y * row_bytes, (h - 1 - y) * row_bytes);
            for k in 0..row_bytes {
                self.data.swap(a + k, b + k);
            }
        }
    }

    pub fn rotate180(&mut self) {
        self.flip_horizontal();
        self.flip_vertical();
    }

    pub fn resize_nearest(&self, width: u32, height: u32) -> Pixels {
        if checked_pixels(width, height).is_none() {
            return Pixels::new(0, 0);
        }
        let (w, h) = (width, height);
        if w == 0 || h == 0 || self.is_empty() {
            return Pixels::new(w, h);
        }
        let mut out = Pixels::new(w, h);
        let xr = self.width as f32 / w as f32;
        let yr = self.height as f32 / h as f32;
        for y in 0..h {
            let sy = (y as f32 * yr).floor() as i32;
            for x in 0..w {
                let sx = (x as f32 * xr).floor() as i32;
                if let (Some(si), Some(di)) = (self.index(sx, sy), out.index(x as i32, y as i32)) {
                    out.data[di..di + BPP].copy_from_slice(&self.data[si..si + BPP]);
                }
            }
        }
        out
    }

    pub fn resize_bilinear(&self, width: u32, height: u32) -> Pixels {
        if checked_pixels(width, height).is_none() {
            return Pixels::new(0, 0);
        }
        let (w, h) = (width, height);
        if w == 0 || h == 0 || self.is_empty() {
            return Pixels::new(w, h);
        }
        let mut out = Pixels::new(w, h);
        let sx = self.width as f32 / w as f32;
        let sy = self.height as f32 / h as f32;
        for y in 0..h {
            let fy = (y as f32 + 0.5) * sy - 0.5;
            for x in 0..w {
                let fx = (x as f32 + 0.5) * sx - 0.5;
                let c = self.sample_bilinear(fx, fy);
                if let Some(di) = out.index(x as i32, y as i32) {
                    out.data[di..di + BPP].copy_from_slice(&pack(c));
                }
            }
        }
        out
    }

    pub fn thumbnail(&self, max_dim: u32) -> Pixels {
        self.scaled_to_fit(max_dim).unwrap_or_else(|| self.clone())
    }

    pub fn scaled_to_fit(&self, max_dim: u32) -> Option<Pixels> {
        if self.is_empty() || max_dim == 0 {
            return None;
        }
        let longest = self.width.max(self.height);
        if longest <= max_dim {
            return None;
        }
        let scale = max_dim as f32 / longest as f32;
        let w = ((self.width as f32 * scale).round() as u32).max(1);
        let h = ((self.height as f32 * scale).round() as u32).max(1);
        Some(self.resize_bilinear(w, h))
    }

    pub fn sample_nearest(&self, fx: f32, fy: f32) -> Color {
        if self.is_empty() {
            return Color::TRANSPARENT;
        }
        let x = fx.floor().clamp(0.0, self.width as f32 - 1.0) as i32;
        let y = fy.floor().clamp(0.0, self.height as f32 - 1.0) as i32;
        self.get(x, y).unwrap_or(Color::TRANSPARENT)
    }

    pub fn sample_bilinear(&self, fx: f32, fy: f32) -> Color {
        if self.is_empty() {
            return Color::TRANSPARENT;
        }
        if !fx.is_finite() || !fy.is_finite() {
            return Color::TRANSPARENT;
        }
        let x0 = fx.floor();
        let y0 = fy.floor();
        let tx = (fx - x0).clamp(0.0, 1.0);
        let ty = (fy - y0).clamp(0.0, 1.0);
        let x0 = (x0 as i32).clamp(0, self.width as i32 - 1);
        let y0 = (y0 as i32).clamp(0, self.height as i32 - 1);
        let x1 = (x0 + 1).min(self.width as i32 - 1);
        let y1 = (y0 + 1).min(self.height as i32 - 1);
        let c00 = self.get(x0, y0).unwrap_or(Color::TRANSPARENT);
        let c10 = self.get(x1, y0).unwrap_or(Color::TRANSPARENT);
        let c01 = self.get(x0, y1).unwrap_or(Color::TRANSPARENT);
        let c11 = self.get(x1, y1).unwrap_or(Color::TRANSPARENT);
        let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let lerp = |a: Color, b: Color, t: f32| Color {
            r: mix(a.r, b.r, t),
            g: mix(a.g, b.g, t),
            b: mix(a.b, b.b, t),
            a: mix(a.a, b.a, t),
        };
        lerp(lerp(c00, c10, tx), lerp(c01, c11, tx), ty)
    }

    pub fn for_each_pixel(&mut self, mut f: impl FnMut(i32, i32, Color) -> Color) {
        let w = self.width as usize;
        for y in 0..self.height as usize {
            for x in 0..w {
                let i = (y * w + x) * BPP;
                let c = unpack(&self.data[i..i + BPP]);
                self.data[i..i + BPP].copy_from_slice(&pack(f(x as i32, y as i32, c)));
            }
        }
    }

    pub fn map(&self, mut f: impl FnMut(i32, i32, Color) -> Color) -> Pixels {
        let mut out = self.clone();
        out.for_each_pixel(&mut f);
        out
    }

    pub fn grayscale(&self) -> Pixels {
        self.map(|_, _, c| {
            let l = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
            Color::rgba(l, l, l, c.a)
        })
    }

    pub fn inverted(&self) -> Pixels {
        self.map(|_, _, c| Color::rgba(1.0 - c.r, 1.0 - c.g, 1.0 - c.b, c.a))
    }

    pub fn tinted(&self, color: Color) -> Pixels {
        self.map(|_, _, c| Color::rgba(c.r * color.r, c.g * color.g, c.b * color.b, c.a * color.a))
    }

    pub fn multiply_alpha(&self, factor: f32) -> Pixels {
        let f = if factor.is_finite() {
            factor.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.map(move |_, _, c| Color { a: c.a * f, ..c })
    }

    pub fn has_alpha(&self) -> bool {
        self.data.chunks_exact(BPP).any(|p| p[3] != 255)
    }

    pub fn opaque_bounds(&self) -> Option<Rect> {
        if self.is_empty() {
            return None;
        }
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0u32;
        let mut max_y = 0u32;
        for y in 0..self.height {
            for x in 0..self.width {
                let i = ((y as usize * self.width as usize) + x as usize) * BPP;
                if self.data[i + 3] != 0 {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
        if min_x > max_x || min_y > max_y {
            return None;
        }
        Some(Rect::from_xywh(
            min_x as f32,
            min_y as f32,
            (max_x - min_x + 1) as f32,
            (max_y - min_y + 1) as f32,
        ))
    }

    pub fn to_source(&self, id: u64, version: u64) -> ImageSource {
        ImageSource::Dynamic {
            id,
            width: self.width,
            height: self.height,
            data: Rc::from(self.data.clone().into_boxed_slice()),
            version,
        }
    }

    pub fn into_source(self, id: u64, version: u64) -> ImageSource {
        ImageSource::Dynamic {
            id,
            width: self.width,
            height: self.height,
            data: Rc::from(self.data.into_boxed_slice()),
            version,
        }
    }

    fn rect_bounds(&self, rect: Rect) -> Option<(i32, i32, i32, i32)> {
        if self.is_empty() || rect.is_empty() {
            return None;
        }
        let x0 = rect.left().round() as i32;
        let y0 = rect.top().round() as i32;
        let x1 = rect.right().round() as i32;
        let y1 = rect.bottom().round() as i32;
        let x0 = x0.max(0);
        let y0 = y0.max(0);
        let x1 = x1.min(self.width as i32);
        let y1 = y1.min(self.height as i32);
        if x0 >= x1 || y0 >= y1 {
            return None;
        }
        Some((x0, y0, x1, y1))
    }
}

pub const MAX_DIM: u32 = 16384;

fn checked_pixels(width: u32, height: u32) -> Option<usize> {
    if width > MAX_DIM || height > MAX_DIM {
        return None;
    }
    (width as usize).checked_mul(height as usize)
}

fn bounds_rect(x0: i32, y0: i32, x1: i32, y1: i32) -> Rect {
    if x0 >= x1 || y0 >= y1 {
        return Rect::ZERO;
    }
    Rect::from_xywh(x0 as f32, y0 as f32, (x1 - x0) as f32, (y1 - y0) as f32)
}

fn pack(color: Color) -> [u8; BPP] {
    let q = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    [q(color.r), q(color.g), q(color.b), q(color.a)]
}

fn unpack(bytes: &[u8]) -> Color {
    Color::rgba(
        bytes[0] as f32 / 255.0,
        bytes[1] as f32 / 255.0,
        bytes[2] as f32 / 255.0,
        bytes[3] as f32 / 255.0,
    )
}

fn blend_over(dst: Color, src: Color) -> [u8; BPP] {
    if src.a <= 0.0 {
        return pack(dst);
    }
    if src.a >= 1.0 {
        return pack(src);
    }
    let inv = 1.0 - src.a;
    let out_a = src.a + dst.a * inv;
    if out_a <= 0.0 {
        return pack(Color::TRANSPARENT);
    }
    let mix = |s: f32, d: f32| (s * src.a + d * dst.a * inv) / out_a;
    pack(Color::rgba(
        mix(src.r, dst.r),
        mix(src.g, dst.g),
        mix(src.b, dst.b),
        out_a,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red() -> Color {
        Color::rgb8(255, 0, 0)
    }

    #[test]
    fn new_allocates_zeroed_rgba() {
        let px = Pixels::new(3, 2);
        assert_eq!((px.width(), px.height()), (3, 2));
        assert_eq!(px.data().len(), 3 * 2 * 4);
        assert!(px.data().iter().all(|&b| b == 0));
        assert!(px.has_alpha());
        assert_eq!(px.pixel_count(), 6);
    }

    #[test]
    fn oversized_request_is_empty_not_panic() {
        let px = Pixels::new(u32::MAX, 2);
        assert!(px.is_empty());
        assert_eq!(px.data().len(), 0);
        assert_eq!(Pixels::new(MAX_DIM + 1, 1).width(), 0);
    }

    #[test]
    fn from_rgba_validates_length() {
        assert!(Pixels::from_rgba(2, 2, vec![0u8; 16]).is_some());
        assert!(Pixels::from_rgba(2, 2, vec![0u8; 15]).is_none());
        assert!(Pixels::from_rgba(0, 5, Vec::new()).is_some());
        assert!(Pixels::from_rgba(MAX_DIM + 1, 1, vec![0u8; 4]).is_none());
    }

    #[test]
    fn set_get_and_bounds() {
        let mut px = Pixels::new(2, 2);
        assert!(px.set(1, 1, red()));
        assert_eq!(px.get(1, 1), Some(red()));
        assert!(!px.set(2, 0, red()), "out of bounds");
        assert_eq!(px.get(-1, 0), None);
        assert!(px.contains(0, 1));
        assert!(!px.contains(2, 0));
    }

    #[test]
    fn blend_is_source_over() {
        let mut px = Pixels::new(1, 1);
        px.set(0, 0, Color::rgb8(0, 0, 255));
        px.blend(0, 0, Color::rgba(1.0, 0.0, 0.0, 0.5));
        let c = px.get(0, 0).unwrap();
        assert!((c.r - 0.5).abs() < 0.02, "{c:?}");
        assert!((c.b - 0.5).abs() < 0.02, "{c:?}");
        assert!((c.a - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fill_and_fill_rect_clip_to_bounds() {
        let mut px = Pixels::new(4, 4);
        assert_eq!(
            px.fill_rect(Rect::from_xywh(1.0, 1.0, 2.0, 2.0), red()),
            Rect::from_xywh(1.0, 1.0, 2.0, 2.0)
        );
        assert_eq!(px.get(1, 1), Some(red()));
        assert_eq!(px.get(2, 2), Some(red()));
        assert_eq!(px.get(0, 0), Some(Color::TRANSPARENT));

        let clipped = px.fill_rect(Rect::from_xywh(3.0, 3.0, 5.0, 5.0), red());
        assert_eq!(clipped, Rect::from_xywh(3.0, 3.0, 1.0, 1.0));
        px.fill(Color::GREEN);
        assert_eq!(px.get(0, 0), Some(Color::GREEN));
    }

    #[test]
    fn blit_and_blend_from_copy_region() {
        let mut src = Pixels::filled(2, 2, red());
        src.set(0, 0, Color::TRANSPARENT);
        let mut dst = Pixels::filled(4, 4, Color::rgb8(0, 0, 255));
        let r = dst.blit(&src, 1, 1);
        assert_eq!(r, Rect::from_xywh(1.0, 1.0, 2.0, 2.0));
        assert_eq!(dst.get(1, 1), Some(Color::TRANSPARENT), "blit overwrites");
        assert_eq!(dst.get(2, 2), Some(red()));

        let mut dst2 = Pixels::filled(4, 4, Color::rgb8(0, 0, 255));
        dst2.blend_from(&src, 1, 1);
        assert_eq!(
            dst2.get(1, 1),
            Some(Color::rgb8(0, 0, 255)),
            "alpha 0 keeps dst"
        );
    }

    #[test]
    fn crop_clips_to_image() {
        let mut px = Pixels::filled(4, 3, red());
        px.set(0, 0, Color::GREEN);
        let c = px.crop(Rect::from_xywh(0.0, 0.0, 2.0, 2.0));
        assert_eq!((c.width(), c.height()), (2, 2));
        assert_eq!(c.get(0, 0), Some(Color::GREEN));
        assert_eq!(c.get(1, 1), Some(red()));
        assert!(px.crop(Rect::from_xywh(10.0, 10.0, 5.0, 5.0)).is_empty());
    }

    #[test]
    fn flips_and_rotate_match_mirror() {
        let mut px = Pixels::new(3, 1);
        px.set(0, 0, red());
        px.flip_horizontal();
        assert_eq!(px.get(2, 0), Some(red()));
        px.flip_vertical();
        assert_eq!(px.get(2, 0), Some(red()));
        px.rotate180();
        assert_eq!(px.get(0, 0), Some(red()));
    }

    #[test]
    fn thumbnail_only_scales_down_and_keeps_aspect() {
        let px = Pixels::filled(200, 100, red());
        let t = px.thumbnail(50);
        assert_eq!((t.width(), t.height()), (50, 25));
        assert_eq!(px.thumbnail(500).width(), 200, "small image untouched");
        assert!(px.scaled_to_fit(500).is_none());
        let up = px.resize_nearest(400, 200);
        assert_eq!((up.width(), up.height()), (400, 200));
    }

    #[test]
    fn nearest_resize_preserves_hard_edges() {
        let mut px = Pixels::new(2, 2);
        px.set(0, 0, red());
        let up = px.resize_nearest(4, 4);
        assert_eq!(up.get(1, 1), Some(red()));
        assert_eq!(up.get(0, 0), Some(red()));
        assert_eq!(up.get(3, 3), Some(Color::TRANSPARENT));
    }

    #[test]
    fn bilinear_sample_averages_neighbours() {
        let mut px = Pixels::new(2, 1);
        px.set(0, 0, Color::BLACK);
        px.set(1, 0, Color::WHITE);
        let mid = px.sample_bilinear(0.5, 0.0);
        assert!((mid.r - 0.5).abs() < 1e-6, "{mid:?}");
        assert_eq!(px.sample_nearest(0.0, 0.0), Color::BLACK);
    }

    #[test]
    fn color_transforms_keep_alpha() {
        let px = Pixels::filled(2, 2, Color::rgba(1.0, 0.0, 0.0, 0.5));
        let g = px.grayscale();
        let c = g.get(0, 0).unwrap();
        assert!((c.r - c.g).abs() < 1e-6 && (c.g - c.b).abs() < 1e-6);
        assert!((c.a - 0.5).abs() < 0.01);
        let inv = px.inverted();
        assert!((inv.get(1, 1).unwrap().r - 0.0).abs() < 1e-6);
        let tint = Pixels::filled(1, 1, Color::WHITE).tinted(Color::rgba(1.0, 0.0, 0.0, 1.0));
        assert_eq!(tint.get(0, 0), Some(red()));
        let faded = Pixels::filled(1, 1, Color::WHITE).multiply_alpha(0.25);
        assert!((faded.get(0, 0).unwrap().a - 0.25).abs() < 0.01);
    }

    #[test]
    fn opaque_bounds_tightly_wraps_content() {
        let mut px = Pixels::new(10, 10);
        px.set(3, 4, red());
        px.set(5, 6, red());
        assert_eq!(
            px.opaque_bounds(),
            Some(Rect::from_xywh(3.0, 4.0, 3.0, 3.0))
        );
        assert_eq!(Pixels::new(2, 2).opaque_bounds(), None);
    }

    #[test]
    fn source_conversions_carry_dimensions() {
        let px = Pixels::filled(3, 2, red());
        assert_eq!(px.to_source(7, 1).extent(), Some((3, 2)));
        assert_eq!(px.clone().into_source(7, 1).dynamic_version(), Some(1));
        assert_eq!(px.to_source(9, 1).cache_key(), 9);
    }

    #[test]
    fn map_applies_pixel_fn() {
        let px = Pixels::filled(2, 2, Color::rgb8(10, 20, 30));
        let out = px.map(|_, _, c| Color::rgba(c.r, c.g, c.b, 0.5));
        assert!((out.get(1, 1).unwrap().a - 0.5).abs() < 0.01);
        let mut inplace = px.clone();
        inplace.for_each_pixel(|_, _, _| Color::BLACK);
        assert_eq!(inplace.get(0, 0), Some(Color::BLACK));
    }
}
