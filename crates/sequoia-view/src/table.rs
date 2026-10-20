use std::cmp::Ordering;
use std::rc::Rc;

use sequoia_core::Length;
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;
use sequoia_style::TableStyle;

use crate::any::AnyView;
use crate::scroll_view::ScrollbarVisibility;
use crate::view::View;
use crate::views::{Column, Row, Text, Unit};
use crate::virtual_list::VirtualList;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortDir {
    #[default]
    None,
    Asc,
    Desc,
}

impl SortDir {
    pub fn next(self) -> Self {
        match self {
            SortDir::None => SortDir::Asc,
            SortDir::Asc => SortDir::Desc,
            SortDir::Desc => SortDir::None,
        }
    }

    pub fn arrow(self) -> &'static str {
        match self {
            SortDir::None => "",
            SortDir::Asc => " ▲",
            SortDir::Desc => " ▼",
        }
    }
}

type SortKeyFn<R> = Rc<dyn Fn(&R, &R) -> Ordering>;
type MatchesFn<R> = Rc<dyn Fn(&R, &str) -> bool>;
type CellFn<R> = Rc<dyn Fn(&R) -> AnyView>;

pub struct TableColumn<R> {
    pub header: String,
    pub width: Length,
    pub sortable: bool,
    pub sort_key: Option<SortKeyFn<R>>,
    pub filter: Signal<String>,
    pub filter_enabled: bool,
    pub matches: MatchesFn<R>,
    pub cell: CellFn<R>,
}

impl<R: 'static> Clone for TableColumn<R> {
    fn clone(&self) -> Self {
        Self {
            header: self.header.clone(),
            width: self.width,
            sortable: self.sortable,
            sort_key: self.sort_key.clone(),
            filter: self.filter.clone(),
            filter_enabled: self.filter_enabled,
            matches: self.matches.clone(),
            cell: self.cell.clone(),
        }
    }
}

impl<R: 'static> TableColumn<R> {
    pub fn new(header: impl Into<String>) -> Self {
        Self {
            header: header.into(),
            width: Length::Fill(1.0),
            sortable: false,
            sort_key: None,
            filter: Signal::new(String::new()),
            filter_enabled: false,
            matches: Rc::new(|_, _| true),
            cell: Rc::new(|_| AnyView::new(Unit)),
        }
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    pub fn sort_key(mut self, f: impl Fn(&R, &R) -> Ordering + 'static) -> Self {
        self.sort_key = Some(Rc::new(f));
        self.sortable = true;
        self
    }

    pub fn filterable(mut self) -> Self {
        self.filter_enabled = true;
        self
    }

    pub fn matches(mut self, f: impl Fn(&R, &str) -> bool + 'static) -> Self {
        self.matches = Rc::new(f);
        self.filter_enabled = true;
        self
    }

    pub fn cell<V: View + 'static>(mut self, f: impl Fn(&R) -> V + 'static) -> Self {
        self.cell = Rc::new(move |r| AnyView::new(f(r)));
        self
    }

    pub fn text_cell(mut self, f: impl Fn(&R) -> String + 'static) -> Self
    where
        R: Clone,
    {
        self.cell = Rc::new(move |r: &R| AnyView::new(Text::new(f(r))));
        self
    }
}

pub struct Table<R> {
    rows: Rc<dyn Fn() -> Vec<R>>,
    key: Rc<dyn Fn(&R) -> u64>,
    columns: Vec<TableColumn<R>>,
    sort: Signal<Option<(usize, SortDir)>>,
    row_height: f32,
    overscan: usize,
    scrollbar: ScrollbarVisibility,
    style: Option<TableStyle>,
    label: String,
}

impl<R: Clone + 'static> Clone for Table<R> {
    fn clone(&self) -> Self {
        Self {
            rows: self.rows.clone(),
            key: self.key.clone(),
            columns: self.columns.clone(),
            sort: self.sort.clone(),
            row_height: self.row_height,
            overscan: self.overscan,
            scrollbar: self.scrollbar,
            style: self.style,
            label: self.label.clone(),
        }
    }
}

impl<R: Clone + 'static> Table<R> {
    pub fn new(rows: impl Fn() -> Vec<R> + 'static) -> Self {
        Self {
            rows: Rc::new(rows),
            key: Rc::new(|_| 0),
            columns: Vec::new(),
            sort: Signal::new(None),
            row_height: 28.0,
            overscan: 4,
            scrollbar: ScrollbarVisibility::Auto,
            style: None,
            label: String::new(),
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn style(mut self, style: TableStyle) -> Self {
        self.style = Some(style);
        self
    }

    fn header_size(&self) -> f32 {
        self.style.and_then(|s| s.header_size).unwrap_or(12.0)
    }

    fn header_color(&self) -> Option<sequoia_core::Color> {
        self.style.and_then(|s| s.header_color)
    }

    fn stripe(&self, index: usize) -> Option<sequoia_core::Color> {
        if index % 2 == 1 {
            self.style.and_then(|s| s.row_stripe)
        } else {
            None
        }
    }

    pub fn key(mut self, key: impl Fn(&R) -> u64 + 'static) -> Self {
        self.key = Rc::new(key);
        self
    }

    pub fn column(mut self, col: TableColumn<R>) -> Self {
        self.columns.push(col);
        self
    }

    pub fn row_height(mut self, h: f32) -> Self {
        self.row_height = h.max(1.0);
        self
    }

    pub fn overscan(mut self, n: usize) -> Self {
        self.overscan = n;
        self
    }

    pub fn scrollbar_visibility(mut self, v: ScrollbarVisibility) -> Self {
        self.scrollbar = v;
        self
    }

    pub fn sort_signal(&self) -> Signal<Option<(usize, SortDir)>> {
        self.sort.clone()
    }

    pub fn column_filter(&self, index: usize) -> Option<Signal<String>> {
        self.columns.get(index).map(|c| c.filter.clone())
    }

    pub fn columns_for_test(&self) -> Vec<TableColumn<R>> {
        self.columns.clone()
    }

    pub fn filtered_sorted(&self) -> Vec<R> {
        let mut out = (self.rows)();
        out.retain(|r| {
            self.columns.iter().all(|c| {
                if !c.filter_enabled {
                    return true;
                }
                let q = c.filter.get_untracked();
                if q.is_empty() {
                    return true;
                }
                (c.matches)(r, &q)
            })
        });
        if let Some((idx, dir)) = self.sort.get_untracked()
            && dir != SortDir::None
            && let Some(col) = self.columns.get(idx)
            && let Some(key) = &col.sort_key
        {
            match dir {
                SortDir::Asc => out.sort_by(|a, b| key(a, b)),
                SortDir::Desc => out.sort_by(|a, b| key(b, a)),
                SortDir::None => {}
            }
        }
        out
    }

    fn header(&self) -> AnyView {
        let mut row = Row::new().gap(4.0);
        for (i, col) in self.columns.iter().enumerate() {
            let header = col.header.clone();
            let sortable = col.sortable && col.sort_key.is_some();
            let sort = self.sort.clone();
            let dir = self
                .sort
                .with_untracked(|s| s.filter(|(j, _)| *j == i).map(|(_, d)| d))
                .unwrap_or(SortDir::None);
            let label = format!("{header}{}", dir.arrow());
            let cell: AnyView = if sortable {
                AnyView::new(HeaderCell {
                    label,
                    size: self.header_size(),
                    color: self.header_color(),
                    sort,
                    index: i,
                })
            } else {
                let mut txt = Text::new(label).size(self.header_size()).bold();
                if let Some(color) = self.header_color() {
                    txt = txt.color(color);
                }
                AnyView::new(txt)
            };
            row = match col.width {
                Length::Fill(w) => row.child_flex(cell, w.max(0.0)),
                _ => row.child(cell),
            };
        }
        AnyView::new(row)
    }

    fn list(&self) -> AnyView {
        let me = self.clone();
        let list = VirtualList::new(move || me.filtered_sorted())
            .key({
                let key = self.key.clone();
                move |r: &R| key(r)
            })
            .item_indexed({
                let columns = self.columns.clone();
                let stripe_of = self.clone();
                move |index: usize, r: &R| {
                    let mut row = Row::new().gap(4.0);
                    for col in &columns {
                        let cell = (col.cell)(r);
                        row = match col.width {
                            Length::Fill(w) => row.child_flex(cell, w.max(0.0)),
                            _ => row.child(cell),
                        };
                    }
                    match stripe_of.stripe(index) {
                        Some(bg) => row.background(bg),
                        None => row,
                    }
                }
            })
            .item_height(self.row_height)
            .overscan(self.overscan)
            .scrollbar_visibility(self.scrollbar);
        AnyView::new(list)
    }
}

impl<R: Clone + 'static> View for Table<R> {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn accessibility(&self) -> Option<crate::a11y::Accessible> {
        Some(crate::a11y::Accessible::new(
            crate::a11y::AccessibleRole::Table,
            self.label.clone(),
        ))
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let _ = (self.rows)();
        let _ = self.sort.get();
        for col in &self.columns {
            if col.filter_enabled {
                let _ = col.filter.get();
            }
        }
        Some(vec![self.header(), self.list()])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }
}

#[derive(Clone)]
struct HeaderCell {
    label: String,
    size: f32,
    color: Option<sequoia_core::Color>,
    sort: Signal<Option<(usize, SortDir)>>,
    index: usize,
}

impl View for HeaderCell {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let mut txt = Text::new(self.label.clone()).size(self.size).bold();
        if let Some(color) = self.color {
            txt = txt.color(color);
        }
        Some(vec![AnyView::new(txt)])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn cursor_icon(&self, _state: sequoia_style::VisualState) -> sequoia_input::CursorIcon {
        sequoia_input::CursorIcon::Pointer
    }

    fn on_event(&self, event: &sequoia_input::Event, ctx: &mut crate::view::EventCtx) {
        use sequoia_input::{Event, PointerButton, PointerEvent};
        match event {
            Event::Pointer(PointerEvent::Up {
                button: PointerButton::Left,
                pos,
                ..
            }) if ctx.rect.contains(*pos) => {
                let i = self.index;
                self.sort.update(|s| {
                    let next = match s {
                        Some((j, d)) if *j == i => d.next(),
                        _ => SortDir::Asc,
                    };
                    *s = if next == SortDir::None {
                        None
                    } else {
                        Some((i, next))
                    };
                });
                ctx.handled();
            }
            _ => {}
        }
    }
}

pub fn table_filters<R: Clone + 'static>(table: &Table<R>) -> AnyView {
    use crate::views::Unit as U;
    let mut row = Row::new().gap(4.0);
    let mut any = false;
    for col in &table.columns {
        if col.filter_enabled {
            any = true;
            let filter = col.filter.clone();

            let txt = Text::with(move || {
                let q = filter.get();
                if q.is_empty() {
                    String::new()
                } else {
                    format!("⊃ {q}")
                }
            })
            .size(11.0);
            row = match col.width {
                Length::Fill(w) => row.child_flex(txt, w.max(0.0)),
                _ => row.child(txt),
            };
        }
    }
    if any {
        AnyView::new(row)
    } else {
        AnyView::new(U)
    }
}

#[allow(dead_code)]
fn _uses_column() {
    let _ = Column::new();
}
