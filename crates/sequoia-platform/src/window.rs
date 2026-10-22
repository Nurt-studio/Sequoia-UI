use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use sequoia_core::WindowId;
use sequoia_render::{PowerPreference, PresentMode};
use sequoia_view::{AnyView, View};

#[derive(Clone, Debug)]
pub struct WindowSettings {
    pub(crate) min_size: Option<(f64, f64)>,
    pub(crate) max_size: Option<(f64, f64)>,
    pub(crate) position: Option<(f64, f64)>,
    pub(crate) resizable: bool,
    pub(crate) maximized: bool,
    pub(crate) fullscreen: bool,
    pub(crate) always_on_top: bool,
    pub(crate) decorations: bool,
    pub(crate) transparent: bool,
    pub(crate) visible: bool,
    pub(crate) icon: Option<String>,
    pub(crate) icon_tint: Option<sequoia_core::Color>,
    pub(crate) present_mode: PresentMode,
    pub(crate) power_preference: PowerPreference,
    pub(crate) fps_limit: Option<u32>,
    pub(crate) show_fps: bool,
    pub(crate) scale_override: Option<f64>,
    pub(crate) cursor_visible: bool,
    pub(crate) cursor_grab: bool,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            min_size: None,
            max_size: None,
            position: None,
            resizable: true,
            maximized: false,
            fullscreen: false,
            always_on_top: false,
            decorations: true,
            transparent: false,
            visible: true,
            icon: None,
            icon_tint: None,
            present_mode: PresentMode::Vsync,
            power_preference: PowerPreference::HighPerformance,
            fps_limit: None,
            show_fps: false,
            scale_override: None,
            cursor_visible: true,
            cursor_grab: false,
        }
    }
}

#[derive(Clone)]
pub struct WindowSpec {
    pub(crate) title: String,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) root: Option<AnyView>,
    pub(crate) settings: WindowSettings,
    pub(crate) on_close_request: Option<Rc<dyn Fn() -> bool>>,
}

impl std::fmt::Debug for WindowSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowSpec")
            .field("title", &self.title)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("has_root", &self.root.is_some())
            .field("settings", &self.settings)
            .field("has_on_close", &self.on_close_request.is_some())
            .finish()
    }
}

impl WindowSpec {
    pub fn new(title: impl Into<String>, root: impl View + 'static) -> Self {
        Self {
            title: title.into(),
            width: 1024.0,
            height: 768.0,
            root: Some(AnyView::new(root)),
            settings: WindowSettings::default(),
            on_close_request: None,
        }
    }

    pub fn untitled() -> Self {
        Self {
            title: "Sequoia".into(),
            width: 1024.0,
            height: 768.0,
            root: None,
            settings: WindowSettings::default(),
            on_close_request: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: f64, height: f64) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn root(mut self, root: impl View + 'static) -> Self {
        self.root = Some(AnyView::new(root));
        self
    }

    pub fn on_close_request(mut self, f: impl Fn() -> bool + 'static) -> Self {
        self.on_close_request = Some(Rc::new(f));
        self
    }

    pub fn min_size(mut self, width: f64, height: f64) -> Self {
        self.settings.min_size = Some((width, height));
        self
    }

    pub fn max_size(mut self, width: f64, height: f64) -> Self {
        self.settings.max_size = Some((width, height));
        self
    }

    pub fn position(mut self, x: f64, y: f64) -> Self {
        self.settings.position = Some((x, y));
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.settings.resizable = resizable;
        self
    }

    pub fn maximized(mut self, maximized: bool) -> Self {
        self.settings.maximized = maximized;
        self
    }

    pub fn fullscreen(mut self, fullscreen: bool) -> Self {
        self.settings.fullscreen = fullscreen;
        self
    }

    pub fn always_on_top(mut self, always_on_top: bool) -> Self {
        self.settings.always_on_top = always_on_top;
        self
    }

    pub fn decorations(mut self, decorations: bool) -> Self {
        self.settings.decorations = decorations;
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.settings.transparent = transparent;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.settings.visible = visible;
        self
    }

    pub fn window_icon(mut self, name: impl Into<String>) -> Self {
        self.settings.icon = Some(name.into());
        self
    }

    pub fn window_icon_tint(mut self, tint: sequoia_core::Color) -> Self {
        self.settings.icon_tint = Some(tint);
        self
    }

    pub fn present_mode(mut self, mode: PresentMode) -> Self {
        self.settings.present_mode = mode;
        self
    }

    pub fn power_preference(mut self, power: PowerPreference) -> Self {
        self.settings.power_preference = power;
        self
    }

    pub fn fps_limit(mut self, fps: u32) -> Self {
        self.settings.fps_limit = Some(fps.max(1));
        self
    }

    pub fn show_fps(mut self, show: bool) -> Self {
        self.settings.show_fps = show;
        self
    }

    pub fn scale_factor(mut self, scale: f64) -> Self {
        self.settings.scale_override = Some(scale);
        self
    }

    pub fn cursor_visible(mut self, visible: bool) -> Self {
        self.settings.cursor_visible = visible;
        self
    }

    pub fn cursor_grab(mut self, grab: bool) -> Self {
        self.settings.cursor_grab = grab;
        self
    }
}

#[derive(Clone, Debug)]
pub(crate) enum WindowRequest {
    Open(WindowId),

    Close(WindowId),

    SetTitle(WindowId, String),

    Minimize(WindowId),

    ToggleMaximize(WindowId),

    BeginDrag(WindowId),
}

struct WindowsInner {
    next: Cell<u64>,

    specs: RefCell<HashMap<WindowId, WindowSpec>>,

    pending: RefCell<Vec<WindowRequest>>,
}

#[derive(Clone)]
pub struct Windows {
    inner: Rc<WindowsInner>,
}

impl Default for Windows {
    fn default() -> Self {
        Self::new()
    }
}

impl Windows {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(WindowsInner {
                next: Cell::new(0),
                specs: RefCell::new(HashMap::new()),
                pending: RefCell::new(Vec::new()),
            }),
        }
    }

    fn alloc(&self) -> WindowId {
        let id = WindowId::new(self.inner.next.get());
        self.inner.next.set(id.get() + 1);
        id
    }

    pub(crate) fn push_initial(&self, spec: WindowSpec) -> WindowId {
        let id = self.alloc();
        self.inner.specs.borrow_mut().insert(id, spec);
        id
    }

    pub fn open(&self, spec: WindowSpec) -> WindowId {
        let id = self.alloc();
        self.inner.specs.borrow_mut().insert(id, spec);
        self.inner
            .pending
            .borrow_mut()
            .push(WindowRequest::Open(id));
        id
    }

    pub fn close(&self, id: WindowId) -> bool {
        let existed = self.inner.specs.borrow_mut().remove(&id).is_some();
        if existed {
            self.inner
                .pending
                .borrow_mut()
                .push(WindowRequest::Close(id));
        }
        existed
    }

    pub fn set_title(&self, id: WindowId, title: impl Into<String>) -> bool {
        let title = title.into();
        let mut specs = self.inner.specs.borrow_mut();
        let Some(spec) = specs.get_mut(&id) else {
            return false;
        };
        spec.title = title.clone();
        self.inner
            .pending
            .borrow_mut()
            .push(WindowRequest::SetTitle(id, title));
        true
    }

    pub fn minimize(&self, id: WindowId) -> bool {
        if !self.contains(id) {
            return false;
        }
        self.inner
            .pending
            .borrow_mut()
            .push(WindowRequest::Minimize(id));
        true
    }

    pub fn toggle_maximize(&self, id: WindowId) -> bool {
        if !self.contains(id) {
            return false;
        }
        self.inner
            .pending
            .borrow_mut()
            .push(WindowRequest::ToggleMaximize(id));
        true
    }

    pub fn begin_drag(&self, id: WindowId) -> bool {
        if !self.contains(id) {
            return false;
        }
        self.inner
            .pending
            .borrow_mut()
            .push(WindowRequest::BeginDrag(id));
        true
    }

    pub fn contains(&self, id: WindowId) -> bool {
        self.inner.specs.borrow().contains_key(&id)
    }

    pub fn count(&self) -> usize {
        self.inner.specs.borrow().len()
    }

    pub fn ids(&self) -> Vec<WindowId> {
        self.inner.specs.borrow().keys().copied().collect()
    }

    pub fn title_of(&self, id: WindowId) -> Option<String> {
        self.inner.specs.borrow().get(&id).map(|s| s.title.clone())
    }

    pub(crate) fn spec_of(&self, id: WindowId) -> Option<WindowSpec> {
        self.inner.specs.borrow().get(&id).cloned()
    }

    pub(crate) fn take_requests(&self) -> Vec<WindowRequest> {
        std::mem::take(&mut *self.inner.pending.borrow_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_view::Unit;

    fn spec(title: &str) -> WindowSpec {
        WindowSpec::new(title, Unit)
    }

    #[test]
    fn initial_ids_start_at_primary() {
        let w = Windows::new();
        let a = w.push_initial(spec("A"));
        let b = w.push_initial(spec("B"));
        assert_eq!(a, WindowId::PRIMARY);
        assert_eq!(b, WindowId::new(1));
        assert!(
            w.take_requests().is_empty(),
            "startup windows queue nothing"
        );
    }

    #[test]
    fn open_queues_and_registers() {
        let w = Windows::new();
        w.push_initial(spec("main"));
        let id = w.open(spec("second"));
        assert_eq!(id, WindowId::new(1));
        assert!(w.contains(id));
        assert_eq!(w.count(), 2);
        assert_eq!(w.title_of(id).as_deref(), Some("second"));
        let reqs = w.take_requests();
        assert_eq!(reqs.len(), 1);
        assert!(matches!(reqs[0], WindowRequest::Open(i) if i == id));
        assert!(w.take_requests().is_empty(), "drain clears the queue");
    }

    #[test]
    fn close_removes_and_queues() {
        let w = Windows::new();
        w.push_initial(spec("main"));
        let id = w.open(spec("second"));
        w.take_requests();
        assert!(w.close(id));
        assert!(!w.contains(id));
        assert_eq!(w.count(), 1);
        let reqs = w.take_requests();
        assert_eq!(reqs.len(), 1);
        assert!(matches!(reqs[0], WindowRequest::Close(i) if i == id));
        assert!(!w.close(id), "second close is a no-op");
        assert!(!w.close(WindowId::new(999)), "unknown id is a no-op");
    }

    #[test]
    fn set_title_updates_spec_and_queues() {
        let w = Windows::new();
        let id = w.push_initial(spec("old"));
        assert!(w.set_title(id, "new"));
        assert_eq!(w.title_of(id).as_deref(), Some("new"));
        let reqs = w.take_requests();
        assert_eq!(reqs.len(), 1);
        assert!(matches!(&reqs[0], WindowRequest::SetTitle(i, t) if *i == id && t == "new"));
        assert!(!w.set_title(WindowId::new(999), "x"));
    }

    #[test]
    fn window_controls_queue_and_reject_unknown() {
        let w = Windows::new();
        let id = w.push_initial(spec("main"));
        assert!(w.minimize(id));
        assert!(w.toggle_maximize(id));
        assert!(w.begin_drag(id));
        let reqs = w.take_requests();
        assert_eq!(reqs.len(), 3);
        assert!(matches!(reqs[0], WindowRequest::Minimize(i) if i == id));
        assert!(matches!(reqs[1], WindowRequest::ToggleMaximize(i) if i == id));
        assert!(matches!(reqs[2], WindowRequest::BeginDrag(i) if i == id));

        let unknown = WindowId::new(999);
        assert!(!w.minimize(unknown));
        assert!(!w.toggle_maximize(unknown));
        assert!(!w.begin_drag(unknown));
        assert!(w.take_requests().is_empty());
    }

    #[test]
    fn open_then_close_before_drain_skips_creation() {
        let w = Windows::new();
        w.push_initial(spec("main"));
        let id = w.open(spec("flash"));
        assert!(w.close(id));
        let reqs = w.take_requests();
        assert_eq!(reqs.len(), 2);
        assert!(w.spec_of(id).is_none());
    }

    #[test]
    fn spec_builder_defaults() {
        let s = WindowSpec::new("T", Unit);
        assert_eq!(s.title, "T");
        assert_eq!((s.width, s.height), (1024.0, 768.0));
        assert!(s.root.is_some());
        assert!(s.settings.resizable && s.settings.decorations && s.settings.visible);
        let s2 = WindowSpec::new("T", Unit)
            .size(400.0, 300.0)
            .decorations(false)
            .always_on_top(true)
            .fps_limit(30);
        assert_eq!((s2.width, s2.height), (400.0, 300.0));
        assert!(!s2.settings.decorations);
        assert!(s2.settings.always_on_top);
        assert_eq!(s2.settings.fps_limit, Some(30));
    }

    #[test]
    fn show_fps_defaults_off_and_builder_enables() {
        let s = WindowSpec::new("T", Unit);
        assert!(!s.settings.show_fps);
        let s2 = WindowSpec::new("T", Unit).show_fps(true);
        assert!(s2.settings.show_fps);
    }
}
