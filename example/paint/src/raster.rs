use sequoia_ui::prelude::*;

use crate::model::{BrushKind, FillMode, ShapeKind};

fn hash2(x: i32, y: i32, i: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ (i as u32).wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^ (h >> 16)
}

fn hash01(x: i32, y: i32, i: i32) -> f32 {
    (hash2(x, y, i) & 0xFFFF) as f32 / 65535.0
}

fn blend_set(px: &mut Pixels, x: i32, y: i32, color: Color) {
    if color.a >= 0.999 {
        px.set(x, y, color);
    } else if color.a > 0.0 {
        px.blend(x, y, color);
    }
}

fn stamp_circle(px: &mut Pixels, cx: f32, cy: f32, radius: f32, color: Color) {
    let r = radius.max(0.5);
    let x0 = (cx - r).floor() as i32;
    let x1 = (cx + r).ceil() as i32;
    let y0 = (cy - r).floor() as i32;
    let y1 = (cy + r).ceil() as i32;
    if x1 - x0 > 256 || y1 - y0 > 256 {
        return;
    }
    let rr = r * r;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            if dx * dx + dy * dy <= rr {
                blend_set(px, x, y, color);
            }
        }
    }
}

fn stamp_square(px: &mut Pixels, cx: f32, cy: f32, radius: f32, color: Color) {
    let r = radius.max(0.5);
    let x0 = (cx - r).floor() as i32;
    let x1 = (cx + r).ceil() as i32;
    let y0 = (cy - r).floor() as i32;
    let y1 = (cy + r).ceil() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            blend_set(px, x, y, color);
        }
    }
}

fn stamp_ellipse_rot(
    px: &mut Pixels,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    angle: f32,
    color: Color,
) {
    let reach = rx.max(ry) + 1.0;
    let (s, c) = angle.sin_cos();
    let x0 = (cx - reach).floor() as i32;
    let x1 = (cx + reach).ceil() as i32;
    let y0 = (cy - reach).floor() as i32;
    let y1 = (cy + reach).ceil() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let lx = dx * c + dy * s;
            let ly = -dx * s + dy * c;
            if (lx * lx) / (rx * rx) + (ly * ly) / (ry * ry) <= 1.0 {
                blend_set(px, x, y, color);
            }
        }
    }
}

pub(crate) fn stamp(px: &mut Pixels, cx: f32, cy: f32, width: f32, color: Color, brush: BrushKind) {
    let radius = (width * 0.5).max(0.5);
    match brush {
        BrushKind::Brush | BrushKind::NaturalPencil => {
            if width <= 1.0 {
                blend_set(px, cx.floor() as i32, cy.floor() as i32, color);
            } else {
                stamp_circle(px, cx, cy, radius, color);
            }
        }
        BrushKind::Airbrush => {
            let n = ((width * width * 2.0) as i32).clamp(3, 64);
            for i in 0..n {
                let a = hash01(cx as i32, cy as i32, i) * std::f32::consts::TAU;
                let r = hash01(cx as i32, cy as i32, i + 97).sqrt() * radius * 1.8;
                let x = cx + a.cos() * r;
                let y = cy + a.sin() * r;
                blend_set(
                    px,
                    x.floor() as i32,
                    y.floor() as i32,
                    color.with_alpha(0.16),
                );
            }
        }
        BrushKind::Calligraphy1 => {
            stamp_ellipse_rot(
                px,
                cx,
                cy,
                radius * 1.15,
                (radius * 0.28).max(0.7),
                -0.7,
                color,
            );
        }
        BrushKind::Calligraphy2 => {
            stamp_ellipse_rot(
                px,
                cx,
                cy,
                radius * 0.9,
                (radius * 0.42).max(0.7),
                0.65,
                color,
            );
        }
        BrushKind::Crayon => {
            let n = (width * width * 1.5) as i32;
            for i in 0..n.max(4) {
                let a = hash01(cx as i32, cy as i32, i) * std::f32::consts::TAU;
                let r = hash01(cx as i32, cy as i32, i + 31).sqrt() * radius;
                let x = (cx + a.cos() * r).floor() as i32;
                let y = (cy + a.sin() * r).floor() as i32;
                blend_set(px, x, y, color.with_alpha(0.85));
            }
        }
        BrushKind::Marker => {
            stamp_square(px, cx, cy, radius * 0.85, color.with_alpha(0.45));
        }
        BrushKind::Watercolor => {
            for i in 0..3 {
                let a = i as f32 * 1.9 + cx * 0.01 + cy * 0.013;
                let x = cx + a.cos() * radius * 0.35;
                let y = cy + a.sin() * radius * 0.35;
                stamp_circle(px, x, y, radius * 1.15, color.with_alpha(0.09));
            }
        }
        BrushKind::Highlighter => {
            stamp_square(px, cx, cy, radius, color.with_alpha(0.22));
        }
    }
}

pub(crate) fn stroke_segment(
    px: &mut Pixels,
    from: Point,
    to: Point,
    width: f32,
    color: Color,
    brush: BrushKind,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let dist = (dx * dx + dy * dy).sqrt();
    let step = (width * 0.22).max(0.55);
    let n = (dist / step).ceil().max(1.0) as i32;
    for i in 0..=n {
        let t = i as f32 / n as f32;
        stamp(px, from.x + dx * t, from.y + dy * t, width, color, brush);
    }
}

pub(crate) fn stroke_path(
    px: &mut Pixels,
    points: &[Point],
    width: f32,
    color: Color,
    brush: BrushKind,
    closed: bool,
) {
    if points.len() < 2 {
        return;
    }
    for pair in points.windows(2) {
        stroke_segment(px, pair[0], pair[1], width, color, brush);
    }
    if closed {
        stroke_segment(
            px,
            *points.last().unwrap_or(&Point::ZERO),
            points[0],
            width,
            color,
            brush,
        );
    }
}

pub(crate) fn fill_polygon(px: &mut Pixels, points: &[Point], color: Color) {
    if points.len() < 3 || color.a <= 0.0 {
        return;
    }
    let min_y = points
        .iter()
        .map(|p| p.y)
        .fold(f32::INFINITY, f32::min)
        .floor() as i32;
    let max_y = points
        .iter()
        .map(|p| p.y)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil() as i32;
    if max_y - min_y > 16384 {
        return;
    }
    let mut xs: Vec<f32> = Vec::new();
    for y in min_y..max_y {
        let yc = y as f32 + 0.5;
        xs.clear();
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            if (a.y <= yc && b.y > yc) || (b.y <= yc && a.y > yc) {
                let t = (yc - a.y) / (b.y - a.y);
                xs.push(a.x + (b.x - a.x) * t);
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut i = 0;
        while i + 1 < xs.len() {
            let x0 = xs[i].ceil() as i32;
            let x1 = xs[i + 1].floor() as i32;
            for x in x0..=x1 {
                blend_set(px, x, y, color);
            }
            i += 2;
        }
    }
}

pub(crate) fn flood_fill(px: &mut Pixels, sx: i32, sy: i32, color: Color) {
    let Some(target) = px.get(sx, sy) else {
        return;
    };
    if close_color(target, color) {
        return;
    }
    let w = px.width() as i32;
    let h = px.height() as i32;
    if w <= 0 || h <= 0 {
        return;
    }
    let mut stack = vec![(sx, sy)];
    let mut guard = 0usize;
    let max = (w as usize * h as usize).saturating_add(16);
    while let Some((x, y)) = stack.pop() {
        guard += 1;
        if guard > max * 2 {
            break;
        }
        if x < 0 || y < 0 || x >= w || y >= h {
            continue;
        }
        let Some(p) = px.get(x, y) else { continue };
        if !close_color(p, target) {
            continue;
        }
        px.set(x, y, color);
        stack.push((x + 1, y));
        stack.push((x - 1, y));
        stack.push((x, y + 1));
        stack.push((x, y - 1));
    }
}

fn close_color(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() <= 0.02
        && (a.g - b.g).abs() <= 0.02
        && (a.b - b.b).abs() <= 0.02
        && (a.a - b.a).abs() <= 0.02
}

fn map_unit(points: &[(f32, f32)], r: Rect) -> Vec<Point> {
    points
        .iter()
        .map(|(x, y)| Point::new(r.left() + r.size.width * x, r.top() + r.size.height * y))
        .collect()
}

fn regular_polygon(cx: f32, cy: f32, rx: f32, ry: f32, sides: u32, rot: f32) -> Vec<Point> {
    let mut out = Vec::with_capacity(sides as usize);
    for i in 0..sides {
        let a = rot + i as f32 / sides as f32 * std::f32::consts::TAU;
        out.push(Point::new(cx + a.cos() * rx, cy + a.sin() * ry));
    }
    out
}

fn star_points(cx: f32, cy: f32, rx: f32, ry: f32, points: u32, rot: f32) -> Vec<Point> {
    let mut out = Vec::with_capacity(points as usize * 2);
    for i in 0..points * 2 {
        let a = rot + i as f32 / (points * 2) as f32 * std::f32::consts::TAU;
        let k = if i % 2 == 0 { 1.0 } else { 0.42 };
        out.push(Point::new(cx + a.cos() * rx * k, cy + a.sin() * ry * k));
    }
    out
}

fn rounded_rect(r: Rect, radius: f32) -> Vec<Point> {
    let rad = radius
        .max(0.0)
        .min(r.size.width * 0.5)
        .min(r.size.height * 0.5);
    if rad <= 0.5 {
        return map_unit(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], r);
    }
    let mut out = Vec::new();
    let corners = [
        (
            r.left() + rad,
            r.top() + rad,
            std::f32::consts::PI,
            1.5 * std::f32::consts::PI,
        ),
        (
            r.right() - rad,
            r.top() + rad,
            1.5 * std::f32::consts::PI,
            2.0 * std::f32::consts::PI,
        ),
        (
            r.right() - rad,
            r.bottom() - rad,
            0.0,
            0.5 * std::f32::consts::PI,
        ),
        (
            r.left() + rad,
            r.bottom() - rad,
            0.5 * std::f32::consts::PI,
            std::f32::consts::PI,
        ),
    ];
    for (cx, cy, a0, a1) in corners {
        for i in 0..=6 {
            let t = i as f32 / 6.0;
            let a = a0 + (a1 - a0) * t;
            out.push(Point::new(cx + a.cos() * rad, cy + a.sin() * rad));
        }
    }
    out
}

pub(crate) fn shape_points(kind: ShapeKind, a: Point, b: Point) -> (Vec<Point>, bool) {
    let x0 = a.x.min(b.x);
    let x1 = a.x.max(b.x);
    let y0 = a.y.min(b.y);
    let y1 = a.y.max(b.y);
    let r = Rect::from_xywh(x0, y0, (x1 - x0).max(1.0), (y1 - y0).max(1.0));
    let cx = (x0 + x1) * 0.5;
    let cy = (y0 + y1) * 0.5;
    let rx = (x1 - x0).abs().max(1.0) * 0.5;
    let ry = (y1 - y0).abs().max(1.0) * 0.5;
    let (vx0, vy0, vx1, vy1) = (a.x, a.y, b.x, b.y);
    match kind {
        ShapeKind::Line => (vec![Point::new(vx0, vy0), Point::new(vx1, vy1)], false),
        ShapeKind::Curve => {
            let mut pts = Vec::with_capacity(28);
            for i in 0..=24 {
                let t = i as f32 / 24.0;
                let u = 1.0 - t;
                let p0 = Point::new(vx0, vy0);
                let p1 = Point::new(vx0 + (vx1 - vx0) * 0.33, vy0);
                let p2 = Point::new(vx0 + (vx1 - vx0) * 0.66, vy1);
                let p3 = Point::new(vx1, vy1);
                pts.push(Point::new(
                    u * u * u * p0.x
                        + 3.0 * u * u * t * p1.x
                        + 3.0 * u * t * t * p2.x
                        + t * t * t * p3.x,
                    u * u * u * p0.y
                        + 3.0 * u * u * t * p1.y
                        + 3.0 * u * t * t * p2.y
                        + t * t * t * p3.y,
                ));
            }
            (pts, false)
        }
        ShapeKind::Oval => (
            regular_polygon(cx, cy, rx, ry, 72, -std::f32::consts::FRAC_PI_2),
            true,
        ),
        ShapeKind::Rectangle => (
            vec![
                Point::new(x0, y0),
                Point::new(x1, y0),
                Point::new(x1, y1),
                Point::new(x0, y1),
            ],
            true,
        ),
        ShapeKind::RoundedRectangle => (
            rounded_rect(r, r.size.width.min(r.size.height) * 0.18),
            true,
        ),
        ShapeKind::Polygon => (
            regular_polygon(cx, cy, rx, ry, 5, -std::f32::consts::FRAC_PI_2),
            true,
        ),
        ShapeKind::Triangle => (
            vec![Point::new(cx, y0), Point::new(x1, y1), Point::new(x0, y1)],
            true,
        ),
        ShapeKind::RightTriangle => (
            vec![Point::new(x0, y0), Point::new(x0, y1), Point::new(x1, y1)],
            true,
        ),
        ShapeKind::Diamond => (
            vec![
                Point::new(cx, y0),
                Point::new(x1, cy),
                Point::new(cx, y1),
                Point::new(x0, cy),
            ],
            true,
        ),
        ShapeKind::Pentagon => (
            regular_polygon(cx, cy, rx, ry, 5, -std::f32::consts::FRAC_PI_2),
            true,
        ),
        ShapeKind::Hexagon => (regular_polygon(cx, cy, rx, ry, 6, 0.0), true),
        ShapeKind::ArrowRight
        | ShapeKind::ArrowLeft
        | ShapeKind::ArrowUp
        | ShapeKind::ArrowDown => {
            let unit: [(f32, f32); 7] = [
                (0.0, 0.34),
                (0.62, 0.34),
                (0.62, 0.0),
                (1.0, 0.5),
                (0.62, 1.0),
                (0.62, 0.66),
                (0.0, 0.66),
            ];
            let pts = if kind == ShapeKind::ArrowRight {
                map_unit(&unit, r)
            } else {
                let mut p: Vec<Point> = unit
                    .iter()
                    .map(|(x, y)| {
                        let (nx, ny) = match kind {
                            ShapeKind::ArrowLeft => (1.0 - *x, *y),
                            ShapeKind::ArrowUp => (*y, *x),
                            ShapeKind::ArrowDown => (1.0 - *y, 1.0 - *x),
                            _ => (*x, *y),
                        };
                        Point::new(r.left() + r.size.width * nx, r.top() + r.size.height * ny)
                    })
                    .collect();
                if kind == ShapeKind::ArrowUp || kind == ShapeKind::ArrowDown {
                    p.reverse();
                }
                p
            };
            (pts, true)
        }
        ShapeKind::Star4 => (
            star_points(cx, cy, rx, ry, 4, -std::f32::consts::FRAC_PI_2),
            true,
        ),
        ShapeKind::Star5 => (
            star_points(cx, cy, rx, ry, 5, -std::f32::consts::FRAC_PI_2),
            true,
        ),
        ShapeKind::Star6 => (star_points(cx, cy, rx, ry, 6, 0.0), true),
        ShapeKind::Callout => {
            let mut pts = rounded_rect(
                Rect::from_xywh(x0, y0, r.size.width, r.size.height * 0.76),
                r.size.width.min(r.size.height) * 0.12,
            );
            let tail_x = x0 + r.size.width * 0.22;
            pts.push(Point::new(tail_x + r.size.width * 0.1, y1));
            pts.push(Point::new(
                tail_x + r.size.width * 0.2,
                y0 + r.size.height * 0.75,
            ));
            (pts, true)
        }
        ShapeKind::OvalCallout => {
            let mut pts = regular_polygon(
                cx,
                y0 + r.size.height * 0.38,
                rx,
                r.size.height * 0.38,
                48,
                -std::f32::consts::FRAC_PI_2,
            );
            pts.push(Point::new(x0 + r.size.width * 0.32, y1));
            pts.push(Point::new(
                x0 + r.size.width * 0.46,
                y0 + r.size.height * 0.72,
            ));
            (pts, true)
        }
        ShapeKind::Heart => {
            let mut pts = Vec::with_capacity(64);
            for i in 0..64 {
                let t = i as f32 / 64.0 * std::f32::consts::TAU;
                let hx = 16.0 * t.sin().powi(3);
                let hy = -(13.0 * t.cos()
                    - 5.0 * (2.0 * t).cos()
                    - 2.0 * (3.0 * t).cos()
                    - (4.0 * t).cos());
                let nx = 0.5 + hx / 34.0;
                let ny = 0.5 + hy / 34.0;
                pts.push(Point::new(
                    r.left() + r.size.width * nx,
                    r.top() + r.size.height * ny,
                ));
            }
            (pts, true)
        }
        ShapeKind::Lightning => {
            let unit = [
                (0.48, 0.0),
                (0.18, 0.52),
                (0.43, 0.52),
                (0.3, 1.0),
                (0.82, 0.4),
                (0.55, 0.4),
                (0.75, 0.0),
            ];
            (map_unit(&unit, r), true)
        }
    }
}

pub(crate) fn raster_shape(
    px: &mut Pixels,
    kind: ShapeKind,
    a: Point,
    b: Point,
    mode: FillMode,
    outline: Color,
    fill: Color,
    width: f32,
) {
    let (points, closed) = shape_points(kind, a, b);
    if points.len() < 2 {
        return;
    }
    if closed && mode != FillMode::Outline {
        fill_polygon(px, &points, fill);
    }
    if !closed || mode != FillMode::Filled {
        stroke_path(
            px,
            &points,
            width.max(1.0),
            outline,
            BrushKind::Brush,
            closed,
        );
    }
}

pub(crate) fn rotate_90(src: &Pixels, clockwise: bool) -> Pixels {
    let (w, h) = (src.width(), src.height());
    let mut out = Pixels::new(h, w);
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            if let Some(c) = src.get(x, y) {
                let (dx, dy) = if clockwise {
                    (h as i32 - 1 - y, x)
                } else {
                    (y, w as i32 - 1 - x)
                };
                out.set(dx, dy, c);
            }
        }
    }
    out
}

pub(crate) fn flip(src: &Pixels, horizontal: bool) -> Pixels {
    let (w, h) = (src.width(), src.height());
    let mut out = Pixels::new(w, h);
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            if let Some(c) = src.get(x, y) {
                let (dx, dy) = if horizontal {
                    (w as i32 - 1 - x, y)
                } else {
                    (x, h as i32 - 1 - y)
                };
                out.set(dx, dy, c);
            }
        }
    }
    out
}

pub(crate) fn resize_canvas(src: &Pixels, width: u32, height: u32) -> Pixels {
    let mut out = Pixels::filled(width, height, Color::WHITE);
    out.blend_from(src, 0, 0);
    out
}
