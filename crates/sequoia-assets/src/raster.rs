use crate::svg::{
    FillRule, Seg, Stroke, StrokeLineCap, StrokeLineJoin, SubPath, SvgIcon, SvgPoint,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sprite {
    pub width: u32,

    pub height: u32,

    pub alpha: Vec<u8>,
}

impl Sprite {
    pub fn is_blank(&self) -> bool {
        self.alpha.iter().all(|&a| a == 0)
    }
}

const SS: u32 = 8;

pub const MAX_RASTER_PX: u32 = 512;

const FLATTEN_TOL_PX: f32 = 0.15;

pub(crate) fn rasterize(icon: &SvgIcon, size_px: u32) -> Sprite {
    let dim = size_px.clamp(1, MAX_RASTER_PX);
    let size = dim as f32;
    let [vx, vy, vw, vh] = icon.viewbox;
    let (vw, vh) = (vw.max(f32::EPSILON), vh.max(f32::EPSILON));

    let s = size / vw.max(vh);

    let off_x = ((size - vw * s) * 0.5 - vx * s).round();
    let off_y = ((size - vh * s) * 0.5 - vy * s).round();
    let tx = |p: SvgPoint| SvgPoint::new(p.x * s + off_x, p.y * s + off_y);

    let mut coverage = vec![0u8; dim as usize * dim as usize];
    for shape in &icon.shapes {
        if shape.fill {
            let polys: Vec<Vec<SvgPoint>> = shape
                .subpaths
                .iter()
                .map(|sp| {
                    let transformed = transform_subpath(sp, &tx);
                    flatten_subpath(&transformed, FLATTEN_TOL_PX)
                })
                .collect();
            if !polys.is_empty() {
                let cov = shape_coverage(&polys, dim, shape.fill_rule);
                for (dst, src) in coverage.iter_mut().zip(cov) {
                    *dst = (*dst).max(src);
                }
            }
        }

        if let Some(stroke) = &shape.stroke {
            let width_px = stroke.width * s;
            if width_px.is_finite() && width_px > 1e-6 {
                let dash_px: Vec<f32> = stroke.dash_array.iter().map(|v| v * s).collect();
                let offset_px = stroke.dash_offset * s;
                let mut stroke_polys: Vec<Vec<SvgPoint>> = Vec::new();
                for sp in &shape.subpaths {
                    let transformed = transform_subpath(sp, &tx);
                    let (points, closed) = flatten_stroke_subpath(&transformed, FLATTEN_TOL_PX);
                    if points.len() < 2 {
                        if points.len() == 1 && !closed && stroke.cap == StrokeLineCap::Round {
                            stroke_polys.push(dot_poly(points[0], width_px * 0.5));
                        }
                        continue;
                    }
                    let pieces = dash_polyline(&points, closed, &dash_px, offset_px);
                    for (piece, piece_closed) in pieces {
                        if piece.len() < 2 {
                            continue;
                        }
                        stroke_polyline(&piece, piece_closed, width_px, stroke, &mut stroke_polys);
                    }
                }
                if !stroke_polys.is_empty() {
                    let cov = shape_coverage(&stroke_polys, dim, FillRule::NonZero);
                    for (dst, src) in coverage.iter_mut().zip(cov) {
                        *dst = (*dst).max(src);
                    }
                }
            }
        }
    }

    Sprite {
        width: dim,
        height: dim,
        alpha: coverage,
    }
}

fn transform_subpath(sp: &SubPath, tx: &impl Fn(SvgPoint) -> SvgPoint) -> SubPath {
    SubPath {
        start: tx(sp.start),
        segs: sp
            .segs
            .iter()
            .map(|seg| match *seg {
                Seg::Line { to } => Seg::Line { to: tx(to) },
                Seg::Quad { ctrl, to } => Seg::Quad {
                    ctrl: tx(ctrl),
                    to: tx(to),
                },
                Seg::Cubic { c1, c2, to } => Seg::Cubic {
                    c1: tx(c1),
                    c2: tx(c2),
                    to: tx(to),
                },
                Seg::Close => Seg::Close,
            })
            .collect(),
    }
}

fn flatten_subpath(sp: &SubPath, tol: f32) -> Vec<SvgPoint> {
    let mut pts = vec![sp.start];
    let mut cur = sp.start;
    for seg in &sp.segs {
        match *seg {
            Seg::Line { to } => {
                pts.push(to);
                cur = to;
            }
            Seg::Quad { ctrl, to } => {
                let err = dist_to_chord(ctrl, cur, to);
                let steps = steps_for(err, tol);
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    pts.push(eval_quad(cur, ctrl, to, t));
                }
                cur = to;
            }
            Seg::Cubic { c1, c2, to } => {
                let err = dist_to_chord(c1, cur, to).max(dist_to_chord(c2, cur, to));
                let steps = steps_for(err, tol);
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    pts.push(eval_cubic(cur, c1, c2, to, t));
                }
                cur = to;
            }
            Seg::Close => {
                if pts.last() != Some(&sp.start) {
                    pts.push(sp.start);
                }
                cur = sp.start;
            }
        }
    }
    pts
}

fn steps_for(err: f32, tol: f32) -> usize {
    ((err / tol).sqrt().ceil() as usize).clamp(1, 64)
}

fn dist_to_chord(ctrl: SvgPoint, from: SvgPoint, to: SvgPoint) -> f32 {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len2 = dx * dx + dy * dy;
    if len2 < f32::EPSILON {
        return (ctrl.x - from.x).abs() + (ctrl.y - from.y).abs();
    }
    let t = (((ctrl.x - from.x) * dx + (ctrl.y - from.y) * dy) / len2).clamp(0.0, 1.0);
    let px = from.x + t * dx;
    let py = from.y + t * dy;
    ((ctrl.x - px).powi(2) + (ctrl.y - py).powi(2)).sqrt()
}

fn eval_quad(p0: SvgPoint, c: SvgPoint, p1: SvgPoint, t: f32) -> SvgPoint {
    let u = 1.0 - t;
    SvgPoint::new(
        u * u * p0.x + 2.0 * u * t * c.x + t * t * p1.x,
        u * u * p0.y + 2.0 * u * t * c.y + t * t * p1.y,
    )
}

fn eval_cubic(p0: SvgPoint, c1: SvgPoint, c2: SvgPoint, p1: SvgPoint, t: f32) -> SvgPoint {
    let u = 1.0 - t;
    SvgPoint::new(
        u * u * u * p0.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * p1.x,
        u * u * u * p0.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * p1.y,
    )
}

fn flatten_stroke_subpath(sp: &SubPath, tol: f32) -> (Vec<SvgPoint>, bool) {
    let closed = matches!(sp.segs.last(), Some(Seg::Close));
    let mut pts = vec![sp.start];
    let mut cur = sp.start;
    for seg in &sp.segs {
        match *seg {
            Seg::Line { to } => {
                pts.push(to);
                cur = to;
            }
            Seg::Quad { ctrl, to } => {
                let err = dist_to_chord(ctrl, cur, to);
                let steps = steps_for(err, tol);
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    pts.push(eval_quad(cur, ctrl, to, t));
                }
                cur = to;
            }
            Seg::Cubic { c1, c2, to } => {
                let err = dist_to_chord(c1, cur, to).max(dist_to_chord(c2, cur, to));
                let steps = steps_for(err, tol);
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    pts.push(eval_cubic(cur, c1, c2, to, t));
                }
                cur = to;
            }
            Seg::Close => {
                cur = sp.start;
            }
        }
    }

    if closed && pts.len() >= 2 && pts.last() == Some(&sp.start) {
        pts.pop();
    }

    let mut clean: Vec<SvgPoint> = Vec::with_capacity(pts.len());
    for p in pts {
        if clean.last() != Some(&p) {
            clean.push(p);
        }
    }

    (clean, closed)
}

fn dash_polyline(
    points: &[SvgPoint],
    closed: bool,
    dash_array: &[f32],
    dash_offset: f32,
) -> Vec<(Vec<SvgPoint>, bool)> {
    let norm: Vec<f32> = dash_array
        .iter()
        .copied()
        .filter(|v| v.is_finite() && *v > 1e-4)
        .collect();
    if norm.is_empty() {
        return vec![(points.to_vec(), closed)];
    }

    let mut pattern = norm;
    if pattern.len() % 2 == 1 {
        let copy = pattern.clone();
        pattern.extend(copy);
    }
    let total: f32 = pattern.iter().sum();
    if !(total.is_finite()) || total <= 1e-4 {
        return vec![(points.to_vec(), closed)];
    }

    let mut idx = 0usize;
    let mut remaining: f32;
    let mut on: bool;
    let mut pos = if dash_offset.is_finite() {
        dash_offset.rem_euclid(total)
    } else {
        0.0
    };
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
    let started_on = on;

    let mut segs: Vec<(SvgPoint, SvgPoint)> = Vec::new();
    for w in points.windows(2) {
        if direction(w[0], w[1]).is_some() {
            segs.push((w[0], w[1]));
        }
    }
    if closed && points.len() >= 2 {
        let (a, b) = (points[points.len() - 1], points[0]);
        if direction(a, b).is_some() {
            segs.push((a, b));
        }
    }
    if segs.is_empty() {
        return Vec::new();
    }

    let mut out: Vec<(Vec<SvgPoint>, bool)> = Vec::new();
    let mut current: Vec<SvgPoint> = Vec::new();

    for (a, b) in segs {
        let seg_len = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        if !(seg_len.is_finite()) || seg_len <= 1e-6 {
            continue;
        }
        let mut travelled = 0.0f32;

        while travelled < seg_len - 1e-4 {
            let step = remaining.min(seg_len - travelled);
            let t0 = travelled / seg_len;
            let t1 = (travelled + step) / seg_len;
            let p0 = SvgPoint::new(a.x + (b.x - a.x) * t0, a.y + (b.y - a.y) * t0);
            let p1 = SvgPoint::new(a.x + (b.x - a.x) * t1, a.y + (b.y - a.y) * t1);
            if on {
                if current.is_empty() {
                    current.push(p0);
                }

                if current.last() != Some(&p1) {
                    current.push(p1);
                }
            } else if current.is_empty() {
            }
            let before = travelled;
            travelled += step;
            remaining -= step;
            if !(travelled > before) {
                break;
            }
            if remaining <= 1e-4 {
                if on {
                    if current.len() >= 2 {
                        out.push((std::mem::take(&mut current), false));
                    } else {
                        current.clear();
                    }
                } else {
                    current = Vec::new();
                }
                on = !on;
                idx = (idx + 1) % pattern.len();
                remaining = pattern[idx];

                if on && travelled < seg_len - 1e-4 {
                    current.push(p1);
                }
            }
        }
    }
    if on && current.len() >= 2 {
        out.push((current, false));
    }

    if closed && started_on && on && out.len() >= 2 {
        let first = out.remove(0);
        if let Some(last) = out.last_mut() {
            let mut merged = std::mem::take(&mut last.0);
            for p in first.0.into_iter().skip(1) {
                if merged.last() != Some(&p) {
                    merged.push(p);
                }
            }
            last.0 = merged;
        } else {
            out.push(first);
        }
    }
    out
}

fn stroke_polyline(
    points: &[SvgPoint],
    closed: bool,
    width_px: f32,
    stroke: &Stroke,
    out: &mut Vec<Vec<SvgPoint>>,
) {
    let half = width_px * 0.5;
    if !(half.is_finite()) || half <= 0.0 {
        return;
    }

    let mut segs: Vec<(SvgPoint, SvgPoint)> = Vec::new();
    for w in points.windows(2) {
        if direction(w[0], w[1]).is_some() {
            segs.push((w[0], w[1]));
        }
    }
    if closed && points.len() >= 2 {
        let (a, b) = (points[points.len() - 1], points[0]);
        if direction(a, b).is_some() {
            segs.push((a, b));
        }
    }
    if segs.is_empty() {
        return;
    }

    for &(a, b) in &segs {
        push_segment_quad(a, b, half, out);
    }

    let joint_count = if closed {
        segs.len()
    } else {
        segs.len().saturating_sub(1)
    };
    for i in 0..joint_count {
        let (_, v) = segs[i];
        let next = segs[(i + 1) % segs.len()];
        let (Some(d0), Some(d1)) = (direction(segs[i].0, v), direction(v, next.1)) else {
            continue;
        };
        push_join(v, d0, d1, half, stroke, out);
    }

    if !closed
        && stroke.cap != StrokeLineCap::Butt
        && let (Some(first), Some(last)) = (segs.first(), segs.last())
    {
        if let Some(d) = direction(first.0, first.1) {
            push_cap(first.0, SvgPoint::new(-d.x, -d.y), half, stroke.cap, out);
        }
        if let Some(d) = direction(last.0, last.1) {
            push_cap(last.1, d, half, stroke.cap, out);
        }
    }
}

fn dot_poly(center: SvgPoint, radius: f32) -> Vec<SvgPoint> {
    let steps = arc_steps(radius, std::f32::consts::TAU, FLATTEN_TOL_PX).max(8);
    let mut poly = Vec::with_capacity(steps);
    for i in 0..steps {
        let a = i as f32 / steps as f32 * std::f32::consts::TAU;
        poly.push(SvgPoint::new(
            center.x + a.cos() * radius,
            center.y + a.sin() * radius,
        ));
    }
    poly
}

fn push_segment_quad(a: SvgPoint, b: SvgPoint, half: f32, out: &mut Vec<Vec<SvgPoint>>) {
    let Some(d) = direction(a, b) else {
        return;
    };
    let n = SvgPoint::new(d.y, -d.x);
    let quad = vec![
        SvgPoint::new(a.x + n.x * half, a.y + n.y * half),
        SvgPoint::new(b.x + n.x * half, b.y + n.y * half),
        SvgPoint::new(b.x - n.x * half, b.y - n.y * half),
        SvgPoint::new(a.x - n.x * half, a.y - n.y * half),
    ];
    push_poly(quad, out);
}

fn push_join(
    v: SvgPoint,
    d0: SvgPoint,
    d1: SvgPoint,
    half: f32,
    stroke: &Stroke,
    out: &mut Vec<Vec<SvgPoint>>,
) {
    let cross = d0.x * d1.y - d0.y * d1.x;
    if cross.abs() < 1e-6 && d0.x * d1.x + d0.y * d1.y > 0.0 {
        return;
    }
    let side = if cross >= 0.0 { 1.0 } else { -1.0 };
    let n0 = SvgPoint::new(d0.y * side, -d0.x * side);
    let n1 = SvgPoint::new(d1.y * side, -d1.x * side);
    let m0 = SvgPoint::new(v.x + n0.x * half, v.y + n0.y * half);
    let m1 = SvgPoint::new(v.x + n1.x * half, v.y + n1.y * half);
    match stroke.join {
        StrokeLineJoin::Round => {
            let a0 = n0.y.atan2(n0.x);
            let a1 = n1.y.atan2(n1.x);
            let delta = wrap_pi(a1 - a0);
            let steps = arc_steps(half, delta.abs(), FLATTEN_TOL_PX).max(1);
            let mut poly = Vec::with_capacity(steps + 2);
            poly.push(v);
            for i in 0..=steps {
                let a = a0 + delta * (i as f32 / steps as f32);
                poly.push(SvgPoint::new(v.x + a.cos() * half, v.y + a.sin() * half));
            }
            push_poly(poly, out);
        }
        StrokeLineJoin::Bevel => {
            push_poly(vec![v, m0, m1], out);
        }
        StrokeLineJoin::Miter => match line_intersection(m0, d0, m1, d1) {
            Some(tip) => {
                let limit = stroke.miter_limit.max(1.0) * half;
                let dist = ((tip.x - v.x).powi(2) + (tip.y - v.y).powi(2)).sqrt();
                if dist.is_finite() && dist <= limit {
                    push_poly(vec![v, m0, tip, m1], out);
                } else {
                    push_poly(vec![v, m0, m1], out);
                }
            }
            None => push_poly(vec![v, m0, m1], out),
        },
    }
}

fn push_cap(
    end: SvgPoint,
    dir: SvgPoint,
    half: f32,
    cap: StrokeLineCap,
    out: &mut Vec<Vec<SvgPoint>>,
) {
    let len = (dir.x * dir.x + dir.y * dir.y).sqrt();
    if !(len.is_finite()) || len <= f32::MIN_POSITIVE {
        return;
    }
    let d = SvgPoint::new(dir.x / len, dir.y / len);
    let n = SvgPoint::new(d.y, -d.x);
    match cap {
        StrokeLineCap::Butt => {}
        StrokeLineCap::Square => {
            let e = SvgPoint::new(end.x + d.x * half, end.y + d.y * half);
            push_poly(
                vec![
                    SvgPoint::new(end.x + n.x * half, end.y + n.y * half),
                    SvgPoint::new(e.x + n.x * half, e.y + n.y * half),
                    SvgPoint::new(e.x - n.x * half, e.y - n.y * half),
                    SvgPoint::new(end.x - n.x * half, end.y - n.y * half),
                ],
                out,
            );
        }
        StrokeLineCap::Round => {
            let a0 = n.y.atan2(n.x);
            let steps = arc_steps(half, std::f32::consts::PI, FLATTEN_TOL_PX).max(4);
            let mut poly = Vec::with_capacity(steps + 1);
            for i in 0..=steps {
                let a = a0 + std::f32::consts::PI * (i as f32 / steps as f32);
                poly.push(SvgPoint::new(
                    end.x + a.cos() * half,
                    end.y + a.sin() * half,
                ));
            }
            push_poly(poly, out);
        }
    }
}

fn push_poly(points: Vec<SvgPoint>, out: &mut Vec<Vec<SvgPoint>>) {
    if points.len() < 3 {
        return;
    }
    let area = signed_area(&points);
    if !(area.is_finite()) || area.abs() <= 1e-9 {
        return;
    }
    if area < 0.0 {
        let mut rev = points;
        rev.reverse();
        out.push(rev);
    } else {
        out.push(points);
    }
}

fn signed_area(points: &[SvgPoint]) -> f32 {
    let mut sum = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        sum += a.x * b.y - b.x * a.y;
    }
    sum * 0.5
}

fn direction(a: SvgPoint, b: SvgPoint) -> Option<SvgPoint> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if !len.is_finite() || len <= 1e-6 {
        return None;
    }
    Some(SvgPoint::new(dx / len, dy / len))
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

fn arc_steps(radius: f32, span_rad: f32, tol: f32) -> usize {
    if !(radius.is_finite()) || !(span_rad.is_finite()) || radius <= tol {
        return 1;
    }
    let cos_half = (1.0 - tol / radius).clamp(-1.0, 1.0);
    let max_step = 2.0 * cos_half.acos();
    if !(max_step.is_finite()) || max_step <= 0.0 {
        return 1;
    }
    ((span_rad.abs() / max_step).ceil() as usize).clamp(1, 64)
}

fn line_intersection(p: SvgPoint, d: SvgPoint, q: SvgPoint, e: SvgPoint) -> Option<SvgPoint> {
    let denom = d.x * e.y - d.y * e.x;
    if !denom.is_finite() || denom.abs() < 1e-6 {
        return None;
    }
    let t = ((q.x - p.x) * e.y - (q.y - p.y) * e.x) / denom;
    let pt = SvgPoint::new(p.x + d.x * t, p.y + d.y * t);
    if pt.x.is_finite() && pt.y.is_finite() {
        Some(pt)
    } else {
        None
    }
}

fn shape_coverage(polys: &[Vec<SvgPoint>], size: u32, rule: FillRule) -> Vec<u8> {
    let mut cov = vec![0u8; (size * size) as usize];
    let step = 1.0 / SS as f32;
    let samples = (SS * SS) as f32;
    for py in 0..size {
        for px in 0..size {
            let mut inside = 0u32;
            for sy in 0..SS {
                for sx in 0..SS {
                    let p = SvgPoint::new(
                        px as f32 + (sx as f32 + 0.5) * step,
                        py as f32 + (sy as f32 + 0.5) * step,
                    );
                    let winding = winding_at(polys, p);
                    let hit = match rule {
                        FillRule::NonZero => winding != 0,
                        FillRule::EvenOdd => winding % 2 != 0,
                    };
                    if hit {
                        inside += 1;
                    }
                }
            }
            let alpha = (inside as f32 / samples * 255.0).round() as u8;
            cov[(py * size + px) as usize] = alpha;
        }
    }
    cov
}

fn winding_at(polys: &[Vec<SvgPoint>], p: SvgPoint) -> i32 {
    let mut winding = 0i32;
    for poly in polys {
        if poly.len() < 2 {
            continue;
        }
        let mut a = poly[poly.len() - 1];
        for &b in poly {
            if (a.y > p.y) != (b.y > p.y) {
                let t = (p.y - a.y) / (b.y - a.y);
                let x = a.x + t * (b.x - a.x);
                if x > p.x {
                    winding += if b.y > a.y { 1 } else { -1 };
                }
            }
            a = b;
        }
    }
    winding
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::svg::{FillRule, Shape, SubPath, parse_svg};

    fn square_sub(x: f32, y: f32, w: f32, h: f32) -> SubPath {
        SubPath {
            start: SvgPoint::new(x, y),
            segs: vec![
                Seg::Line {
                    to: SvgPoint::new(x + w, y),
                },
                Seg::Line {
                    to: SvgPoint::new(x + w, y + h),
                },
                Seg::Line {
                    to: SvgPoint::new(x, y + h),
                },
                Seg::Close,
            ],
        }
    }

    fn square_icon(x: f32, y: f32, w: f32, h: f32, rule: FillRule) -> SvgIcon {
        SvgIcon {
            viewbox: [0.0, 0.0, 10.0, 10.0],
            shapes: vec![Shape::filled(vec![square_sub(x, y, w, h)], rule)],
        }
    }

    #[test]
    fn full_square_is_opaque() {
        let sprite = rasterize(&square_icon(0.0, 0.0, 10.0, 10.0, FillRule::NonZero), 20);
        assert_eq!(sprite.width, 20);
        assert!(sprite.alpha.iter().all(|&a| a == 255));
        assert!(!sprite.is_blank());
    }

    #[test]
    fn half_square_is_about_half_coverage() {
        let sprite = rasterize(&square_icon(0.0, 0.0, 5.0, 10.0, FillRule::NonZero), 20);
        for py in 0..10u32 {
            for px in 0..20u32 {
                let a = sprite.alpha[(py * 20 + px) as usize];
                if px < 10 {
                    assert_eq!(a, 255, "left half full ({px},{py})");
                } else {
                    assert_eq!(a, 0, "right half empty ({px},{py})");
                }
            }
        }
    }

    #[test]
    fn edge_pixels_are_antialiased() {
        let sprite = rasterize(&square_icon(0.0, 0.0, 5.0, 5.0, FillRule::NonZero), 15);
        let full: u32 = sprite.alpha.iter().filter(|&&a| a == 255).count() as u32;
        let empty: u32 = sprite.alpha.iter().filter(|&&a| a == 0).count() as u32;
        let mid: u32 = sprite.alpha.iter().filter(|&&a| a > 0 && a < 255).count() as u32;
        assert!(mid > 0, "edge has intermediate pixels");
        assert_eq!(full + empty + mid, 225);

        let sum: u32 = sprite.alpha.iter().map(|&a| a as u32).sum();
        let approx = sum / 255;
        assert!(
            (52..=61).contains(&approx),
            "coverage ~= 56px, got {approx}"
        );
    }

    #[test]
    fn evenodd_makes_hole_nonzero_does_not() {
        let sub = |x: f32, y: f32, s: f32| SubPath {
            start: SvgPoint::new(x, y),
            segs: vec![
                Seg::Line {
                    to: SvgPoint::new(x + s, y),
                },
                Seg::Line {
                    to: SvgPoint::new(x + s, y + s),
                },
                Seg::Line {
                    to: SvgPoint::new(x, y + s),
                },
                Seg::Close,
            ],
        };
        let icon = SvgIcon {
            viewbox: [0.0, 0.0, 10.0, 10.0],
            shapes: vec![Shape::filled(
                vec![sub(1.0, 1.0, 8.0), sub(3.0, 3.0, 4.0)],
                FillRule::EvenOdd,
            )],
        };
        let sprite = rasterize(&icon, 20);
        let center = sprite.alpha[(10 * 20 + 10) as usize];
        assert_eq!(center, 0, "hole center empty (evenodd)");
        let edge = sprite.alpha[(2 * 20 + 2) as usize];
        assert_eq!(edge, 255, "ring full");

        let mut icon2 = icon.clone();
        icon2.shapes[0].fill_rule = FillRule::NonZero;
        let sprite2 = rasterize(&icon2, 20);
        assert_eq!(
            sprite2.alpha[(10 * 20 + 10) as usize],
            255,
            "nonzero fills the hole"
        );
    }

    #[test]
    fn circle_from_svg_rounds_corners() {
        let icon = parse_svg(r#"<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/></svg>"#)
            .unwrap();
        let sprite = rasterize(&icon, 24);
        assert_eq!(sprite.alpha[0], 0, "corner empty");
        assert_eq!(sprite.alpha[(12 * 24 + 12) as usize], 255, "center full");

        let sum: u32 = sprite.alpha.iter().map(|&a| a as u32).sum();
        let approx = sum / 255;
        assert!(
            (300..=328).contains(&approx),
            "circle area ~= 314px, got {approx}"
        );
    }

    #[test]
    fn oversize_request_is_clamped_not_oom() {
        let icon = square_icon(0.0, 0.0, 10.0, 10.0, FillRule::NonZero);
        let sprite = rasterize(&icon, u32::MAX);
        assert_eq!(
            (sprite.width, sprite.height),
            (MAX_RASTER_PX, MAX_RASTER_PX)
        );
        assert_eq!(
            sprite.alpha.len(),
            MAX_RASTER_PX as usize * MAX_RASTER_PX as usize
        );
        assert_eq!(rasterize(&icon, 0).width, 1, "zero to minimum 1");
    }

    #[test]
    fn overlapping_shapes_are_union() {
        let icon = SvgIcon {
            viewbox: [0.0, 0.0, 10.0, 10.0],
            shapes: vec![
                Shape::filled(vec![square_sub(0.0, 0.0, 6.0, 10.0)], FillRule::NonZero),
                Shape::filled(vec![square_sub(4.0, 0.0, 6.0, 10.0)], FillRule::NonZero),
            ],
        };
        let sprite = rasterize(&icon, 10);
        assert!(sprite.alpha.iter().all(|&a| a == 255), "union = full rect");
    }

    #[test]
    fn centering_keeps_aspect() {
        let icon =
            parse_svg(r#"<svg viewBox="0 0 20 10"><rect width="20" height="10"/></svg>"#).unwrap();
        let sprite = rasterize(&icon, 20);
        assert!(sprite.alpha[..20 * 5].iter().all(|&a| a == 0), "top margin");
        assert!(
            sprite.alpha[20 * 5..20 * 15].iter().all(|&a| a == 255),
            "center full"
        );
        assert!(
            sprite.alpha[20 * 15..].iter().all(|&a| a == 0),
            "bottom margin"
        );
    }

    #[test]
    fn outline_line_is_not_blank() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12 H19"/></svg>"#,
        )
        .unwrap();
        let sprite = rasterize(&icon, 24);
        assert!(!sprite.is_blank(), "outline icon must not be blank");

        assert_eq!(sprite.alpha[(12 * 24 + 12) as usize], 255, "line center");
        assert_eq!(sprite.alpha[0], 0, "corner empty");
    }

    #[test]
    fn outline_circle_renders_ring() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2"><circle cx="12" cy="12" r="9"/></svg>"#,
        )
        .unwrap();
        let sprite = rasterize(&icon, 48);
        assert!(!sprite.is_blank());

        assert_eq!(
            sprite.alpha[(24 * 48 + 24) as usize],
            0,
            "outlined circle center empty"
        );

        let top = sprite.alpha[(6 * 48 + 24) as usize];
        assert!(top > 128, "ring top covered, alpha={top}");
    }

    #[test]
    fn fill_none_without_stroke_is_blank() {
        let icon = parse_svg(r#"<svg viewBox="0 0 10 10"><path fill="none" d="M0 0 H10"/></svg>"#)
            .unwrap();
        assert!(icon.shapes.is_empty());
        let empty = SvgIcon {
            viewbox: [0.0, 0.0, 10.0, 10.0],
            shapes: Vec::new(),
        };
        assert!(rasterize(&empty, 16).is_blank());
    }

    #[test]
    fn stroke_line_element_renders() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><line x1="1" y1="5" x2="9" y2="5" stroke="black" stroke-width="1"/></svg>"#,
        )
        .unwrap();
        let sprite = rasterize(&icon, 20);
        assert!(!sprite.is_blank(), "<line> paints with stroke");
    }

    #[test]
    fn fill_and_stroke_union() {
        let icon = parse_svg(
            r#"<svg viewBox="0 0 10 10"><rect x="2" y="2" width="6" height="6" fill="black" stroke="black" stroke-width="2"/></svg>"#,
        )
        .unwrap();
        let sprite = rasterize(&icon, 20);
        assert!(!sprite.is_blank());
        assert_eq!(sprite.alpha[(10 * 20 + 10) as usize], 255, "center full");
    }

    #[test]
    fn dashed_stroke_has_gaps() {
        let solid = parse_svg(
            r#"<svg viewBox="0 0 10 10"><path fill="none" stroke="black" stroke-width="2" d="M1 5 H9"/></svg>"#,
        )
        .unwrap();
        let dashed = parse_svg(
            r#"<svg viewBox="0 0 10 10"><path fill="none" stroke="black" stroke-width="2" stroke-dasharray="2 2" d="M1 5 H9"/></svg>"#,
        )
        .unwrap();
        let s_solid = rasterize(&solid, 40);
        let s_dash = rasterize(&dashed, 40);
        let sum = |s: &crate::raster::Sprite| -> u32 { s.alpha.iter().map(|&a| a as u32).sum() };
        let (a, b) = (sum(&s_solid), sum(&s_dash));
        assert!(b > 0, "dashed not empty");
        assert!(b < a, "dashed has gaps: solid={a} dash={b}");
    }

    #[test]
    fn dash_absorption_terminates() {
        let pts = vec![SvgPoint::new(0.0, 0.0), SvgPoint::new(5.12e11, 0.0)];
        let out = dash_polyline(&pts, false, &[1.0e6, 2.0e-4, 2.0e-4, 2.0e-4], 0.0);
        assert_eq!(
            out.len(),
            1,
            "pierwszy dash 1e6 jest widoczny, reszta przerwana"
        );
    }

    #[test]
    fn round_cap_extends_beyond_butt() {
        let butt = parse_svg(
            r#"<svg viewBox="0 0 10 10"><path fill="none" stroke="black" stroke-width="2" stroke-linecap="butt" d="M2 5 H8"/></svg>"#,
        )
        .unwrap();
        let round = parse_svg(
            r#"<svg viewBox="0 0 10 10"><path fill="none" stroke="black" stroke-width="2" stroke-linecap="round" d="M2 5 H8"/></svg>"#,
        )
        .unwrap();
        let sum =
            |icon: &SvgIcon| -> u32 { rasterize(icon, 40).alpha.iter().map(|&a| a as u32).sum() };
        let (b, r) = (sum(&butt), sum(&round));
        assert!(r > b, "round cap adds semicircles: butt={b} round={r}");
    }
}
