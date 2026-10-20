use std::rc::Rc;

use sequoia_core::{Constraints, Length, Rect, Size};
use sequoia_layout::{LayoutCtx, LayoutStyle, resolve_cross};

use crate::any::AnyView;
use crate::runtime::{ElementId, Runtime};
use crate::view::View;
use crate::views::Unit;

pub struct List<T> {
    items: Rc<dyn Fn() -> Vec<T>>,
    key: Option<Rc<dyn Fn(&T) -> u64>>,
    item: Rc<dyn Fn(&T) -> AnyView>,
}

impl<T: 'static> List<T> {
    pub fn new<F: Fn() -> Vec<T> + 'static>(items: F) -> Self {
        Self {
            items: Rc::new(items),
            key: None,
            item: Rc::new(|_| AnyView::new(Unit)),
        }
    }

    pub fn key<K: Fn(&T) -> u64 + 'static>(mut self, key: K) -> Self {
        self.key = Some(Rc::new(key));
        self
    }

    pub fn item<I, F>(mut self, item: F) -> Self
    where
        I: View + 'static,
        F: Fn(&T) -> I + 'static,
    {
        self.item = Rc::new(move |t: &T| AnyView::new(item(t)));
        self
    }
}

impl<T: 'static> View for List<T> {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let items = (self.items)();
        let key = self.key.clone();
        let item = self.item.clone();
        Some(
            items
                .iter()
                .enumerate()
                .map(|(i, it)| {
                    let k = match &key {
                        Some(f) => f(it),
                        None => i as u64,
                    };
                    (k, item(it))
                })
                .collect(),
        )
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }

    fn arrange_override(
        &self,
        rt: &Runtime,
        el: ElementId,
        rect: Rect,
        ctx: &mut LayoutCtx,
    ) -> bool {
        let children = rt.children(el);
        if children.is_empty() {
            return true;
        }
        let mut rows: Vec<(ElementId, f32, f32)> = Vec::with_capacity(children.len());
        let mut total = 0.0_f32;
        for child in children {
            let cs = rt.child_layout_style(child);
            let m = rt.measure_node(
                child,
                Constraints {
                    min: Size::ZERO,
                    max: Size::new(rect.size.width, f32::INFINITY),
                },
                ctx,
            );
            let w = cs
                .clamp_width(resolve_cross(cs.width, m.width, rect.size.width))
                .max(0.0);
            let h = cs.clamp_height(m.height).max(0.0);
            total += h;
            rows.push((child, w, h));
        }

        if total <= 0.0 && rect.size.height.is_finite() && rect.size.height > 0.0 {
            let even = rect.size.height / rows.len() as f32;
            for row in &mut rows {
                row.2 = even;
            }
        }
        let mut cursor = rect.top();
        for (child, w, h) in rows {
            rt.arrange_node(child, Rect::from_xywh(rect.left(), cursor, w, h), ctx);
            cursor += h;
        }
        true
    }
}
