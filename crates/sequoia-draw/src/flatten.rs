use sequoia_core::Point;

use crate::TOLERANCE;
use crate::path::{Path, PathSeg};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Contour {
    pub points: Vec<Point>,

    pub closed: bool,
}

impl Contour {
    pub fn is_degenerate(&self) -> bool {
        self.points.len() < 2
    }

    pub fn edge_count(&self) -> usize {
        if self.closed {
            self.points.len()
        } else {
            self.points.len().saturating_sub(1)
        }
    }
}

const MAX_SUBDIVISION: u32 = 4096;

pub fn flatten(path: &Path, tolerance: f32) -> Vec<Contour> {
    let tol = if tolerance.is_finite() {
        tolerance.max(0.01)
    } else {
        TOLERANCE
    };
    let mut out: Vec<Contour> = Vec::new();
    let mut current = Contour::default();
    let mut pen = Point::ZERO;
    let mut start = Point::ZERO;
    let mut open = false;

    for seg in path.segs() {
        match *seg {
            PathSeg::Move(p) => {
                finish(&mut current, &mut out, open);
                current = Contour {
                    points: vec![p],
                    closed: false,
                };
                pen = p;
                start = p;
                open = true;
            }
            PathSeg::Line(p) => {
                if !open {
                    begin(&mut current, pen);
                    start = pen;
                    open = true;
                }
                push_point(&mut current.points, p);
                pen = p;
            }
            PathSeg::Quad(c, p) => {
                if !open {
                    begin(&mut current, pen);
                    start = pen;
                    open = true;
                }
                flatten_quad(&mut current.points, pen, c, p, tol);
                pen = p;
            }
            PathSeg::Cubic(c1, c2, p) => {
                if !open {
                    begin(&mut current, pen);
                    start = pen;
                    open = true;
                }
                flatten_cubic(&mut current.points, pen, c1, c2, p, tol);
                pen = p;
            }
            PathSeg::Close => {
                if open {
                    push_point(&mut current.points, start);
                    current.closed = true;
                    finish(&mut current, &mut out, true);
                    current = Contour::default();
                    pen = start;
                    open = false;
                }
            }
        }
    }

    finish(&mut current, &mut out, open);
    out.retain(|c| !c.is_degenerate());
    out
}

fn begin(current: &mut Contour, pen: Point) {
    current.points.clear();
    current.points.push(pen);
}

fn finish(current: &mut Contour, out: &mut Vec<Contour>, open: bool) {
    if open && current.points.len() >= 2 {
        let mut contour = std::mem::take(current);
        contour.points.dedup_by(|a, b| a == b);
        if contour.points.len() >= 2 {
            out.push(contour);
        }
    } else {
        current.points.clear();
    }
}

fn push_point(points: &mut Vec<Point>, p: Point) {
    if points.last() != Some(&p) {
        points.push(p);
    }
}

pub(crate) fn flatten_quad(out: &mut Vec<Point>, p0: Point, c: Point, p2: Point, tol: f32) {
    let mid = Point::new((p0.x + p2.x) * 0.5, (p0.y + p2.y) * 0.5);
    let dev = ((c.x - mid.x).powi(2) + (c.y - mid.y).powi(2)).sqrt() * 0.5;
    if dev <= tol {
        push_point(out, p2);
        return;
    }

    let p01 = lerp(p0, c, 0.5);
    let p12 = lerp(c, p2, 0.5);
    let m = lerp(p01, p12, 0.5);
    flatten_quad(out, p0, p01, m, tol);
    flatten_quad(out, m, p12, p2, tol);
}

pub(crate) fn flatten_cubic(
    out: &mut Vec<Point>,
    p0: Point,
    c1: Point,
    c2: Point,
    p3: Point,
    tol: f32,
) {
    let mut steps = 0;
    flatten_cubic_rec(out, p0, c1, c2, p3, tol, &mut steps);
}

fn flatten_cubic_rec(
    out: &mut Vec<Point>,
    p0: Point,
    c1: Point,
    c2: Point,
    p3: Point,
    tol: f32,
    steps: &mut u32,
) {
    if *steps >= MAX_SUBDIVISION {
        push_point(out, p3);
        return;
    }
    let d1 = dist_to_segment(c1, p0, p3);
    let d2 = dist_to_segment(c2, p0, p3);
    if (d1.max(d2)) * 0.75 <= tol {
        push_point(out, p3);
        return;
    }
    *steps += 1;
    let p01 = lerp(p0, c1, 0.5);
    let p12 = lerp(c1, c2, 0.5);
    let p23 = lerp(c2, p3, 0.5);
    let p012 = lerp(p01, p12, 0.5);
    let p123 = lerp(p12, p23, 0.5);
    let m = lerp(p012, p123, 0.5);
    flatten_cubic_rec(out, p0, p01, p012, m, tol, steps);
    flatten_cubic_rec(out, m, p123, p23, p3, tol, steps);
}

fn lerp(a: Point, b: Point, t: f32) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

pub(crate) fn dist_to_segment(p: Point, a: Point, b: Point) -> f32 {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let len2 = abx * abx + aby * aby;
    if len2 <= f32::MIN_POSITIVE {
        return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
    }
    let t = (((p.x - a.x) * abx + (p.y - a.y) * aby) / len2).clamp(0.0, 1.0);
    let dx = p.x - (a.x + abx * t);
    let dy = p.y - (a.y + aby * t);
    (dx * dx + dy * dy).sqrt()
}

pub(crate) fn arc_steps(radius: f32, span_rad: f32, tol: f32) -> usize {
    if radius <= tol || !span_rad.is_finite() {
        return 1;
    }
    let cos_half = (1.0 - tol / radius).clamp(-1.0, 1.0);
    let max_step = 2.0 * cos_half.acos();
    if max_step <= 0.0 {
        return 1;
    }
    ((span_rad.abs() / max_step).ceil() as usize).clamp(1, MAX_SUBDIVISION as usize)
}
