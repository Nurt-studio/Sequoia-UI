//! Drawing layers, grid, selection and guides.

use sequoia_ui::prelude::*;
use sequoia_ui::view::{BrushStroke, Fill, Painter};

use crate::core::layer::Layer;
use crate::core::snap::Guides;

use super::{GRID_COLOR, GUIDE_COLOR, SEL_COLOR, SEL_FILL};
use crate::core::geometry::{GRID_SIZE, HANDLE_PX};

pub(crate) fn draw_grid_screen(p: &mut Painter, pan_x: f32, pan_y: f32, zoom: f32) {
    let size = p.size();
    let step = GRID_SIZE * zoom;
    if step < 8.0 {
        return;
    }
    let stroke = BrushStroke::new(1.0);
    let off_x = (-pan_x * zoom).rem_euclid(step);
    let off_y = (-pan_y * zoom).rem_euclid(step);
    let mut x = off_x;
    while x <= size.width {
        p.line(
            Point::new(x, 0.0),
            Point::new(x, size.height),
            stroke,
            GRID_COLOR,
        );
        x += step;
    }
    let mut y = off_y;
    while y <= size.height {
        p.line(
            Point::new(0.0, y),
            Point::new(size.width, y),
            stroke,
            GRID_COLOR,
        );
        y += step;
    }
}

pub(crate) fn draw_layer(p: &mut Painter, l: &Layer) {
    if !l.visible || l.opacity <= 0.0 {
        return;
    }
    let rect = Rect::from_xywh(l.x, l.y, l.w, l.h);
    let fill = Fill::solid(l.fill);
    match l.kind {
        "FRAME" | "RECTANGLE" => {
            p.rounded(rect, l.radius, fill);
            if l.stroke_width > 0.0 {
                p.rect_outline(rect, l.radius, BrushStroke::new(l.stroke_width), l.stroke);
            }
        }
        "ELLIPSE" => {
            let cx = l.x + l.w * 0.5;
            let cy = l.y + l.h * 0.5;
            let rx = l.w * 0.5;
            let ry = l.h * 0.5;
            p.ellipse(Point::new(cx, cy), rx, ry, l.fill);
            if l.stroke_width > 0.0 {
                p.ellipse_outline(
                    Point::new(cx, cy),
                    rx,
                    ry,
                    BrushStroke::new(l.stroke_width),
                    l.stroke,
                );
            }
        }
        "TEXT" => {
            p.text_tl(&l.text, Point::new(l.x, l.y), l.font_size, l.fill);
        }
        _ => {}
    }
}

/// Hairline rectangle drawn as four filled quads.
///
/// A stroked outline is a shape-cache entry per rectangle; above the cache's
/// 256-entry capacity every frame re-tessellates all outlines and frame time
/// jumps from <1 ms to >20 ms. Four quads look the same for axis-aligned
/// rectangles, use four render items instead of eighteen and skip the cache.
fn rect_hairline(p: &mut Painter, rect: Rect, width: f32, color: Color) {
    let w = width.max(0.0);
    if w <= 0.0 || rect.is_empty() {
        return;
    }
    // Strokes are centered on the edge, so the ring extends half a width out.
    let half = w * 0.5;
    let x = rect.left() - half;
    let y = rect.top() - half;
    let outer_w = rect.size.width + w;
    let outer_h = rect.size.height + w;
    let sides = (outer_h - w * 2.0).max(0.0);
    p.fill(Rect::from_xywh(x, y, outer_w, w), Fill::solid(color));
    p.fill(
        Rect::from_xywh(x, y + outer_h - w, outer_w, w),
        Fill::solid(color),
    );
    if sides > 0.0 {
        p.fill(Rect::from_xywh(x, y + w, w, sides), Fill::solid(color));
        p.fill(
            Rect::from_xywh(x + outer_w - w, y + w, w, sides),
            Fill::solid(color),
        );
    }
}

/// Draws one selected item. Resize handles are only shown for a single
/// selection; a marquee selection just outlines every item it covers.
pub(crate) fn draw_selection(p: &mut Painter, l: &Layer, zoom: f32, handles: bool) {
    let hs = HANDLE_PX / zoom;
    let rect = Rect::from_xywh(l.x, l.y, l.w, l.h);
    // `BrushStroke::screen(1.5)` is 1.5 logical px regardless of zoom.
    rect_hairline(p, rect, 1.5 / zoom.max(0.01), SEL_COLOR);
    if !handles {
        return;
    }
    let h2 = hs * 0.5;
    let pts = [
        (l.x, l.y),
        (l.x + l.w * 0.5, l.y),
        (l.x + l.w, l.y),
        (l.x + l.w, l.y + l.h * 0.5),
        (l.x + l.w, l.y + l.h),
        (l.x + l.w * 0.5, l.y + l.h),
        (l.x, l.y + l.h),
        (l.x, l.y + l.h * 0.5),
    ];
    for (hx, hy) in pts {
        let r = Rect::from_xywh(hx - h2, hy - h2, hs, hs);
        p.fill(r, Fill::solid(Color::WHITE));
        p.rect_outline(r, 0.0, BrushStroke::screen(1.2), SEL_COLOR);
    }
}

pub(crate) fn draw_marquee(p: &mut Painter, x: f32, y: f32, w: f32, h: f32) {
    p.fill(Rect::from_xywh(x, y, w, h), Fill::solid(SEL_FILL));
    p.rect_outline(
        Rect::from_xywh(x, y, w, h),
        0.0,
        BrushStroke::screen(1.0),
        SEL_COLOR,
    );
}

pub(crate) fn draw_ghost(p: &mut Painter, tool: &str, x: f32, y: f32, w: f32, h: f32) {
    let stroke = BrushStroke::screen(1.5);
    let fill = Fill::solid(SEL_FILL);
    let rect = Rect::from_xywh(x, y, w, h);
    match tool {
        "O" | "ELLIPSE" => {
            let cx = x + w * 0.5;
            let cy = y + h * 0.5;
            let rx = w * 0.5;
            let ry = h * 0.5;
            p.ellipse(Point::new(cx, cy), rx, ry, SEL_FILL);
            p.ellipse_outline(Point::new(cx, cy), rx, ry, stroke, SEL_COLOR);
        }
        "R" | "RECTANGLE" => {
            p.rounded(rect, 8.0, fill);
            p.rect_outline(rect, 8.0, stroke, SEL_COLOR);
        }
        _ => {
            p.fill(rect, fill);
            p.rect_outline(rect, 0.0, stroke, SEL_COLOR);
        }
    }
}

pub(crate) fn draw_guides(
    p: &mut Painter,
    guides: &Guides,
    wx0: f32,
    wx1: f32,
    wy0: f32,
    wy1: f32,
) {
    let stroke = BrushStroke::screen(1.0);
    for &gx in &guides.v {
        p.line(
            Point::new(gx, wy0),
            Point::new(gx, wy1),
            stroke,
            GUIDE_COLOR,
        );
    }
    for &gy in &guides.h {
        p.line(
            Point::new(wx0, gy),
            Point::new(wx1, gy),
            stroke,
            GUIDE_COLOR,
        );
    }
}
