use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use sequoia_core::Length;
use sequoia_dock::{DockModel, PanelId};
use sequoia_layout::LayoutStyle;
use sequoia_reactive::Signal;

use crate::any::AnyView;
use crate::dock_view::{PanelRenderer, TitleProvider, default_title};
use crate::view::{PaintCtx, View};
use crate::views::{Text, Unit};

#[derive(Clone, Debug)]
pub struct DetachedWindow {
    pub panel: PanelId,

    pub title: String,

    pub w: f64,
    pub h: f64,
}

#[derive(Clone)]
pub struct DockWindowHost {
    model: Signal<DockModel>,
    title: TitleProvider,
    on_open: Rc<dyn Fn(DetachedWindow)>,
    on_close: Rc<dyn Fn(u64)>,
    known: Rc<RefCell<HashMap<u64, u64>>>,
}

impl DockWindowHost {
    pub fn new(model: Signal<DockModel>, _render: impl Fn(PanelId) -> AnyView + 'static) -> Self {
        Self {
            model,
            title: Rc::new(default_title),
            on_open: Rc::new(|_| {}),
            on_close: Rc::new(|_| {}),
            known: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn title_provider(mut self, f: impl Fn(PanelId) -> String + 'static) -> Self {
        self.title = Rc::new(f);
        self
    }

    pub fn on_open(mut self, f: impl Fn(DetachedWindow) + 'static) -> Self {
        self.on_open = Rc::new(f);
        self
    }

    pub fn on_close(mut self, f: impl Fn(u64) + 'static) -> Self {
        self.on_close = Rc::new(f);
        self
    }

    pub fn tracked(&self) -> Vec<(PanelId, u64)> {
        self.known
            .borrow()
            .iter()
            .map(|(p, w)| (PanelId::new(*p), *w))
            .collect()
    }

    pub fn detach(
        model: &Signal<DockModel>,
        panel: PanelId,
        w: f32,
        h: f32,
        alloc: impl FnOnce(DetachedWindow) -> u64,
    ) -> bool {
        let floated = {
            let mut ok = false;
            model.update(|m| {
                ok = m.apply(sequoia_dock::DockOp::Float {
                    panel,
                    x: 80.0,
                    y: 80.0,
                    w,
                    h,
                });
            });
            ok
        };
        if !floated && !model.get_untracked().is_floating(panel) {
            return false;
        }
        let current = model.get_untracked().floating_of(panel);
        let (w, h) = current.map(|f| (f.w, f.h)).unwrap_or((w, h));
        let win = alloc(DetachedWindow {
            panel,
            title: format!("{panel}"),
            w: w.max(1.0) as f64,
            h: h.max(1.0) as f64,
        });
        let mut attached = false;
        model.update(|m| {
            attached = m.attach_window(panel, Some(win));
        });
        if !attached {
            return false;
        }
        true
    }

    fn sync(&self) {
        let m = self.model.get_untracked();
        let mut current: HashMap<u64, (u64, String, f32, f32)> = HashMap::new();
        for f in &m.floating {
            if let Some(win) = f.window {
                current.insert(f.panel.get(), (win, (self.title)(f.panel), f.w, f.h));
            }
        }
        let mut known = self.known.borrow_mut();

        let mut opens: Vec<DetachedWindow> = Vec::new();
        for (panel, (win, title, w, h)) in &current {
            match known.get(panel) {
                Some(known_win) if *known_win == *win => {}
                Some(old) => {
                    let old = *old;
                    (self.on_close)(old);
                    opens.push(DetachedWindow {
                        panel: PanelId::new(*panel),
                        title: title.clone(),
                        w: (*w).max(1.0) as f64,
                        h: (*h).max(1.0) as f64,
                    });
                    known.insert(*panel, *win);
                }
                None => {
                    opens.push(DetachedWindow {
                        panel: PanelId::new(*panel),
                        title: title.clone(),
                        w: (*w).max(1.0) as f64,
                        h: (*h).max(1.0) as f64,
                    });
                    known.insert(*panel, *win);
                }
            }
        }

        let stale: Vec<u64> = known
            .keys()
            .filter(|p| !current.contains_key(*p))
            .copied()
            .collect();
        let mut closes: Vec<u64> = Vec::new();
        for panel in stale {
            if let Some(win) = known.remove(&panel) {
                closes.push(win);
            }
        }
        drop(known);
        for d in opens {
            (self.on_open)(d);
        }
        for w in closes {
            (self.on_close)(w);
        }
    }
}

impl View for DockWindowHost {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn keyed_children(&self) -> Option<Vec<(u64, AnyView)>> {
        let _ = self.model.get();
        Some(Vec::new())
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn hit_testable(&self) -> bool {
        false
    }

    fn on_unmount(&self) {
        let wins: Vec<u64> = self.known.borrow().values().copied().collect();
        self.known.borrow_mut().clear();
        for w in wins {
            (self.on_close)(w);
        }
    }

    fn paint_self(&self, _ctx: &mut PaintCtx) {
        self.sync();
    }
}

#[derive(Clone)]
pub struct DockWindowRoot {
    model: Signal<DockModel>,
    panel: PanelId,
    render: PanelRenderer,
}

impl DockWindowRoot {
    pub fn new(
        model: Signal<DockModel>,
        panel: PanelId,
        render: impl Fn(PanelId) -> AnyView + 'static,
    ) -> Self {
        Self {
            model,
            panel,
            render: Rc::new(render),
        }
    }

    pub fn panel(&self) -> PanelId {
        self.panel
    }
}

impl View for DockWindowRoot {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let m = self.model.get();
        if m.contains(self.panel) {
            Some(vec![(self.render)(self.panel)])
        } else {
            Some(vec![AnyView::new(Text::new(
                "Panel zadokowany gdzie indziej",
            ))])
        }
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }
}
