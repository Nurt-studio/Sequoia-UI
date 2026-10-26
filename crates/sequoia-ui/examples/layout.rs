//! Layout demo — every layout primitive in one window.
//!
//! Run with:
//!     cargo run --example layout -p sequoia-ui
//!
//! The shell itself is a `DockArea` (top / left / fill). The content is a
//! `Grid` of cards; each card isolates one primitive:
//!
//! * `Column`   — vertical stacking, children share height via `child_flex`
//! * `Row`      — horizontal stacking, children share width
//! * `Flex`     — same as `Row`, but with explicit 1 : 2 : 3 grow weights
//! * `Grid`     — fixed tracks plus explicit `child_span` placements
//! * `Stack`    — layered children with pins (`top_right`, `bottom_left`)
//! * `Wrap`     — fixed-size items flow onto the next line when space runs out
//! * `DockArea` — top / left / right / bottom / fill edges of a rectangle
//!
//! Resize the window: the grid stretches, `Wrap` reflows and the docked
//! edges keep their thickness while the fill area absorbs the rest.

use sequoia_ui::prelude::*;

const BG: Color = Color::hex(0x0E0E12);
const CARD_BG: Color = Color::hex(0x191920);
const CARD_BORDER: Color = Color::hex(0x2A2A34);
const SIDEBAR_BG: Color = Color::hex(0x14141A);
const TEXT: Color = Color::hex(0xF3F3F5);
const MUTED: Color = Color::hex(0x93939F);
const FAINT: Color = Color::hex(0x6C6C78);

const BLUE: Color = Color::hex(0x4DA3FF);
const GREEN: Color = Color::hex(0x7BC97B);
const AMBER: Color = Color::hex(0xE5A24D);
const PINK: Color = Color::hex(0xE06C9F);
const PURPLE: Color = Color::hex(0x9B7BFF);
const TEAL: Color = Color::hex(0x4DD6C1);

fn main() {
    App::new()
        .title("Layout Demo")
        .size(1280.0, 880.0)
        .min_size(1000.0, 720.0)
        .root(app_view())
        .run()
        .expect("event loop failed");
}

fn app_view() -> DockArea {
    DockArea::new()
        .fill_size()
        .background(BG)
        .padding(16.0)
        .gap(12.0)
        .top(header())
        .left(sidebar())
        .bottom(footer())
        .fill(layout_grid())
}

// ── Shell ─────────────────────────────────────────────────────────────
fn header() -> Row {
    Row::new()
        .fill_width()
        .align_items(Align::Center)
        .gap(16.0)
        .child(
            Column::new()
                .gap(3.0)
                .child(
                    Text::new("Layout primitives")
                        .size(26.0)
                        .bold()
                        .color(TEXT)
                        .truncate_fill(),
                )
                .child(
                    Text::new(
                        "One window, seven primitives. Resize it and watch Wrap reflow, Grid stretch and DockArea keep its edges.",
                    )
                    .size(13.0)
                    .color(MUTED)
                    .truncate_fill(),
                ),
        )
        .child(Spacer::flex(1.0))
        .child(chip("resizable window", BLUE))
}

fn sidebar() -> Container {
    let mut list = Column::new()
        .fill_size()
        .padding(14.0)
        .gap(7.0)
        .child(
            Text::new("DockArea")
                .size(13.0)
                .bold()
                .color(TEXT)
                .truncate_fill(),
        )
        .child(
            Text::new("top + left + fill")
                .size(11.0)
                .color(MUTED)
                .truncate_fill(),
        );

    for (label, color) in [
        ("Column", BLUE),
        ("Row", GREEN),
        ("Flex", AMBER),
        ("Grid", PINK),
        ("Stack", PURPLE),
        ("Wrap", TEAL),
        ("DockArea", BLUE),
    ] {
        list = list.child(
            Row::new()
                .fill_width()
                .align_items(Align::Center)
                .gap(8.0)
                .child(Container::new().square(10.0).radius(3.0).background(color))
                .child(Text::new(label).size(12.0).color(TEXT).truncate_fill()),
        );
    }

    list = list.child(Spacer::vertical(1.0)).child(
        Text::new("The window shell is docked too: header on top, legend on the left, cards filling the rest.")
            .size(11.0)
            .color(FAINT)
            .truncate_fill(),
    );

    Container::new()
        .width(190.0)
        .fill_height()
        .background(SIDEBAR_BG)
        .border(1.0, CARD_BORDER)
        .radius(12.0)
        .clip(true)
        .child(list)
}

fn footer() -> Row {
    Row::new()
        .fill_width()
        .align_items(Align::Center)
        .gap(10.0)
        .child(
            Text::new(
                "child_flex splits free space by weight · Wrap never squeezes children · DockArea pins edges first, then fill takes the rest.",
            )
            .size(12.0)
            .color(MUTED)
            .truncate_fill(),
        )
        .child(Spacer::flex(1.0))
        .child(
            Text::new("sequoia-ui layout demo")
                .size(11.0)
                .color(FAINT)
                .truncate_fill(),
        )
}

// ── Card grid ─────────────────────────────────────────────────────────
fn layout_grid() -> Grid {
    Grid::new([
        Track::minmax(230.0, Track::Fill(1.0)),
        Track::minmax(230.0, Track::Fill(1.0)),
        Track::minmax(230.0, Track::Fill(1.0)),
    ])
    .rows([
            Track::minmax(176.0, Track::Fill(1.0)),
            Track::minmax(176.0, Track::Fill(1.0)),
            Track::Px(184.0),
        ])
        .gap(14.0)
    .fill_size()
    .child_span(
        card(
            "Column",
            "Children stack top → bottom; equal child_flex weights split the height.",
            demo_column(),
        ),
        0,
        0,
        1,
        1,
    )
    .child_span(
        card(
            "Row",
            "Children line up left → right; equal child_flex weights split the width.",
            demo_row(),
        ),
        1,
        0,
        1,
        1,
    )
    .child_span(
        card(
            "Flex",
            "Grow weights 1 : 2 : 3 — free width is divided proportionally.",
            demo_flex(),
        ),
        2,
        0,
        1,
        1,
    )
    .child_span(
        card(
            "Grid",
            "Explicit tracks and placements: one item spans 2 columns, another 2 rows.",
            demo_grid(),
        ),
        0,
        1,
        1,
        1,
    )
    .child_span(
        card(
            "Stack",
            "Children overlap; the center card sits between pinned corner badges.",
            demo_stack(),
        ),
        1,
        1,
        1,
        1,
    )
    .child_span(
        card(
            "Wrap",
            "Fixed-size pills flow onto new lines; they keep their size instead of shrinking.",
            demo_wrap(),
        ),
        2,
        1,
        1,
        1,
    )
    .child_span(
        card(
            "DockArea",
            "Top and bottom span the full width first, left and right take the remaining height, fill takes what is left.",
            demo_dock_area(),
        ),
        0,
        2,
        3,
        1,
    )
}

fn card<V: View + 'static>(title: &'static str, note: &'static str, demo: V) -> Container {
    Container::new()
        .fill_size()
        .min_width(230.0)
        .min_height(176.0)
        .background(CARD_BG)
        .border(1.0, CARD_BORDER)
        .radius(12.0)
        .clip(true)
        .padding(16.0)
        .child(
            Column::new()
                .fill_size()
                .gap(10.0)
                .child(
                    Column::new()
                        .fill_width()
                        .gap(2.0)
                        .child(
                            Text::new(title)
                                .size(16.0)
                                .bold()
                                .color(TEXT)
                                .truncate_fill(),
                        )
                        .child(Text::new(note).size(12.0).color(MUTED).truncate_fill()),
                )
                .child_flex(demo, 1.0),
        )
}

// ── Demo 1: Column ────────────────────────────────────────────────────
fn demo_column() -> Column {
    Column::new()
        .fill_size()
        .gap(6.0)
        .child_flex(tile("first", BLUE), 1.0)
        .child_flex(tile("second", GREEN), 1.0)
        .child_flex(tile("third", AMBER), 1.0)
}

// ── Demo 2: Row ───────────────────────────────────────────────────────
fn demo_row() -> Row {
    Row::new()
        .fill_size()
        .gap(6.0)
        .child_flex(tile("left", BLUE), 1.0)
        .child_flex(tile("middle", GREEN), 1.0)
        .child_flex(tile("right", AMBER), 1.0)
}

// ── Demo 3: Flex ──────────────────────────────────────────────────────
fn demo_flex() -> Row {
    Row::new()
        .fill_size()
        .gap(6.0)
        .child_flex(tile("grow 1", BLUE), 1.0)
        .child_flex(tile("grow 2", GREEN), 2.0)
        .child_flex(tile("grow 3", AMBER), 3.0)
}

// ── Demo 4: Grid ──────────────────────────────────────────────────────
fn demo_grid() -> Grid {
    Grid::new([Track::Fill(1.0), Track::Fill(1.0), Track::Fill(1.0)])
        .rows([Track::Fill(1.0), Track::Fill(1.0)])
        .gap(6.0)
        .fill_size()
        .child_span(tile("span 2 cols", BLUE), 0, 0, 2, 1)
        .child_span(tile("1 × 1", GREEN), 2, 0, 1, 1)
        .child_span(tile("1 × 1", AMBER), 0, 1, 1, 1)
        .child_span(tile("span 2 cols", PINK), 1, 1, 2, 1)
}

// ── Demo 5: Stack ─────────────────────────────────────────────────────
fn demo_stack() -> Stack {
    Stack::new()
        .fill_size()
        .fill(
            Container::new()
                .fill_size()
                .radius(8.0)
                .background(BLUE.with_alpha(0.20))
                .border(1.0, BLUE.with_alpha(0.55))
                .child(
                    Column::new().fill_size().padding(10.0).child(
                        Text::new("fill layer")
                            .size(12.0)
                            .medium()
                            .color(TEXT)
                            .truncate_fill(),
                    ),
                ),
        )
        .child(boxed("centered", GREEN).width(150.0).height(76.0))
        .child_at(
            boxed("top-right", AMBER).width(84.0).height(26.0),
            StackPos::top_right().offset(-10.0, 10.0),
        )
        .child_at(
            boxed("bottom-left", PINK).width(88.0).height(26.0),
            StackPos::bottom_left().offset(10.0, -10.0),
        )
}

// ── Demo 6: Wrap ──────────────────────────────────────────────────────
fn demo_wrap() -> Wrap {
    let mut wrap = Wrap::new().fill_size().gap(6.0).align_items(Align::Stretch);
    for (label, color) in [
        ("one", BLUE),
        ("two", GREEN),
        ("three", AMBER),
        ("four", PINK),
        ("five", PURPLE),
        ("six", TEAL),
        ("seven", BLUE),
        ("eight", GREEN),
    ] {
        wrap = wrap.child(boxed(label, color).width(92.0).height(38.0));
    }
    wrap
}

// ── Demo 7: DockArea ──────────────────────────────────────────────────
fn demo_dock_area() -> DockArea {
    DockArea::new()
        .fill_size()
        .gap(6.0)
        .top(boxed("top", BLUE).fill_width().height(24.0))
        .bottom(boxed("bottom", PINK).fill_width().height(22.0))
        .left(boxed("left", GREEN).width(86.0).fill_height())
        .right(boxed("right", AMBER).width(74.0).fill_height())
        .fill(tile("fill", PURPLE))
}

// ── Helpers ───────────────────────────────────────────────────────────
fn tile(label: &'static str, accent: Color) -> Container {
    boxed(label, accent).fill_size()
}

fn boxed(label: &'static str, accent: Color) -> Container {
    Container::new()
        .radius(8.0)
        .background(accent.with_alpha(0.20))
        .border(1.0, accent.with_alpha(0.55))
        .child(
            Column::new().fill_size().center().child(
                Text::new(label)
                    .size(12.0)
                    .medium()
                    .color(TEXT)
                    .truncate_fill(),
            ),
        )
}

fn chip(label: &'static str, accent: Color) -> Row {
    Row::new()
        .height(28.0)
        .padding_xy(12.0, 0.0)
        .radius(14.0)
        .background(accent.with_alpha(0.16))
        .border(1.0, accent.with_alpha(0.45))
        .align_items(Align::Center)
        .child(
            Text::new(label)
                .size(11.0)
                .medium()
                .color(accent)
                .truncate_fill(),
        )
}
