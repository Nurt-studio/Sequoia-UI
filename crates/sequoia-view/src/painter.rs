#![forbid(unsafe_code)]

use sequoia_core::{
    BrushStroke, Color, Fill, GradientKind, GradientStops, ImageSource, Pixels, Point, Rect, Size,
    Transform, Vec2,
};
use sequoia_text::FontWeight;

use crate::paint_draw::crisp_axis_line;
use crate::view::PaintCtx;

pub struct Painter<'a, 'b> {
    ctx: &'a mut PaintCtx<'b>,
    origin: Point,
    size: Size,

    transform: Transform,
}

impl<'a, 'b> Painter<'a, 'b> {
    pub fn new(ctx: &'a mut PaintCtx<'b>) -> Self {
        debug_assert!(
            ctx.draw_offset.x == 0.0 && ctx.draw_offset.y == 0.0,
            "Painter not using draw_offset - remove translate/set_draw_offset"
        );
        let origin = ctx.rect.origin;
        let size = ctx.rect.size;
        Self {
            ctx,
            origin,
            size,
            transform: Transform::IDENTITY,
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn rect(&self) -> Rect {
        Rect::new(Point::ZERO, self.size)
    }

    pub fn origin_global(&self) -> Point {
        self.origin
    }

    pub fn ctx(&mut self) -> &mut PaintCtx<'b> {
        self.ctx
    }

    pub fn push_list_transform(&mut self, t: Transform) {
        self.ctx.list.push_transform(t);
    }

    pub fn pop_list_transform(&mut self) {
        self.ctx.list.pop_transform();
    }

    pub fn with_transform<R>(&mut self, t: Transform, f: impl FnOnce(&mut Self) -> R) -> R {
        let prev = self.transform;
        self.transform = t.then(prev);
        let r = f(self);
        self.transform = prev;
        r
    }

    pub fn unproject(&self, p: Point) -> Option<Point> {
        let inv = self.transform.inverse()?;
        Some(inv.apply(Point::new(p.x - self.origin.x, p.y - self.origin.y)))
    }

    fn unit_scale(&self) -> f32 {
        self.transform.apply_vec(Vec2::new(1.0, 0.0)).length()
    }

    fn stroke_style(&self, stroke: BrushStroke) -> sequoia_draw::StrokeStyle {
        use sequoia_draw::StrokeStyle;
        let w = (stroke.width * self.unit_scale()).max(0.0);
        if stroke.rounded {
            StrokeStyle::round(w)
        } else {
            StrokeStyle::new(w)
        }
    }

    pub fn to_global_rect(&self, r: Rect) -> Rect {
        if r.is_empty() {
            return r;
        }
        let r = self.transform.apply_rect_aabb(r);
        Rect::from_xywh(
            r.left() + self.origin.x,
            r.top() + self.origin.y,
            r.size.width,
            r.size.height,
        )
    }

    pub fn to_global_point(&self, p: Point) -> Point {
        let p = self.transform.apply(p);
        Point::new(p.x + self.origin.x, p.y + self.origin.y)
    }

    pub fn to_local_point(&self, p: Point) -> Point {
        Point::new(p.x - self.origin.x, p.y - self.origin.y)
    }

    pub fn fill(&mut self, rect: Rect, fill: Fill) -> Rect {
        self.rounded(rect, 0.0, fill)
    }

    pub fn rounded(&mut self, rect: Rect, radius: f32, fill: Fill) -> Rect {
        if rect.is_empty() || !fill.is_visible() {
            return Rect::ZERO;
        }
        let global = self.to_global_rect(rect);

        let radius = radius * self.unit_scale();
        let radius = if radius.is_finite() {
            radius
                .clamp(0.0, global.size.width.min(global.size.height) * 0.5)
                .max(0.0)
        } else {
            0.0
        };
        match fill {
            Fill::Solid(color) => {
                if color.a <= 0.0 {
                    return Rect::ZERO;
                }
                self.ctx.list.push_quad(global, color, radius);
                rect
            }
            Fill::Gradient { kind, stops } => self.push_gradient(rect, radius, kind, stops),
        }
    }

    pub fn circle(&mut self, center: Point, radius: f32, fill: Fill) -> Rect {
        if !(radius > 0.0) || !radius.is_finite() || !fill.is_visible() {
            return Rect::ZERO;
        }
        let rect = Rect::from_xywh(
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
        );

        let gr = radius * self.unit_scale();
        if !(gr > 0.0) {
            return Rect::ZERO;
        }
        match fill {
            Fill::Solid(color) => {
                if color.a <= 0.0 {
                    return Rect::ZERO;
                }
                let global = self.to_global_rect(rect);
                self.ctx.list.push_quad(global, color, gr);
                rect
            }
            Fill::Gradient { kind, stops } => {
                let kind = match kind {
                    GradientKind::Radial { radius: r } if r <= 0.0 => {
                        GradientKind::Radial { radius: gr }
                    }
                    other => other,
                };
                self.push_gradient(rect, gr, kind, stops)
            }
        }
    }

    pub fn ring(&mut self, center: Point, radius: f32, width: f32, color: Color) -> Rect {
        if !(radius > 0.0) || !(width > 0.0) || !width.is_finite() || color.a <= 0.0 {
            return Rect::ZERO;
        }
        let k = self.unit_scale();
        let mut path = sequoia_draw::Path::new();
        path.circle(self.to_global_point(center), radius * k);
        let bounds = self
            .ctx
            .stroke(&path, &self.stroke_style(BrushStroke::new(width)), color);
        self.to_local_bounds(bounds)
    }

    pub fn line(&mut self, from: Point, to: Point, stroke: BrushStroke, color: Color) -> Rect {
        if !stroke.is_visible() || color.a <= 0.0 {
            return Rect::ZERO;
        }
        let style = self.stroke_style(stroke);
        let from = self.to_global_point(from);
        let to = self.to_global_point(to);
        let (from, to) = crisp_axis_line(
            from,
            to,
            stroke.width * self.unit_scale(),
            self.ctx.scale_factor(),
        );
        let mut path = sequoia_draw::Path::new();
        path.move_to(from).line_to(to);
        let bounds = self.ctx.stroke(&path, &style, color);
        self.to_local_bounds(bounds)
    }

    pub fn polyline(&mut self, points: &[Point], stroke: BrushStroke, color: Color) -> Rect {
        if points.len() < 2 || !stroke.is_visible() || color.a <= 0.0 {
            return Rect::ZERO;
        }
        use sequoia_draw::Path;
        let style = self.stroke_style(stroke);
        let mut path = Path::new();
        let mut it = points.iter();

        path.move_to(self.to_global_point(*it.next().unwrap_or(&Point::ZERO)));
        for p in it {
            path.line_to(self.to_global_point(*p));
        }
        let bounds = self.ctx.stroke(&path, &style, color);
        self.to_local_bounds(bounds)
    }

    pub fn curve(
        &mut self,
        p0: Point,
        c0: Point,
        c1: Point,
        p1: Point,
        stroke: BrushStroke,
        color: Color,
    ) -> Rect {
        let span = (p1.x - p0.x).abs() + (p1.y - p0.y).abs();
        let n = ((span / 12.0) as usize).clamp(4, 32);
        let mut pts = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            pts.push(Point::new(
                u * u * u * p0.x
                    + 3.0 * u * u * t * c0.x
                    + 3.0 * u * t * t * c1.x
                    + t * t * t * p1.x,
                u * u * u * p0.y
                    + 3.0 * u * u * t * c0.y
                    + 3.0 * u * t * t * c1.y
                    + t * t * t * p1.y,
            ));
        }
        self.polyline(&pts, stroke, color)
    }

    pub fn rect_outline(
        &mut self,
        rect: Rect,
        radius: f32,
        stroke: BrushStroke,
        color: Color,
    ) -> Rect {
        if rect.is_empty() || !stroke.is_visible() || color.a <= 0.0 {
            return Rect::ZERO;
        }
        use sequoia_draw::Path;
        let style = self.stroke_style(stroke);
        let mut path = Path::new();
        let global = self.to_global_rect(rect);
        let radius = radius * self.unit_scale();
        if radius > 0.0 {
            path.rounded_rect(global, radius);
        } else {
            path.rect(global);
        }
        let bounds = self.ctx.stroke(&path, &style, color);
        self.to_local_bounds(bounds)
    }

    pub fn polygon(&mut self, points: &[Point], color: Color) -> Rect {
        if points.len() < 3 || color.a <= 0.0 {
            return Rect::ZERO;
        }
        let global: Vec<Point> = points.iter().map(|p| self.to_global_point(*p)).collect();
        let bounds = self.ctx.polygon(&global, color);
        self.to_local_bounds(bounds)
    }

    pub fn ellipse(&mut self, center: Point, rx: f32, ry: f32, color: Color) -> Rect {
        if !(rx > 0.0) || !(ry > 0.0) || color.a <= 0.0 {
            return Rect::ZERO;
        }
        let kx = self.transform.apply_vec(Vec2::new(1.0, 0.0)).length();
        let ky = self.transform.apply_vec(Vec2::new(0.0, 1.0)).length();
        let bounds = self
            .ctx
            .ellipse(self.to_global_point(center), rx * kx, ry * ky, color);
        self.to_local_bounds(bounds)
    }

    pub fn text(&mut self, text: &str, baseline: Point, size: f32, color: Color) {
        if text.is_empty() || !(size > 0.0) || color.a <= 0.0 {
            return;
        }
        let g = self.to_global_point(baseline);
        self.ctx.draw_text(text, (g.x, g.y), size, color);
    }

    pub fn text_tl(&mut self, text: &str, top_left: Point, size: f32, color: Color) {
        let baseline = Point::new(top_left.x, PaintCtx::baseline_top(top_left.y, size));
        self.text(text, baseline, size, color);
    }

    pub fn text_tl_rotated(
        &mut self,
        text: &str,
        top_left: Point,
        size: f32,
        color: Color,
        degrees: f32,
        pivot: Point,
    ) {
        let pivot = self.to_global_point(pivot);
        let t = Transform::rotate_around_degrees(degrees, pivot);
        self.push_list_transform(t);
        self.text_tl(text, top_left, size, color);
        self.pop_list_transform();
    }

    pub fn text_styled(
        &mut self,
        text: &str,
        baseline: Point,
        size: f32,
        weight: FontWeight,
        color: Color,
    ) {
        if text.is_empty() || !(size > 0.0) || color.a <= 0.0 {
            return;
        }
        let g = self.to_global_point(baseline);
        self.ctx
            .draw_text_weighted(text, (g.x, g.y), size, weight, color);
    }

    pub fn text_width(&mut self, text: &str, size: f32) -> f32 {
        if text.is_empty() || !(size > 0.0) {
            return 0.0;
        }
        self.ctx.text_width(text, size)
    }

    pub fn text_width_weighted(&mut self, text: &str, size: f32, weight: FontWeight) -> f32 {
        if text.is_empty() || !(size > 0.0) {
            return 0.0;
        }
        self.ctx.text_width_weighted(text, size, weight)
    }

    pub fn text_center_x(&mut self, text: &str, center_x: f32, top: f32, size: f32, color: Color) {
        let width = self.text_width(text, size);
        self.text_tl(text, Point::new(center_x - width * 0.5, top), size, color);
    }

    pub fn text_center(&mut self, rect: Rect, text: &str, size: f32, color: Color) {
        let metrics = self.ctx.text_metrics(text, size);
        let top = rect.top() + (rect.size.height - metrics.height) * 0.5;
        self.text_tl(
            text,
            Point::new(rect.center().x - metrics.width * 0.5, top),
            size,
            color,
        );
    }

    pub fn image(&mut self, rect: Rect, uv: [f32; 4], source: ImageSource) -> Rect {
        if rect.is_empty() {
            return Rect::ZERO;
        }
        let global = self.to_global_rect(rect);
        self.ctx.list.push_image(global, uv, source);
        rect
    }

    pub fn pixels(&mut self, rect: Rect, pixels: &Pixels) -> Rect {
        let full = Rect::from_xywh(0.0, 0.0, pixels.width() as f32, pixels.height() as f32);
        self.pixels_region(rect, pixels, full)
    }

    pub fn pixels_region(&mut self, dst: Rect, pixels: &Pixels, src: Rect) -> Rect {
        if dst.is_empty() {
            return Rect::ZERO;
        }
        let Some(uv) = Self::pixel_uv(pixels.width(), pixels.height(), src) else {
            return Rect::ZERO;
        };
        let global = self.to_global_rect(dst);
        self.ctx.list.push_image(global, uv, pixels.to_source());
        dst
    }

    pub fn pixel_uv(width: u32, height: u32, src: Rect) -> Option<[f32; 4]> {
        if width == 0 || height == 0 {
            return None;
        }
        let (w, h) = (width as f32, height as f32);
        let x0 = src.left().clamp(0.0, w);
        let y0 = src.top().clamp(0.0, h);
        let x1 = src.right().clamp(0.0, w);
        let y1 = src.bottom().clamp(0.0, h);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        Some([x0 / w, y0 / h, x1 / w, y1 / h])
    }

    pub fn pixel_at(rect: Rect, width: u32, height: u32, p: Point) -> Option<(i32, i32)> {
        if width == 0 || height == 0 || rect.is_empty() {
            return None;
        }
        let fx = (p.x - rect.left()) / rect.size.width * width as f32;
        let fy = (p.y - rect.top()) / rect.size.height * height as f32;
        if !fx.is_finite() || !fy.is_finite() || fx < 0.0 || fy < 0.0 {
            return None;
        }
        if fx >= width as f32 || fy >= height as f32 {
            return None;
        }
        Some((fx.floor() as i32, fy.floor() as i32))
    }

    pub fn knob(&mut self, center: Point, radius: f32, fill: Color) -> Rect {
        if !(radius > 0.0) || !radius.is_finite() {
            return Rect::ZERO;
        }
        let outer = self.circle(center, radius, Fill::solid(Color::WHITE));
        let inner_r = radius - 2.0;
        if inner_r > 0.0 {
            let inner = self.circle(center, inner_r, Fill::solid(fill));
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
        self.fill(rect, Fill::solid(Color::from_hsva(h, 1.0, 1.0, 1.0)));
        self.fill(
            rect,
            Fill::horizontal(GradientStops::two(
                Color::WHITE,
                Color::rgba(1.0, 1.0, 1.0, 0.0),
            )),
        );
        self.fill(
            rect,
            Fill::vertical(GradientStops::two(
                Color::rgba(0.0, 0.0, 0.0, 0.0),
                Color::BLACK,
            )),
        );
        rect
    }

    pub fn grid(&mut self, step: f32, color: Color) -> u32 {
        self.grid_offset(step, Vec2::ZERO, color)
    }

    pub fn grid_offset(&mut self, step: f32, offset: Vec2, color: Color) -> u32 {
        if step <= 0.0 || color.a <= 0.0 || self.size.width <= 0.0 || self.size.height <= 0.0 {
            return 0;
        }
        let mut drawn = 0u32;
        let ox = offset.x.rem_euclid(step);
        let oy = offset.y.rem_euclid(step);
        let w = self.size.width;
        let h = self.size.height;
        let mut x = ox;
        while x < w {
            self.fill(Rect::from_xywh(x, 0.0, 1.0, h), Fill::solid(color));
            drawn += 1;
            if drawn > 4096 {
                break;
            }
            x += step;
        }
        let mut y = oy;
        while y < h {
            self.fill(Rect::from_xywh(0.0, y, w, 1.0), Fill::solid(color));
            drawn += 1;
            if drawn > 8192 {
                break;
            }
            y += step;
        }
        drawn
    }

    pub fn axes(&mut self, origin: Point, color: Color, width: f32) -> Rect {
        if !(width > 0.0) || color.a <= 0.0 {
            return Rect::ZERO;
        }
        let stroke = BrushStroke::new(width);
        let w = self.size.width;
        let h = self.size.height;
        let a = self.line(
            Point::new(0.0, origin.y),
            Point::new(w, origin.y),
            stroke,
            color,
        );
        let b = self.line(
            Point::new(origin.x, 0.0),
            Point::new(origin.x, h),
            stroke,
            color,
        );
        a.union(b)
    }

    pub fn frame(&mut self, rect: Rect, color: Color, width: f32) -> Rect {
        self.rect_outline(rect, 0.0, BrushStroke::new(width), color)
    }

    pub fn cross(&mut self, center: Point, half: f32, width: f32, color: Color) -> Rect {
        if !(half > 0.0) || color.a <= 0.0 {
            return Rect::ZERO;
        }
        let s = BrushStroke::round(width);
        let a = self.line(
            Point::new(center.x - half, center.y),
            Point::new(center.x + half, center.y),
            s,
            color,
        );
        let b = self.line(
            Point::new(center.x, center.y - half),
            Point::new(center.x, center.y + half),
            s,
            color,
        );
        a.union(b)
    }

    pub fn marquee(&mut self, a: Point, b: Point, fill: Fill, outline: Color) -> Rect {
        let rect = Rect::from_xywh(
            a.x.min(b.x),
            a.y.min(b.y),
            (a.x - b.x).abs(),
            (a.y - b.y).abs(),
        );
        if rect.is_empty() {
            return Rect::ZERO;
        }
        self.fill(rect, fill);
        self.rect_outline(rect, 0.0, BrushStroke::new(1.0), outline);
        rect
    }

    pub fn ellipse_outline(
        &mut self,
        center: Point,
        rx: f32,
        ry: f32,
        stroke: BrushStroke,
        color: Color,
    ) -> Rect {
        if !(rx > 0.0) || !(ry > 0.0) || !stroke.is_visible() || color.a <= 0.0 {
            return Rect::ZERO;
        }
        use sequoia_draw::Path;
        let style = self.stroke_style(stroke);
        let kx = self.transform.apply_vec(Vec2::new(1.0, 0.0)).length();
        let ky = self.transform.apply_vec(Vec2::new(0.0, 1.0)).length();
        let mut path = Path::new();
        path.ellipse(self.to_global_point(center), rx * kx, ry * ky);
        let bounds = self.ctx.stroke(&path, &style, color);
        self.to_local_bounds(bounds)
    }

    pub fn capsule(&mut self, from: Point, to: Point, radius: f32, color: Color) -> Rect {
        if !(radius > 0.0) || color.a <= 0.0 {
            return Rect::ZERO;
        }
        let bounds = self.ctx.capsule(
            self.to_global_point(from),
            self.to_global_point(to),
            radius,
            color,
        );
        self.to_local_bounds(bounds)
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
        if color.a <= 0.0 {
            return Rect::ZERO;
        }
        let bounds = self.ctx.arrow(
            self.to_global_point(from),
            self.to_global_point(to),
            width,
            head_len,
            head_width,
            color,
        );
        self.to_local_bounds(bounds)
    }

    pub fn x_marker(&mut self, center: Point, half: f32, width: f32, color: Color) -> Rect {
        if !(half > 0.0) || color.a <= 0.0 {
            return Rect::ZERO;
        }
        let bounds = self
            .ctx
            .x_marker(self.to_global_point(center), half, width, color);
        self.to_local_bounds(bounds)
    }

    pub fn regular_polygon(
        &mut self,
        center: Point,
        radius: f32,
        sides: u32,
        rotation_deg: f32,
        color: Color,
    ) -> Rect {
        if !(radius > 0.0) || sides < 3 || color.a <= 0.0 {
            return Rect::ZERO;
        }
        let bounds = self.ctx.regular_polygon(
            self.to_global_point(center),
            radius,
            sides,
            rotation_deg,
            color,
        );
        self.to_local_bounds(bounds)
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
        if !(outer_r > 0.0) || !(inner_r > 0.0) || points < 2 || color.a <= 0.0 {
            return Rect::ZERO;
        }
        let bounds = self.ctx.star(
            self.to_global_point(center),
            outer_r,
            inner_r,
            points,
            rotation_deg,
            color,
        );
        self.to_local_bounds(bounds)
    }

    pub fn dots(&mut self, step: f32, offset: Vec2, radius: f32, color: Color) -> u32 {
        if step <= 0.0 || !(radius > 0.0) || color.a <= 0.0 {
            return 0;
        }
        if self.size.width <= 0.0 || self.size.height <= 0.0 {
            return 0;
        }
        let mut n = 0u32;
        let ox = offset.x.rem_euclid(step);
        let oy = offset.y.rem_euclid(step);
        let mut y = oy;
        while y < self.size.height {
            let mut x = ox;
            while x < self.size.width {
                self.circle(Point::new(x, y), radius, Fill::solid(color));
                n += 1;
                if n > 16384 {
                    return n;
                }
                x += step;
            }
            y += step;
        }
        n
    }

    pub fn grid_split(
        &mut self,
        step: f32,
        every: u32,
        offset: Vec2,
        minor: Color,
        major: Color,
    ) -> u32 {
        if step <= 0.0 || every == 0 {
            return 0;
        }
        if self.size.width <= 0.0 || self.size.height <= 0.0 {
            return 0;
        }
        let every = every.max(1);
        let mut drawn = 0u32;
        let ox = offset.x.rem_euclid(step);
        let oy = offset.y.rem_euclid(step);
        let w = self.size.width;
        let h = self.size.height;
        let mut i = 0u32;
        let mut x = ox;
        while x < w {
            let c = if i % every == every - 1 { major } else { minor };
            self.fill(Rect::from_xywh(x, 0.0, 1.0, h), Fill::solid(c));
            drawn += 1;
            if drawn > 4096 {
                break;
            }
            x += step;
            i += 1;
        }
        let mut j = 0u32;
        let mut y = oy;
        while y < h {
            let c = if j % every == every - 1 { major } else { minor };
            self.fill(Rect::from_xywh(0.0, y, w, 1.0), Fill::solid(c));
            drawn += 1;
            if drawn > 8192 {
                break;
            }
            y += step;
            j += 1;
        }
        drawn
    }

    pub fn clipped<R>(&mut self, rect: Rect, f: impl FnOnce(&mut Self) -> R) -> R {
        let global = self.to_global_rect(rect);
        self.ctx.list.push_clip(global);
        let r = f(self);
        self.ctx.list.pop_clip();
        r
    }

    pub fn clipped_rounded<R>(
        &mut self,
        rect: Rect,
        radius: f32,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let global = self.to_global_rect(rect);
        self.ctx.push_rounded_clip(global, radius);
        let r = f(self);
        self.ctx.pop_clip();
        r
    }

    fn push_gradient(
        &mut self,
        rect: Rect,
        radius: f32,
        kind: GradientKind,
        stops: GradientStops,
    ) -> Rect {
        if stops.len() <= sequoia_core::MAX_GRADIENT_STOPS {
            let global = self.to_global_rect(rect);
            self.ctx.list.push_gradient(global, radius, kind, stops);
            return rect;
        }

        self.push_gradient_bands(rect, radius, kind, stops)
    }

    fn push_gradient_bands(
        &mut self,
        rect: Rect,
        shape_radius: f32,
        kind: GradientKind,
        stops: GradientStops,
    ) -> Rect {
        const RADIAL_BANDS: usize = 64;
        let vertical = match kind {
            GradientKind::Linear { angle_deg } => {
                let (dx, dy) = GradientKind::Linear { angle_deg }.direction();
                dy.abs() >= dx.abs()
            }
            GradientKind::Radial { .. } => true,
        };
        let global = self.to_global_rect(rect);
        let mut bounds = Rect::ZERO;
        match kind {
            GradientKind::Radial { radius } => {
                let center = Point::new(
                    global.left() + global.size.width * 0.5,
                    global.top() + global.size.height * 0.5,
                );
                let extent = if radius > 0.0 {
                    radius
                } else {
                    global.size.width.hypot(global.size.height) * 0.5
                };
                if extent <= 0.0 {
                    return Rect::ZERO;
                }
                if shape_radius > 0.0 {
                    self.ctx.push_rounded_clip(global, shape_radius);
                } else {
                    self.ctx.list.push_clip(global);
                }
                for i in (0..RADIAL_BANDS).rev() {
                    let t0 = i as f32 / RADIAL_BANDS as f32;
                    let t1 = (i + 1) as f32 / RADIAL_BANDS as f32;
                    let [r, g, b, a] = stops.sample_linear((t0 + t1) * 0.5);
                    let color = Color { r, g, b, a }.from_linear();
                    let r1 = extent * t1;
                    let band = Rect::from_xywh(center.x - r1, center.y - r1, r1 * 2.0, r1 * 2.0);
                    self.ctx.list.push_quad(band, color, r1);
                }
                self.ctx.pop_clip();
                bounds = global;
            }
            GradientKind::Linear { .. } => {
                let extent = if vertical {
                    global.size.height
                } else {
                    global.size.width
                };
                let bands = (extent.ceil() as usize).clamp(1, 256);
                let scale = {
                    let s = self.ctx.scale_factor();
                    if s.is_finite() && s > 0.0 { s } else { 1.0 }
                };
                let origin = if vertical {
                    global.top()
                } else {
                    global.left()
                };
                let edges =
                    crate::paint_draw::band_edges(origin, extent, bands, 1.0 / scale, 0.5 / scale);
                for (i, (lo, hi)) in edges.into_iter().enumerate() {
                    let t = (i as f32 + 0.5) / bands as f32;
                    let [r, g, b, a] = stops.sample_linear(t);
                    let color = Color { r, g, b, a }.from_linear();
                    let band = if vertical {
                        Rect::from_xywh(global.left(), lo, global.size.width, hi - lo)
                    } else {
                        Rect::from_xywh(lo, global.top(), hi - lo, global.size.height)
                    };
                    self.ctx.list.push_quad(band, color, 0.0);
                    bounds = bounds.union(band);
                }
            }
        }
        if bounds.is_empty() {
            return Rect::ZERO;
        }

        Rect::from_xywh(
            bounds.left() - self.origin.x,
            bounds.top() - self.origin.y,
            bounds.size.width,
            bounds.size.height,
        )
    }

    fn to_local_bounds(&self, bounds: Rect) -> Rect {
        if bounds.is_empty() {
            return Rect::ZERO;
        }
        let r = Rect::from_xywh(
            bounds.left() - self.origin.x,
            bounds.top() - self.origin.y,
            bounds.size.width,
            bounds.size.height,
        );

        match self.transform.inverse() {
            Some(inv) => inv.apply_rect_aabb(r),
            None => Rect::ZERO,
        }
    }
}
