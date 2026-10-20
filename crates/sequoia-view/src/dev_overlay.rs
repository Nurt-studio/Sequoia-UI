use sequoia_layout::LayoutStyle;

use crate::any::AnyView;
use crate::runtime::Runtime;
use crate::view::View;
use crate::views::{Column, Row, Text, Unit};

#[derive(Clone)]
pub struct DevOverlay {
    rt: Runtime,
    tick: sequoia_reactive::Signal<u64>,
    show_errors: sequoia_reactive::Signal<bool>,
}

impl DevOverlay {
    pub fn new(rt: Runtime) -> Self {
        Self {
            rt,
            tick: sequoia_reactive::Signal::new(0),
            show_errors: sequoia_reactive::Signal::new(true),
        }
    }

    pub fn summary_untracked(&self) -> String {
        let s = self.rt.stats();
        format!(
            "el={} mounts={} rebuilds={} unmounts={} errors={}",
            self.rt.element_count(),
            s.mounts,
            s.rebuilds,
            s.unmounts,
            s.errors
        )
    }
}

impl View for DevOverlay {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let _ = self.tick.get();
        let show_errors = self.show_errors.get();
        let s = self.rt.stats();
        let header = Text::new(format!(
            "el={} mounts={} rebuilds={} unmounts={} errors={}",
            self.rt.element_count(),
            s.mounts,
            s.rebuilds,
            s.unmounts,
            s.errors
        ))
        .size(11.0);
        let tick = self.tick.clone();
        let refresh = RefreshBtn {
            on_press: std::rc::Rc::new(move || tick.update(|n| *n += 1)),
        };
        let show = self.show_errors.clone();
        let toggle = RefreshBtn {
            on_press: std::rc::Rc::new(move || show.update(|v| *v = !*v)),
        };
        let mut col = Column::new()
            .gap(6.0)
            .child(header)
            .child(Row::new().gap(8.0).child(refresh).child(toggle));
        if show_errors {
            for e in self.rt.errors().into_iter().rev().take(8) {
                col = col.child(Text::new(format!("⚠ {}: {}", e.type_name, e.message)).size(11.0));
            }
        }
        Some(vec![AnyView::new(col)])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
    }
}

#[derive(Clone)]
struct RefreshBtn {
    on_press: std::rc::Rc<dyn Fn()>,
}

impl View for RefreshBtn {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![AnyView::new(Text::new("↻ refresh").size(11.0).bold())])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
    }

    fn on_event(&self, event: &sequoia_input::Event, ctx: &mut crate::view::EventCtx) {
        match event {
            sequoia_input::Event::Pointer(sequoia_input::PointerEvent::Up {
                button: sequoia_input::PointerButton::Left,
                pos,
                ..
            }) if ctx.rect.contains(*pos) => {
                (self.on_press)();
                ctx.handled();
            }
            _ => {}
        }
    }
}
