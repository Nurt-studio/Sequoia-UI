//! Pill-style search field with a hand-drawn magnifier.
//!
//! The old input used the registry "search" glyph on a flat grey box; this
//! one is a rounded pill (fill `#1E1E20`, radius 8) with a thin-stroke
//! magnifier on the left and a clear ("✕") button once there is text.

use sequoia_ui::prelude::*;
use sequoia_ui::view::BrushStroke;

use crate::theme::colors;

/// Thin-stroke magnifier: circle + 45° handle, drawn by hand instead of the
/// old registry icon.
fn magnifier(size: f32, color: Color) -> impl View + 'static {
    Canvas::new()
        .width(size)
        .height(size)
        .focusable(false)
        .on_draw(move |p| {
            let stroke = BrushStroke::screen(1.4);
            // Circle centered slightly up-left, handle to the bottom-right.
            let r = size * 0.30;
            let cx = size * 0.42;
            let cy = size * 0.42;
            p.ellipse_outline(Point::new(cx, cy), r, r, stroke, color);
            let hx = cx + r * 0.72;
            let hy = cy + r * 0.72;
            p.line(
                Point::new(hx, hy),
                Point::new(size * 0.88, size * 0.88),
                stroke,
                color,
            );
        })
}

fn pill(query: Signal<String>, cancellable: bool) -> impl View + 'static {
    let q_clear = query.clone();
    let q_empty = query.clone();
    let mut row = Row::new()
        .fill_width()
        .height(26.0)
        .padding(Edges::xy(9.0, 0.0))
        .radius(8.0)
        .background(Color::hex(0x1E1E20))
        .border(1.0, Color::hex(0x2E2E33))
        .align_items(Align::Center)
        .child(magnifier(12.0, colors::active().icon_inactive))
        .child(Spacer::fixed(6.0))
        .child_flex(
            TextField::new(query.clone())
                .placeholder("Search...")
                .font_size(11.0)
                .text_color(colors::active().text)
                .background(Color::TRANSPARENT)
                .border(1.0, Color::TRANSPARENT),
            1.0,
        );
    if cancellable {
        // Rebuilt with the signal (the body tracks `q_empty`), so the button
        // appears only while there is something to clear.
        if !q_empty.get().is_empty() {
            row = row.child(
                Container::new()
                    .width(16.0)
                    .height(16.0)
                    .center()
                    .radius(8.0)
                    .hover(Color::hex(0x353537))
                    .on_click(move || q_clear.set(String::new()))
                    .child(
                        Text::new("✕")
                            .size(9.0)
                            .color(colors::active().icon_inactive),
                    ),
            );
        }
    }
    row
}

/// Search pill bound to `query` (Layers panel, node-graph browser).
pub(crate) fn search_field(query: Signal<String>) -> impl View + 'static {
    pill(query, false)
}

/// Search pill with a clear ("✕") button while the query is not empty.
pub(crate) fn search_field_cancellable(query: Signal<String>) -> impl View + 'static {
    SearchCancellable { query }
}

struct SearchCancellable {
    query: Signal<String>,
}

impl View for SearchCancellable {
    fn body(&self) -> impl View + 'static {
        pill(self.query.clone(), true)
    }
}
