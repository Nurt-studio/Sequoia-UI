#![forbid(unsafe_code)]

use sequoia_core::{Color, Quad, Rect};

mod cache;
mod flatten;
mod gradient;
mod path;
mod raster;
mod stroke;
mod svg_path;
mod transform;
mod waveform;

pub use cache::{DEFAULT_CAPACITY, Shape, ShapeCache, ShapeStats};
pub use flatten::{Contour, flatten};
pub use gradient::{Gradient, GradientStop};
pub use path::{FillRule, Path, PathError, PathSeg, Radius};
pub use raster::{FillStyle, RasterOptions, fill, fill_contours, fill_styled};
pub use stroke::{Dash, LineCap, LineJoin, StrokeStyle, stroke, stroke_path};
pub use transform::Transform;
pub use waveform::{
    MinMax, Peaks, WaveformCache, WaveformOpts, bake_strip, layout_bars, waveform_line_points,
    white_noise,
};

pub const TOLERANCE: f32 = 0.25;

pub const DEFAULT_OPTIONS: RasterOptions = RasterOptions {
    tolerance: TOLERANCE,
    row_height: 1.0,
    merge_tol: 0.35,
    aa_samples: 4,
    analytic: true,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub rect: Rect,

    pub radius: f32,

    pub alpha: f32,
}

impl Span {
    pub fn rect(rect: Rect) -> Self {
        Self {
            rect,
            radius: 0.0,
            alpha: 1.0,
        }
    }

    pub fn rounded(rect: Rect, radius: f32) -> Self {
        Self {
            rect,
            radius,
            alpha: 1.0,
        }
    }

    pub fn solid(rect: Rect) -> Self {
        Self::rect(rect)
    }

    pub fn edge(rect: Rect, coverage: f32) -> Self {
        Self::rect(rect).with_alpha(coverage)
    }

    #[must_use]
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    pub fn quad(self, color: Color) -> Quad {
        Quad {
            rect: self.rect,
            color: Color {
                a: color.a * self.alpha,
                ..color
            },
            radius: self.radius,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
        }
    }

    pub fn is_visible(&self) -> bool {
        !self.rect.is_empty() && self.alpha > 0.0
    }
}

pub(crate) fn union_rect(a: Rect, b: Rect) -> Rect {
    if a.is_empty() {
        return b;
    }
    if b.is_empty() {
        return a;
    }
    let x0 = a.left().min(b.left());
    let y0 = a.top().min(b.top());
    let x1 = a.right().max(b.right());
    let y1 = a.bottom().max(b.bottom());
    Rect::from_xywh(x0, y0, x1 - x0, y1 - y0)
}
