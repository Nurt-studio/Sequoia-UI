//! Drawing layers, grid, selection and guides.

use sequoia_ui::core::{BrushStop, GradientStops};
use sequoia_ui::prelude::*;
use sequoia_ui::view::{BrushStroke, Fill, Painter};

use crate::core::layer::Layer;
use crate::core::snap::Guides;
use crate::state::present::Fx;

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
    let alpha = (l.opacity / 100.0).clamp(0.0, 1.0);
    if !l.visible || alpha <= 0.0 {
        return;
    }
    let rect = Rect::from_xywh(l.x, l.y, l.w, l.h);
    let radius = if l.kind == "ELLIPSE" {
        l.w.min(l.h) * 0.5
    } else {
        l.radius
    };
    if matches!(l.kind, "FRAME" | "RECTANGLE" | "ELLIPSE") {
        draw_shadows(p, l, rect, radius, alpha);
    }
    let body = |p: &mut Painter| paint_body(p, l, rect, radius, alpha, l.font_size);
    let filter_radius = if l.kind == "TEXT" { 0.0 } else { radius };
    if l.blur > 0.0 {
        p.filter(rect, filter_radius, l.blur, body);
    } else {
        body(p);
    }
}

fn paint_body(p: &mut Painter, l: &Layer, rect: Rect, radius: f32, alpha: f32, font_size: f32) {
    let stroke = with_alpha(l.stroke, alpha);
    match l.kind {
        "FRAME" | "RECTANGLE" => {
            p.rounded(rect, radius, layer_fill(l, alpha));
            if l.stroke_width > 0.0 {
                p.rect_outline(rect, radius, BrushStroke::new(l.stroke_width), stroke);
            }
        }
        "ELLIPSE" => {
            let cx = rect.center().x;
            let cy = rect.center().y;
            let rx = rect.size.width * 0.5;
            let ry = rect.size.height * 0.5;
            if l.gradient.is_some() {
                p.rounded(rect, radius, layer_fill(l, alpha));
            } else {
                p.ellipse(Point::new(cx, cy), rx, ry, with_alpha(l.fill, alpha));
            }
            if l.stroke_width > 0.0 {
                p.ellipse_outline(
                    Point::new(cx, cy),
                    rx,
                    ry,
                    BrushStroke::new(l.stroke_width),
                    stroke,
                );
            }
        }
        "TEXT" => {
            p.text_tl(
                &l.text,
                Point::new(rect.left(), rect.top()),
                font_size,
                with_alpha(l.fill, alpha),
            );
        }
        _ => {}
    }
}

/// Solid color or gradient for one layer, with the paint-time alpha folded in.
fn layer_fill(l: &Layer, alpha: f32) -> Fill {
    let Some(g) = &l.gradient else {
        return Fill::solid(with_alpha(l.fill, alpha));
    };
    let stops: Vec<BrushStop> = g
        .sorted_stops()
        .iter()
        .map(|s| BrushStop::new(s.at, with_alpha(s.color, alpha)))
        .collect();
    let stops = GradientStops::new(stops);
    match g.kind {
        crate::model::svelte_ast::GradientKind::Linear => Fill::linear(stops, g.angle),
        crate::model::svelte_ast::GradientKind::Radial => Fill::radial(stops),
    }
}

fn draw_shadows(p: &mut Painter, l: &Layer, rect: Rect, radius: f32, alpha: f32) {
    for s in &l.effects {
        if !s.enabled || s.color.a <= 0.0 {
            continue;
        }
        p.shadow(
            rect,
            radius,
            s.blur,
            s.spread,
            Vec2::new(s.x, s.y),
            with_alpha(s.color, alpha),
        );
    }
}

/// `Color::hexa` from an existing color with a new alpha (0..1).
fn with_alpha(c: Color, alpha: f32) -> Color {
    let (r, g, b, a) = c.to_rgba8();
    let a = ((a as f32 * alpha).round().clamp(0.0, 255.0)) as u32;
    Color::hexa(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | a)
}

/// Present-mode drawing: the layer rect is scaled around its own center and
/// translated by the transition offsets; alpha multiplies every color.
pub(crate) fn draw_layer_fx(p: &mut Painter, l: &Layer, fx: &Fx) {
    if !l.visible || l.opacity <= 0.0 || fx.alpha <= 0.0 {
        return;
    }
    let alpha = fx.alpha * (l.opacity / 100.0);
    let s = fx.scale.max(0.001);
    let cx = l.x + l.w * 0.5;
    let cy = l.y + l.h * 0.5;
    let x = cx + (l.x - cx) * s + fx.dx;
    let y = cy + (l.y - cy) * s + fx.dy;
    let w = (l.w * s).max(0.5);
    let h = (l.h * s).max(0.5);
    let rect = Rect::from_xywh(x, y, w, h);
    let mut scaled = l.clone();
    scaled.effects = l
        .effects
        .iter()
        .map(|e| {
            let mut e = e.clone();
            e.x *= s;
            e.y *= s;
            e.blur *= s;
            e.spread *= s;
            e
        })
        .collect();
    scaled.blur = l.blur * s;
    let radius = if l.kind == "ELLIPSE" {
        w.min(h) * 0.5
    } else {
        l.radius * s
    };
    if matches!(l.kind, "FRAME" | "RECTANGLE" | "ELLIPSE") {
        draw_shadows(p, &scaled, rect, radius, alpha);
    }
    let body = |p: &mut Painter| paint_body(p, l, rect, radius, alpha, l.font_size * s);
    let filter_radius = if l.kind == "TEXT" { 0.0 } else { radius };
    if scaled.blur > 0.0 {
        p.filter(rect, filter_radius, scaled.blur, body);
    } else {
        body(p);
    }
}

/// The slot "hole" of a master (design doc §4.5, Phase 4): a dashed violet
/// outline with the slot name, shown while the container is empty.
pub(crate) fn draw_slot_hole(p: &mut Painter, l: &Layer) {
    let color = Color::hexa(0xCFA6F599);
    let rect = Rect::from_xywh(
        l.x + 2.0,
        l.y + 2.0,
        (l.w - 4.0).max(8.0),
        (l.h - 4.0).max(8.0),
    );
    let dash = 4.0;
    let gap = 3.0;
    let mut step = |x0: f32, y0: f32, x1: f32, y1: f32| {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 0.0 {
            return;
        }
        let ux = dx / len;
        let uy = dy / len;
        let mut t = 0.0;
        while t < len {
            let end = (t + dash).min(len);
            p.line(
                Point::new(x0 + ux * t, y0 + uy * t),
                Point::new(x0 + ux * end, y0 + uy * end),
                BrushStroke::new(1.0),
                color,
            );
            t += dash + gap;
        }
    };
    step(
        rect.left(),
        rect.top(),
        rect.left() + rect.size.width,
        rect.top(),
    );
    step(
        rect.left(),
        rect.top() + rect.size.height,
        rect.left() + rect.size.width,
        rect.top() + rect.size.height,
    );
    step(
        rect.left(),
        rect.top(),
        rect.left(),
        rect.top() + rect.size.height,
    );
    step(
        rect.left() + rect.size.width,
        rect.top(),
        rect.left() + rect.size.width,
        rect.top() + rect.size.height,
    );
    let label = format!("slot: {}", l.slot.clone().unwrap_or_default());
    let tw = p.text_width(&label, 9.0);
    p.text_tl(
        &label,
        Point::new(rect.left() + (rect.size.width - tw) * 0.5, rect.top() + 4.0),
        9.0,
        color,
    );
}

/// Green PRESENT badge in the canvas corner (design doc §7).
pub(crate) fn draw_present_badge(p: &mut Painter, size: Size) {
    let label = "● PRESENT — click elements · Esc exits";
    let tw = p.text_width(label, 10.0);
    let w = tw + 20.0;
    let h = 24.0;
    let x = (size.width - w) * 0.5;
    let y = 10.0;
    p.rounded(
        Rect::from_xywh(x, y, w, h),
        12.0,
        Fill::solid(Color::hexa(0x4ADE8026)),
    );
    p.rect_outline(
        Rect::from_xywh(x, y, w, h),
        12.0,
        BrushStroke::new(1.0),
        Color::hex(0x4ADE80),
    );
    p.text_tl(
        label,
        Point::new(x + 10.0, y + 7.0),
        10.0,
        Color::hex(0x4ADE80),
    );
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

/// Draws the text caret for the layer being edited inline.
pub(crate) fn draw_text_caret(p: &mut Painter, l: &Layer, text: &str, caret: usize, zoom: f32) {
    let mut caret = caret.min(text.len());
    while caret > 0 && !text.is_char_boundary(caret) {
        caret -= 1;
    }
    let x = l.x + p.text_width(&text[..caret], l.font_size);
    let height = l.font_size * 1.3;
    let width = 1.5 / zoom.max(0.01);
    p.fill(Rect::from_xywh(x, l.y, width, height), Fill::solid(l.fill));
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
