use sequoia_ui::prelude::*;

use crate::model::*;

pub const HANDLE_DIST: f32 = 24.0;
pub const HANDLE_RADIUS: f32 = 5.0;
pub const HANDLE_HIT: f32 = 10.0;

#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub w: f32,
    pub line_h: f32,
    pub lines: usize,
    pub box_w: f32,
    pub box_h: f32,
}

pub struct Raster {
    pub pixels: Pixels,
    pub offset: Vec2,
}

pub fn line_height(size: f32) -> f32 {
    size * 1.35
}

pub fn metrics(text: &str, size: f32) -> Metrics {
    let size = size.clamp(4.0, 256.0);
    let mut ts = sequoia_ui::text::TextSystem::new();
    let mut w: f32 = 0.0;
    let mut lines = 0usize;
    for line in text.split('\n') {
        lines += 1;
        w = w.max(ts.text_width(line, size));
    }
    let lines = lines.max(1);
    let w = w.max(2.0);
    let line_h = line_height(size);
    Metrics {
        w,
        line_h,
        lines,
        box_w: w + 8.0,
        box_h: line_h * lines as f32 + 4.0,
    }
}

pub fn box_rect(p: Point, m: &Metrics) -> Rect {
    Rect::from_xywh(p.x - 2.0, p.y - 2.0, m.box_w, m.box_h)
}

pub fn normalize_angle(deg: f32) -> f32 {
    let d = deg.rem_euclid(360.0);
    if d > 180.0 { d - 360.0 } else { d }
}

pub fn handle_point(center: Point, box_h: f32, angle_deg: f32) -> Point {
    let a = angle_deg.to_radians();
    let d = box_h * 0.5 + HANDLE_DIST;
    Point::new(center.x + d * a.sin(), center.y - d * a.cos())
}

pub fn root_point(center: Point, box_h: f32, angle_deg: f32) -> Point {
    let a = angle_deg.to_radians();
    let d = box_h * 0.5;
    Point::new(center.x + d * a.sin(), center.y - d * a.cos())
}

pub fn commit(state: &mut State) {
    let Some(editor) = state.text.take() else {
        return;
    };
    if editor.text.trim().is_empty() {
        state.bump();
        return;
    }
    let Some(raster) = rasterize(&editor.text, editor.size, state.color1, editor.angle_deg) else {
        state.set_status("Could not render text");
        return;
    };
    state.push_undo();
    state.doc.blend_from(
        &raster.pixels,
        (editor.pos.x + raster.offset.x).floor() as i32,
        (editor.pos.y + raster.offset.y).floor() as i32,
    );
    state.touch_doc();
    state.set_status("Text added");
}

pub fn cancel(state: &mut State) {
    if state.text.take().is_some() {
        state.bump();
    }
}

pub fn rasterize(text: &str, size: f32, color: Color, angle_deg: f32) -> Option<Raster> {
    let size = size.clamp(4.0, 256.0);
    let m = metrics(text, size);
    let box_w = m.box_w.max(1.0);
    let box_h = m.box_h.max(1.0);
    let mut box_view = Stack::new().size(Length::px(box_w), Length::px(box_h));
    for (i, line) in text.split('\n').enumerate() {
        box_view = box_view.overlay(
            Text::new(line.to_string()).size(size).color(color),
            2.0,
            2.0 + i as f32 * m.line_h,
        );
    }
    let angle = normalize_angle(angle_deg);
    let (sin, cos) = angle.to_radians().sin_cos();
    let canvas_w = (box_w * cos.abs() + box_h * sin.abs()).ceil() + 2.0;
    let canvas_h = (box_w * sin.abs() + box_h * cos.abs()).ceil() + 2.0;
    let view = Stack::new()
        .size(Length::px(canvas_w), Length::px(canvas_h))
        .overlay(
            box_view.rotate(angle),
            (canvas_w - box_w) * 0.5,
            (canvas_h - box_h) * 0.5,
        );
    let opts = sequoia_ui::export::ExportOptions::new(canvas_w, canvas_h)
        .scale(1.0)
        .transparent();
    let img = sequoia_ui::export::render_to_image(view, &opts).ok()?;
    let pivot = Vec2::new(box_w * 0.5 - 2.0, box_h * 0.5 - 2.0);
    let offset = pivot - Vec2::new(canvas_w * 0.5, canvas_h * 0.5);
    Some(Raster {
        pixels: Pixels::from_rgba(img.width(), img.height(), img.into_raw())?,
        offset,
    })
}
