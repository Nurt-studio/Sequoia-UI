use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use sequoia_core::Length;
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;
use sequoia_style::TreeStyle;

use crate::any::AnyView;
use crate::scroll_view::ScrollbarVisibility;
use crate::view::{EventCtx, View};
use crate::views::{Row, Spacer, Text, Unit};
use crate::virtual_list::VirtualList;

use sequoia_input::{Event, PointerButton, PointerEvent};

#[derive(Clone)]
pub struct TreeNode<T> {
    pub id: String,
    pub value: T,
    pub children: Vec<TreeNode<T>>,
    pub expanded: Signal<bool>,
}

impl<T: Clone> TreeNode<T> {
    pub fn leaf(id: impl Into<String>, value: T) -> Self {
        Self {
            id: id.into(),
            value,
            children: Vec::new(),
            expanded: Signal::new(false),
        }
    }

    pub fn branch(id: impl Into<String>, value: T, children: Vec<TreeNode<T>>) -> Self {
        Self {
            id: id.into(),
            value,
            children,
            expanded: Signal::new(true),
        }
    }

    pub fn expanded(self, expanded: bool) -> Self {
        self.expanded.set(expanded);
        self
    }
}

#[derive(Clone)]
pub struct FlatRow<T> {
    pub id: String,
    pub depth: usize,
    pub has_children: bool,
    pub expanded: Signal<bool>,
    pub value: T,
}

pub struct TreeView<T> {
    roots: Rc<dyn Fn() -> Vec<TreeNode<T>>>,
    row: Rc<dyn Fn(&T, usize) -> AnyView>,
    row_height: f32,
    indent: f32,
    overscan: usize,
    scrollbar: ScrollbarVisibility,
    style: Option<TreeStyle>,
    label: String,
    expanded: Rc<RefCell<HashMap<String, Signal<bool>>>>,
}

impl<T: Clone + 'static> Clone for TreeView<T> {
    fn clone(&self) -> Self {
        Self {
            roots: self.roots.clone(),
            row: self.row.clone(),
            row_height: self.row_height,
            indent: self.indent,
            overscan: self.overscan,
            scrollbar: self.scrollbar,
            style: self.style,
            label: self.label.clone(),
            expanded: Rc::clone(&self.expanded),
        }
    }
}

impl<T: Clone + 'static> TreeView<T> {
    pub fn new(roots: impl Fn() -> Vec<TreeNode<T>> + 'static) -> Self {
        Self {
            roots: Rc::new(roots),
            row: Rc::new(|_, _| AnyView::new(Unit)),
            row_height: 28.0,
            indent: 16.0,
            overscan: 4,
            scrollbar: ScrollbarVisibility::Auto,
            style: None,
            label: String::new(),
            expanded: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn style(mut self, style: TreeStyle) -> Self {
        self.style = Some(style);
        self
    }

    fn font_size(&self) -> f32 {
        self.style.and_then(|s| s.font_size).unwrap_or(12.0)
    }

    fn arrow_color(&self) -> Option<sequoia_core::Color> {
        self.style.and_then(|s| s.arrow_color)
    }

    pub fn row<V: View + 'static>(mut self, f: impl Fn(&T, usize) -> V + 'static) -> Self {
        self.row = Rc::new(move |v, d| AnyView::new(f(v, d)));
        self
    }

    pub fn row_height(mut self, h: f32) -> Self {
        self.row_height = h.max(1.0);
        self
    }

    pub fn indent(mut self, px: f32) -> Self {
        self.indent = px.max(0.0);
        self
    }

    pub fn overscan(mut self, n: usize) -> Self {
        self.overscan = n;
        self
    }

    pub fn flatten(&self) -> Vec<FlatRow<T>> {
        fn walk<T: Clone>(
            nodes: &[TreeNode<T>],
            depth: usize,
            expanded: &mut HashMap<String, Signal<bool>>,
            out: &mut Vec<FlatRow<T>>,
        ) {
            for n in nodes {
                let signal = expanded
                    .entry(n.id.clone())
                    .or_insert_with(|| n.expanded.clone())
                    .clone();
                let is_open = signal.get_untracked();
                out.push(FlatRow {
                    id: n.id.clone(),
                    depth,
                    has_children: !n.children.is_empty(),
                    expanded: signal,
                    value: n.value.clone(),
                });
                if !n.children.is_empty() && is_open {
                    walk(&n.children, depth + 1, expanded, out);
                }
            }
        }

        let roots = (self.roots)();
        let mut expanded = self.expanded.borrow_mut();
        let mut out = Vec::new();
        walk(&roots, 0, &mut expanded, &mut out);
        for r in &out {
            let _ = r.expanded.get();
        }
        out
    }
}

#[derive(Clone)]
struct ArrowBtn {
    expanded: Signal<bool>,
    has_children: bool,
    size: f32,
    color: Option<sequoia_core::Color>,
}

impl View for ArrowBtn {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let glyph = if !self.has_children {
            "  "
        } else if self.expanded.get() {
            "▾ "
        } else {
            "▸ "
        };
        let mut txt = Text::new(glyph).size(self.size);
        if let Some(color) = self.color {
            txt = txt.color(color);
        }
        Some(vec![AnyView::new(txt)])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        match event {
            Event::Pointer(PointerEvent::Up {
                button: PointerButton::Left,
                pos,
                ..
            }) if ctx.rect.contains(*pos) && self.has_children => {
                self.expanded.update(|e| *e = !*e);
                ctx.handled();
            }
            _ => {}
        }
    }
}

impl<T: Clone + 'static> View for TreeView<T> {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn accessibility(&self) -> Option<crate::a11y::Accessible> {
        Some(crate::a11y::Accessible::new(
            crate::a11y::AccessibleRole::Tree,
            self.label.clone(),
        ))
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let me = self.clone();
        let indent = self.indent;
        let row_fn2 = self.row.clone();
        let font_size = self.font_size();
        let arrow_color = self.arrow_color();
        let list = VirtualList::new(move || me.flatten())
            .key(|r: &FlatRow<T>| {
                let mut h = 0u64;
                for b in r.id.bytes() {
                    h = h.wrapping_mul(31).wrapping_add(b as u64);
                }
                h
            })
            .item(move |r: &FlatRow<T>| {
                let pad = (r.depth as f32) * indent;
                Row::new()
                    .gap(2.0)
                    .child(Spacer::fixed_width(pad))
                    .child(ArrowBtn {
                        expanded: r.expanded.clone(),
                        has_children: r.has_children,
                        size: font_size,
                        color: arrow_color,
                    })
                    .child_flex(row_fn2(&r.value, r.depth), 1.0)
            })
            .item_height(self.row_height)
            .overscan(self.overscan)
            .scrollbar_visibility(self.scrollbar);
        Some(vec![AnyView::new(list)])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }
}
