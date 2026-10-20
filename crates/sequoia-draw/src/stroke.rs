use sequoia_core::{Point, Rect};

use crate::Span;
use crate::flatten::Contour;
use crate::raster::{self, RasterOptions};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LineCap {
    #[default]
    Butt,

    Round,

    Square,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LineJoin {
    #[default]
    Miter,

    Round,

    Bevel,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dash {
    pub array: Vec<f32>,

    pub offset: f32,
}

impl Dash {
    pub fn new(array: impl Into<Vec<f32>>) -> Self {
        Self {
            array: array.into(),
            offset: 0.0,
        }
    }

    pub fn offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }

    fn normalized(&self) -> Option<Vec<f32>> {
        let mut array: Vec<f32> = self
            .array
            .iter()
            .copied()
            .filter(|v| v.is_finite() && *v > 1e-4)
            .collect();
        if array.is_empty() {
            return None;
        }
        if array.len() % 2 == 1 {
            let copy = array.clone();
            array.extend(copy);
        }
        Some(array)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pub width: f32,

    pub cap: LineCap,

    pub join: LineJoin,

    pub miter_limit: f32,

    pub dash: Option<Dash>,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            width: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 4.0,
            dash: None,
        }
    }
}

impl StrokeStyle {
    pub fn new(width: f32) -> Self {
        Self {
            width,
            ..Self::default()
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn cap(mut self, cap: LineCap) -> Self {
        self.cap = cap;
        self
    }

    pub fn join(mut self, join: LineJoin) -> Self {
        self.join = join;
        self
    }

    pub fn miter_limit(mut self, limit: f32) -> Self {
        self.miter_limit = limit;
        self
    }

    pub fn dash(mut self, dash: impl Into<Dash>) -> Self {
        self.dash = Some(dash.into());
        self
    }

    pub fn round(width: f32) -> Self {
        Self::new(width).cap(LineCap::Round).join(LineJoin::Round)
    }

    pub(crate) fn fingerprint(&self) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |v: u64| {
            hash ^= v;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        };
        eat(u64::from(self.width.to_bits()));
        eat(match self.cap {
            LineCap::Butt => 0,
            LineCap::Round => 1,
            LineCap::Square => 2,
        });
        eat(match self.join {
            LineJoin::Miter => 0,
            LineJoin::Round => 1,
            LineJoin::Bevel => 2,
        });
        eat(u64::from(self.miter_limit.to_bits()));
        if let Some(dash) = &self.dash {
            eat(1);
            eat(u64::from(dash.offset.to_bits()));
            for v in &dash.array {
                eat(u64::from(v.to_bits()));
            }
        } else {
            eat(0);
        }
        hash
    }
}

impl From<f32> for Dash {
    fn from(array: f32) -> Self {
        Self::new(vec![array])
    }
}

impl From<Vec<f32>> for Dash {
    fn from(array: Vec<f32>) -> Self {
        Self::new(array)
    }
}

impl From<[f32; 2]> for Dash {
    fn from(array: [f32; 2]) -> Self {
        Self::new(array.to_vec())
    }
}

impl From<[f32; 4]> for Dash {
    fn from(array: [f32; 4]) -> Self {
        Self::new(array.to_vec())
    }
}

pub fn stroke(
    contours: &[Contour],
    style: &StrokeStyle,
    opts: RasterOptions,
    out: &mut Vec<Span>,
) -> Rect {
    let width = style.width;
    if !width.is_finite() || width <= 0.0 {
        return Rect::ZERO;
    }
    let outline = outline_contours(contours, style, opts.tolerance);
    if outline.is_empty() {
        return Rect::ZERO;
    }

    raster::fill_contours(&outline, crate::FillRule::NonZero, opts, out)
}

pub fn stroke_path(
    path: &crate::Path,
    style: &StrokeStyle,
    opts: RasterOptions,
    out: &mut Vec<Span>,
) -> Rect {
    let contours = path.contours(opts.tolerance);
    stroke(&contours, style, opts, out)
}

fn outline_contours(contours: &[Contour], style: &StrokeStyle, tolerance: f32) -> Vec<Contour> {
    let half = style.width * 0.5;
    let dash = style
        .dash
        .as_ref()
        .and_then(|d| d.normalized().map(|pattern| (pattern, d)));
    let mut out: Vec<Contour> = Vec::new();
    for contour in contours {
        let mut points: Vec<Point> = contour.points.clone();

        if points.len() >= 2 && points[0] == points[points.len() - 1] {
            points.pop();
        }
        if points.len() < 2 {
            continue;
        }
        let closed = contour.closed;
        let pieces: Vec<Contour> = match &dash {
            Some((pattern, d)) => dash_contour(&points, closed, pattern, d),
            None => vec![Contour { points, closed }],
        };
        for piece in &pieces {
            stroke_polyline(piece, half, style, tolerance, &mut out);
        }
    }
    out
}

fn stroke_polyline(
    piece: &Contour,
    half: f32,
    style: &StrokeStyle,
    tolerance: f32,
    out: &mut Vec<Contour>,
) {
    let pts = &piece.points;
    if pts.len() < 2 {
        return;
    }
    let mut segs: Vec<(Point, Point)> = Vec::with_capacity(pts.len());
    for w in pts.windows(2) {
        segs.push((w[0], w[1]));
    }
    if piece.closed {
        segs.push((pts[pts.len() - 1], pts[0]));
    }

    for &(a, b) in &segs {
        push_segment_quad(a, b, half, out);
    }

    let joint_count = if piece.closed {
        segs.len()
    } else {
        segs.len().saturating_sub(1)
    };
    for i in 0..joint_count {
        let (_, v) = segs[i];
        let d0 = direction(segs[i].0, v);
        let d1 = direction(v, segs[(i + 1) % segs.len()].1);
        if let (Some(d0), Some(d1)) = (d0, d1) {
            push_join(v, d0, d1, half, style, tolerance, out);
        }
    }

    if !piece.closed
        && style.cap != LineCap::Butt
        && let (Some(first), Some(last)) = (segs.first(), segs.last())
    {
        if let Some(d) = direction(first.0, first.1) {
            push_cap(
                first.0,
                Point::new(-d.x, -d.y),
                half,
                style.cap,
                tolerance,
                out,
            );
        }
        if let Some(d) = direction(last.0, last.1) {
            push_cap(last.1, d, half, style.cap, tolerance, out);
        }
    }
}

fn push_segment_quad(a: Point, b: Point, half: f32, out: &mut Vec<Contour>) {
    let Some(d) = direction(a, b) else {
        return;
    };
    let n = normal(d);
    let quad = vec![
        Point::new(a.x + n.x * half, a.y + n.y * half),
        Point::new(b.x + n.x * half, b.y + n.y * half),
        Point::new(b.x - n.x * half, b.y - n.y * half),
        Point::new(a.x - n.x * half, a.y - n.y * half),
    ];
    push_contour(quad, out);
}

fn push_join(
    v: Point,
    d0: Point,
    d1: Point,
    half: f32,
    style: &StrokeStyle,
    tolerance: f32,
    out: &mut Vec<Contour>,
) {
    let cross = d0.x * d1.y - d0.y * d1.x;
    if cross.abs() < 1e-6 && d0.x * d1.x + d0.y * d1.y > 0.0 {
        return;
    }

    let side = if cross >= 0.0 { 1.0 } else { -1.0 };
    let n0 = scale(normal(d0), side);
    let n1 = scale(normal(d1), side);
    let m0 = Point::new(v.x + n0.x * half, v.y + n0.y * half);
    let m1 = Point::new(v.x + n1.x * half, v.y + n1.y * half);

    match style.join {
        LineJoin::Round => {
            let a0 = n0.y.atan2(n0.x);
            let a1 = n1.y.atan2(n1.x);
            let delta = wrap_pi(a1 - a0);
            let steps = arc_steps_for(half, delta.abs(), tolerance);
            let mut poly = Vec::with_capacity(steps + 2);
            poly.push(v);
            for i in 0..=steps {
                let a = a0 + delta * (i as f32 / steps as f32);
                poly.push(Point::new(v.x + a.cos() * half, v.y + a.sin() * half));
            }
            push_contour(poly, out);
        }
        LineJoin::Bevel => {
            push_contour(vec![v, m0, m1], out);
        }
        LineJoin::Miter => match line_intersection(m0, d0, m1, d1) {
            Some(tip) => {
                let limit = style.miter_limit.max(1.0) * half;
                let dist = ((tip.x - v.x).powi(2) + (tip.y - v.y).powi(2)).sqrt();
                if dist <= limit {
                    push_contour(vec![v, m0, tip, m1], out);
                } else {
                    push_contour(vec![v, m0, m1], out);
                }
            }
            None => push_contour(vec![v, m0, m1], out),
        },
    }
}

fn push_cap(
    end: Point,
    dir: Point,
    half: f32,
    cap: LineCap,
    tolerance: f32,
    out: &mut Vec<Contour>,
) {
    let len = (dir.x * dir.x + dir.y * dir.y).sqrt();
    if len <= f32::MIN_POSITIVE {
        return;
    }
    let d = Point::new(dir.x / len, dir.y / len);
    let n = normal(d);
    match cap {
        LineCap::Butt => {}
        LineCap::Square => {
            let e = Point::new(end.x + d.x * half, end.y + d.y * half);
            push_contour(
                vec![
                    Point::new(end.x + n.x * half, end.y + n.y * half),
                    Point::new(e.x + n.x * half, e.y + n.y * half),
                    Point::new(e.x - n.x * half, e.y - n.y * half),
                    Point::new(end.x - n.x * half, end.y - n.y * half),
                ],
                out,
            );
        }
        LineCap::Round => {
            let a0 = n.y.atan2(n.x);
            let steps = arc_steps_for(half, std::f32::consts::PI, tolerance).max(2);
            let mut poly = Vec::with_capacity(steps + 1);
            for i in 0..=steps {
                let a = a0 + std::f32::consts::PI * (i as f32 / steps as f32);
                poly.push(Point::new(end.x + a.cos() * half, end.y + a.sin() * half));
            }
            push_contour(poly, out);
        }
    }
}

fn dash_contour(points: &[Point], closed: bool, pattern: &[f32], dash: &Dash) -> Vec<Contour> {
    let total: f32 = pattern.iter().sum();
    if total <= 1e-4 {
        return vec![Contour {
            points: points.to_vec(),
            closed,
        }];
    }

    let mut idx = 0usize;
    let mut remaining;
    let mut on;
    let mut pos = dash.offset.rem_euclid(total);
    loop {
        let len = pattern[idx];
        if pos < len {
            remaining = len - pos;
            on = idx.is_multiple_of(2);
            break;
        }
        pos -= len;
        idx = (idx + 1) % pattern.len();
    }

    let mut segs: Vec<(Point, Point)> = points.windows(2).map(|w| (w[0], w[1])).collect();
    if closed {
        segs.push((points[points.len() - 1], points[0]));
    }

    let mut out = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    for (a, b) in segs {
        if direction(a, b).is_none() {
            continue;
        };
        let seg_len = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        let mut travelled = 0.0;
        while travelled < seg_len - 1e-4 {
            let step = remaining.min(seg_len - travelled);
            travelled += step;
            remaining -= step;
            let t = travelled / seg_len;
            let p = Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
            if on {
                if current.is_empty() {
                    current.push(a);
                }
                current.push(p);
            }
            if remaining <= 1e-4 {
                if on {
                    if current.len() >= 2 {
                        out.push(Contour {
                            points: std::mem::take(&mut current),
                            closed: false,
                        });
                    }
                    current.clear();
                } else {
                    current = vec![p];
                }
                on = !on;
                idx = (idx + 1) % pattern.len();
                remaining = pattern[idx];
            }
        }
    }
    if on && current.len() >= 2 {
        out.push(Contour {
            points: current,
            closed: false,
        });
    }
    out
}

fn push_contour(points: Vec<Point>, out: &mut Vec<Contour>) {
    if points.len() < 3 {
        return;
    }
    let area = signed_area(&points);
    if area.abs() <= 1e-9 {
        return;
    }
    let points = if area < 0.0 {
        let mut rev = points;
        rev.reverse();
        rev
    } else {
        points
    };
    out.push(Contour {
        points,
        closed: false,
    });
}

fn signed_area(points: &[Point]) -> f32 {
    let mut sum = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        sum += a.x * b.y - b.x * a.y;
    }
    sum * 0.5
}

fn direction(a: Point, b: Point) -> Option<Point> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if !len.is_finite() || len <= 1e-6 {
        return None;
    }
    Some(Point::new(dx / len, dy / len))
}

fn normal(d: Point) -> Point {
    Point::new(d.y, -d.x)
}

fn scale(p: Point, s: f32) -> Point {
    Point::new(p.x * s, p.y * s)
}

fn wrap_pi(angle: f32) -> f32 {
    let two_pi = std::f32::consts::TAU;
    let mut a = angle % two_pi;
    if a > std::f32::consts::PI {
        a -= two_pi;
    } else if a <= -std::f32::consts::PI {
        a += two_pi;
    }
    a
}

fn arc_steps_for(radius: f32, span: f32, tolerance: f32) -> usize {
    crate::flatten::arc_steps(radius, span, tolerance).max(1)
}

fn line_intersection(p: Point, d: Point, q: Point, e: Point) -> Option<Point> {
    let denom = d.x * e.y - d.y * e.x;
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = ((q.x - p.x) * e.y - (q.y - p.y) * e.x) / denom;
    Some(Point::new(p.x + d.x * t, p.y + d.y * t))
}
