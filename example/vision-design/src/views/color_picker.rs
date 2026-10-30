use std::cell::Cell;
use std::rc::Rc;

use sequoia_ui::prelude::*;
use sequoia_ui::view::{BrushStroke, Fill, GradientStops};

use crate::colors;
use crate::state::{ColorPickerState, EditorState, PickerTarget};

const SV_W: f32 = 220.0;
const SV_H: f32 = 140.0;
const HUE_W: f32 = 220.0;
const HUE_H: f32 = 14.0;
const HEADER_H: f32 = 30.0;

// ------------------------------------------------------------------ math

pub(crate) fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let v = max;
    let s = if max <= 0.0 { 0.0 } else { d / max };
    let h = if d <= 0.0 {
        0.0
    } else if (max - r).abs() < 1e-6 {
        60.0 * (((g - b) / d) % 6.0)
    } else if (max - g).abs() < 1e-6 {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (if h < 0.0 { h + 360.0 } else { h }, s, v)
}

pub(crate) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let hp = ((h / 60.0) % 6.0 + 6.0) % 6.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (r1 + m, g1 + m, b1 + m)
}

pub(crate) fn parse_hex(hex: &str) -> Option<(f32, f32, f32)> {
    let h = hex.trim().trim_start_matches('#');
    match h.len() {
        3 => {
            let r = u8::from_str_radix(&h[0..1], 16).ok()? as f32 / 15.0;
            let g = u8::from_str_radix(&h[1..2], 16).ok()? as f32 / 15.0;
            let b = u8::from_str_radix(&h[2..3], 16).ok()? as f32 / 15.0;
            Some((r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&h[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&h[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&h[4..6], 16).ok()? as f32 / 255.0;
            Some((r, g, b))
        }
        _ => None,
    }
}

pub(crate) fn to_hex(r: f32, g: f32, b: f32) -> String {
    let q = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", q(r), q(g), q(b))
}

fn hue_color(h: f32) -> Color {
    let (r, g, b) = hsv_to_rgb(h, 1.0, 1.0);
    let q = |x: f32| (x * 255.0).round() as u8;
    Color::rgb8(q(r), q(g), q(b))
}

fn effective_hue(hex: &str, fallback: f32) -> f32 {
    parse_hex(hex)
        .map(|(r, g, b)| {
            let (h, s, v) = rgb_to_hsv(r, g, b);
            if s > 0.001 && v > 0.001 { h } else { fallback }
        })
        .unwrap_or(fallback)
}

fn apply_sv(pos: Point, fill: &Signal<Color>, hex: &Signal<String>, hue: &Signal<f32>) {
    let s = (pos.x / SV_W).clamp(0.0, 1.0);
    let v = 1.0 - (pos.y / SV_H).clamp(0.0, 1.0);
    let h = effective_hue(&hex.get_untracked(), hue.get_untracked());
    let (r, g, b) = hsv_to_rgb(h, s, v);
    let q = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    fill.set(Color::rgb8(q(r), q(g), q(b)));
    hex.set(to_hex(r, g, b));
    hue.set(h);
}

fn apply_hue(pos: Point, fill: &Signal<Color>, hex: &Signal<String>, hue: &Signal<f32>) {
    let new_h = ((pos.x / HUE_W) * 360.0).clamp(0.0, 360.0);
    let (_, s, v) = parse_hex(&hex.get_untracked())
        .map(|(r, g, b)| rgb_to_hsv(r, g, b))
        .unwrap_or((0.0, 1.0, 1.0));
    let s = if s < 0.001 { 1.0 } else { s };
    let v = if v < 0.001 { 1.0 } else { v };
    let (r, g, b) = hsv_to_rgb(new_h, s, v);
    let q = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    fill.set(Color::rgb8(q(r), q(g), q(b)));
    hex.set(to_hex(r, g, b));
    hue.set(new_h);
}

// ------------------------------------------------------------------ picker body widgets

fn sv_pad(fill: Signal<Color>, hex: Signal<String>, hue: Signal<f32>) -> impl View + 'static {
    let f_track = fill.clone();
    let h_track = hex.clone();
    let h_draw = hex.clone();
    let hue_draw = hue.clone();
    let f_ptr = fill.clone();
    let h_ptr = hex.clone();
    let hue_ptr = hue.clone();

    let dragging = Rc::new(Cell::new(false));
    let d_down = dragging.clone();
    let d_move = dragging.clone();
    let d_up = dragging.clone();

    Container::new()
        .width(SV_W)
        .height(SV_H)
        .radius(6.0)
        .overflow(Overflow::Hidden)
        .child(
            Canvas::new()
                .width(SV_W)
                .height(SV_H)
                .focusable(false)
                .track(move || {
                    let _ = f_track.get();
                    let _ = h_track.get();
                })
                .on_draw(move |p| {
                    let size = p.size();
                    let rect = Rect::from_xywh(0.0, 0.0, size.width, size.height);
                    let h = effective_hue(&h_draw.get_untracked(), hue_draw.get_untracked());

                    p.fill(rect, Fill::solid(hue_color(h)));
                    p.fill(
                        rect,
                        Fill::horizontal(GradientStops::two(
                            Color::WHITE,
                            Color::WHITE.with_alpha(0.0),
                        )),
                    );
                    p.fill(
                        rect,
                        Fill::vertical(GradientStops::two(Color::TRANSPARENT, Color::BLACK)),
                    );

                    if let Some((r, g, b)) = parse_hex(&h_draw.get_untracked()) {
                        let (_, s, v) = rgb_to_hsv(r, g, b);
                        let cx = s * size.width;
                        let cy = (1.0 - v) * size.height;
                        let c = Point::new(cx, cy);
                        p.circle(c, 6.0, Fill::solid(Color::WHITE));
                        p.ellipse_outline(c, 6.0, 6.0, BrushStroke::new(1.5), Color::BLACK);
                    }
                })
                .on_pointer(move |pos, ev, ctx| match ev {
                    PointerEvent::Down {
                        button: PointerButton::Left,
                        ..
                    } => {
                        d_down.set(true);
                        apply_sv(pos, &f_ptr, &h_ptr, &hue_ptr);
                        ctx.handled();
                    }
                    PointerEvent::Move { .. } if d_move.get() => {
                        apply_sv(pos, &f_ptr, &h_ptr, &hue_ptr);
                        ctx.handled();
                    }
                    PointerEvent::Up { .. } => {
                        d_up.set(false);
                        ctx.handled();
                    }
                    _ => {}
                }),
        )
}

fn hue_slider(fill: Signal<Color>, hex: Signal<String>, hue: Signal<f32>) -> impl View + 'static {
    let h_track = hex.clone();
    let hue_track = hue.clone();
    let h_draw = hex.clone();
    let hue_draw = hue.clone();
    let f_ptr = fill.clone();
    let h_ptr = hex.clone();
    let hue_ptr = hue.clone();

    let dragging = Rc::new(Cell::new(false));
    let d_down = dragging.clone();
    let d_move = dragging.clone();
    let d_up = dragging.clone();

    Container::new()
        .width(HUE_W)
        .height(HUE_H)
        .radius(7.0)
        .overflow(Overflow::Hidden)
        .child(
            Canvas::new()
                .width(HUE_W)
                .height(HUE_H)
                .focusable(false)
                .track(move || {
                    let _ = h_track.get();
                    let _ = hue_track.get();
                })
                .on_draw(move |p| {
                    let size = p.size();
                    let seg = size.width / 6.0;
                    for i in 0..6 {
                        let x0 = seg * i as f32;
                        let x1 = seg * (i as f32 + 1.0);
                        let c0 = hue_color(i as f32 * 60.0);
                        let c1 = hue_color((i as f32 + 1.0) * 60.0);
                        p.fill(
                            Rect::from_xywh(x0, 0.0, x1 - x0 + 0.5, size.height),
                            Fill::horizontal(GradientStops::two(c0, c1)),
                        );
                    }
                    let h = effective_hue(&h_draw.get_untracked(), hue_draw.get_untracked());
                    let mx = (h / 360.0) * size.width;
                    let bar = Rect::from_xywh(mx - 2.0, 0.0, 4.0, size.height);
                    p.fill(bar, Fill::solid(Color::WHITE));
                    p.rect_outline(bar, 0.0, BrushStroke::new(1.0), Color::BLACK);
                })
                .on_pointer(move |pos, ev, ctx| match ev {
                    PointerEvent::Down {
                        button: PointerButton::Left,
                        ..
                    } => {
                        d_down.set(true);
                        apply_hue(pos, &f_ptr, &h_ptr, &hue_ptr);
                        ctx.handled();
                    }
                    PointerEvent::Move { .. } if d_move.get() => {
                        apply_hue(pos, &f_ptr, &h_ptr, &hue_ptr);
                        ctx.handled();
                    }
                    PointerEvent::Up { .. } => {
                        d_up.set(false);
                        ctx.handled();
                    }
                    _ => {}
                }),
        )
}

fn picker_content(
    fill: Signal<Color>,
    hex: Signal<String>,
    hue: Signal<f32>,
) -> impl View + 'static {
    let hex_submit = hex.clone();
    let f_submit = fill.clone();
    let hue_submit = hue.clone();

    Column::new()
        .padding(10.0)
        .gap(10.0)
        .child(sv_pad(fill.clone(), hex.clone(), hue.clone()))
        .child(hue_slider(fill.clone(), hex.clone(), hue.clone()))
        .child(
            Row::new()
                .gap(8.0)
                .align_items(Align::Center)
                .child(
                    Container::new()
                        .width(28.0)
                        .height(28.0)
                        .radius(6.0)
                        .border(1.0, Color::hexa(0xFFFFFF2A))
                        .background(fill.get()),
                )
                .child_flex(
                    TextField::new(hex.clone())
                        .font_size(12.0)
                        .text_color(Color::WHITE)
                        .background(Color::hex(0x141416))
                        .border(1.0, colors::STROKE_SOFT)
                        .radius(6.0)
                        .padding(Edges::xy(8.0, 5.0))
                        .on_submit(move |text: String| {
                            if let Some((r, g, b)) = parse_hex(&text) {
                                let q = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
                                f_submit.set(Color::rgb8(q(r), q(g), q(b)));
                                hex_submit.set(to_hex(r, g, b));
                                let (h, s, v) = rgb_to_hsv(r, g, b);
                                if s > 0.001 && v > 0.001 {
                                    hue_submit.set(h);
                                }
                            }
                        }),
                    1.0,
                ),
        )
}

// ------------------------------------------------------------------ PickerHeader leaf

#[derive(Clone)]
struct PickerHeader {
    state: ColorPickerState,
}

impl Leaf for PickerHeader {
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
            .width(Length::Fill(1.0))
            .height(Length::px(HEADER_H))
    }

    fn track(&self) {
        let _ = self.state.target.get();
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let l = ctx.rect.left();
        let t = ctx.rect.top();
        let w = ctx.rect.size.width;
        let h = ctx.rect.size.height;

        ctx.quad(ctx.rect, Color::hex(0x2C2C2E), 0.0);
        ctx.quad(Rect::from_xywh(l, t + h - 1.0, w, 1.0), colors::STROKE, 0.0);

        let title = match self.state.target.get_untracked() {
            PickerTarget::Fill => "Fill Color",
            PickerTarget::Stroke => "Stroke Color",
        };
        ctx.draw_text_tl(title, (l + 10.0, t + 8.0), 12.0, Color::hex(0xD0D0D0));
        ctx.draw_text_tl("×", (l + w - 20.0, t + 4.0), 16.0, Color::hex(0xD0D0D0));
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        match event {
            Event::Pointer(PointerEvent::Down {
                button: PointerButton::Left,
                pos,
                ..
            }) => {
                let l = ctx.rect.left();
                let t = ctx.rect.top();
                let w = ctx.rect.size.width;
                let h = ctx.rect.size.height;

                if pos.x >= l + w - 24.0 && pos.y >= t && pos.y <= t + h {
                    self.state.open.set(false);
                    ctx.handled();
                    return;
                }

                let panel_pos = self.state.pos.get_untracked();
                *self.state.drag_offset.borrow_mut() =
                    Vec2::new(pos.x - panel_pos.x, pos.y - panel_pos.y);
                self.state.dragging.set(true);
                ctx.capture_pointer();
                ctx.handled();
            }
            Event::Pointer(PointerEvent::Move { pos, .. }) if self.state.dragging.get() => {
                let off = *self.state.drag_offset.borrow();
                self.state.pos.set(Vec2::new(pos.x - off.x, pos.y - off.y));
                ctx.handled();
            }
            Event::Pointer(PointerEvent::Up { .. }) if self.state.dragging.get() => {
                self.state.dragging.set(false);
                ctx.release_pointer();
                ctx.handled();
            }
            _ => {}
        }
    }

    fn has_visual_state(&self) -> bool {
        false
    }
    fn cursor(&self, _: VisualState) -> CursorIcon {
        CursorIcon::Grab
    }
}
impl_view_for_leaf!(PickerHeader);

// ------------------------------------------------------------------ ColorWell leaf

#[derive(Clone)]
struct ColorWell {
    color: Signal<Color>,
    picker: ColorPickerState,
    target: PickerTarget,
}

impl Leaf for ColorWell {
    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
            .width(Length::px(28.0))
            .height(Length::px(28.0))
    }

    fn track(&self) {
        let _ = self.color.get();
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let c = self.color.get_untracked();
        ctx.bordered_quad(ctx.rect, c, 6.0, 1.0, Color::hexa(0xFFFFFF2A));
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if let Event::Pointer(PointerEvent::Down {
            button: PointerButton::Left,
            pos,
            ..
        }) = event
        {
            let same = self.picker.open.get_untracked()
                && self.picker.target.get_untracked() == self.target;
            if same {
                self.picker.open.set(false);
            } else {
                if !self.picker.has_positioned.get() {
                    self.picker.pos.set(Vec2::new(
                        (pos.x - 280.0).max(20.0),
                        (pos.y - 20.0).max(40.0),
                    ));
                    self.picker.has_positioned.set(true);
                }
                self.picker.target.set(self.target);
                self.picker.open.set(true);
            }
            ctx.handled();
        }
    }

    fn has_visual_state(&self) -> bool {
        true
    }
    fn cursor(&self, _: VisualState) -> CursorIcon {
        CursorIcon::Pointer
    }
}
impl_view_for_leaf!(ColorWell);

// ------------------------------------------------------------------ public API

pub(crate) fn color_well(
    state: &EditorState,
    color: Signal<Color>,
    target: PickerTarget,
) -> impl View + 'static {
    ColorWell {
        color,
        picker: state.picker.clone(),
        target,
    }
}

#[derive(Clone)]
struct ColorPickerRoot {
    state: EditorState,
}

impl View for ColorPickerRoot {
    fn body(&self) -> impl View + 'static {
        let open = self.state.picker.open.clone();
        if !open.get() {
            return AnyView::new(Unit);
        }

        let pos = self.state.picker.pos.get();
        let target = self.state.picker.target.get();
        let (fill, hex) = match target {
            PickerTarget::Fill => (self.state.sel_fill.clone(), self.state.sel_fill_hex.clone()),
            PickerTarget::Stroke => (
                self.state.sel_stroke.clone(),
                self.state.sel_stroke_hex.clone(),
            ),
        };
        let hue = self.state.picker.hue.clone();

        let header = PickerHeader {
            state: self.state.picker.clone(),
        };

        AnyView::new(
            Popper::new(open.clone())
                .anchor_rect(Rect::from_xywh(pos.x, pos.y, 0.0, 0.0))
                .placement(Placement::BottomStart)
                .offset(0.0, 0.0)
                .dismiss(Dismiss::Manual)
                .child(
                    DismissableLayer::wrap(
                        Container::new()
                            .radius(8.0)
                            .background(Color::hex(0x232325))
                            .border(1.0, colors::STROKE)
                            .overflow(Overflow::Hidden)
                            .child(
                                Column::new()
                                    .child(header)
                                    .child(picker_content(fill, hex, hue)),
                            )
                            .shadow(
                                ShadowSpec::new(Color::hexa(0x00000099), 40.0).offset(0.0, 16.0),
                            ),
                    )
                    .dismiss_on_escape(true)
                    .bind(open),
                ),
        )
    }
}

pub(crate) fn color_picker(state: &EditorState) -> impl View + 'static {
    ColorPickerRoot {
        state: state.clone(),
    }
}
