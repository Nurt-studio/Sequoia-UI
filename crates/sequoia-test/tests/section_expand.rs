#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::rc::Rc;

use sequoia_core::{Color, Length};
use sequoia_elements::prelude::*;
use sequoia_input::Key;
use sequoia_reactive::Signal;
use sequoia_test::Harness;

fn tool_button() -> impl View + 'static {
    Container::new()
        .width(24.0)
        .height(24.0)
        .radius(3.0)
        .background(Color::BURLYWOOD)
}

#[derive(Clone)]
struct SectionState {
    open: Signal<bool>,
    popup: PopupState,
    selected: Signal<Option<usize>>,
}

impl SectionState {
    fn new() -> Self {
        let open = Signal::new(false);
        Self {
            popup: PopupState::new(open.clone()),
            open,
            selected: Signal::new(None),
        }
    }
}

#[derive(Clone)]
struct SectionWidget {
    name: String,
    tools: Vec<AnyView>,

    visible: usize,

    columns: usize,

    fill: bool,

    st: SectionState,
    on_select: Option<Rc<dyn Fn(usize)>>,
}

impl SectionWidget {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tools: Vec::new(),
            visible: 4,
            columns: 4,
            fill: false,
            st: SectionState::new(),
            on_select: None,
        }
    }

    fn tool(mut self, t: impl View + 'static) -> Self {
        self.tools.push(AnyView::new(t));
        self
    }

    fn visible(mut self, n: usize) -> Self {
        self.visible = n;
        self
    }

    fn columns(mut self, n: usize) -> Self {
        self.columns = n.max(1);
        self
    }

    fn fill_share(mut self) -> Self {
        self.fill = true;
        self
    }

    fn on_select(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }

    fn title_row(&self) -> Row {
        Row::new()
            .justify_content(Justify::Center)
            .fill_width()
            .child(
                Text::new(self.name.clone())
                    .size(12.0)
                    .color(Color::hex(0x5F5F5F)),
            )
    }

    fn tool_cell(&self, index: usize) -> AnyView {
        let border = if self.st.selected.get_untracked() == Some(index) {
            Color::hex(0x4DA3FF)
        } else {
            Color::TRANSPARENT
        };
        let selected = self.st.selected.clone();
        let popup = self.st.popup.clone();
        let on_select = self.on_select.clone();
        let tool = self.tools[index].clone();
        AnyView::new(Container::new().padding(2.0).border(1.5, border).child(
            Container::new().child(tool).on_click(move || {
                selected.set(Some(index));
                if let Some(f) = &on_select {
                    f(index);
                }
                popup.close();
            }),
        ))
    }

    fn footer(&self, close: bool) -> AnyView {
        let chevron = if close || self.st.open.get_untracked() {
            "▴"
        } else {
            "▾"
        };
        let popup = self.st.popup.clone();
        let has_tools = !self.tools.is_empty();
        AnyView::new(
            Container::new()
                .fill_width()
                .height(12.0)
                .background(Color::hex(0xD9D9D9))
                .child(
                    Row::new()
                        .fill_width()
                        .fill_height()
                        .justify_content(Justify::Center)
                        .align_items(Align::Center)
                        .child(Text::new(chevron).size(8.0).color(Color::hex(0x8A8A8A))),
                )
                .on_press(move || {
                    if close {
                        popup.close();
                    } else if has_tools {
                        popup.toggle();
                    }
                }),
        )
    }

    fn collapsed(&self) -> AnyView {
        let mut wrap = Wrap::new().gap(4.0);
        for i in 0..self.visible.min(self.tools.len()) {
            wrap = wrap.child(self.tool_cell(i));
        }
        AnyView::new(
            Column::new()
                .padding(4.0)
                .gap(12.0)
                .child(self.title_row())
                .child(wrap),
        )
    }
}

impl View for SectionWidget {
    fn body(&self) -> impl View + 'static {
        self.st.popup.subscribe();
        let _ = self.st.open.get();
        let _ = self.st.selected.get();
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.collapsed(), self.footer(false)])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: if self.fill {
                Length::Fill(1.0)
            } else {
                Length::Fit
            },
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if let Event::Key(key) = event
            && key.key == Key::ESCAPE
            && self.st.open.get_untracked()
        {
            self.st.popup.close_in(ctx);
        }
    }

    fn on_unmount(&self) {
        self.st.popup.forget();
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let section = self.clone();
        let anchor = ctx.element;
        let width = ctx.rect.size.width.max(0.0);
        self.st.popup.sync_show(ctx, move || {
            OverlayEntry::new(ExpandedPanel {
                section: section.clone(),
                width,
            })
            .anchor(Anchor::View(anchor))
            .position(Position::Relative)
            .offset(0.0, 0.0)
            .layer(OverlayLayer::Popup)
            .dismiss(Dismiss::OutsideClick)
        });
    }
}

#[derive(Clone)]
struct ExpandedPanel {
    section: SectionWidget,

    width: f32,
}

impl View for ExpandedPanel {
    fn body(&self) -> impl View + 'static {
        let _ = self.section.st.selected.get();
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let s = &self.section;
        let mut grid = Grid::new(vec![Track::Fill(1.0); s.columns]).gap(4.0);
        for i in 0..s.tools.len() {
            grid = grid.child(s.tool_cell(i));
        }
        Some(vec![AnyView::new(
            Container::new()
                .width(self.width)
                .background(Color::hex(0xF6F6F6))
                .border(1.0, Color::hex(0xD9D9D9))
                .padding(8.0)
                .child(
                    Column::new()
                        .gap(8.0)
                        .child(s.title_row())
                        .child(grid)
                        .child(s.footer(true)),
                ),
        )])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if let Event::Key(key) = event
            && key.key == Key::ESCAPE
            && self.section.st.open.get_untracked()
        {
            self.section.st.popup.close_in(ctx);
        }
    }
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1.5
}

fn bar() -> (SectionWidget, SectionWidget, impl View + 'static) {
    let a = SectionWidget::new("Operacje")
        .tool(tool_button())
        .tool(tool_button())
        .tool(tool_button())
        .tool(tool_button())
        .tool(tool_button())
        .tool(tool_button())
        .visible(4)
        .columns(4);
    let b = SectionWidget::new("Rysunki")
        .tool(tool_button())
        .tool(tool_button())
        .tool(tool_button())
        .visible(4)
        .columns(4);
    let sep = Panel::new().width(1.5).fill_height();
    let row = Row::new()
        .background(Color::hex(0xF6F6F6))
        .fill_width()
        .children((a.clone(), sep, b.clone()));
    (a, b, row)
}

fn open_a(h: &Harness) {
    let row = h.root().unwrap();
    let sec_a = h.children(row)[0];
    let footer = h.children(sec_a)[1];
    let r = h.rect(footer).unwrap();
    h.click(r.left() + r.size.width * 0.5, r.top() + r.size.height * 0.5);
    h.layout(800.0, 600.0);
    h.paint();
    h.tick();
}

#[test]
fn closed_section_has_no_overlay_and_footer_below_wrap() {
    let (_, _, row) = bar();
    let h = Harness::new(row);
    h.layout(800.0, 600.0);
    h.paint();
    h.tick();
    assert!(h.overlay_ids().is_empty());

    let sec_a = h.children(h.root().unwrap())[0];
    let parts = h.children(sec_a);
    assert_eq!(parts.len(), 2);
    let inner = h.children(parts[0]);
    let wrap = inner[1];
    assert_eq!(h.children(wrap).len(), 4, "collapsed shows `visible`");
    let wrap_r = h.rect(wrap).unwrap();
    let foot_r = h.rect(parts[1]).unwrap();
    assert!(
        foot_r.top() >= wrap_r.top() + wrap_r.size.height - 0.01,
        "footer under wrap: {foot_r:?} vs {wrap_r:?}"
    );
}

#[test]
fn footer_opens_panel_covering_section_and_spilling_below() {
    let (_, _, row) = bar();
    let h = Harness::new(row);
    h.layout(800.0, 600.0);
    h.paint();
    h.tick();
    let sec_before = h.rect(h.children(h.root().unwrap())[0]).unwrap();

    open_a(&h);
    assert_eq!(h.overlay_ids().len(), 1, "jeden panel");

    let ov = h.overlay_rect(h.overlay_ids()[0]).unwrap();
    let sec_a = h.rect(h.children(h.root().unwrap())[0]).unwrap();
    assert!(approx(ov.left(), sec_a.left()), "od lewej sekcji: {ov:?}");
    assert!(approx(ov.top(), sec_a.top()), "from section top: {ov:?}");
    assert!(
        approx(ov.size.width, sec_a.size.width),
        "panel tak szeroki jak sekcja: {ov:?} vs {sec_a:?}"
    );
    assert!(
        ov.top() + ov.size.height > sec_a.top() + sec_a.size.height + 1.0,
        "sticks out below the section: {ov:?} vs {sec_a:?}"
    );
    assert!(
        approx(sec_a.size.width, sec_before.size.width)
            && approx(sec_a.size.height, sec_before.size.height),
        "section layout untouched by overlay"
    );
}

#[test]
fn press_opens_panel_without_release() {
    let (_, _, row) = bar();
    let h = Harness::new(row);
    h.layout(800.0, 600.0);
    h.paint();
    h.tick();

    let sec_a = h.children(h.root().unwrap())[0];
    let footer = h.children(sec_a)[1];
    let r = h.rect(footer).unwrap();
    h.pointer_down(r.left() + r.size.width * 0.5, r.top() + r.size.height * 0.5);
    h.layout(800.0, 600.0);
    h.paint();
    h.tick();

    assert_eq!(h.overlay_ids().len(), 1, "press alone opens");
    let ov = h.overlay_rect(h.overlay_ids()[0]).unwrap();
    let sec_now = h.rect(h.children(h.root().unwrap())[0]).unwrap();
    assert!(approx(ov.left(), sec_now.left()));
    assert!(approx(ov.top(), sec_now.top()));
    assert!(approx(ov.size.width, sec_now.size.width));
    assert!(
        ov.top() + ov.size.height > sec_now.top() + sec_now.size.height + 1.0,
        "panel sticks out below"
    );
}

#[test]
fn pick_tool_selects_and_closes() {
    let picked = Rc::new(std::cell::Cell::new(None::<usize>));
    let p = picked.clone();
    let mut a = SectionWidget::new("Operacje").visible(4).columns(4);
    for _ in 0..6 {
        a = a.tool(tool_button());
    }
    a = a.on_select(move |i| p.set(Some(i)));
    let sel_sig = a.st.selected.clone();
    let h = Harness::new(Row::new().fill_width().child(a));
    h.layout(800.0, 600.0);
    h.paint();
    h.tick();

    let sec = h.children(h.root().unwrap())[0];
    let footer = h.children(sec)[1];
    let fr = h.rect(footer).unwrap();
    h.click(
        fr.left() + fr.size.width * 0.5,
        fr.top() + fr.size.height * 0.5,
    );
    h.layout(800.0, 600.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1);

    let inner = h.children(sec);
    let title_h = h.rect(h.children(inner[0])[0]).unwrap().size.height;
    let ov = h.overlay_rect(h.overlay_ids()[0]).unwrap();
    let cell_cx = ov.left() + 8.0 + 2.0 + 12.0;
    let cell_cy = ov.top() + 8.0 + title_h + 8.0 + 2.0 + 12.0;
    h.click(cell_cx, cell_cy);
    h.layout(800.0, 600.0);
    h.paint();
    h.tick();

    assert_eq!(picked.get(), Some(0), "callback with index");
    assert_eq!(sel_sig.get_untracked(), Some(0), "selection in signal");
    assert!(h.overlay_ids().is_empty(), "selection closes the panel");
}

#[test]
fn escape_closes_panel() {
    let (_, _, row) = bar();
    let h = Harness::new(row);
    h.layout(800.0, 600.0);
    h.paint();
    h.tick();
    open_a(&h);
    assert_eq!(h.overlay_ids().len(), 1);

    h.key(Key::ESCAPE);
    h.layout(800.0, 600.0);
    h.paint();
    h.tick();
    assert!(h.overlay_ids().is_empty(), "Escape closes");
}

#[test]
fn narrow_window_grows_section_and_panel_follows() {
    let a = SectionWidget::new("Operacje")
        .tool(tool_button())
        .tool(tool_button())
        .tool(tool_button())
        .tool(tool_button())
        .tool(tool_button())
        .tool(tool_button())
        .columns(2)
        .fill_share();
    let b = SectionWidget::new("Rysunki")
        .tool(tool_button())
        .tool(tool_button())
        .tool(tool_button())
        .columns(2)
        .fill_share();
    let sep = Panel::new().width(1.5).fill_height();
    let row = Row::new()
        .background(Color::hex(0xF6F6F6))
        .fill_width()
        .children((a, sep, b));
    let h = Harness::new(row);
    h.layout(800.0, 600.0);
    h.paint();
    h.tick();
    let wide_h = h
        .rect(h.children(h.root().unwrap())[0])
        .unwrap()
        .size
        .height;

    h.layout(150.0, 600.0);
    h.paint();
    h.tick();
    let sec_a = h.rect(h.children(h.root().unwrap())[0]).unwrap();
    assert!(
        sec_a.size.height > wide_h + 1.0,
        "section grows after wrap: {} vs {wide_h}",
        sec_a.size.height
    );
    let parts = h.children(h.children(h.root().unwrap())[0]);
    let inner = h.children(parts[0]);
    let wrap_r = h.rect(inner[1]).unwrap();
    let foot_r = h.rect(parts[1]).unwrap();
    assert!(
        foot_r.top() >= wrap_r.top() + wrap_r.size.height - 0.01,
        "footer still under wrap"
    );

    let row_el = h.root().unwrap();
    let sec_el = h.children(row_el)[0];
    let footer = h.children(sec_el)[1];
    let fr = h.rect(footer).unwrap();
    h.click(
        fr.left() + fr.size.width * 0.5,
        fr.top() + fr.size.height * 0.5,
    );
    h.layout(150.0, 600.0);
    h.paint();
    h.tick();
    assert_eq!(h.overlay_ids().len(), 1);
    let ov = h.overlay_rect(h.overlay_ids()[0]).unwrap();
    let sec_now = h.rect(h.children(h.root().unwrap())[0]).unwrap();
    assert!(approx(ov.top(), sec_now.top()), "panel from section top");
    assert!(
        ov.top() + ov.size.height > sec_now.top() + sec_now.size.height + 1.0,
        "panel sticks out below"
    );
}
