use std::rc::Rc;

use sequoia_core::{Color, ImageSource, Point, Rect, Shadow, Vec2};
use sequoia_draw::{
    Dash, FillRule, Gradient, GradientStop, MinMax, Path, Peaks, Radius, RasterOptions, Shape,
    ShapeCache, StrokeStyle, Transform, WaveformOpts, bake_strip, layout_bars,
    waveform_line_points,
};

use crate::track::Orientation;
use crate::view::PaintCtx;

fn line_style(width: f32) -> StrokeStyle {
    StrokeStyle::new(width)
}

pub const GRADIENT_BAND_PX: f32 = 1.0;

pub(crate) const GRADIENT_BANDS_MAX: usize = 256;

pub(crate) fn gradient_band_count(extent: f32, band: f32) -> usize {
    let band = if band.is_finite() && band > 0.0 {
        band
    } else {
        GRADIENT_BAND_PX
    };
    ((extent / band).ceil().max(1.0) as usize).clamp(1, GRADIENT_BANDS_MAX)
}

pub(crate) fn band_edges(
    origin: f32,
    extent: f32,
    count: usize,
    snap: f32,
    over: f32,
) -> Vec<(f32, f32)> {
    let count = count.max(1);
    let snap = if snap.is_finite() && snap > 0.0 {
        snap
    } else {
        1.0
    };
    let over = if over.is_finite() && over > 0.0 {
        over
    } else {
        0.0
    };
    (0..count)
        .map(|i| {
            let last = i + 1 == count;
            let lo = if i == 0 {
                origin
            } else {
                ((origin + extent * i as f32 / count as f32) / snap).round() * snap
            };
            let mut hi = if last {
                origin + extent
            } else {
                ((origin + extent * (i + 1) as f32 / count as f32) / snap).round() * snap + over
            };
            if hi < lo + 0.01 {
                hi = lo + 0.01;
            }
            (lo, hi)
        })
        .collect()
}

impl PaintCtx<'_> {
    pub fn shapes(&self) -> &ShapeCache {
        self.shapes
    }

    pub fn raster_options(&self) -> RasterOptions {
        RasterOptions::default().for_scale(self.scale_factor())
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.draw_offset.x += dx;
        self.draw_offset.y += dy;
    }

    pub fn set_draw_offset(&mut self, offset: impl Into<Point>) {
        let p = offset.into();
        self.draw_offset = Vec2::new(p.x, p.y);
    }

    pub fn draw_offset(&self) -> Vec2 {
        self.draw_offset
    }

    pub fn reset_draw_offset(&mut self) {
        self.draw_offset = Vec2::ZERO;
    }

    pub fn save(&mut self) {
        self.offset_stack.push(self.draw_offset);
    }

    pub fn restore(&mut self) {
        if let Some(prev) = self.offset_stack.pop() {
            self.draw_offset = prev;
        }
    }

    pub fn path_from_svg(&self, data: &str) -> Option<Path> {
        match Path::from_svg(data) {
            Ok(path) => Some(path),
            Err(err) => {
                log::warn!("invalid SVG path: {err}");
                None
            }
        }
    }

    pub fn rect(&mut self, rect: Rect, color: Color) {
        self.quad(rect, color, 0.0);
    }

    pub fn rounded(&mut self, rect: Rect, radius: f32, color: Color) {
        self.quad(rect, color, radius.max(0.0));
    }

    pub fn circle(&mut self, center: Point, radius: f32, color: Color) -> Rect {
        self.disk(center, radius, color)
    }

    pub fn fill_gradient_rect(
        &mut self,
        rect: Rect,
        stops: &[GradientStop],
        orientation: Orientation,
    ) -> Rect {
        if rect.is_empty() || stops.is_empty() {
            return Rect::ZERO;
        }
        let rect = self.shifted_rect(rect);
        if rect.is_empty() {
            return Rect::ZERO;
        }
        let gradient = match orientation {
            Orientation::Horizontal => {
                Gradient::linear((rect.left(), 0.0), (rect.right(), 0.0), stops.to_vec())
            }
            Orientation::Vertical => {
                Gradient::linear((0.0, rect.top()), (0.0, rect.bottom()), stops.to_vec())
            }
        };
        let mut bounds = Rect::ZERO;

        let over = self.band_px(0.5);
        match orientation {
            Orientation::Horizontal => {
                let mut x = rect.left();
                while x < rect.right() {
                    let nx = (x + 1.0).min(rect.right());
                    let extra = if nx < rect.right() { over } else { 0.0 };
                    let c = gradient.color_at(Point::new((x + nx) * 0.5, rect.top()));
                    let band = Rect::from_xywh(x, rect.top(), nx - x + extra, rect.size.height);
                    self.list.push_quad(band, c, 0.0);
                    bounds = bounds.union(band);
                    x = nx;
                }
            }
            Orientation::Vertical => {
                let mut y = rect.top();
                while y < rect.bottom() {
                    let ny = (y + 1.0).min(rect.bottom());
                    let extra = if ny < rect.bottom() { over } else { 0.0 };
                    let c = gradient.color_at(Point::new(rect.left(), (y + ny) * 0.5));
                    let band = Rect::from_xywh(rect.left(), y, rect.size.width, ny - y + extra);
                    self.list.push_quad(band, c, 0.0);
                    bounds = bounds.union(band);
                    y = ny;
                }
            }
        }
        bounds
    }

    pub fn knob(&mut self, center: Point, radius: f32, fill: Color) -> Rect {
        if !(radius > 0.0) || !radius.is_finite() {
            return Rect::ZERO;
        }
        let outer = self.disk(center, radius, Color::WHITE);
        let inner_r = radius - 2.0;
        if inner_r > 0.0 {
            let inner = self.disk(center, inner_r, fill);
            outer.union(inner)
        } else {
            outer
        }
    }

    pub fn sv_field(&mut self, rect: Rect, hue01: f32) -> Rect {
        if rect.is_empty() {
            return Rect::ZERO;
        }
        let h = hue01.clamp(0.0, 1.0);
        self.fill_rect(rect, Color::from_hsva(h, 1.0, 1.0, 1.0));
        self.fill_gradient_rect(
            rect,
            &[
                GradientStop::new(0.0, Color::WHITE),
                GradientStop::new(1.0, Color::rgba(1.0, 1.0, 1.0, 0.0)),
            ],
            Orientation::Horizontal,
        );
        self.fill_gradient_rect(
            rect,
            &[
                GradientStop::new(0.0, Color::rgba(0.0, 0.0, 0.0, 0.0)),
                GradientStop::new(1.0, Color::BLACK),
            ],
            Orientation::Vertical,
        );
        self.shifted_rect(rect)
    }

    pub fn path(&mut self, path: &Path, width: f32, color: Color) -> Rect {
        self.stroke(path, &line_style(width.max(0.0)), color)
    }

    pub fn text(&mut self, text: &str, size: f32, color: Color) {
        let origin = (self.rect.origin.x, self.rect.origin.y);
        self.draw_text(text, origin, size.max(0.0), color);
    }

    pub fn fill(&mut self, path: &Path, color: Color) -> Rect {
        self.fill_rule(path, FillRule::NonZero, color)
    }

    pub fn fill_rule(&mut self, path: &Path, rule: FillRule, color: Color) -> Rect {
        let opts = self.raster_options();
        let shape = self.shapes.fill(path, rule, opts);
        self.draw_shape(&shape, Point::ZERO, color)
    }

    pub fn stroke(&mut self, path: &Path, style: &StrokeStyle, color: Color) -> Rect {
        let opts = self.raster_options();
        let shape = self.shapes.stroke(path, style, opts);
        self.draw_shape(&shape, Point::ZERO, color)
    }

    pub fn shape(&mut self, shape: &Shape, color: Color) -> Rect {
        self.draw_shape(shape, Point::ZERO, color)
    }

    pub fn draw_shape(&mut self, shape: &Shape, at: Point, color: Color) -> Rect {
        let offset = Vec2::new(self.draw_offset.x + at.x, self.draw_offset.y + at.y);
        let mut bounds = Rect::ZERO;
        for span in shape.iter() {
            let rect = if offset.x == 0.0 && offset.y == 0.0 {
                span.rect
            } else {
                Rect::from_xywh(
                    span.rect.origin.x + offset.x,
                    span.rect.origin.y + offset.y,
                    span.rect.size.width,
                    span.rect.size.height,
                )
            };
            let color = Color {
                a: color.a * span.alpha,
                ..color
            };
            self.list.push_quad(rect, color, span.radius);
            bounds = bounds.union(rect);
        }
        bounds
    }

    pub fn fill_gradient(&mut self, path: &Path, gradient: &Gradient) -> Rect {
        self.fill_gradient_rule(path, FillRule::NonZero, gradient)
    }

    pub fn fill_gradient_rule(&mut self, path: &Path, rule: FillRule, gradient: &Gradient) -> Rect {
        let opts = self.raster_options().analytic(false);
        let shape = self.shapes.fill(path, rule, opts);
        let band = self.band_px(GRADIENT_BAND_PX);
        self.draw_shape_gradient(&shape, Point::ZERO, gradient, band)
    }

    pub fn rounded_rect_gradient(&mut self, rect: Rect, radius: f32, gradient: &Gradient) -> Rect {
        if rect.is_empty() {
            return Rect::ZERO;
        }
        let mut path = Path::new();
        path.rounded_rect(rect, radius);
        self.fill_gradient(&path, gradient)
    }

    pub fn stroke_gradient(
        &mut self,
        path: &Path,
        style: &StrokeStyle,
        gradient: &Gradient,
    ) -> Rect {
        let opts = self.raster_options().analytic(false);
        let shape = self.shapes.stroke(path, style, opts);
        let band = self.band_px(GRADIENT_BAND_PX);
        self.draw_shape_gradient(&shape, Point::ZERO, gradient, band)
    }

    pub fn shadow(
        &mut self,
        rect: Rect,
        radius: f32,
        blur: f32,
        offset: Point,
        color: Color,
    ) -> Rect {
        let base = shift(rect, self.draw_offset);
        if base.is_empty() || color.a <= 0.0 || !(blur > 0.0) {
            return Rect::ZERO;
        }
        let shadow = Shadow {
            rect: base,
            color,
            radius: shadow_radius(radius, base),
            blur,
            spread: 0.0,
            offset: Vec2::new(offset.x, offset.y),
            inset: false,
        };
        let bounds = shadow.bounds();
        self.list.push_shadow(shadow);
        bounds
    }

    pub fn inner_shadow(
        &mut self,
        rect: Rect,
        radius: f32,
        blur: f32,
        offset: Point,
        color: Color,
    ) -> Rect {
        let base = shift(rect, self.draw_offset);
        if base.is_empty() || color.a <= 0.0 || !(blur > 0.0) {
            return Rect::ZERO;
        }
        self.list.push_shadow(Shadow {
            rect: base,
            color,
            radius: shadow_radius(radius, base),
            blur,
            spread: 0.0,
            offset: Vec2::new(offset.x, offset.y),
            inset: true,
        });
        base
    }

    pub fn shape_gradient_bands(
        &self,
        shape: &Shape,
        at: Point,
        gradient: &Gradient,
        band: f32,
    ) -> Vec<(Rect, Color, f32)> {
        let offset = Vec2::new(self.draw_offset.x + at.x, self.draw_offset.y + at.y);
        let band = if band > 0.0 {
            band
        } else {
            self.band_px(GRADIENT_BAND_PX)
        };
        let band = band.max(0.05);
        let snap = self.band_px(1.0);
        let vertical = gradient.is_vertical();
        let mut out = Vec::new();
        for span in shape.iter() {
            let rect = shift(span.rect, offset);
            if rect.is_empty() || span.alpha <= 0.0 {
                continue;
            }
            if span.radius > 0.0 {
                let center = Point::new(
                    rect.left() + rect.size.width * 0.5,
                    rect.top() + rect.size.height * 0.5,
                );
                let color = gradient.color_at(center);
                out.push((rect, tint(color, span.alpha), span.radius));
                continue;
            }
            let origin = if vertical { rect.top() } else { rect.left() };
            let extent = if vertical {
                rect.size.height
            } else {
                rect.size.width
            };
            let count = gradient_band_count(extent, band);

            for (lo, hi) in band_edges(origin, extent, count, snap, snap * 0.5) {
                if hi <= lo {
                    continue;
                }
                let sub = if vertical {
                    Rect::from_xywh(rect.left(), lo, rect.size.width, hi - lo)
                } else {
                    Rect::from_xywh(lo, rect.top(), hi - lo, rect.size.height)
                };
                let color = gradient.color_at(Point::new(
                    sub.left() + sub.size.width * 0.5,
                    sub.top() + sub.size.height * 0.5,
                ));
                out.push((sub, tint(color, span.alpha), 0.0));
            }
        }
        out
    }

    pub fn push_gradient_bands(&mut self, bands: &[(Rect, Color, f32)]) -> Rect {
        let mut bounds = Rect::ZERO;
        for (rect, color, radius) in bands {
            if rect.is_empty() {
                continue;
            }
            self.list.push_quad(*rect, *color, *radius);
            bounds = bounds.union(*rect);
        }
        bounds
    }

    pub fn draw_shape_gradient(
        &mut self,
        shape: &Shape,
        at: Point,
        gradient: &Gradient,
        band: f32,
    ) -> Rect {
        let bands = self.shape_gradient_bands(shape, at, gradient, band);
        self.push_gradient_bands(&bands)
    }

    pub fn shape_gradient(&mut self, shape: &Shape, gradient: &Gradient) -> Rect {
        let band = self.band_px(GRADIENT_BAND_PX);
        self.draw_shape_gradient(shape, Point::ZERO, gradient, band)
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) -> Rect {
        let rect = self.shifted_rect(rect);
        self.list.push_quad(rect, color, 0.0);
        rect
    }

    pub fn bars(&mut self, bars: &[Rect], color: Color) -> Rect {
        if bars.is_empty() || color.a <= 0.0 {
            return Rect::ZERO;
        }
        let off = self.draw_offset;
        let mut bounds = Rect::ZERO;
        if off.x == 0.0 && off.y == 0.0 {
            self.list.push_waveform(bars, color);
            for r in bars {
                bounds = bounds.union(*r);
            }
        } else {
            let shifted: Vec<Rect> = bars
                .iter()
                .map(|r| {
                    Rect::from_xywh(
                        r.left() + off.x,
                        r.top() + off.y,
                        r.size.width,
                        r.size.height,
                    )
                })
                .collect();
            self.list.push_waveform(&shifted, color);
            for r in &shifted {
                bounds = bounds.union(*r);
            }
        }
        bounds
    }

    pub fn waveform(
        &mut self,
        area: Rect,
        peaks: &[MinMax],
        opts: WaveformOpts,
        color: Color,
    ) -> Rect {
        if area.is_empty() || peaks.is_empty() || color.a <= 0.0 {
            return Rect::ZERO;
        }
        let area = self.shifted_rect(area);
        let bars = layout_bars(area, peaks, opts);
        if bars.is_empty() {
            return Rect::ZERO;
        }
        self.list.push_waveform(&bars, color);
        area
    }

    pub fn waveform_window(
        &mut self,
        area: Rect,
        peaks: &Peaks,
        start_peak: usize,
        count: usize,
        opts: WaveformOpts,
        color: Color,
    ) -> (Rect, usize) {
        if area.is_empty() || peaks.is_empty() || color.a <= 0.0 {
            return (Rect::ZERO, 0);
        }
        let width = (area.size.width.ceil() as usize).clamp(1, 8192);
        let (level, cols) = peaks.query_for_width(start_peak, count, width);
        if cols.is_empty() {
            return (Rect::ZERO, level);
        }
        let bounds = self.waveform(area, &cols, opts, color);
        (bounds, level)
    }

    pub fn waveform_line(
        &mut self,
        area: Rect,
        peaks: &[MinMax],
        width: f32,
        color: Color,
    ) -> Rect {
        if area.is_empty() || peaks.is_empty() || color.a <= 0.0 {
            return Rect::ZERO;
        }
        let pts = waveform_line_points(area, peaks);
        if pts.len() < 2 {
            return Rect::ZERO;
        }
        let mut path = Path::new();
        path.polyline(&pts);
        let opts = self.raster_options();
        let shape = self
            .shapes
            .stroke(&path, &StrokeStyle::round(width.clamp(0.5, 8.0)), opts);
        let off = self.draw_offset;
        let mut rects = Vec::with_capacity(shape.len().min(16384));
        let mut bounds = Rect::ZERO;
        for span in shape.iter() {
            let r = span.rect;
            let r = if off.x == 0.0 && off.y == 0.0 {
                r
            } else {
                Rect::from_xywh(
                    r.left() + off.x,
                    r.top() + off.y,
                    r.size.width,
                    r.size.height,
                )
            };
            if r.is_empty() {
                continue;
            }
            bounds = bounds.union(r);
            rects.push(r);
            if rects.len() >= 16384 {
                break;
            }
        }
        if rects.is_empty() {
            return Rect::ZERO;
        }
        self.list.push_waveform(&rects, color);
        bounds
    }

    pub fn waveform_image(
        &mut self,
        area: Rect,
        peaks: &[MinMax],
        fg: Color,
        bg: Color,
        id: u64,
        version: u64,
    ) -> Rect {
        if area.is_empty() || peaks.is_empty() {
            return Rect::ZERO;
        }
        let w = (area.size.width.ceil() as u32).clamp(8, 4096);
        let h = (area.size.height.ceil() as u32).clamp(8, 1024);
        let cols: Vec<MinMax> = if peaks.len() == w as usize {
            peaks.to_vec()
        } else {
            let n = peaks.len();
            let mut out = Vec::with_capacity(w as usize);
            for i in 0..(w as usize) {
                let a = (i * n) / (w as usize);
                let mut b = ((i + 1) * n) / (w as usize);
                if b <= a {
                    b = (a + 1).min(n);
                }
                let mut m = peaks[a];
                for &v in &peaks[a + 1..b] {
                    m = m.merge(v);
                }
                out.push(m);
            }
            out
        };
        let data = bake_strip(&cols, w, h, fg, bg);
        if data.is_empty() {
            return Rect::ZERO;
        }
        let area = self.shifted_rect(area);
        self.list.push_image(
            area,
            [0.0, 0.0, 1.0, 1.0],
            ImageSource::Dynamic {
                id,
                width: w,
                height: h,
                data: Rc::from(data.into_boxed_slice()),
                version,
            },
        );
        area
    }

    pub fn disk(&mut self, center: Point, radius: f32, color: Color) -> Rect {
        if !(radius > 0.0) {
            return Rect::ZERO;
        }
        let shape = self.shapes.disk(center, radius);
        self.draw_shape(&shape, Point::ZERO, color)
    }

    pub fn rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) -> Rect {
        let shape = self.shapes.rounded_rect(rect, radius);
        self.draw_shape(&shape, Point::ZERO, color)
    }

    pub fn ellipse(&mut self, center: Point, rx: f32, ry: f32, color: Color) -> Rect {
        if !(rx > 0.0) || !(ry > 0.0) {
            return Rect::ZERO;
        }
        let mut path = Path::new();
        path.ellipse(center, rx, ry);
        self.fill(&path, color)
    }

    pub fn ellipse_outline(
        &mut self,
        center: Point,
        rx: f32,
        ry: f32,
        style: &StrokeStyle,
        color: Color,
    ) -> Rect {
        if !(rx > 0.0) || !(ry > 0.0) {
            return Rect::ZERO;
        }
        let mut path = Path::new();
        path.ellipse(center, rx, ry);
        self.stroke(&path, style, color)
    }

    pub fn rect_outline(&mut self, rect: Rect, style: &StrokeStyle, color: Color) -> Rect {
        if rect.is_empty() {
            return Rect::ZERO;
        }
        let mut path = Path::new();
        path.rect(rect);
        self.stroke(&path, style, color)
    }

    pub fn rounded_rect_outline(
        &mut self,
        rect: Rect,
        radius: f32,
        style: &StrokeStyle,
        color: Color,
    ) -> Rect {
        if rect.is_empty() {
            return Rect::ZERO;
        }
        let mut path = Path::new();
        path.rounded_rect(rect, radius);
        self.stroke(&path, style, color)
    }

    pub fn rounded_rect_corners(
        &mut self,
        rect: Rect,
        radii: impl Into<Radius>,
        color: Color,
    ) -> Rect {
        if rect.is_empty() {
            return Rect::ZERO;
        }
        let mut path = Path::new();
        path.rounded_rect(rect, radii);
        self.fill(&path, color)
    }

    pub fn rounded_rect_corners_outline(
        &mut self,
        rect: Rect,
        radii: impl Into<Radius>,
        style: &StrokeStyle,
        color: Color,
    ) -> Rect {
        if rect.is_empty() {
            return Rect::ZERO;
        }
        let mut path = Path::new();
        path.rounded_rect(rect, radii);
        self.stroke(&path, style, color)
    }

    pub fn capsule(&mut self, from: Point, to: Point, radius: f32, color: Color) -> Rect {
        if !(radius > 0.0) {
            return Rect::ZERO;
        }
        if from == to {
            return self.disk(from, radius, color);
        }
        let style = StrokeStyle::round(radius * 2.0);
        self.line_styled(from, to, &style, color)
    }

    pub fn arrow(
        &mut self,
        from: Point,
        to: Point,
        width: f32,
        head_len: f32,
        head_width: f32,
        color: Color,
    ) -> Rect {
        self.arrow_styled(from, to, &line_style(width), head_len, head_width, color)
    }

    pub fn arrow_styled(
        &mut self,
        from: Point,
        to: Point,
        style: &StrokeStyle,
        head_len: f32,
        head_width: f32,
        color: Color,
    ) -> Rect {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let len = (dx * dx + dy * dy).sqrt();
        if !(len > 1e-6) {
            return Rect::ZERO;
        }
        let (ux, uy) = (dx / len, dy / len);
        let hl = head_len.clamp(0.0, len);
        let hw = head_width.max(0.0);
        let mut bounds = Rect::ZERO;
        if hl <= 0.0 || hw <= 0.0 {
            return self.line_styled(from, to, style, color);
        }
        let base = Point::new(to.x - ux * hl, to.y - uy * hl);
        if len - hl > 1e-4 {
            bounds = bounds.union(self.line_styled(from, base, style, color));
        }
        let (px, py) = (-uy, ux);
        let mut head = Path::new();
        head.polygon(&[
            to,
            Point::new(base.x + px * hw * 0.5, base.y + py * hw * 0.5),
            Point::new(base.x - px * hw * 0.5, base.y - py * hw * 0.5),
        ]);
        bounds.union(self.fill(&head, color))
    }

    pub fn cross(&mut self, center: Point, half: f32, width: f32, color: Color) -> Rect {
        if !(half > 0.0) {
            return Rect::ZERO;
        }
        let style = StrokeStyle::round(width);
        let h = self.line_styled(
            Point::new(center.x - half, center.y),
            Point::new(center.x + half, center.y),
            &style,
            color,
        );
        let v = self.line_styled(
            Point::new(center.x, center.y - half),
            Point::new(center.x, center.y + half),
            &style,
            color,
        );
        h.union(v)
    }

    pub fn x_marker(&mut self, center: Point, half: f32, width: f32, color: Color) -> Rect {
        if !(half > 0.0) {
            return Rect::ZERO;
        }
        let style = StrokeStyle::round(width);
        let a = self.line_styled(
            Point::new(center.x - half, center.y - half),
            Point::new(center.x + half, center.y + half),
            &style,
            color,
        );
        let b = self.line_styled(
            Point::new(center.x - half, center.y + half),
            Point::new(center.x + half, center.y - half),
            &style,
            color,
        );
        a.union(b)
    }

    pub fn regular_polygon(
        &mut self,
        center: Point,
        radius: f32,
        sides: u32,
        rotation_deg: f32,
        color: Color,
    ) -> Rect {
        let pts = regular_corners(center, radius, sides, rotation_deg);
        if pts.is_empty() {
            return Rect::ZERO;
        }
        self.polygon(&pts, color)
    }

    pub fn regular_polygon_outline(
        &mut self,
        center: Point,
        radius: f32,
        sides: u32,
        rotation_deg: f32,
        style: &StrokeStyle,
        color: Color,
    ) -> Rect {
        let pts = regular_corners(center, radius, sides, rotation_deg);
        if pts.is_empty() {
            return Rect::ZERO;
        }
        let mut path = Path::new();
        path.polygon(&pts);
        self.stroke(&path, style, color)
    }

    pub fn star(
        &mut self,
        center: Point,
        outer_r: f32,
        inner_r: f32,
        points: u32,
        rotation_deg: f32,
        color: Color,
    ) -> Rect {
        let pts = star_corners(center, outer_r, inner_r, points, rotation_deg);
        if pts.is_empty() {
            return Rect::ZERO;
        }
        self.polygon(&pts, color)
    }

    pub fn star_outline(
        &mut self,
        center: Point,
        outer_r: f32,
        inner_r: f32,
        points: u32,
        rotation_deg: f32,
        style: &StrokeStyle,
        color: Color,
    ) -> Rect {
        let pts = star_corners(center, outer_r, inner_r, points, rotation_deg);
        if pts.is_empty() {
            return Rect::ZERO;
        }
        let mut path = Path::new();
        path.polygon(&pts);
        self.stroke(&path, style, color)
    }

    pub fn ring(&mut self, center: Point, radius: f32, width: f32, color: Color) -> Rect {
        let mut path = Path::new();
        path.circle(center, radius);
        self.stroke(&path, &StrokeStyle::new(width), color)
    }

    pub fn arc(
        &mut self,
        center: Point,
        radius: f32,
        from_deg: f32,
        to_deg: f32,
        width: f32,
        color: Color,
    ) -> Rect {
        let mut path = Path::new();
        path.arc(center, radius, from_deg, to_deg);
        self.stroke(&path, &line_style(width), color)
    }

    pub fn sector(
        &mut self,
        center: Point,
        radius: f32,
        from_deg: f32,
        to_deg: f32,
        color: Color,
    ) -> Rect {
        if !(radius > 0.0) || !from_deg.is_finite() || !to_deg.is_finite() {
            return Rect::ZERO;
        }
        if (to_deg - from_deg).abs() < 1e-6 {
            return Rect::ZERO;
        }
        let mut path = Path::new();
        path.move_to(center);
        path.arc(center, radius, from_deg, to_deg);
        path.close();
        self.fill(&path, color)
    }

    pub fn donut_sector(
        &mut self,
        center: Point,
        inner_r: f32,
        outer_r: f32,
        from_deg: f32,
        to_deg: f32,
        color: Color,
    ) -> Rect {
        if !(outer_r > 0.0) || !from_deg.is_finite() || !to_deg.is_finite() {
            return Rect::ZERO;
        }
        if (to_deg - from_deg).abs() < 1e-6 {
            return Rect::ZERO;
        }
        let inner = if inner_r.is_finite() {
            inner_r.clamp(0.0, outer_r)
        } else {
            0.0
        };
        if inner <= 0.0 {
            return self.sector(center, outer_r, from_deg, to_deg, color);
        }
        let (a, b) = (from_deg.to_radians(), to_deg.to_radians());
        let outer_from = Point::new(center.x + a.cos() * outer_r, center.y + a.sin() * outer_r);
        let inner_to = Point::new(center.x + b.cos() * inner, center.y + b.sin() * inner);
        let mut path = Path::new();
        path.move_to(outer_from);
        path.arc(center, outer_r, from_deg, to_deg);
        path.line_to(inner_to);
        path.arc(center, inner, to_deg, from_deg);
        path.close();
        self.fill(&path, color)
    }

    pub fn line(&mut self, from: Point, to: Point, width: f32, color: Color) -> Rect {
        self.line_styled(from, to, &line_style(width), color)
    }

    pub fn line_crisp(&mut self, from: Point, to: Point, width: f32, color: Color) -> Rect {
        let (from, to) = crisp_axis_line(from, to, width, self.scale_factor());
        self.line(from, to, width, color)
    }

    pub fn dotted_line(
        &mut self,
        from: Point,
        to: Point,
        width: f32,
        spacing: f32,
        color: Color,
    ) -> Rect {
        let radius = width * 0.5;
        if !(radius > 0.0) || !(spacing > 0.0) || !spacing.is_finite() {
            return Rect::ZERO;
        }
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let len = (dx * dx + dy * dy).sqrt();
        if !(len > 1e-6) {
            return self.disk(from, radius, color);
        }

        const MAX_DOTS: usize = 4096;
        let count = ((len / spacing).floor() as usize).min(MAX_DOTS);
        let mut bounds = Rect::ZERO;
        for i in 0..=count {
            let t = (i as f32 * spacing).min(len);
            let k = t / len;
            let p = Point::new(from.x + dx * k, from.y + dy * k);
            bounds = bounds.union(self.disk(p, radius, color));
        }

        if (count as f32 * spacing - len).abs() > 1e-3 {
            bounds = bounds.union(self.disk(to, radius, color));
        }
        bounds
    }

    pub fn line_styled(
        &mut self,
        from: Point,
        to: Point,
        style: &StrokeStyle,
        color: Color,
    ) -> Rect {
        let mut path = Path::new();
        path.move_to(from).line_to(to);
        self.stroke(&path, style, color)
    }

    pub fn dashed_line(
        &mut self,
        from: Point,
        to: Point,
        width: f32,
        dash: impl Into<Dash>,
        color: Color,
    ) -> Rect {
        let style = line_style(width).dash(dash);
        self.line_styled(from, to, &style, color)
    }

    pub fn outline(&mut self, points: &[Point], style: &StrokeStyle, color: Color) -> Rect {
        let mut path = Path::new();
        path.polyline(points);
        self.stroke(&path, style, color)
    }

    pub fn polygon(&mut self, points: &[Point], color: Color) -> Rect {
        let mut path = Path::new();
        path.polygon(points);
        self.fill(&path, color)
    }

    pub fn spline(
        &mut self,
        points: &[Point],
        tension: f32,
        style: &StrokeStyle,
        color: Color,
    ) -> Rect {
        let mut path = Path::new();
        path.spline(points, tension);
        self.stroke(&path, style, color)
    }

    pub fn curve(
        &mut self,
        from: Point,
        c1: Point,
        c2: Point,
        to: Point,
        style: &StrokeStyle,
        color: Color,
    ) -> Rect {
        let mut path = Path::new();
        path.move_to(from).cubic_to(c1, c2, to);
        self.stroke(&path, style, color)
    }

    pub fn curve_quad(
        &mut self,
        from: Point,
        control: Point,
        to: Point,
        style: &StrokeStyle,
        color: Color,
    ) -> Rect {
        let mut path = Path::new();
        path.move_to(from).quad_to(control, to);
        self.stroke(&path, style, color)
    }

    pub fn draw_svg(&mut self, data: &str, style: Option<&StrokeStyle>, color: Color) -> Rect {
        let Some(path) = self.path_from_svg(data) else {
            return Rect::ZERO;
        };
        match style {
            Some(style) => self.stroke(&path, style, color),
            None => self.fill(&path, color),
        }
    }

    pub fn draw_transformed(
        &mut self,
        path: &Path,
        transform: Transform,
        style: Option<&StrokeStyle>,
        color: Color,
    ) -> Rect {
        let path = path.transformed(transform);
        match style {
            Some(style) => self.stroke(&path, style, color),
            None => self.fill(&path, color),
        }
    }

    fn shifted_rect(&self, rect: Rect) -> Rect {
        if self.draw_offset.x == 0.0 && self.draw_offset.y == 0.0 {
            rect
        } else {
            Rect::from_xywh(
                rect.origin.x + self.draw_offset.x,
                rect.origin.y + self.draw_offset.y,
                rect.size.width,
                rect.size.height,
            )
        }
    }
}

impl PaintCtx<'_> {
    fn band_px(&self, px: f32) -> f32 {
        let scale = self.scale_factor();
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        px.max(0.05) / scale
    }
}

fn shadow_radius(radius: f32, rect: Rect) -> f32 {
    radius
        .clamp(0.0, rect.size.width.min(rect.size.height) * 0.5)
        .max(0.0)
}

fn crisp_axis_line(from: Point, to: Point, width: f32, scale: f32) -> (Point, Point) {
    let s = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let px = 1.0 / s;
    let width_px = width * s;
    if !width.is_finite() || width <= 0.0 || (width_px - width_px.round()).abs() > 0.01 {
        return (from, to);
    }
    let half = if (width_px.round() as i64) % 2 != 0 {
        px * 0.5
    } else {
        0.0
    };
    let dx = (to.x - from.x).abs();
    let dy = (to.y - from.y).abs();
    if dy <= 1e-4 && dx > 1e-4 {
        let y = (from.y / px).round() * px + half;
        (Point::new(from.x, y), Point::new(to.x, y))
    } else if dx <= 1e-4 && dy > 1e-4 {
        let x = (from.x / px).round() * px + half;
        (Point::new(x, from.y), Point::new(x, to.y))
    } else {
        (from, to)
    }
}

fn shift(rect: Rect, offset: Vec2) -> Rect {
    if offset.x == 0.0 && offset.y == 0.0 {
        rect
    } else {
        Rect::from_xywh(
            rect.origin.x + offset.x,
            rect.origin.y + offset.y,
            rect.size.width,
            rect.size.height,
        )
    }
}

fn tint(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}

fn regular_corners(center: Point, radius: f32, sides: u32, rotation_deg: f32) -> Vec<Point> {
    if !(radius > 0.0) {
        return Vec::new();
    }
    let n = sides.clamp(3, 64);
    let base = rotation_deg.to_radians();
    (0..n)
        .map(|i| {
            let a = base + i as f32 * std::f32::consts::TAU / n as f32;
            Point::new(center.x + a.cos() * radius, center.y + a.sin() * radius)
        })
        .collect()
}

fn star_corners(
    center: Point,
    outer_r: f32,
    inner_r: f32,
    points: u32,
    rotation_deg: f32,
) -> Vec<Point> {
    if !(outer_r > 0.0) || !(inner_r > 0.0) {
        return Vec::new();
    }

    let n = points.clamp(2, 64);
    let base = rotation_deg.to_radians();
    (0..2 * n)
        .map(|i| {
            let r = if i % 2 == 0 { outer_r } else { inner_r };
            let a = base + i as f32 * std::f32::consts::PI / n as f32;
            Point::new(center.x + a.cos() * r, center.y + a.sin() * r)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_bands_capped_per_span() {
        assert_eq!(gradient_band_count(10.0, 1.0), 10);
        assert_eq!(
            gradient_band_count(800.0, 1.0),
            GRADIENT_BANDS_MAX,
            "800px cover is max 256 quads per span, not 800"
        );
        assert_eq!(gradient_band_count(0.0, 1.0), 1, "zawsze co najmniej 1");
        assert_eq!(gradient_band_count(-5.0, 1.0), 1);
        assert_eq!(
            gradient_band_count(100.0, 0.0),
            gradient_band_count(100.0, GRADIENT_BAND_PX),
            "bad band returns to default"
        );
    }

    #[test]
    fn gradient_bands_cover_span_on_fractional_origin() {
        let e = band_edges(100.5, 198.0, 198, 1.0, 0.5);
        assert_eq!(e.len(), 198);
        assert!((e[0].0 - 100.5).abs() < 1e-6, "pierwszy pas od originu");
        assert!(
            (e.last().unwrap().1 - 298.5).abs() < 1e-6,
            "last to the end"
        );
        for (i, w) in e.windows(2).enumerate() {
            let phi = w[0].1;
            let (lo, hi) = (w[1].0, w[1].1);
            assert!(lo <= phi + 1e-4, "dziura przed pasem {i}");
            assert!((lo - lo.round()).abs() < 1e-4, "edge {i} outside mesh");

            assert!(hi - lo > 0.0 && hi - lo <= 2.6, "wrong belt size {i}");
        }

        let e2 = band_edges(100.25, 50.0, 50, 0.5, 0.25);
        assert_eq!(e2.len(), 50);
        for w in e2.windows(2) {
            assert!(w[1].0 <= w[0].1 + 1e-4);
            assert!(((w[1].0 * 2.0).round() - w[1].0 * 2.0).abs() < 1e-3);
        }
    }

    #[test]
    fn star_corner_count_is_clamped() {
        let c = Point::new(0.0, 0.0);
        assert_eq!(
            star_corners(c, 10.0, 5.0, u32::MAX, 0.0).len(),
            128,
            "top clamp 64 arms"
        );
        assert_eq!(star_corners(c, 10.0, 5.0, 65, 0.0).len(), 128);
        assert_eq!(star_corners(c, 10.0, 5.0, 5, 0.0).len(), 10);
        assert_eq!(star_corners(c, 10.0, 5.0, 1, 0.0).len(), 4, "dolny clamp 2");
        assert_eq!(star_corners(c, 10.0, 5.0, 0, 0.0).len(), 4);
    }
}
