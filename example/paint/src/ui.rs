use sequoia_ui::core::{BrushStroke, Fill, GradientStops};
use sequoia_ui::prelude::*;
use sequoia_ui::view::Painter;

use crate::assets;
use crate::icons::draw_icon;
use crate::layout::{self, CanvasTransform, DialogLayout, Layout};
use crate::model::*;
use crate::model::{Dialog, Icon};
use crate::raster;
use crate::text;

fn text_centered(g: &mut Painter, text: &str, center_x: f32, top: f32, size: f32, color: Color) {
    g.text_center_x(text, center_x, top, size, color);
}

fn border(g: &mut Painter, r: Rect, color: Color) {
    g.line(
        Point::new(r.left(), r.bottom()),
        Point::new(r.right(), r.bottom()),
        BrushStroke::new(1.0),
        color,
    );
}

pub(crate) fn paint(g: &mut Painter, state: &State, l: &Layout, pointer: Point) {
    g.fill(l.size_rect(), Fill::solid(COL_BG));

    if !state.view.fullscreen {
        paint_qat(g, state, l, pointer);
        paint_tabs(g, state, l, pointer);
        paint_ribbon(g, state, l, pointer);
    }
    paint_workspace(g, state, l, pointer);
    if let Some(bs) = l.backstage {
        paint_backstage(g, state, l, bs, pointer);
    }
    if !state.view.fullscreen && state.view.status {
        paint_status(g, state, l, pointer);
    }
    if state.dialog.is_some() {
        paint_dialog(g, state, l, pointer);
    }
}

trait LayoutExt {
    fn size_rect(&self) -> Rect;
}

impl LayoutExt for Layout {
    fn size_rect(&self) -> Rect {
        Rect::from_xywh(0.0, 0.0, self.size.width, self.size.height)
    }
}

fn paint_qat(g: &mut Painter, state: &State, l: &Layout, pointer: Point) {
    g.fill(l.qat, Fill::solid(Color::WHITE));
    assets::draw_named(g, "app", l.app_icon);
    for c in l.controls.iter().filter(|c| c.style == CtlStyle::Qat) {
        draw_control(g, state, c, pointer);
    }
    assets::draw_named(g, "caret", l.qat_caret);

    let title = if state.dirty {
        format!("{}* - Paint", state.font_file_name())
    } else {
        format!("{} - Paint", state.font_file_name())
    };
    let center = (l.qat_caret.right() + l.caption_min.left()) * 0.5;
    text_centered(g, &title, center, l.qat.top() + 8.0, 12.0, COL_TEXT);

    let close_hover = l.caption_close.contains(pointer);
    for (rect, name, hover_color) in [
        (l.caption_min, "min", Color::hex(0xE5E5E5)),
        (l.caption_max, "max", Color::hex(0xE5E5E5)),
        (l.caption_close, "close", Color::hex(0xE81123)),
    ] {
        let hover = rect.contains(pointer);
        if hover {
            g.fill(rect, Fill::solid(hover_color));
        }
        let icon_rect = Rect::from_xywh(rect.center().x - 5.0, rect.center().y - 5.0, 10.0, 10.0);
        if rect == l.caption_close && close_hover {
            assets::draw_named(g, "close_white", icon_rect);
        } else {
            assets::draw_named(g, name, icon_rect);
        }
    }
}

fn paint_tabs(g: &mut Painter, state: &State, l: &Layout, pointer: Point) {
    g.fill(l.tabs, Fill::solid(Color::hex(0xF0F0F0)));

    let file_hover = l.tab_file.contains(pointer);
    let (top, bottom) = if file_hover {
        (Color::hex(0x54A9EA), Color::hex(0x2678CF))
    } else {
        (Color::hex(0x47A0E6), Color::hex(0x1E6FC4))
    };
    g.rounded(
        l.tab_file,
        3.0,
        Fill::vertical(GradientStops::two(top, bottom)),
    );
    g.rect_outline(l.tab_file, 3.0, BrushStroke::new(1.0), Color::hex(0x1763AF));
    text_centered(
        g,
        "File",
        l.tab_file.center().x,
        l.tab_file.top() + 5.0,
        12.0,
        Color::WHITE,
    );

    for (rect, tab, label) in [
        (l.tab_home, Tab::Home, "Home"),
        (l.tab_view, Tab::View, "View"),
    ] {
        let active = !state.backstage && state.tab == tab;
        if active {
            g.fill(rect, Fill::solid(Color::hex(0xF6F7F8)));
            g.line(
                Point::new(rect.left(), rect.top()),
                Point::new(rect.right(), rect.top()),
                BrushStroke::new(1.0),
                Color::hex(0xD6D6D6),
            );
            g.line(
                Point::new(rect.left(), rect.top()),
                Point::new(rect.left(), rect.bottom()),
                BrushStroke::new(1.0),
                Color::hex(0xD6D6D6),
            );
            g.line(
                Point::new(rect.right(), rect.top()),
                Point::new(rect.right(), rect.bottom()),
                BrushStroke::new(1.0),
                Color::hex(0xD6D6D6),
            );
        }
        let color = if active {
            Color::hex(0x202020)
        } else if rect.contains(pointer) {
            Color::BLACK
        } else {
            Color::hex(0x2D2D2D)
        };
        text_centered(g, label, rect.center().x, rect.top() + 5.0, 12.0, color);
    }
}

fn paint_ribbon(g: &mut Painter, state: &State, l: &Layout, pointer: Point) {
    g.fill(l.ribbon, Fill::solid(Color::hex(0xF6F7F8)));
    g.line(
        Point::new(l.ribbon.left(), l.ribbon.bottom() - 0.5),
        Point::new(l.ribbon.right(), l.ribbon.bottom() - 0.5),
        BrushStroke::new(1.0),
        Color::hex(0xD6D6D6),
    );
    if state.backstage {
        return;
    }
    for (i, group) in l.groups.iter().enumerate() {
        if i > 0 {
            g.line(
                Point::new(group.rect.left(), l.ribbon.top() + 5.0),
                Point::new(group.rect.left(), l.ribbon.bottom() - 3.0),
                BrushStroke::new(1.0),
                Color::hex(0xDCDCDC),
            );
        }
        text_centered(
            g,
            group.label,
            group.rect.center().x,
            l.ribbon.top() + 81.0,
            11.0,
            Color::hex(0x5D5D5D),
        );
    }
    for c in &l.controls {
        if c.rect.top() < l.ribbon.top() || c.rect.bottom() > l.ribbon.bottom() {
            continue;
        }
        draw_control(g, state, c, pointer);
    }
}

fn draw_control(g: &mut Painter, _state: &State, c: &Control, pointer: Point) {
    let hover = c.rect.contains(pointer);
    let hover_fill = Color::hex(0xE3F1FB);
    let hover_line = Color::hex(0xA8CFEE);
    let pressed_fill = Color::hex(0xBBDCF5);
    let pressed_line = Color::hex(0x5B9BD5);

    match c.style {
        CtlStyle::Qat => {
            if hover {
                g.rounded(c.rect, 2.0, Fill::solid(Color::hex(0xE9F3FC)));
                g.rect_outline(c.rect, 2.0, BrushStroke::new(1.0), hover_line);
            }
            draw_icon(g, c.icon, c.rect, Color::hex(0x1A1A1A));
        }
        CtlStyle::BigText => {
            if hover {
                g.rounded(c.rect, 2.0, Fill::solid(hover_fill));
            } else if c.selected {
                g.rounded(c.rect, 2.0, Fill::solid(pressed_fill));
            } else {
                g.rounded(
                    c.rect,
                    2.0,
                    Fill::vertical(GradientStops::two(Color::WHITE, Color::hex(0xF3F4F5))),
                );
            }
            g.rect_outline(
                c.rect,
                2.0,
                BrushStroke::new(1.0),
                if c.selected {
                    pressed_line
                } else if hover {
                    Color::hex(0x7CB8E8)
                } else {
                    Color::hex(0xD9D9D9)
                },
            );
            let content_h = 46.0;
            let top = c.rect.top() + ((c.rect.size.height - content_h) * 0.5).max(0.0);
            let icon_rect = Rect::from_xywh(c.rect.center().x - 20.0, top, 40.0, 30.0);
            draw_icon(g, c.icon, icon_rect, Color::hex(0x1A1A1A));
            if let Some(label) = c.label {
                draw_label_with_arrow(
                    g,
                    label,
                    c.arrow,
                    c.rect.center().x,
                    top + 33.0,
                    11.0,
                    Color::hex(0x1F1F1F),
                );
            }
        }
        CtlStyle::SmallText => {
            if hover {
                g.rounded(c.rect, 2.0, Fill::solid(hover_fill));
                g.rect_outline(c.rect, 2.0, BrushStroke::new(1.0), hover_line);
            }
            let icon_rect =
                Rect::from_xywh(c.rect.left() + 4.0, c.rect.center().y - 8.0, 16.0, 16.0);
            draw_icon(g, c.icon, icon_rect, Color::hex(0x1F1F1F));
            if let Some(label) = c.label {
                let text_x = icon_rect.right() + 3.0;
                g.text_tl(
                    label,
                    Point::new(text_x, c.rect.center().y - 6.5),
                    11.0,
                    Color::hex(0x1F1F1F),
                );
                if c.arrow {
                    let width = g.text_width(label, 11.0);
                    assets::draw_named(
                        g,
                        "caret",
                        Rect::from_xywh(text_x + width + 3.0, c.rect.center().y - 2.5, 7.0, 7.0),
                    );
                }
            }
        }
        CtlStyle::Tool => {
            if c.selected {
                g.rounded(c.rect, 2.0, Fill::solid(pressed_fill));
                g.rect_outline(c.rect, 2.0, BrushStroke::new(1.0), pressed_line);
            } else if hover {
                g.rounded(c.rect, 2.0, Fill::solid(hover_fill));
                g.rect_outline(c.rect, 2.0, BrushStroke::new(1.0), hover_line);
            }
            let icon_rect =
                Rect::from_xywh(c.rect.center().x - 9.5, c.rect.center().y - 9.5, 19.0, 19.0);
            draw_icon(g, c.icon, icon_rect, Color::hex(0x1F1F1F));
        }
        CtlStyle::Shape => {
            if c.selected {
                g.rounded(c.rect, 1.0, Fill::solid(pressed_fill));
            } else if hover {
                g.rounded(c.rect, 1.0, Fill::solid(hover_fill));
            }
            let icon_rect =
                Rect::from_xywh(c.rect.center().x - 8.5, c.rect.center().y - 8.5, 17.0, 17.0);
            draw_icon(g, c.icon, icon_rect, Color::hex(0x3A6EA5));
        }
        CtlStyle::SizeButton => {
            if hover {
                g.rounded(c.rect, 2.0, Fill::solid(hover_fill));
                g.rect_outline(c.rect, 2.0, BrushStroke::new(1.0), hover_line);
            }
            let icon_rect =
                Rect::from_xywh(c.rect.center().x - 13.0, c.rect.top() + 15.0, 26.0, 26.0);
            draw_icon(g, c.icon, icon_rect, Color::hex(0x56606A));
            assets::draw_named(
                g,
                "caret",
                Rect::from_xywh(c.rect.center().x - 3.5, icon_rect.bottom() + 3.0, 7.0, 7.0),
            );
        }
        CtlStyle::Color1 | CtlStyle::Color2 => {
            if hover {
                g.rounded(c.rect, 2.0, Fill::solid(hover_fill));
            } else if c.selected {
                g.rounded(c.rect, 2.0, Fill::solid(Color::hex(0xE8F2FC)));
            } else {
                g.rounded(c.rect, 2.0, Fill::solid(Color::WHITE));
            }
            g.rect_outline(
                c.rect,
                2.0,
                BrushStroke::new(1.0),
                if c.selected {
                    pressed_line
                } else if hover {
                    Color::hex(0x7CB8E8)
                } else {
                    Color::hex(0xCFCFCF)
                },
            );
            if let Some(color) = c.swatch {
                if c.style == CtlStyle::Color1 {
                    let swatch =
                        Rect::from_xywh(c.rect.center().x - 16.0, c.rect.top() + 11.0, 32.0, 30.0);
                    g.fill(swatch, Fill::solid(color));
                    g.rect_outline(
                        swatch,
                        0.0,
                        BrushStroke::new(1.0),
                        Color::BLACK.with_alpha(0.45),
                    );
                } else {
                    let frame =
                        Rect::from_xywh(c.rect.center().x - 16.0, c.rect.top() + 11.0, 32.0, 30.0);
                    g.fill(frame, Fill::solid(Color::WHITE));
                    g.rect_outline(
                        frame,
                        0.0,
                        BrushStroke::new(1.0),
                        Color::BLACK.with_alpha(0.35),
                    );
                    let inner = Rect::from_xywh(frame.left() + 7.0, frame.top() + 7.0, 21.0, 19.0);
                    g.fill(inner, Fill::solid(color));
                    g.rect_outline(
                        inner,
                        0.0,
                        BrushStroke::new(1.0),
                        Color::BLACK.with_alpha(0.45),
                    );
                }
            }
            if let Some(label) = c.label {
                text_centered(
                    g,
                    label,
                    c.rect.center().x,
                    c.rect.top() + 45.0,
                    11.0,
                    Color::hex(0x1F1F1F),
                );
            }
        }
        CtlStyle::PaletteCell => {
            if let Some(color) = c.swatch {
                let cell = inset(c.rect, 0.5);
                g.fill(cell, Fill::solid(color));
                g.rect_outline(
                    cell,
                    0.0,
                    BrushStroke::new(1.0),
                    Color::hex(0xC4C4C4).with_alpha(0.8),
                );
            }
            if hover {
                g.rect_outline(c.rect, 0.0, BrushStroke::new(2.0), Color::hex(0xF8B84A));
            }
        }
        CtlStyle::EditColors => {
            if hover {
                g.rounded(c.rect, 2.0, Fill::solid(hover_fill));
            } else {
                g.rounded(c.rect, 2.0, Fill::solid(Color::WHITE));
            }
            g.rect_outline(
                c.rect,
                2.0,
                BrushStroke::new(1.0),
                if hover {
                    Color::hex(0x7CB8E8)
                } else {
                    Color::hex(0xCFCFCF)
                },
            );
            draw_icon(
                g,
                c.icon,
                Rect::from_xywh(c.rect.center().x - 15.0, c.rect.top() + 10.0, 30.0, 30.0),
                Color::hex(0x1F1F1F),
            );
            if let Some(label) = c.label {
                text_centered(
                    g,
                    label,
                    c.rect.center().x,
                    c.rect.top() + 45.0,
                    10.5,
                    Color::hex(0x1F1F1F),
                );
            }
        }
        CtlStyle::Checkbox => {
            if hover {
                g.rounded(c.rect, 2.0, Fill::solid(hover_fill));
                g.rect_outline(c.rect, 2.0, BrushStroke::new(1.0), hover_line);
            }
            let box_rect =
                Rect::from_xywh(c.rect.left() + 5.0, c.rect.center().y - 7.0, 14.0, 14.0);
            g.fill(box_rect, Fill::solid(Color::WHITE));
            g.rect_outline(box_rect, 0.0, BrushStroke::new(1.0), Color::hex(0x5F6B76));
            if c.check == Some(true) {
                g.polyline(
                    &[
                        Point::new(box_rect.left() + 3.4, box_rect.top() + 6.2),
                        Point::new(box_rect.left() + 5.8, box_rect.top() + 8.8),
                        Point::new(box_rect.left() + 10.4, box_rect.top() + 3.4),
                    ],
                    BrushStroke::new(1.6),
                    Color::hex(0x2F74C0),
                );
            }
            if let Some(label) = c.label {
                g.text_tl(
                    label,
                    Point::new(box_rect.right() + 6.0, c.rect.center().y - 6.5),
                    11.0,
                    Color::hex(0x1F1F1F),
                );
            }
        }
        CtlStyle::SpinUp | CtlStyle::SpinDown => {
            if hover {
                g.fill(c.rect, Fill::solid(hover_fill));
            }
            let cx = c.rect.center().x;
            let cy = c.rect.center().y;
            let points = if c.style == CtlStyle::SpinUp {
                vec![
                    Point::new(cx - 4.0, cy + 2.5),
                    Point::new(cx + 4.0, cy + 2.5),
                    Point::new(cx, cy - 2.5),
                ]
            } else {
                vec![
                    Point::new(cx - 4.0, cy - 2.5),
                    Point::new(cx + 4.0, cy - 2.5),
                    Point::new(cx, cy + 2.5),
                ]
            };
            g.polygon(&points, Color::hex(0x56606A));
        }
        CtlStyle::Plain => {
            if hover {
                g.rounded(c.rect, 2.0, Fill::solid(hover_fill));
            }
            draw_icon(g, c.icon, inset(c.rect, 3.0), Color::hex(0x1F1F1F));
        }
    }
}

fn draw_label_with_arrow(
    g: &mut Painter,
    label: &str,
    arrow: bool,
    center_x: f32,
    top: f32,
    size: f32,
    color: Color,
) {
    let text_width = g.text_width(label, size);
    let arrow_width = if arrow { 7.0 + 4.0 } else { 0.0 };
    let left = center_x - (text_width + arrow_width) * 0.5;
    g.text_tl(label, Point::new(left, top), size, color);
    if arrow {
        assets::draw_named(
            g,
            "caret",
            Rect::from_xywh(left + text_width + 4.0, top + 3.0, 7.0, 7.0),
        );
    }
}

fn inset(r: Rect, k: f32) -> Rect {
    Rect::from_xywh(
        r.left() + k,
        r.top() + k,
        (r.size.width - k * 2.0).max(0.0),
        (r.size.height - k * 2.0).max(0.0),
    )
}

fn paint_workspace(g: &mut Painter, state: &State, l: &Layout, pointer: Point) {
    g.fill(l.workspace, Fill::solid(Color::WHITE));
    if state.view.rulers && !state.view.fullscreen {
        paint_rulers(g, state, l);
    }
    g.fill(l.viewport, Fill::solid(Color::WHITE));

    g.clipped(l.viewport, |g| {
        g.fill(l.canvas, Fill::solid(Color::WHITE));
        g.pixels(l.canvas, &state.doc);
        if state.view.grid {
            paint_grid(g, state, l);
        }
        paint_selection_and_preview(g, state, l);
        paint_text_preview(g, state, l);
    });
    g.rect_outline(l.canvas, 0.0, BrushStroke::new(1.0), Color::hex(0xB9B9B9));

    if let Some(v) = l.vscroll {
        paint_scrollbar(g, v, true, state, l, pointer);
    }
    if let Some(h) = l.hscroll {
        paint_scrollbar(g, h, false, state, l, pointer);
    }
    if let Some(panel) = l.thumb_panel {
        g.fill(
            Rect::from_xywh(
                panel.left() + 3.0,
                panel.top() + 3.0,
                panel.size.width,
                panel.size.height,
            ),
            Fill::solid(Color::BLACK.with_alpha(0.18)),
        );
        g.fill(panel, Fill::solid(Color::WHITE));
        g.rect_outline(panel, 0.0, BrushStroke::new(1.0), Color::hex(0x9A9A9A));
        g.text_tl(
            "Thumbnail",
            Point::new(panel.left() + 4.0, panel.top() + 4.0),
            11.0,
            Color::hex(0x555555),
        );
        if let Some(tr) = l.thumb_image
            && let Some(thumb) = state
                .doc
                .scaled_to_fit(tr.size.width.max(tr.size.height) as u32)
        {
            let w = thumb.width() as f32;
            let h = thumb.height() as f32;
            let scale = (tr.size.width / w).min(tr.size.height / h);
            let r = Rect::from_xywh(
                tr.center().x - w * scale * 0.5,
                tr.center().y - h * scale * 0.5,
                w * scale,
                h * scale,
            );
            g.fill(r, Fill::solid(Color::WHITE));
            g.pixels(r, &thumb);
        }
    }
}

fn paint_grid(g: &mut Painter, state: &State, l: &Layout) {
    let zoom = state.zoom();
    if zoom * 10.0 < 5.0 {
        return;
    }
    let step = 10.0 * zoom;
    let color = Color::BLACK.with_alpha(0.13);
    let start_x = ((l.viewport.left() - l.canvas.left()) / step)
        .ceil()
        .max(0.0) as i32;
    let end_x = (((l.viewport.right() - l.canvas.left()) / step).floor() as i32)
        .min((state.doc.width() as f32 / 10.0) as i32);
    for i in start_x..=end_x {
        let x = (l.canvas.left() + i as f32 * step).round();
        g.line(
            Point::new(x, l.canvas.top()),
            Point::new(x, l.canvas.bottom()),
            BrushStroke::new(1.0),
            color,
        );
    }
    let start_y = ((l.viewport.top() - l.canvas.top()) / step).ceil().max(0.0) as i32;
    let end_y = (((l.viewport.bottom() - l.canvas.top()) / step).floor() as i32)
        .min((state.doc.height() as f32 / 10.0) as i32);
    for i in start_y..=end_y {
        let y = (l.canvas.top() + i as f32 * step).round();
        g.line(
            Point::new(l.canvas.left(), y),
            Point::new(l.canvas.right(), y),
            BrushStroke::new(1.0),
            color,
        );
    }
}

fn doc_to_screen(l: &Layout, state: &State, p: Point) -> Point {
    CanvasTransform::new(l, state).to_screen(p)
}

fn paint_selection_and_preview(g: &mut Painter, state: &State, l: &Layout) {
    let transform = CanvasTransform::new(l, state);
    let zoom = transform.zoom;
    if let Some(sel) = &state.selection {
        let r = transform.rect_to_screen(sel.rect.as_rect());
        if let Some(pix) = &sel.floating {
            g.pixels(r, pix);
        }
        ants(g, r);
    }
    if let Drag::Select { start } = &state.drag {
        let r = screen_rect_between(l, state, *start, state.pointer);
        ants(g, r);
    }
    if let Drag::Shape { start, current, .. } = &state.drag {
        let (points, closed) = raster::shape_points(state.shape, *start, *current);
        let screen: Vec<Point> = points.iter().map(|p| doc_to_screen(l, state, *p)).collect();
        let width = (state.width() * zoom).max(1.0);
        if closed && state.fill != FillMode::Outline {
            g.polygon(&screen, state.color2);
        }
        if !closed || state.fill != FillMode::Filled {
            if closed {
                let mut all = screen.clone();
                if let Some(first) = screen.first() {
                    all.push(*first);
                }
                g.polyline(&all, BrushStroke::round(width), state.color1);
            } else {
                g.polyline(&screen, BrushStroke::round(width), state.color1);
            }
        }
    }
}

fn screen_rect_between(l: &Layout, state: &State, a: Point, b: Point) -> Rect {
    let transform = CanvasTransform::new(l, state);
    let p0 = transform.to_screen(Point::new(a.x.min(b.x), a.y.min(b.y)));
    let p1 = transform.to_screen(Point::new(a.x.max(b.x), a.y.max(b.y)));
    Rect::from_xywh(p0.x, p0.y, (p1.x - p0.x).max(1.0), (p1.y - p0.y).max(1.0))
}

fn ants(g: &mut Painter, r: Rect) {
    let step = 5.0;
    dashed(
        g,
        Point::new(r.left(), r.top()),
        Point::new(r.right(), r.top()),
        step,
        Color::BLACK,
    );
    dashed(
        g,
        Point::new(r.left(), r.bottom()),
        Point::new(r.right(), r.bottom()),
        step,
        Color::WHITE,
    );
    dashed(
        g,
        Point::new(r.left(), r.top()),
        Point::new(r.left(), r.bottom()),
        step,
        Color::WHITE,
    );
    dashed(
        g,
        Point::new(r.right(), r.top()),
        Point::new(r.right(), r.bottom()),
        step,
        Color::BLACK,
    );
}

fn dashed(g: &mut Painter, a: Point, b: Point, step: f32, first: Color) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return;
    }
    let n = (len / step).ceil() as i32;
    for i in 0..n {
        if i % 2 == 1 {
            continue;
        }
        let t0 = i as f32 / n as f32;
        let t1 = ((i + 1) as f32 / n as f32).min(1.0);
        g.line(
            Point::new(a.x + dx * t0, a.y + dy * t0),
            Point::new(a.x + dx * t1, a.y + dy * t1),
            BrushStroke::new(1.0),
            first,
        );
    }
}

fn paint_text_preview(g: &mut Painter, state: &State, l: &Layout) {
    let Some(t) = &state.text else { return };
    let transform = CanvasTransform::new(l, state);
    let zoom = transform.zoom;
    let size = (t.size * zoom).clamp(1.0, 1600.0);
    let p = transform.to_screen(t.pos);
    let lines: Vec<&str> = t.text.split('\n').collect();
    let line_h = text::line_height(size);
    let w = lines
        .iter()
        .map(|line| g.text_width(line, size))
        .fold(2.0f32, f32::max);
    let m = text::Metrics {
        line_h,
        box_w: w + 8.0,
        box_h: line_h * lines.len() as f32 + 4.0,
    };
    let r = text::box_rect(p, &m);
    let center = r.center();
    let angle = t.angle_deg;
    let rotate = Transform::rotate_around_degrees(angle, center);
    for (i, line) in lines.iter().enumerate() {
        g.text_tl_rotated(
            line,
            Point::new(p.x, p.y + i as f32 * line_h),
            size,
            state.color1,
            angle,
            center,
        );
    }
    let corners = [
        r.origin,
        Point::new(r.right(), r.top()),
        Point::new(r.right(), r.bottom()),
        Point::new(r.left(), r.bottom()),
        r.origin,
    ];
    let outline: Vec<Point> = corners.iter().map(|q| rotate.apply(*q)).collect();
    g.polyline(&outline, BrushStroke::new(1.0), COL_ACCENT.with_alpha(0.55));
    let last = lines.last().copied().unwrap_or("");
    let caret_x = p.x + g.text_width(last, size);
    let caret_y = p.y + (lines.len() - 1) as f32 * line_h;
    g.line(
        rotate.apply(Point::new(caret_x, caret_y)),
        rotate.apply(Point::new(caret_x, caret_y + line_h)),
        BrushStroke::new(1.0),
        Color::BLACK,
    );
    let root = text::root_point(center, m.box_h, angle);
    let handle = text::handle_point(center, m.box_h, angle);
    g.line(
        root,
        handle,
        BrushStroke::new(1.0),
        COL_ACCENT.with_alpha(0.6),
    );
    g.circle(handle, text::HANDLE_RADIUS + 1.0, Fill::solid(COL_ACCENT));
    g.circle(handle, text::HANDLE_RADIUS, Fill::solid(Color::WHITE));
    g.text_tl(
        &format!("{:.0}°", text::normalize_angle(angle)),
        Point::new(handle.x + 10.0, handle.y - 8.0),
        11.0,
        COL_TEXT,
    );
}

fn paint_rulers(g: &mut Painter, state: &State, l: &Layout) {
    g.fill(l.top_ruler, Fill::solid(COL_RULER));
    g.fill(l.left_ruler, Fill::solid(COL_RULER));
    g.fill(l.ruler_corner, Fill::solid(COL_RULER));
    border(g, l.top_ruler, COL_BORDER);
    g.line(
        Point::new(l.left_ruler.right(), l.left_ruler.top()),
        Point::new(l.left_ruler.right(), l.left_ruler.bottom()),
        BrushStroke::new(1.0),
        COL_BORDER,
    );
    let transform = CanvasTransform::new(l, state);
    let zoom = transform.zoom;
    let mut step = 10.0;
    for s in [10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0] {
        step = s;
        if s * zoom >= 46.0 {
            break;
        }
    }
    let doc_w = state.doc.width() as f32;
    let doc_h = state.doc.height() as f32;
    let mut d = 0.0;
    while d <= doc_w {
        let x = transform.to_screen(Point::new(d, 0.0)).x;
        if x >= l.top_ruler.left() && x <= l.top_ruler.right() {
            g.line(
                Point::new(x, l.top_ruler.bottom() - 6.0),
                Point::new(x, l.top_ruler.bottom()),
                BrushStroke::new(1.0),
                COL_MUTED,
            );
            g.text_tl(
                &format!("{}", d as i32),
                Point::new(x + 2.0, l.top_ruler.top() + 1.0),
                8.5,
                COL_MUTED,
            );
        }
        d += step;
    }
    let mut d = 0.0;
    while d <= doc_h {
        let y = transform.to_screen(Point::new(0.0, d)).y;
        if y >= l.left_ruler.top() && y <= l.left_ruler.bottom() {
            g.line(
                Point::new(l.left_ruler.right() - 6.0, y),
                Point::new(l.left_ruler.right(), y),
                BrushStroke::new(1.0),
                COL_MUTED,
            );
            if d > 0.0 {
                let label = format!("{}", d as i32);
                let size = 8.0;
                let tw = g.text_width(&label, size);
                let center =
                    Point::new(l.left_ruler.right() - 3.0 - size * 0.5, y - 2.0 - tw * 0.5);
                let p = Point::new(center.x - tw * 0.5, center.y - size * 0.5);
                g.text_tl_rotated(&label, p, size, COL_MUTED, -90.0, center);
            }
        }
        d += step;
    }
}

fn paint_scrollbar(
    g: &mut Painter,
    bar: Rect,
    vertical: bool,
    state: &State,
    l: &Layout,
    pointer: Point,
) {
    g.fill(bar, Fill::solid(COL_BG));
    let axis = if vertical {
        layout::ScrollAxis::Vertical
    } else {
        layout::ScrollAxis::Horizontal
    };
    let thumb = layout::scroll_thumb(bar, axis, layout::scroll_metrics(state, l, axis));
    let hover = bar.contains(pointer);
    g.rounded(
        thumb,
        if vertical {
            thumb.size.width * 0.5
        } else {
            thumb.size.height * 0.5
        },
        Fill::solid(if hover {
            Color::hex(0xB8B8B8)
        } else {
            Color::hex(0xCDCDCD)
        }),
    );
}

fn paint_status(g: &mut Painter, state: &State, l: &Layout, pointer: Point) {
    g.fill(l.status, Fill::solid(Color::hex(0xF0F0F0)));
    g.line(
        Point::new(l.status.left(), l.status.top() + 0.5),
        Point::new(l.status.right(), l.status.top() + 0.5),
        BrushStroke::new(1.0),
        Color::hex(0xD9D9D9),
    );
    let fresh = state
        .status
        .as_ref()
        .map(|(_, t)| t.elapsed().as_secs_f32() < 3.0)
        .unwrap_or(false);
    let left = if fresh {
        state
            .status
            .as_ref()
            .map(|(s, _)| s.clone())
            .unwrap_or_default()
    } else if l.canvas.contains(pointer) || l.viewport.contains(pointer) {
        format!(
            "{}, {}px",
            state.pointer.x.floor().max(0.0) as i32,
            state.pointer.y.floor().max(0.0) as i32
        )
    } else {
        String::new()
    };
    g.text_tl(
        &left,
        Point::new(l.status.left() + 10.0, l.status.top() + 6.0),
        12.0,
        Color::hex(0x333333),
    );

    let size_text = format!("{} x {}px", state.doc.width(), state.doc.height());
    let size_width = g.text_width(&size_text, 12.0);
    let size_right = l.zoom_minus.left() - 30.0;
    g.text_tl(
        &size_text,
        Point::new(size_right - size_width, l.status.top() + 6.0),
        12.0,
        Color::hex(0x333333),
    );

    let track = Rect::from_xywh(
        l.zoom_slider.left(),
        l.zoom_slider.center().y - 1.5,
        l.zoom_slider.size.width,
        3.0,
    );
    g.rounded(track, 2.0, Fill::solid(Color::hex(0xB5B5B5)));
    let hx = layout::zoom_handle_x(l, state);
    let thumb = Rect::from_xywh(hx - 4.0, track.center().y - 7.5, 8.0, 15.0);
    g.rounded(thumb, 4.0, Fill::solid(Color::hex(0x6B6B6B)));
    g.rect_outline(thumb, 4.0, BrushStroke::new(1.0), Color::hex(0x4A4A4A));

    for (rect, plus) in [(l.zoom_minus, false), (l.zoom_plus, true)] {
        if rect.contains(pointer) {
            g.rounded(rect, 2.0, Fill::solid(Color::hex(0xE3F1FB)));
            g.rect_outline(rect, 2.0, BrushStroke::new(1.0), Color::hex(0xA8CFEE));
        }
        let cx = rect.center().x;
        let cy = rect.center().y;
        g.line(
            Point::new(cx - 4.0, cy),
            Point::new(cx + 4.0, cy),
            BrushStroke::new(1.4),
            Color::hex(0x4A5560),
        );
        if plus {
            g.line(
                Point::new(cx, cy - 4.0),
                Point::new(cx, cy + 4.0),
                BrushStroke::new(1.4),
                Color::hex(0x4A5560),
            );
        }
    }
    let zoom_text = format!("{}%", state.zoom_percent() as i32);
    let zoom_width = g.text_width(&zoom_text, 12.0);
    let zoom_right = l.status.right() - 10.0;
    g.text_tl(
        &zoom_text,
        Point::new(zoom_right - zoom_width, l.status.top() + 6.0),
        12.0,
        Color::hex(0x333333),
    );
}

fn paint_backstage(g: &mut Painter, state: &State, l: &Layout, bs: Rect, pointer: Point) {
    g.fill(bs, Fill::solid(Color::WHITE));
    let left = Rect::from_xywh(bs.left(), bs.top(), 262.0, bs.size.height);
    g.fill(left, Fill::solid(Color::hex(0xF3F3F3)));
    g.line(
        Point::new(left.right(), left.top()),
        Point::new(left.right(), left.bottom()),
        BrushStroke::new(1.0),
        COL_BORDER,
    );
    if l.back_close.contains(pointer) {
        g.rounded(l.back_close, 4.0, Fill::solid(COL_HOVER));
    }
    draw_icon(g, Icon::Close, inset(l.back_close, 7.0), COL_TEXT);
    g.text_tl(
        "File",
        Point::new(left.left() + 58.0, bs.top() + 14.0),
        24.0,
        COL_TEXT,
    );
    for item in &l.back_items {
        let hover = item.rect.contains(pointer);
        if hover {
            g.rounded(item.rect, 3.0, Fill::solid(COL_HOVER));
        }
        let ir = Rect::from_xywh(
            item.rect.left() + 10.0,
            item.rect.center().y - 10.0,
            20.0,
            20.0,
        );
        draw_icon(g, item.icon, ir, if hover { COL_ACCENT } else { COL_TEXT });
        g.text_tl(
            item.label,
            Point::new(ir.right() + 12.0, item.rect.center().y - 7.0),
            12.5,
            COL_TEXT,
        );
    }
    let content_x = left.right() + 40.0;
    g.text_tl(
        "Recent images",
        Point::new(content_x, bs.top() + 34.0),
        17.0,
        COL_TEXT,
    );
    if state.recent.is_empty() {
        g.text_tl(
            "No recently opened files.",
            Point::new(content_x, bs.top() + 72.0),
            12.0,
            COL_MUTED,
        );
    } else {
        let mut y = bs.top() + 66.0;
        for path in state.recent.iter().rev().take(8) {
            let label = path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            g.text_tl(&label, Point::new(content_x, y), 12.0, COL_TEXT);
            g.text_tl(
                &path.display().to_string(),
                Point::new(content_x, y + 15.0),
                9.5,
                COL_MUTED,
            );
            y += 38.0;
        }
    }
    let preview_area = Rect::from_xywh(bs.right() - 330.0, bs.top() + 90.0, 290.0, 220.0);
    if let Some(thumb) = state.doc.scaled_to_fit(260) {
        let w = thumb.width() as f32;
        let h = thumb.height() as f32;
        let scale = (preview_area.size.width / w).min(preview_area.size.height / h);
        let r = Rect::from_xywh(
            preview_area.center().x - w * scale * 0.5,
            preview_area.center().y - h * scale * 0.5,
            w * scale,
            h * scale,
        );
        g.fill(r, Fill::solid(Color::WHITE));
        g.pixels(r, &thumb);
        g.rect_outline(r, 0.0, BrushStroke::new(1.0), COL_BORDER);
        g.text_tl(
            &state.font_file_name(),
            Point::new(preview_area.left(), preview_area.bottom() + 14.0),
            11.0,
            COL_MUTED,
        );
        g.text_tl(
            &format!("{} x {} pixels", state.doc.width(), state.doc.height()),
            Point::new(preview_area.left(), preview_area.bottom() + 30.0),
            11.0,
            COL_MUTED,
        );
    }
}

fn paint_dialog(g: &mut Painter, state: &State, l: &Layout, pointer: Point) {
    g.fill(l.size_rect(), Fill::solid(Color::BLACK.with_alpha(0.25)));
    let Some(dl) = layout::dialog_layout(state, l.size) else {
        return;
    };
    let card = dl.card;
    g.fill(
        Rect::from_xywh(
            card.left() + 5.0,
            card.top() + 6.0,
            card.size.width,
            card.size.height,
        ),
        Fill::solid(Color::BLACK.with_alpha(0.18)),
    );
    g.fill(card, Fill::solid(Color::WHITE));
    g.rect_outline(card, 0.0, BrushStroke::new(1.0), COL_BORDER);
    let title_bar = Rect::from_xywh(card.left(), card.top(), card.size.width, 34.0);
    g.fill(title_bar, Fill::solid(Color::hex(0xF5F5F5)));
    border(g, title_bar, COL_BORDER);
    let title = match state.dialog.as_ref().unwrap() {
        Dialog::Resize(_) => "Resize image",
        Dialog::Properties => "Image properties",
        Dialog::About => "About Paint",
        Dialog::EditColors(_) => "Edit colors",
        Dialog::ConfirmNew => "Paint",
    };
    g.text_tl(
        title,
        Point::new(card.left() + 12.0, card.top() + 9.0),
        12.5,
        COL_TEXT,
    );
    if dl.close.contains(pointer) {
        g.rounded(dl.close, 3.0, Fill::solid(COL_HOVER));
    }
    draw_icon(g, Icon::Close, inset(dl.close, 4.0), COL_TEXT);
    match state.dialog.as_ref().unwrap() {
        Dialog::Resize(r) => paint_resize_dialog(g, r, &dl, pointer),
        Dialog::Properties => paint_properties_dialog(g, state, &dl, pointer),
        Dialog::About => paint_about_dialog(g, &dl, pointer),
        Dialog::EditColors(e) => paint_edit_colors(g, state, e, &dl, pointer),
        Dialog::ConfirmNew => paint_confirm_dialog(g, &dl, pointer),
    }
}

fn draw_button(g: &mut Painter, r: Rect, label: &str, primary: bool, hovered: bool) {
    let bg = if primary {
        if hovered {
            Color::hex(0x1F4E8C)
        } else {
            COL_ACCENT
        }
    } else if hovered {
        COL_HOVER
    } else {
        Color::hex(0xF7F7F7)
    };
    g.rounded(r, 3.0, Fill::solid(bg));
    g.rect_outline(
        r,
        3.0,
        BrushStroke::new(1.0),
        if primary { COL_ACCENT } else { COL_BORDER },
    );
    text_centered(
        g,
        label,
        r.center().x,
        r.center().y - 6.0,
        11.5,
        if primary { Color::WHITE } else { COL_TEXT },
    );
}

fn paint_resize_dialog(g: &mut Painter, r: &ResizeDialog, dl: &DialogLayout, pointer: Point) {
    let card = dl.card;
    g.text_tl(
        "Width (pixels):",
        Point::new(card.left() + 24.0, card.top() + 72.0),
        12.0,
        COL_TEXT,
    );
    g.text_tl(
        "Height (pixels):",
        Point::new(card.left() + 24.0, card.top() + 114.0),
        12.0,
        COL_TEXT,
    );
    for (i, field) in dl.fields.iter().enumerate() {
        g.fill(*field, Fill::solid(Color::WHITE));
        g.rect_outline(
            *field,
            2.0,
            BrushStroke::new(if r.focus == i { 2.0 } else { 1.0 }),
            if r.focus == i { COL_ACCENT } else { COL_BORDER },
        );
        let text = if i == 0 { &r.width } else { &r.height };
        g.text_tl(
            text,
            Point::new(field.left() + 6.0, field.top() + 6.0),
            12.0,
            COL_TEXT,
        );
    }
    g.text_tl(
        "Maximum size: 8192 x 8192 pixels",
        Point::new(card.left() + 24.0, card.top() + 150.0),
        10.0,
        COL_MUTED,
    );
    draw_button(g, dl.ok, "OK", true, dl.ok.contains(pointer));
    draw_button(g, dl.cancel, "Cancel", false, dl.cancel.contains(pointer));
}

fn paint_properties_dialog(g: &mut Painter, state: &State, dl: &DialogLayout, pointer: Point) {
    let card = dl.card;
    let lines = [
        format!("File name: {}", state.font_file_name()),
        format!(
            "Dimensions: {} x {} pixels",
            state.doc.width(),
            state.doc.height()
        ),
        "Resolution: 96 DPI".to_string(),
        "Format: RGB (24-bit)".to_string(),
        format!(
            "Data size: about {} KB",
            (state.doc.width() as u64 * state.doc.height() as u64 * 4) / 1024
        ),
        format!(
            "Location: {}",
            state
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "not saved".to_string())
        ),
    ];
    for (i, line) in lines.iter().enumerate() {
        g.text_tl(
            line,
            Point::new(card.left() + 24.0, card.top() + 56.0 + i as f32 * 22.0),
            11.5,
            COL_TEXT,
        );
    }
    draw_button(g, dl.ok, "OK", true, dl.ok.contains(pointer));
}

fn paint_about_dialog(g: &mut Painter, dl: &DialogLayout, pointer: Point) {
    let card = dl.card;
    let logo = Rect::from_xywh(card.left() + 28.0, card.top() + 58.0, 64.0, 64.0);
    g.rounded(logo, 10.0, Fill::solid(COL_ACCENT));
    let colors = [
        Color::hex(0xFF7F27),
        Color::hex(0x22B14C),
        Color::hex(0x00A2E8),
        Color::hex(0xFFF200),
    ];
    for (i, c) in colors.iter().enumerate() {
        g.circle(
            Point::new(
                logo.left() + 18.0 + (i % 2) as f32 * 26.0,
                logo.top() + 18.0 + (i / 2) as f32 * 26.0,
            ),
            9.0,
            Fill::solid(*c),
        );
    }
    g.text_tl(
        "Paint",
        Point::new(card.left() + 112.0, card.top() + 56.0),
        24.0,
        COL_TEXT,
    );
    g.text_tl(
        "MS Paint clone for SequoiaUI",
        Point::new(card.left() + 112.0, card.top() + 90.0),
        12.0,
        COL_TEXT,
    );
    g.text_tl(
        "Built in Rust with SequoiaUI",
        Point::new(card.left() + 112.0, card.top() + 110.0),
        11.0,
        COL_MUTED,
    );
    g.text_tl(
        "Version 1.0",
        Point::new(card.left() + 112.0, card.top() + 130.0),
        11.0,
        COL_MUTED,
    );
    g.text_tl(
        "Drawing, brushes, shapes, selection, text dialogs and file saving.",
        Point::new(card.left() + 28.0, card.top() + 160.0),
        11.0,
        COL_MUTED,
    );
    draw_button(g, dl.ok, "OK", true, dl.ok.contains(pointer));
}

fn paint_confirm_dialog(g: &mut Painter, dl: &DialogLayout, pointer: Point) {
    let card = dl.card;
    g.text_tl(
        "Save changes to the image?",
        Point::new(card.left() + 24.0, card.top() + 66.0),
        13.0,
        COL_TEXT,
    );
    g.text_tl(
        "Unsaved changes will be lost.",
        Point::new(card.left() + 24.0, card.top() + 90.0),
        10.5,
        COL_MUTED,
    );
    draw_button(g, dl.save, "Save", true, dl.save.contains(pointer));
    draw_button(
        g,
        dl.dont_save,
        "Don't save",
        false,
        dl.dont_save.contains(pointer),
    );
    draw_button(g, dl.cancel, "Cancel", false, dl.cancel.contains(pointer));
}

fn paint_edit_colors(
    g: &mut Painter,
    state: &State,
    e: &ColorEditor,
    dl: &DialogLayout,
    pointer: Point,
) {
    let card = dl.card;
    let color = Color::rgb(e.r, e.g, e.b);
    let preview = Rect::from_xywh(card.left() + 24.0, card.top() + 54.0, 72.0, 72.0);
    g.fill(preview, Fill::solid(color));
    g.rect_outline(
        preview,
        2.0,
        BrushStroke::new(1.0),
        Color::BLACK.with_alpha(0.5),
    );
    for (ch, track) in &dl.sliders {
        let (label, value) = match ch {
            SliderChannel::R => ("R", e.r),
            SliderChannel::G => ("G", e.g),
            SliderChannel::B => ("B", e.b),
        };
        g.text_tl(
            label,
            Point::new(track.left() - 16.0, track.top() + 2.0),
            12.0,
            COL_TEXT,
        );
        g.rounded(
            Rect::from_xywh(track.left(), track.center().y - 3.0, track.size.width, 6.0),
            3.0,
            Fill::solid(Color::hex(0xD8D8D8)),
        );
        let knob = Rect::from_xywh(
            track.left() + track.size.width * value,
            track.center().y - 7.0,
            10.0,
            14.0,
        );
        g.fill(knob, Fill::solid(Color::WHITE));
        g.rect_outline(knob, 2.0, BrushStroke::new(1.0), COL_MUTED);
        g.text_tl(
            &format!("{}", (value * 255.0).round() as i32),
            Point::new(track.right() + 12.0, track.top() + 2.0),
            11.0,
            COL_TEXT,
        );
    }
    g.text_tl(
        "Custom colors",
        Point::new(card.left() + 24.0, card.top() + 162.0),
        10.0,
        COL_MUTED,
    );
    for (i, r) in dl.palette.iter().enumerate() {
        if let Some(c) = state.palette.get(i).copied() {
            g.fill(*r, Fill::solid(c));
            g.rect_outline(
                *r,
                0.0,
                BrushStroke::new(1.0),
                Color::BLACK.with_alpha(0.35),
            );
            if e.target == Some(i) {
                g.rect_outline(*r, 0.0, BrushStroke::new(2.0), COL_ACCENT);
            }
        }
    }
    g.text_tl(
        "Basic palette",
        Point::new(card.left() + 24.0, card.top() + 236.0),
        10.0,
        COL_MUTED,
    );
    for (i, r) in dl.presets.iter().enumerate() {
        if let Some(c) = CUSTOM_COLORS.get(i).copied() {
            g.fill(*r, Fill::solid(c));
            g.rect_outline(
                *r,
                0.0,
                BrushStroke::new(1.0),
                Color::BLACK.with_alpha(0.35),
            );
        }
    }
    draw_button(g, dl.ok, "OK", true, dl.ok.contains(pointer));
    draw_button(g, dl.cancel, "Cancel", false, dl.cancel.contains(pointer));
}
