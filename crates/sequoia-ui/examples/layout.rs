//! Layout demo — card grid that flows with the window.
//!
//! Cards use `Wrap`, not `Row`: with a fixed width they flow to the
//! next row instead of shrinking to slivers, and `Wrap::align_items`
//! keeps them equal height per row. `WithSize` reports the arranged
//! width back so `Text::truncate` can place `…` exactly on the card edge.

use sequoia_core::Color;
use sequoia_ui::prelude::*;
use sequoia_ui::view::{AnyView, WithSize};

const BG: Color = Color::hex(0x0F0F12);
const CARD_BG: Color = Color::hex(0x1A1A1F);
const CARD_BORDER: Color = Color::hex(0x26262E);
const TEXT: Color = Color::hex(0xF2F2F2);
const MUTED: Color = Color::hex(0x8A8A93);

const CARD_WIDTH: f32 = 240.0;
const CARD_PAD: f32 = 20.0;

fn main() {
    let view = Column::new()
        .fill_size()
        .background(BG)
        .padding(32.0)
        .gap(24.0)
        .child(header())
        .child_flex(cards_wrap(), 1.0);

    App::new()
        .title("Layout Demo")
        .size(900.0, 480.0)
        .root(view)
        .run()
        .expect("event loop failed");
}

// ── Header ────────────────────────────────────────────────────────────
fn header() -> Column {
    Column::new()
        .gap(6.0)
        .child(Text::new("Design system").size(28.0).bold().color(TEXT))
        .child(
            Text::new(
                "Three cards laid out with Wrap — they flow to the next row when the window narrows.",
            )
            .size(14.0)
            .color(MUTED)
            .truncate(760.0),
        )
}

// ── Cards row ─────────────────────────────────────────────────────────
// Wrap + fixed card width: wide window → 3 cards side by side,
// narrow window → 3rd card wraps to a second row. `Align::Stretch`
// makes cards in the same row equal height.
fn cards_wrap() -> Wrap {
    Wrap::new()
        .fill_size()
        .gap(20.0)
        .align_items(Align::Stretch)
        .child(card(
            "Fast",
            "Renders on-demand frames with minimal allocations and no layout churn.",
            Color::hex(0x4DA3FF),
        ))
        .child(card(
            "Composable",
            "Row, Column, Stack, Grid and Wrap nest freely — mix them as you like.",
            Color::hex(0x7BC97B),
        ))
        .child(card(
            "Reactive",
            "Signals drive updates automatically — no manual wiring, no stale state.",
            Color::hex(0xE5A24D),
        ))
}

// ── One card ──────────────────────────────────────────────────────────
// Three layers of truncation:
//   1. Text::truncate(avail)     — `avail` = real card content width,
//                                  so `…` lands exactly on the visible edge.
//   2. Column::overflow(Hidden)  — belt-and-braces against font mismatch.
//   3. Container::clip(true)     — final clip at the rounded card bounds.
fn card(title: &'static str, body: &'static str, accent: Color) -> Container {
    Container::new()
        .width(CARD_WIDTH)
        .background(CARD_BG)
        .border(1.0, CARD_BORDER)
        .radius(12.0)
        .clip(true)
        .padding(CARD_PAD)
        .child(
            WithSize::new(move |size| {
                // size.width = the content area inside the container
                // (CARD_WIDTH - padding). The -2.0 keeps `…` off the edge.
                let avail = if size.width > 1.0 {
                    (size.width - 2.0).max(0.0)
                } else {
                    // First frame: WithSize has no measurement yet.
                    CARD_WIDTH - 2.0 * CARD_PAD - 2.0
                };
                AnyView::new(
                    Column::new()
                        .fill_width()
                        .gap(12.0)
                        .overflow(Overflow::Hidden)
                        .child(
                            Container::new()
                                .square(36.0)
                                .radius(18.0)
                                .background(accent.with_alpha(0.20)),
                        )
                        .child(
                            Text::new(title)
                                .size(16.0)
                                .bold()
                                .color(TEXT)
                                .truncate(avail),
                        )
                        .child(
                            Text::new(body)
                                .size(13.0)
                                .color(MUTED)
                                .truncate(avail),
                        ),
                )
            }),
        )
}