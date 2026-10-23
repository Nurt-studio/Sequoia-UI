use sequoia_ui::core::{BrushStroke, Fill};
use sequoia_ui::prelude::*;
use sequoia_ui::view::Painter;

use crate::model::{Icon, ShapeKind};

fn lp(r: Rect, x: f32, y: f32) -> Point {
    Point::new(r.left() + r.size.width * x, r.top() + r.size.height * y)
}

fn poly(g: &mut Painter, r: Rect, pts: &[(f32, f32)], color: Color) {
    let points: Vec<Point> = pts.iter().map(|(x, y)| lp(r, *x, *y)).collect();
    g.polygon(&points, color);
}

fn frame(g: &mut Painter, r: Rect, color: Color, width: f32) {
    g.rect_outline(r, 0.0, BrushStroke::new(width), color);
}

fn line(g: &mut Painter, r: Rect, a: (f32, f32), b: (f32, f32), w: f32, color: Color) {
    g.line(
        lp(r, a.0, a.1),
        lp(r, b.0, b.1),
        BrushStroke::round(w),
        color,
    );
}

fn path(g: &mut Painter, r: Rect, pts: &[(f32, f32)], w: f32, color: Color) {
    let p: Vec<Point> = pts.iter().map(|(x, y)| lp(r, *x, *y)).collect();
    g.polyline(&p, BrushStroke::round(w), color);
}

fn circle(g: &mut Painter, r: Rect, cx: f32, cy: f32, rad: f32, color: Color) {
    g.circle(
        lp(r, cx, cy),
        r.size.width.min(r.size.height) * rad,
        Fill::solid(color),
    );
}

fn dot(g: &mut Painter, r: Rect, cx: f32, cy: f32, rad: f32, color: Color) {
    circle(g, r, cx, cy, rad, color);
}

fn arc(
    g: &mut Painter,
    r: Rect,
    cx: f32,
    cy: f32,
    rad: f32,
    a0: f32,
    a1: f32,
    w: f32,
    color: Color,
) {
    let n = 18;
    let mut pts = Vec::with_capacity(n + 1);
    let scale = r.size.width.min(r.size.height);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let a = a0 + (a1 - a0) * t;
        pts.push(Point::new(
            r.left() + r.size.width * cx + a.cos() * scale * rad,
            r.top() + r.size.height * cy + a.sin() * scale * rad,
        ));
    }
    g.polyline(&pts, BrushStroke::round(w), color);
}

pub(crate) fn draw_icon(g: &mut Painter, icon: Icon, r: Rect, color: Color) {
    if crate::assets::draw(g, icon, r) {
        return;
    }
    let w = (r.size.width.min(r.size.height) * 0.065).max(1.0);
    match icon {
        Icon::Save => {
            g.rounded(r, r.size.width * 0.08, Fill::solid(Color::TRANSPARENT));
            frame(g, inset(r, 0.12), color, w);
            let top = Rect::from_xywh(
                r.left() + r.size.width * 0.32,
                r.top() + r.size.height * 0.12,
                r.size.width * 0.36,
                r.size.height * 0.24,
            );
            frame(g, top, color, w);
            let bottom = Rect::from_xywh(
                r.left() + r.size.width * 0.24,
                r.top() + r.size.height * 0.56,
                r.size.width * 0.52,
                r.size.height * 0.32,
            );
            frame(g, bottom, color, w);
        }
        Icon::Undo => {
            arc(
                g,
                r,
                0.52,
                0.55,
                0.3,
                0.25,
                1.9 * std::f32::consts::PI,
                w,
                color,
            );
            poly(g, r, &[(0.18, 0.43), (0.38, 0.43), (0.28, 0.26)], color);
        }
        Icon::Redo => {
            arc(
                g,
                r,
                0.48,
                0.55,
                0.3,
                0.75 * std::f32::consts::PI,
                -0.25 * std::f32::consts::PI,
                w,
                color,
            );
            poly(g, r, &[(0.82, 0.43), (0.62, 0.43), (0.72, 0.26)], color);
        }
        Icon::Paste => {
            frame(
                g,
                Rect::from_xywh(
                    r.left() + r.size.width * 0.22,
                    r.top() + r.size.height * 0.16,
                    r.size.width * 0.56,
                    r.size.height * 0.72,
                ),
                color,
                w,
            );
            frame(
                g,
                Rect::from_xywh(
                    r.left() + r.size.width * 0.36,
                    r.top() + r.size.height * 0.08,
                    r.size.width * 0.28,
                    r.size.height * 0.16,
                ),
                color,
                w,
            );
            line(g, r, (0.34, 0.45), (0.66, 0.45), w, color);
            line(g, r, (0.34, 0.6), (0.66, 0.6), w, color);
            line(g, r, (0.34, 0.75), (0.56, 0.75), w, color);
        }
        Icon::Cut => {
            g.ring(lp(r, 0.24, 0.7), r.size.width * 0.1, w, color);
            g.ring(lp(r, 0.24, 0.3), r.size.width * 0.1, w, color);
            line(g, r, (0.32, 0.36), (0.82, 0.8), w, color);
            line(g, r, (0.32, 0.64), (0.82, 0.2), w, color);
        }
        Icon::Copy => {
            frame(
                g,
                Rect::from_xywh(
                    r.left() + r.size.width * 0.34,
                    r.top() + r.size.height * 0.12,
                    r.size.width * 0.52,
                    r.size.height * 0.58,
                ),
                color,
                w,
            );
            frame(
                g,
                Rect::from_xywh(
                    r.left() + r.size.width * 0.14,
                    r.top() + r.size.height * 0.3,
                    r.size.width * 0.52,
                    r.size.height * 0.58,
                ),
                color,
                w,
            );
        }
        Icon::Select => {
            dashed_rect(g, inset(r, 0.14), color);
        }
        Icon::Crop => {
            line(g, r, (0.2, 0.08), (0.2, 0.72), w, color);
            line(g, r, (0.08, 0.2), (0.72, 0.2), w, color);
            line(g, r, (0.8, 0.28), (0.8, 0.92), w, color);
            line(g, r, (0.28, 0.8), (0.92, 0.8), w, color);
        }
        Icon::Resize => {
            line(g, r, (0.22, 0.78), (0.78, 0.22), w * 1.2, color);
            poly(g, r, &[(0.16, 0.62), (0.16, 0.86), (0.4, 0.86)], color);
            poly(g, r, &[(0.84, 0.38), (0.84, 0.14), (0.6, 0.14)], color);
        }
        Icon::RotateLeft | Icon::RotateRight => {
            let (a0, a1) = if icon == Icon::RotateLeft {
                (0.7 * std::f32::consts::PI, 2.2 * std::f32::consts::PI)
            } else {
                (-0.2 * std::f32::consts::PI, 1.3 * std::f32::consts::PI)
            };
            arc(g, r, 0.5, 0.53, 0.3, a0, a1, w, color);
            if icon == Icon::RotateLeft {
                poly(g, r, &[(0.16, 0.5), (0.34, 0.5), (0.25, 0.34)], color);
            } else {
                poly(g, r, &[(0.84, 0.5), (0.66, 0.5), (0.75, 0.34)], color);
            }
        }
        Icon::FlipH => {
            line(
                g,
                r,
                (0.5, 0.08),
                (0.5, 0.92),
                w * 0.8,
                color.with_alpha(0.55),
            );
            poly(g, r, &[(0.1, 0.78), (0.1, 0.22), (0.42, 0.5)], color);
            poly(g, r, &[(0.9, 0.22), (0.9, 0.78), (0.58, 0.5)], color);
        }
        Icon::FlipV => {
            line(
                g,
                r,
                (0.08, 0.5),
                (0.92, 0.5),
                w * 0.8,
                color.with_alpha(0.55),
            );
            poly(g, r, &[(0.22, 0.1), (0.78, 0.1), (0.5, 0.42)], color);
            poly(g, r, &[(0.78, 0.9), (0.22, 0.9), (0.5, 0.58)], color);
        }
        Icon::Pencil => {
            line(g, r, (0.22, 0.78), (0.72, 0.28), w * 1.5, color);
            poly(g, r, &[(0.12, 0.88), (0.26, 0.66), (0.34, 0.74)], color);
            poly(
                g,
                r,
                &[(0.7, 0.22), (0.8, 0.32), (0.74, 0.38), (0.64, 0.28)],
                color,
            );
        }
        Icon::Fill => {
            poly(
                g,
                r,
                &[(0.16, 0.52), (0.5, 0.2), (0.78, 0.48), (0.44, 0.8)],
                color,
            );
            line(g, r, (0.78, 0.2), (0.62, 0.36), w, Color::WHITE);
            dot(g, r, 0.82, 0.72, 0.08, color);
        }
        Icon::Text => {
            line(g, r, (0.18, 0.18), (0.82, 0.18), w * 1.4, color);
            line(g, r, (0.5, 0.18), (0.5, 0.86), w * 1.4, color);
            line(g, r, (0.32, 0.86), (0.68, 0.86), w, color);
        }
        Icon::Eraser => {
            poly(
                g,
                r,
                &[(0.18, 0.62), (0.55, 0.2), (0.84, 0.46), (0.47, 0.88)],
                color,
            );
            line(g, r, (0.36, 0.46), (0.64, 0.72), w, Color::WHITE);
            line(g, r, (0.25, 0.72), (0.62, 0.72), w, color.with_alpha(0.4));
        }
        Icon::Picker => {
            line(g, r, (0.2, 0.8), (0.56, 0.44), w * 1.3, color);
            poly(g, r, &[(0.14, 0.86), (0.2, 0.68), (0.32, 0.8)], color);
            g.ring(lp(r, 0.66, 0.34), r.size.width * 0.16, w, color);
            line(g, r, (0.56, 0.44), (0.64, 0.36), w, color);
        }
        Icon::Magnifier | Icon::ZoomIn | Icon::ZoomOut | Icon::ZoomReset => {
            g.ring(lp(r, 0.44, 0.42), r.size.width * 0.24, w * 1.3, color);
            line(g, r, (0.62, 0.62), (0.86, 0.86), w * 1.8, color);
            if icon == Icon::ZoomIn {
                line(g, r, (0.3, 0.42), (0.58, 0.42), w, color);
                line(g, r, (0.44, 0.28), (0.44, 0.56), w, color);
            } else if icon == Icon::ZoomOut {
                line(g, r, (0.3, 0.42), (0.58, 0.42), w, color);
            } else if icon == Icon::ZoomReset {
                g.text_tl("1", lp(r, 0.36, 0.28), r.size.height * 0.34, color);
            }
        }
        Icon::Brush => {
            line(g, r, (0.2, 0.8), (0.58, 0.3), w * 2.0, color);
            poly(g, r, &[(0.12, 0.88), (0.18, 0.66), (0.32, 0.8)], color);
            poly(
                g,
                r,
                &[(0.56, 0.28), (0.74, 0.1), (0.84, 0.2), (0.68, 0.4)],
                color,
            );
        }
        Icon::Shape(kind) => draw_shape_icon(g, r, kind, color),
        Icon::Fit => {
            frame(g, inset(r, 0.12), color.with_alpha(0.6), w);
            line(g, r, (0.06, 0.06), (0.32, 0.32), w, color);
            line(g, r, (0.94, 0.06), (0.68, 0.32), w, color);
            line(g, r, (0.06, 0.94), (0.32, 0.68), w, color);
            line(g, r, (0.94, 0.94), (0.68, 0.68), w, color);
        }
        Icon::Ruler => {
            frame(
                g,
                Rect::from_xywh(
                    r.left() + r.size.width * 0.06,
                    r.top() + r.size.height * 0.32,
                    r.size.width * 0.88,
                    r.size.height * 0.36,
                ),
                color,
                w,
            );
            for i in 1..5 {
                let x = 0.06 + 0.88 * i as f32 / 5.0;
                line(g, r, (x, 0.32), (x, 0.5), w * 0.7, color);
            }
        }
        Icon::Grid => {
            frame(g, inset(r, 0.12), color, w);
            line(g, r, (0.37, 0.12), (0.37, 0.88), w * 0.6, color);
            line(g, r, (0.63, 0.12), (0.63, 0.88), w * 0.6, color);
            line(g, r, (0.12, 0.37), (0.88, 0.37), w * 0.6, color);
            line(g, r, (0.12, 0.63), (0.88, 0.63), w * 0.6, color);
        }
        Icon::StatusBar => {
            frame(g, inset(r, 0.1), color, w);
            line(g, r, (0.1, 0.7), (0.9, 0.7), w, color);
            dot(g, r, 0.25, 0.83, 0.06, color);
            dot(g, r, 0.42, 0.83, 0.06, color);
        }
        Icon::Thumbnail => {
            frame(g, inset(r, 0.1), color, w);
            poly(
                g,
                r,
                &[
                    (0.18, 0.78),
                    (0.42, 0.46),
                    (0.55, 0.62),
                    (0.66, 0.48),
                    (0.84, 0.78),
                ],
                color,
            );
            dot(g, r, 0.3, 0.3, 0.06, color);
        }
        Icon::Fullscreen => {
            line(g, r, (0.1, 0.34), (0.1, 0.1), w, color);
            line(g, r, (0.1, 0.1), (0.34, 0.1), w, color);
            line(g, r, (0.9, 0.34), (0.9, 0.1), w, color);
            line(g, r, (0.9, 0.1), (0.66, 0.1), w, color);
            line(g, r, (0.1, 0.66), (0.1, 0.9), w, color);
            line(g, r, (0.1, 0.9), (0.34, 0.9), w, color);
            line(g, r, (0.9, 0.66), (0.9, 0.9), w, color);
            line(g, r, (0.9, 0.9), (0.66, 0.9), w, color);
        }
        Icon::Check => {
            path(
                g,
                r,
                &[(0.14, 0.52), (0.4, 0.78), (0.86, 0.22)],
                w * 1.8,
                color,
            );
        }
        Icon::Close => {
            line(g, r, (0.2, 0.2), (0.8, 0.8), w * 1.4, color);
            line(g, r, (0.8, 0.2), (0.2, 0.8), w * 1.4, color);
        }
        Icon::File => {
            poly(
                g,
                r,
                &[
                    (0.24, 0.08),
                    (0.6, 0.08),
                    (0.78, 0.28),
                    (0.78, 0.92),
                    (0.24, 0.92),
                ],
                color,
            );
            path(
                g,
                r,
                &[(0.6, 0.08), (0.6, 0.28), (0.78, 0.28)],
                w,
                Color::WHITE,
            );
        }
        Icon::Folder => {
            poly(
                g,
                r,
                &[
                    (0.08, 0.24),
                    (0.42, 0.24),
                    (0.5, 0.34),
                    (0.92, 0.34),
                    (0.92, 0.82),
                    (0.08, 0.82),
                ],
                color,
            );
        }
        Icon::Print => {
            frame(
                g,
                Rect::from_xywh(
                    r.left() + r.size.width * 0.2,
                    r.top() + r.size.height * 0.4,
                    r.size.width * 0.6,
                    r.size.height * 0.3,
                ),
                color,
                w,
            );
            frame(
                g,
                Rect::from_xywh(
                    r.left() + r.size.width * 0.3,
                    r.top() + r.size.height * 0.1,
                    r.size.width * 0.4,
                    r.size.height * 0.3,
                ),
                color,
                w,
            );
            frame(
                g,
                Rect::from_xywh(
                    r.left() + r.size.width * 0.3,
                    r.top() + r.size.height * 0.7,
                    r.size.width * 0.4,
                    r.size.height * 0.2,
                ),
                color,
                w,
            );
        }
        Icon::Info => {
            g.ring(lp(r, 0.5, 0.5), r.size.width * 0.34, w, color);
            dot(g, r, 0.5, 0.32, 0.07, color);
            line(g, r, (0.5, 0.46), (0.5, 0.7), w * 1.2, color);
        }
        Icon::Properties => {
            line(g, r, (0.12, 0.25), (0.88, 0.25), w, color);
            line(g, r, (0.12, 0.5), (0.88, 0.5), w, color);
            line(g, r, (0.12, 0.75), (0.88, 0.75), w, color);
            dot(g, r, 0.34, 0.25, 0.1, color);
            dot(g, r, 0.66, 0.5, 0.1, color);
            dot(g, r, 0.42, 0.75, 0.1, color);
        }
        Icon::Wallpaper => {
            frame(
                g,
                Rect::from_xywh(
                    r.left() + r.size.width * 0.08,
                    r.top() + r.size.height * 0.14,
                    r.size.width * 0.84,
                    r.size.height * 0.6,
                ),
                color,
                w,
            );
            line(g, r, (0.36, 0.86), (0.64, 0.86), w, color);
            line(g, r, (0.5, 0.74), (0.5, 0.86), w, color);
            poly(
                g,
                r,
                &[
                    (0.2, 0.62),
                    (0.42, 0.34),
                    (0.56, 0.5),
                    (0.68, 0.38),
                    (0.8, 0.62),
                ],
                color,
            );
        }
        Icon::Size | Icon::Outline | Icon::FillDropdown | Icon::Wheel => {}
    }
}

fn inset(r: Rect, k: f32) -> Rect {
    Rect::from_xywh(
        r.left() + r.size.width * k,
        r.top() + r.size.height * k,
        r.size.width * (1.0 - k * 2.0),
        r.size.height * (1.0 - k * 2.0),
    )
}

fn dashed_rect(g: &mut Painter, r: Rect, color: Color) {
    let step = (r.size.width / 7.0).max(2.0);
    let y0 = r.top();
    let y1 = r.bottom();
    let x0 = r.left();
    let x1 = r.right();
    let mut x = x0;
    let mut on = true;
    while x < x1 {
        if on {
            let xe = (x + step).min(x1);
            g.line(
                Point::new(x, y0),
                Point::new(xe, y0),
                BrushStroke::new(1.0),
                color,
            );
            g.line(
                Point::new(x, y1),
                Point::new(xe, y1),
                BrushStroke::new(1.0),
                color,
            );
        }
        on = !on;
        x += step;
    }
    let mut y = y0;
    let mut on = true;
    while y < y1 {
        if on {
            let ye = (y + step).min(y1);
            g.line(
                Point::new(x0, y),
                Point::new(x0, ye),
                BrushStroke::new(1.0),
                color,
            );
            g.line(
                Point::new(x1, y),
                Point::new(x1, ye),
                BrushStroke::new(1.0),
                color,
            );
        }
        on = !on;
        y += step;
    }
}

fn draw_shape_icon(g: &mut Painter, r: Rect, kind: ShapeKind, color: Color) {
    let inner = Rect::from_xywh(
        r.left() + r.size.width * 0.12,
        r.top() + r.size.height * 0.18,
        r.size.width * 0.76,
        r.size.height * 0.64,
    );
    let (pts, closed) = crate::raster::shape_points(
        kind,
        inner.origin,
        Point::new(inner.right(), inner.bottom()),
    );
    let mapped: Vec<Point> = pts.iter().map(|p| Point::new(p.x, p.y)).collect();
    if closed {
        g.polygon(&mapped, color);
    } else {
        g.polyline(
            &mapped,
            BrushStroke::round((r.size.width * 0.09).max(1.0)),
            color,
        );
    }
}
