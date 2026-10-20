use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::panic::AssertUnwindSafe;
use std::rc::{Rc, Weak};

use crate::dnd::{
    ActiveDrag, DragEnd, DragId, DragOptions, DragPayload, DragSession, DropPosition, PendingDrag,
    exceeded_threshold, grab_offset, preview_origin,
};
use crate::overlay::{
    Anchor, InputMode, OverlayEntry, OverlayId, OverlayLayer, OverlayStore, compute_position,
};
use sequoia_core::{
    Constraints, Dirty, Edges, Length, Point, Rect, RenderList, Size, Transform, Vec2, WindowId,
};
use sequoia_input::{
    CursorIcon, Event, EventPhase, ImeEvent, Key, KeyEvent, Modifiers, PenTool, PointerButton,
    PointerEvent,
};
use sequoia_layout::{
    Align, Axis, ContainerStyle, Dock, DockStyle, FlexItem, GridPlacement, GridStyle, LayoutCtx,
    LayoutStyle, Overflow, Track, WrapStyle, clamp_to, distribute_flex, dock_layout_order,
    dock_thickness, resolve_cross, resolve_dock, resolve_flex_basis, resolve_main, resolve_tracks,
    sanitize_gap, track_offsets, wrap_lines,
};
use sequoia_reactive::{Environment, ReadSet, Signal, Watcher, capture_reads, flush_effects};
use sequoia_style::VisualState;
use sequoia_text::TextSystem;

use crate::any::{AnyView, ErasedView};
use crate::live::{AnimSpec, AnimState, Live, Prop, SpringSpec};
use crate::surface::SurfaceDraw;
use crate::view::{EventCtx, PaintCtx, View};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ElementId {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

pub trait ViewState: 'static + Default {}
impl<T: Default + 'static> ViewState for T {}

const MAX_ERRORS: usize = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    pub mounts: u64,

    pub unmounts: u64,

    pub rebuilds: u64,

    pub measure_invalidations: u64,

    pub errors: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeError {
    pub element: ElementId,

    pub type_name: String,

    pub message: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InteractState {
    pub hovered: Option<ElementId>,

    pub pressed: Option<ElementId>,

    pub captured: Option<ElementId>,

    pub focus: Option<ElementId>,

    pub cursor: Point,

    pub pointer_seen: bool,
}

struct Element {
    parent: Option<ElementId>,
    key: u64,
    children: Vec<ElementId>,
    view: Option<Rc<dyn ErasedView>>,
    watcher: Option<Watcher>,
    reads: ReadSet,
    dirty: Dirty,
    rect: Cell<Rect>,
}

#[derive(Default)]
struct Arena {
    slots: Vec<Option<Element>>,
    generations: Vec<u32>,
    free: Vec<u32>,
}

impl Arena {
    fn get(&self, id: ElementId) -> Option<&Element> {
        let i = id.index as usize;
        if self.generations.get(i)? != &id.generation {
            return None;
        }
        self.slots.get(i)?.as_ref()
    }
    fn get_mut(&mut self, id: ElementId) -> Option<&mut Element> {
        let i = id.index as usize;
        if self.generations.get(i)? != &id.generation {
            return None;
        }
        self.slots.get_mut(i)?.as_mut()
    }
    fn alloc_slot(&mut self) -> (u32, u32) {
        match self.free.pop() {
            Some(i) => (i, self.generations[i as usize]),
            None => {
                let i = self.slots.len() as u32;
                self.slots.push(None);
                self.generations.push(0);
                (i, 0)
            }
        }
    }
    fn free_slot(&mut self, slot: u32) {
        self.slots[slot as usize] = None;
        self.generations[slot as usize] = self.generations[slot as usize].wrapping_add(1);
        self.free.push(slot);
    }
    fn alive(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }
}

#[derive(Clone)]
struct MeasureEntry {
    constraints: Constraints,
    style: LayoutStyle,
    container: Option<ContainerStyle>,

    grid: Option<GridStyle>,
    placements: Vec<GridPlacement>,

    dock: Option<DockStyle>,

    wrap: Option<WrapStyle>,
    children_len: usize,
    size: Size,
}

struct DispatchCells {
    stopped: Cell<bool>,
    handled: Cell<bool>,
    capture_req: Cell<Option<ElementId>>,
    release_req: Cell<bool>,
    focus_req: Cell<bool>,
}

impl DispatchCells {
    fn new() -> Self {
        Self {
            stopped: Cell::new(false),
            handled: Cell::new(false),
            capture_req: Cell::new(None),
            release_req: Cell::new(false),
            focus_req: Cell::new(false),
        }
    }
}

struct Inner {
    arena: RefCell<Arena>,
    queue: RefCell<Vec<ElementId>>,
    root: Cell<Option<ElementId>>,
    stats: Cell<Stats>,
    measures: RefCell<HashMap<ElementId, MeasureEntry>>,
    interact: Cell<InteractState>,
    environment: RefCell<Environment>,

    env_listener: RefCell<Option<sequoia_reactive::Listener>>,

    redraw: RefCell<Option<Box<dyn Fn()>>>,
    surfaces: RefCell<Vec<SurfaceDraw>>,
    overlays: RefCell<OverlayStore>,

    pending_drag: RefCell<Option<PendingDrag>>,

    active_drag: RefCell<Option<ActiveDrag>>,

    next_drag_id: Cell<u64>,

    viewport: Cell<Rect>,

    window_size: Signal<Size>,

    scale: Cell<f32>,

    safe_insets: Cell<Edges>,

    errors: RefCell<Vec<RuntimeError>>,

    errors_seq: Signal<u64>,

    stylus_pressure: Cell<f32>,

    stylus_tool: Cell<PenTool>,

    touches: RefCell<HashMap<u64, Point>>,

    shapes: sequoia_draw::ShapeCache,

    window: Cell<WindowId>,

    wrap_cache: RefCell<HashMap<(u32, u32, u8), bool>>,

    live_anims: RefCell<HashMap<(ElementId, u64), AnimState>>,

    states: RefCell<HashMap<ElementId, Box<dyn Any>>>,

    layout_epoch: Cell<u64>,

    scoped_cancels: RefCell<HashMap<ElementId, Vec<Box<dyn FnOnce()>>>>,
}

impl Inner {
    fn schedule(&self, id: ElementId) {
        self.mark(id, Dirty::LAYOUT | Dirty::CHILDREN);
    }

    fn mark(&self, id: ElementId, flags: Dirty) {
        if flags.is_empty() {
            return;
        }
        let newly = {
            let mut arena = self.arena.borrow_mut();
            match arena.get_mut(id) {
                Some(e) => {
                    let was = e.dirty;
                    e.dirty |= flags;
                    was.is_empty()
                }
                None => return,
            }
        };
        if newly {
            self.queue.borrow_mut().push(id);
        }

        if let Some(h) = self.redraw.borrow().as_ref() {
            h();
        }
    }
}

#[derive(Clone)]
pub struct Runtime {
    inner: Rc<Inner>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(Inner {
                arena: RefCell::default(),
                queue: RefCell::default(),
                root: Cell::new(None),
                stats: Cell::new(Stats::default()),
                measures: RefCell::new(HashMap::new()),
                interact: Cell::new(InteractState::default()),
                environment: RefCell::new(Environment::new()),
                env_listener: RefCell::new(None),
                shapes: sequoia_draw::ShapeCache::default(),
                redraw: RefCell::new(None),
                surfaces: RefCell::new(Vec::new()),
                overlays: RefCell::default(),
                pending_drag: RefCell::new(None),
                active_drag: RefCell::new(None),
                next_drag_id: Cell::new(0),
                viewport: Cell::new(Rect::ZERO),
                window_size: Signal::new(Size::ZERO),
                scale: Cell::new(1.0),
                safe_insets: Cell::new(Edges::ZERO),
                errors: RefCell::new(Vec::new()),
                errors_seq: Signal::new(0),
                stylus_pressure: Cell::new(1.0),
                stylus_tool: Cell::new(PenTool::Mouse),
                touches: RefCell::new(HashMap::new()),
                window: Cell::new(WindowId::PRIMARY),
                wrap_cache: RefCell::new(HashMap::new()),
                live_anims: RefCell::new(HashMap::new()),
                states: RefCell::new(HashMap::new()),
                layout_epoch: Cell::new(0),
                scoped_cancels: RefCell::new(HashMap::new()),
            }),
        }
    }

    pub fn window_id(&self) -> WindowId {
        self.inner.window.get()
    }

    pub fn set_window_id(&self, id: WindowId) {
        self.inner.window.set(id);
    }

    pub fn window_size(&self) -> Signal<Size> {
        self.inner.window_size.clone()
    }

    pub fn window_size_value(&self) -> Size {
        self.inner.window_size.get_untracked()
    }

    pub fn window_rect(&self) -> Rect {
        self.inner.viewport.get()
    }

    pub fn viewport_rect(&self) -> Rect {
        self.inner.viewport.get()
    }

    pub fn scale(&self) -> f32 {
        self.inner.scale.get()
    }

    pub fn set_scale(&self, scale: f32) {
        let s = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        self.inner.scale.set(s);
    }

    pub fn safe_insets(&self) -> Edges {
        self.inner.safe_insets.get()
    }

    pub fn set_safe_insets(&self, insets: Edges) {
        self.inner.safe_insets.set(insets);
    }

    pub fn set_environment(&self, env: Environment) {
        *self.inner.environment.borrow_mut() = env.clone();
        let weak = Rc::downgrade(&self.inner);
        *self.inner.env_listener.borrow_mut() = Some(env.on_change(move || {
            if let Some(inner) = weak.upgrade()
                && let Some(h) = inner.redraw.borrow().as_ref()
            {
                h();
            }
        }));
    }

    pub fn set_redraw_hook(&self, hook: impl Fn() + 'static) {
        *self.inner.redraw.borrow_mut() = Some(Box::new(hook));
    }

    fn request_redraw(&self) {
        if let Some(h) = self.inner.redraw.borrow().as_ref() {
            h();
        }
    }

    pub fn request_frame(&self) {
        self.request_redraw();
    }

    pub fn live_f32(
        &self,
        el: ElementId,
        prop: Prop,
        target: f32,
        tween: Option<&AnimSpec>,
        spring: Option<&SpringSpec>,
    ) -> (f32, bool) {
        if tween.is_none() && spring.is_none() {
            return (target, false);
        }
        let now = std::time::Instant::now();
        let key = (el, prop.0);
        let mut map = self.inner.live_anims.borrow_mut();
        let entry = map.entry(key).or_insert_with(|| {
            AnimState::F32(crate::live::F32State {
                current: target,
                from: target,
                start: now,
                last_target: target,
                tween: tween.copied(),
                spring: spring.copied(),
                velocity: 0.0,
                last_tick: now,
                initialized: false,
            })
        });

        if !matches!(entry, AnimState::F32(_)) {
            *entry = AnimState::F32(crate::live::F32State {
                current: target,
                from: target,
                start: now,
                last_target: target,
                tween: tween.copied(),
                spring: spring.copied(),
                velocity: 0.0,
                last_tick: now,
                initialized: false,
            });
        }
        let AnimState::F32(st) = entry else {
            return (target, false);
        };
        let (v, running) = AnimState::sample_f32(st, target, tween, spring, now);
        drop(map);
        if running {
            self.request_frame();
        }
        (v, running)
    }

    pub fn live_color(
        &self,
        el: ElementId,
        prop: Prop,
        target: sequoia_core::Color,
        tween: Option<&AnimSpec>,
    ) -> (sequoia_core::Color, bool) {
        if tween.is_none() {
            return (target, false);
        }
        let now = std::time::Instant::now();
        let key = (el, prop.0);
        let mut map = self.inner.live_anims.borrow_mut();
        let entry = map.entry(key).or_insert_with(|| {
            AnimState::Color(crate::live::ColorState {
                current: target,
                from: target,
                start: now,
                last_target: target,
                tween: tween.copied(),
                smooth_rate: None,
                last_tick: now,
                initialized: false,
            })
        });
        if !matches!(entry, AnimState::Color(_)) {
            *entry = AnimState::Color(crate::live::ColorState {
                current: target,
                from: target,
                start: now,
                last_target: target,
                tween: tween.copied(),
                smooth_rate: None,
                last_tick: now,
                initialized: false,
            });
        }
        let AnimState::Color(st) = entry else {
            return (target, false);
        };
        let (v, running) = AnimState::sample_color(st, target, tween, None, now);
        drop(map);
        if running {
            self.request_frame();
        }
        (v, running)
    }

    pub fn live_color_exp(
        &self,
        el: ElementId,
        prop: Prop,
        target: sequoia_core::Color,
        rate: f32,
    ) -> (sequoia_core::Color, bool) {
        if !(rate > 0.0) {
            return (target, false);
        }
        let now = std::time::Instant::now();
        let key = (el, prop.0);
        let mut map = self.inner.live_anims.borrow_mut();
        let entry = map.entry(key).or_insert_with(|| {
            AnimState::Color(crate::live::ColorState {
                current: target,
                from: target,
                start: now,
                last_target: target,
                tween: None,
                smooth_rate: Some(rate),
                last_tick: now,
                initialized: false,
            })
        });
        if !matches!(entry, AnimState::Color(_)) {
            *entry = AnimState::Color(crate::live::ColorState {
                current: target,
                from: target,
                start: now,
                last_target: target,
                tween: None,
                smooth_rate: Some(rate),
                last_tick: now,
                initialized: false,
            });
        }
        let AnimState::Color(st) = entry else {
            return (target, false);
        };
        let (v, running) = AnimState::sample_color(st, target, None, Some(rate), now);
        drop(map);
        if running {
            self.request_frame();
        }
        (v, running)
    }

    pub fn live_vec2(
        &self,
        el: ElementId,
        prop: Prop,
        target: sequoia_core::Vec2,
        tween: Option<&AnimSpec>,
        spring: Option<&SpringSpec>,
    ) -> (sequoia_core::Vec2, bool) {
        if tween.is_none() && spring.is_none() {
            return (target, false);
        }
        let now = std::time::Instant::now();
        let key = (el, prop.0);
        let mut map = self.inner.live_anims.borrow_mut();
        let entry = map.entry(key).or_insert_with(|| {
            AnimState::Vec2(crate::live::Vec2State {
                current: target,
                from: target,
                start: now,
                last_target: target,
                tween: tween.copied(),
                spring: spring.copied(),
                velocity: sequoia_core::Vec2::ZERO,
                last_tick: now,
                initialized: false,
            })
        });
        if !matches!(entry, AnimState::Vec2(_)) {
            *entry = AnimState::Vec2(crate::live::Vec2State {
                current: target,
                from: target,
                start: now,
                last_target: target,
                tween: tween.copied(),
                spring: spring.copied(),
                velocity: sequoia_core::Vec2::ZERO,
                last_tick: now,
                initialized: false,
            });
        }
        let AnimState::Vec2(st) = entry else {
            return (target, false);
        };
        let (v, running) = AnimState::sample_vec2(st, target, tween, spring, now);
        drop(map);
        if running {
            self.request_frame();
        }
        (v, running)
    }

    pub fn resolve_live_f32(&self, el: ElementId, prop: Prop, live: &Live<f32>) -> f32 {
        let target = live.target_untracked();
        let (v, _) = self.live_f32(
            el,
            prop,
            target,
            live.tween_spec().as_ref(),
            live.spring_spec().as_ref(),
        );
        v
    }

    pub fn resolve_live_vec2(
        &self,
        el: ElementId,
        prop: Prop,
        live: &Live<sequoia_core::Vec2>,
    ) -> sequoia_core::Vec2 {
        let target = live.target_untracked();
        let (v, _) = self.live_vec2(
            el,
            prop,
            target,
            live.tween_spec().as_ref(),
            live.spring_spec().as_ref(),
        );
        v
    }

    pub fn live_anim_count(&self) -> usize {
        self.inner.live_anims.borrow().len()
    }

    pub fn shapes(&self) -> &sequoia_draw::ShapeCache {
        &self.inner.shapes
    }

    pub fn mount(&self, root: impl View + 'static) {
        self.mount_any(AnyView::new(root));
    }

    pub fn mount_any(&self, root: AnyView) {
        if let Some(old) = self.inner.root.get() {
            self.unmount_subtree(old);
        }
        let value = unwrap_any(root.into_inner());
        let id = self.alloc(value, None, 0, 0);
        if let Some(v) = self.view_rc(id) {
            self.build_with(id, v);
        }
        if let Some(v) = self.view_rc(id) {
            let type_name = v.type_name().to_string();
            if std::panic::catch_unwind(AssertUnwindSafe(|| {
                v.on_mount();
            }))
            .is_err()
            {
                self.record_error(id, type_name, "on_mount panicked".to_string());
            }
        }
        self.inner.root.set(Some(id));
    }

    pub fn with_state<S: Default + 'static, R>(
        &self,
        el: ElementId,
        f: impl FnOnce(&mut S) -> R,
    ) -> R {
        struct Reinsert<'a> {
            map: &'a RefCell<HashMap<ElementId, Box<dyn Any>>>,
            el: ElementId,
            slot: Option<Box<dyn Any>>,
        }
        impl Drop for Reinsert<'_> {
            fn drop(&mut self) {
                if let Some(slot) = self.slot.take() {
                    self.map.borrow_mut().insert(self.el, slot);
                }
            }
        }

        let slot = match self.inner.states.borrow_mut().remove(&el) {
            Some(b) if b.is::<S>() => b,

            Some(_) | None => Box::new(S::default()),
        };
        let mut guard = Reinsert {
            map: &self.inner.states,
            el,
            slot: Some(slot),
        };
        let s = guard
            .slot
            .as_mut()
            .and_then(|b| b.downcast_mut::<S>())
            .expect("slot ma typ S po sprawdzeniu is::<S>()");
        f(s)
    }

    pub fn read_state<S: Default + 'static, R>(&self, el: ElementId, f: impl FnOnce(&S) -> R) -> R {
        self.with_state::<S, R>(el, |s| f(s))
    }

    pub fn state_count(&self) -> usize {
        self.inner.states.borrow().len()
    }

    pub fn has_state(&self, el: ElementId) -> bool {
        self.inner.states.borrow().contains_key(&el)
    }

    pub fn track_scoped(&self, el: ElementId, cancel: impl FnOnce() + 'static) {
        if !self.is_alive(el) {
            cancel();
            return;
        }
        self.inner
            .scoped_cancels
            .borrow_mut()
            .entry(el)
            .or_default()
            .push(Box::new(cancel));
    }

    pub fn scoped_count(&self) -> usize {
        self.inner
            .scoped_cancels
            .borrow()
            .values()
            .map(|v| v.len())
            .sum()
    }

    pub fn request_rebuild(&self, el: ElementId) {
        self.inner.schedule(el);
    }

    pub fn tick(&self) {
        flush_effects();
        loop {
            let next = self.inner.queue.borrow_mut().pop();
            let Some(id) = next else { break };
            let flags = {
                self.inner
                    .arena
                    .borrow()
                    .get(id)
                    .map(|e| e.dirty)
                    .unwrap_or_else(Dirty::empty)
            };
            if flags.is_empty() {
                continue;
            }
            if let Some(e) = self.inner.arena.borrow_mut().get_mut(id) {
                e.dirty = Dirty::empty();
            }
            if !flags.needs_layout() {
                continue;
            }
            let Some(v) = self.view_rc(id) else { continue };
            self.build_with(id, v);
            let mut s = self.inner.stats.get();
            s.rebuilds += 1;
            self.inner.stats.set(s);
        }
    }

    #[must_use]
    pub fn root(&self) -> Option<ElementId> {
        self.inner.root.get()
    }

    pub fn is_alive(&self, id: ElementId) -> bool {
        self.inner.arena.borrow().get(id).is_some()
    }

    pub fn children(&self, id: ElementId) -> Vec<ElementId> {
        self.inner
            .arena
            .borrow()
            .get(id)
            .map(|e| e.children.clone())
            .unwrap_or_default()
    }

    pub fn children_len(&self, id: ElementId) -> usize {
        self.inner
            .arena
            .borrow()
            .get(id)
            .map(|e| e.children.len())
            .unwrap_or(0)
    }

    pub fn key_of(&self, id: ElementId) -> Option<u64> {
        self.inner.arena.borrow().get(id).map(|e| e.key)
    }

    pub fn rect_of(&self, id: ElementId) -> Option<Rect> {
        self.inner.arena.borrow().get(id).map(|e| e.rect.get())
    }
    pub fn find_by_test_id(&self, test_id: &str) -> Option<ElementId> {
        let arena = self.inner.arena.borrow();
        for (i, slot) in arena.slots.iter().enumerate() {
            let Some(el) = slot else { continue };
            let id = ElementId {
                index: i as u32,
                generation: arena.generations[i],
            };
            if arena.get(id).is_none() {
                continue;
            }
            let Some(view) = el.view.clone() else {
                continue;
            };
            if view.test_id_erased().as_deref() == Some(test_id) {
                return Some(id);
            }
        }
        None
    }

    pub fn element_count(&self) -> usize {
        self.inner.arena.borrow().alive()
    }

    pub fn stats(&self) -> Stats {
        self.inner.stats.get()
    }

    pub fn errors(&self) -> Vec<RuntimeError> {
        self.inner.errors.borrow().clone()
    }

    pub fn errors_seq(&self) -> Signal<u64> {
        self.inner.errors_seq.clone()
    }

    pub fn clear_errors(&self) {
        self.inner.errors.borrow_mut().clear();
    }

    pub fn is_descendant_of(&self, el: ElementId, ancestor: ElementId) -> bool {
        let mut cur = Some(el);
        while let Some(e) = cur {
            if e == ancestor {
                return true;
            }
            cur = self.parent_of(e);
        }
        false
    }

    fn record_error(&self, element: ElementId, type_name: String, message: String) {
        let err = RuntimeError {
            element,
            type_name,
            message,
        };
        {
            let mut errors = self.inner.errors.borrow_mut();
            if errors.len() >= MAX_ERRORS {
                errors.remove(0);
            }
            errors.push(err.clone());
        }
        let mut s = self.inner.stats.get();
        s.errors += 1;
        self.inner.stats.set(s);
        self.inner.errors_seq.update(|n| *n += 1);
        self.notify_boundaries(element, &err);
    }

    fn notify_boundaries(&self, element: ElementId, err: &RuntimeError) {
        let chain: Vec<ElementId> = {
            let Ok(arena) = self.inner.arena.try_borrow() else {
                return;
            };
            let mut out = Vec::new();
            let mut cur = arena.get(element).and_then(|_| self.parent_of(element));
            while let Some(id) = cur {
                out.push(id);
                cur = arena.get(id).and_then(|e| e.parent);
            }
            out
        };
        for id in chain {
            if let Some(boundary) = self.view::<crate::error_boundary::ErrorBoundary>(id) {
                boundary.notify(self, id, err);
                return;
            }
        }
    }

    pub fn interact(&self) -> InteractState {
        self.inner.interact.get()
    }

    pub fn pointer_pos(&self) -> Option<Point> {
        let st = self.inner.interact.get();
        st.pointer_seen.then_some(st.cursor)
    }

    fn subtree_wants_hover(&self, mut el: Option<ElementId>) -> bool {
        while let Some(e) = el {
            if self
                .view_rc(e)
                .map(|v| v.wants_hover_updates_erased())
                .unwrap_or(false)
            {
                return true;
            }
            el = self.parent_of(e);
        }
        false
    }

    pub fn cursor_icon(&self) -> CursorIcon {
        if self.inner.active_drag.borrow().is_some() {
            return CursorIcon::Grabbing;
        }
        let st = self.inner.interact.get();
        let mut cur = st.captured.or(st.hovered);
        while let Some(id) = cur {
            let Some(v) = self.view_rc(id) else {
                break;
            };
            let (state, _) = self.view_state(id);
            let icon = v.cursor_icon_erased(state);
            if icon != CursorIcon::Default {
                return icon;
            }
            cur = self.parent_of(id);
        }
        CursorIcon::Default
    }

    pub fn type_name_of(&self, id: ElementId) -> Option<String> {
        self.inner
            .arena
            .borrow()
            .get(id)?
            .view
            .as_ref()
            .map(|v| v.type_name().to_string())
    }

    pub fn view<T: View + 'static>(&self, id: ElementId) -> Option<Rc<T>> {
        let rc = self.view_rc(id)?;
        let any: Rc<dyn Any> = rc.as_any_rc();
        any.downcast::<T>().ok()
    }

    pub fn layout(&self, bounds: Rect, text: &mut TextSystem) {
        let Some(root) = self.inner.root.get() else {
            return;
        };
        self.inner.wrap_cache.borrow_mut().clear();
        self.inner.viewport.set(bounds);

        self.inner.window_size.set(bounds.size);
        let mut ctx = LayoutCtx::new(text);
        let c = Constraints::loose(Size::new(bounds.size.width, bounds.size.height));
        let measured = self.measure(root, c, &mut ctx);
        self.warn_if_root_not_fill(root, bounds, measured);
        let root_rect = Rect::new(bounds.origin, measured);
        self.arrange(root, root_rect, &mut ctx);

        self.layout_overlays(bounds, &mut ctx);
    }

    pub fn paint(&self, list: &mut RenderList, text: &mut TextSystem) {
        self.inner.surfaces.borrow_mut().clear();

        let doomed = self.inner.overlays.borrow().transient_ids();
        for id in doomed {
            self.overlay_hide(id);
        }
        let Some(root) = self.inner.root.get() else {
            return;
        };
        self.paint_rec(root, Vec2::ZERO, 1.0, Transform::IDENTITY, list, text);

        {
            let viewport = self.inner.viewport.get();
            let mut ctx = LayoutCtx::new(&mut *text);
            self.layout_overlays(viewport, &mut ctx);
        }

        let ids = self.inner.overlays.borrow().ordered_ids();
        for id in ids {
            let el = self
                .inner
                .overlays
                .borrow()
                .get(id)
                .and_then(|e| e.element.get());
            if let Some(el) = el {
                self.paint_rec(el, Vec2::ZERO, 1.0, Transform::IDENTITY, list, text);
            }
        }
    }

    pub fn take_surface_draws(&self) -> Vec<SurfaceDraw> {
        std::mem::take(&mut *self.inner.surfaces.borrow_mut())
    }

    pub fn hit_test(&self, pos: Point) -> Option<ElementId> {
        if let Some(hit) = self.hit_overlays(pos) {
            return Some(hit);
        }

        let root = self.inner.root.get()?;
        self.hit_rec(root, pos)
    }

    fn hit_rec(&self, el: ElementId, pos: Point) -> Option<ElementId> {
        let rect = {
            let arena = self.inner.arena.borrow();
            arena.get(el)?.rect.get()
        };
        if !rect.contains(pos) {
            return None;
        }
        for c in self.children(el).iter().rev() {
            if let Some(h) = self.hit_rec(*c, pos) {
                return Some(h);
            }
        }
        let hitable = self
            .view_rc(el)
            .map(|v| v.hit_testable_erased())
            .unwrap_or(true);
        if hitable { Some(el) } else { None }
    }

    pub fn pointer_move(&self, pos: Point, text: &mut TextSystem) {
        let mut st = self.inner.interact.get();
        st.cursor = pos;
        st.pointer_seen = true;
        let old_hover = st.hovered;
        if st.captured.is_none() {
            st.hovered = self.hit_test(pos);
            self.inner.interact.set(st);
        }
        if old_hover != st.hovered {
            let needs = [old_hover, st.hovered].iter().flatten().any(|&el| {
                let mut cur = Some(el);
                while let Some(e) = cur {
                    if self
                        .view_rc(e)
                        .map(|v| v.has_visual_state_erased())
                        .unwrap_or(false)
                    {
                        return true;
                    }
                    cur = self.parent_of(e);
                }
                false
            });
            if needs {
                for el in [old_hover, st.hovered].iter().flatten() {
                    self.inner.mark(*el, Dirty::PAINT);
                }
                self.request_redraw();
            }
        }

        if self.subtree_wants_hover(st.hovered) {
            self.request_redraw();
        }

        if self.inner.pending_drag.borrow().is_some() || self.inner.active_drag.borrow().is_some() {
            self.poll_pending(pos, Modifiers::NONE, text);
        }
        let ev = Event::Pointer(PointerEvent::Move {
            pos,
            modifiers: Modifiers::NONE,
        });
        match st.captured {
            Some(c) => {
                let _ = self.dispatch_single(c, &ev, pos, text);
            }
            None => {
                if let Some(t) = st.hovered {
                    let _ = self.dispatch_single(t, &ev, pos, text);
                }
            }
        }
        if let Event::Pointer(pe) = &ev {
            match st.captured {
                Some(c) => self.forward_surface_pointer(c, pe),
                None => {
                    if let Some(t) = st.hovered {
                        self.forward_surface_pointer(t, pe);
                    }
                }
            }
        }
    }

    fn forward_surface_pointer(&self, el: ElementId, event: &PointerEvent) {
        let Some(v) = self.view_rc(el) else { return };
        let Some(mut binding) = v.surface_input_erased() else {
            return;
        };
        let Some(rect) = self.rect_of(el) else { return };
        (binding.on_pointer)(event, rect);
    }

    fn forward_surface_key(&self, el: ElementId, event: &KeyEvent) {
        let Some(v) = self.view_rc(el) else { return };
        let Some(mut binding) = v.surface_input_erased() else {
            return;
        };
        (binding.on_key)(event);
    }

    pub fn pointer_down(
        &self,
        pos: Point,
        button: PointerButton,
        mods: Modifiers,
        text: &mut TextSystem,
    ) {
        if button != PointerButton::Left && self.inner.active_drag.borrow().is_some() {
            self.cancel_drag(text);
            return;
        }

        let doomed = self.inner.overlays.borrow().outside_dismiss_ids(pos);
        for id in doomed {
            self.overlay_hide(id);
        }
        let target = self.hit_test(pos);
        {
            let mut st = self.inner.interact.get();
            st.cursor = pos;
            st.pressed = target;

            st.focus = match target {
                None => None,
                Some(t) => self.nearest_focusable(t).or(st.focus),
            };

            if st.focus.is_some_and(|f| !self.is_alive(f)) {
                st.focus = None;
            }
            self.inner.interact.set(st);
        }
        self.dismiss_focus_lost(target);
        if let Some(t) = target {
            let ev = Event::Pointer(PointerEvent::Down {
                button,
                pos,
                modifiers: mods,
            });
            self.dispatch_chain(t, &ev, pos, text);
            if let Event::Pointer(pe) = &ev {
                self.forward_surface_pointer(t, pe);
            }
            if let Some(cap) = self.auto_capture(t) {
                let mut st = self.inner.interact.get();
                st.captured = Some(cap);
                self.inner.interact.set(st);
            }
        }
        self.request_redraw();
    }

    pub fn pointer_up(
        &self,
        pos: Point,
        button: PointerButton,
        mods: Modifiers,
        text: &mut TextSystem,
    ) {
        if self.inner.active_drag.borrow().is_some() {
            if button == PointerButton::Left {
                self.finish_drop(pos, mods, text);
            } else {
                self.cancel_drag(text);
            }
            return;
        }

        if self.inner.pending_drag.borrow().is_some() {
            self.inner.pending_drag.borrow_mut().take();
            let mut st = self.inner.interact.get();
            st.captured = None;
            self.inner.interact.set(st);
        }
        let mut st = self.inner.interact.get();
        st.cursor = pos;
        st.hovered = self.hit_test(pos);
        self.inner.interact.set(st);
        let ev = Event::Pointer(PointerEvent::Up {
            button,
            pos,
            modifiers: mods,
        });
        match st.captured {
            Some(c) => {
                let handled = self.dispatch_single(c, &ev, pos, text);

                if !handled
                    && let Some(p) = st.pressed
                    && p != c
                    && self.is_alive(p)
                {
                    self.dispatch_chain(p, &ev, pos, text);
                }
            }
            None => {
                if let Some(p) = st.pressed {
                    self.dispatch_chain(p, &ev, pos, text);
                }
            }
        }
        if let Event::Pointer(pe) = &ev {
            match st.captured {
                Some(c) => self.forward_surface_pointer(c, pe),
                None => {
                    if let Some(p) = st.pressed {
                        self.forward_surface_pointer(p, pe);
                    }
                }
            }
        }
        let mut st2 = self.inner.interact.get();
        st2.pressed = None;
        st2.captured = None;
        self.inner.interact.set(st2);
        self.request_redraw();
    }

    pub fn wheel(&self, pos: Point, delta: Vec2, modifiers: Modifiers, text: &mut TextSystem) {
        let Some(target) = self.hit_test(pos) else {
            return;
        };
        let ev = Event::Pointer(PointerEvent::Wheel {
            delta,
            pos,
            modifiers,
        });
        self.dispatch_chain(target, &ev, pos, text);
        if let Event::Pointer(pe) = &ev {
            self.forward_surface_pointer(target, pe);
        }
        self.request_redraw();
    }

    pub fn pointer_pressure(&self) -> f32 {
        self.inner.stylus_pressure.get()
    }

    pub fn pointer_tool(&self) -> PenTool {
        self.inner.stylus_tool.get()
    }

    pub fn active_touches(&self) -> Vec<(u64, Point)> {
        self.inner
            .touches
            .borrow()
            .iter()
            .map(|(k, v)| (*k, *v))
            .collect()
    }

    fn set_stylus(&self, pressure: f32, tool: PenTool) {
        self.inner.stylus_pressure.set(pressure.clamp(0.0, 1.0));
        self.inner.stylus_tool.set(tool);
    }

    pub fn pen_down(
        &self,
        pos: Point,
        button: PointerButton,
        pressure: f32,
        tilt: Vec2,
        tool: PenTool,
        mods: Modifiers,
        text: &mut TextSystem,
    ) {
        let _ = tilt;
        self.set_stylus(pressure, tool);
        self.pointer_down(pos, button, mods, text);
    }

    pub fn pen_move(
        &self,
        pos: Point,
        pressure: f32,
        tilt: Vec2,
        tool: PenTool,
        text: &mut TextSystem,
    ) {
        let _ = tilt;
        self.set_stylus(pressure, tool);
        self.pointer_move(pos, text);
    }

    pub fn pen_up(
        &self,
        pos: Point,
        button: PointerButton,
        pressure: f32,
        mods: Modifiers,
        text: &mut TextSystem,
    ) {
        self.set_stylus(pressure, self.inner.stylus_tool.get());
        self.pointer_up(pos, button, mods, text);
    }

    pub fn touch_down(
        &self,
        id: u64,
        pos: Point,
        force: Option<f32>,
        mods: Modifiers,
        text: &mut TextSystem,
    ) {
        let first = self.inner.touches.borrow().is_empty();
        self.inner.touches.borrow_mut().insert(id, pos);
        self.set_stylus(force.unwrap_or(1.0), PenTool::Touch);
        if first {
            self.pointer_down(pos, PointerButton::Left, mods, text);
        }
        if let Some(t) = self.hit_test(pos) {
            let ev = Event::Pointer(PointerEvent::TouchDown {
                id,
                pos,
                force,
                modifiers: mods,
            });
            self.dispatch_chain(t, &ev, pos, text);
            if let Event::Pointer(pe) = &ev {
                self.forward_surface_pointer(t, pe);
            }
        }
        self.request_redraw();
    }

    pub fn touch_move(&self, id: u64, pos: Point, force: Option<f32>, text: &mut TextSystem) {
        let is_first = self
            .inner
            .touches
            .borrow()
            .keys()
            .next()
            .is_some_and(|first| *first == id);
        self.inner.touches.borrow_mut().insert(id, pos);
        self.set_stylus(force.unwrap_or(1.0), PenTool::Touch);
        if is_first {
            self.pointer_move(pos, text);
        }
        let ev = Event::Pointer(PointerEvent::TouchMove {
            id,
            pos,
            force,
            modifiers: Modifiers::NONE,
        });

        match self.inner.interact.get().captured {
            Some(c) => {
                self.dispatch_single(c, &ev, pos, text);
                if let Event::Pointer(pe) = &ev {
                    self.forward_surface_pointer(c, pe);
                }
            }
            None => {
                if let Some(t) = self.hit_test(pos) {
                    self.dispatch_chain(t, &ev, pos, text);
                    if let Event::Pointer(pe) = &ev {
                        self.forward_surface_pointer(t, pe);
                    }
                }
            }
        }
        self.request_redraw();
    }

    pub fn touch_up(&self, id: u64, pos: Point, mods: Modifiers, text: &mut TextSystem) {
        let was_last = {
            let mut touches = self.inner.touches.borrow_mut();
            touches.remove(&id);
            touches.is_empty()
        };
        let ev = Event::Pointer(PointerEvent::TouchUp {
            id,
            pos,
            modifiers: mods,
        });

        match self.inner.interact.get().captured {
            Some(c) => {
                self.dispatch_single(c, &ev, pos, text);
                if let Event::Pointer(pe) = &ev {
                    self.forward_surface_pointer(c, pe);
                }
            }
            None => {
                if let Some(t) = self.hit_test(pos) {
                    self.dispatch_chain(t, &ev, pos, text);
                    if let Event::Pointer(pe) = &ev {
                        self.forward_surface_pointer(t, pe);
                    }
                }
            }
        }
        if was_last {
            self.pointer_up(pos, PointerButton::Left, mods, text);
        } else {
            self.request_redraw();
        }
    }

    pub fn pinch(&self, pos: Point, delta: f32, mods: Modifiers, text: &mut TextSystem) {
        let Some(target) = self.hit_test(pos) else {
            return;
        };
        let ev = Event::Pointer(PointerEvent::Pinch {
            delta,
            pos,
            modifiers: mods,
        });
        self.dispatch_chain(target, &ev, pos, text);
        if let Event::Pointer(pe) = &ev {
            self.forward_surface_pointer(target, pe);
        }
        self.request_redraw();
    }

    pub fn key(&self, key: Key, mods: Modifiers, text: &mut TextSystem) {
        if key == Key::ESCAPE && self.inner.active_drag.borrow().is_some() {
            self.cancel_drag(text);
            return;
        }
        let esc = self.inner.overlays.borrow().escape_dismiss_id();
        if key == Key::ESCAPE
            && let Some(id) = esc
        {
            self.overlay_hide(id);
            return;
        }
        if key == Key::TAB && !self.focused_accepts_tab() {
            self.focus_next(mods.contains(Modifiers::SHIFT));
            self.request_redraw();
            return;
        }
        let st = self.inner.interact.get();
        let Some(f) = st.focus else { return };
        if !self.is_alive(f) {
            return;
        }
        let ev = Event::Key(KeyEvent {
            key,
            modifiers: mods,
        });
        self.dispatch_chain(f, &ev, Point::ZERO, text);
        if let Event::Key(ke) = &ev {
            self.forward_surface_key(f, ke);
        }
        self.request_redraw();
    }

    pub fn ime(&self, event: ImeEvent, text: &mut TextSystem) {
        let st = self.inner.interact.get();
        let Some(f) = st.focus else { return };
        if !self.is_alive(f) {
            return;
        }
        let ev = Event::Ime(event);

        let cursor = st.cursor;
        self.dispatch_chain(f, &ev, cursor, text);
        self.request_redraw();
    }

    pub fn files_hovered(&self, paths: Vec<std::path::PathBuf>, pos: Point, text: &mut TextSystem) {
        let Some(target) = self.hit_test(pos) else {
            return;
        };
        let ev = Event::FilesHovered {
            paths,
            position: pos,
        };
        self.dispatch_chain(target, &ev, pos, text);
        self.request_redraw();
    }

    pub fn files_dropped(&self, paths: Vec<std::path::PathBuf>, pos: Point, text: &mut TextSystem) {
        let Some(target) = self.hit_test(pos) else {
            return;
        };
        let ev = Event::FilesDropped {
            paths,
            position: pos,
        };
        self.dispatch_chain(target, &ev, pos, text);
        self.request_redraw();
    }

    pub fn files_cancelled(&self, text: &mut TextSystem) {
        let pos = self.inner.interact.get().cursor;
        let ev = Event::FilesCancelled;
        if let Some(target) = self.hit_test(pos) {
            self.dispatch_chain(target, &ev, pos, text);
        }
        self.request_redraw();
    }

    pub fn ime_cursor_rect(&self) -> Option<Rect> {
        let st = self.inner.interact.get();
        let f = st.focus?;
        self.rect_of(f)
    }

    pub fn overlay_show(&self, entry: OverlayEntry) -> OverlayId {
        let transient = entry.transient;
        let view = entry.view.clone_inner();
        let value = unwrap_any(view);
        let el = self.alloc(value, None, 0, 0);
        if let Some(v) = self.view_rc(el) {
            self.build_with(el, v);
        }
        if let Some(v) = self.view_rc(el) {
            let type_name = v.type_name().to_string();
            if std::panic::catch_unwind(AssertUnwindSafe(|| {
                v.on_mount();
            }))
            .is_err()
            {
                self.record_error(el, type_name, "on_mount panicked".to_string());
            }
        }
        entry.element.set(Some(el));
        let id = self.inner.overlays.borrow_mut().insert(entry);
        if !transient {
            self.request_redraw();
        }
        id
    }

    pub fn overlay_hide(&self, id: OverlayId) -> bool {
        let Some(entry) = self.inner.overlays.borrow_mut().remove(id) else {
            return false;
        };
        if let Some(el) = entry.element.get() {
            self.unmount_subtree(el);
        }
        self.request_redraw();
        true
    }

    pub fn overlay_rect(&self, id: OverlayId) -> Option<Rect> {
        self.inner.overlays.borrow().get(id).map(|e| e.rect.get())
    }

    pub fn overlay_exists(&self, id: OverlayId) -> bool {
        self.inner.overlays.borrow().get(id).is_some()
    }

    pub fn overlay_element(&self, id: OverlayId) -> Option<ElementId> {
        self.inner
            .overlays
            .borrow()
            .get(id)
            .and_then(|e| e.element.get())
    }

    pub fn accessibility_tree(&self) -> Vec<crate::a11y::AccessibleNode> {
        let Some(root) = self.inner.root.get() else {
            return Vec::new();
        };
        self.accessibility_children(root)
    }

    fn accessibility_children(&self, el: ElementId) -> Vec<crate::a11y::AccessibleNode> {
        let mut out = Vec::new();
        for child in self.children(el) {
            if !self.is_alive(child) {
                continue;
            }
            let kids = self.accessibility_children(child);
            match self.view_rc(child).and_then(|v| v.accessibility_erased()) {
                Some(a) => out.push(crate::a11y::AccessibleNode {
                    element: child,
                    role: a.role,
                    label: a.label,
                    children: kids,
                }),
                None => out.extend(kids),
            }
        }
        out
    }

    pub fn overlay_ids(&self) -> Vec<OverlayId> {
        self.inner.overlays.borrow().ordered_ids()
    }

    pub fn overlay_count(&self) -> usize {
        self.inner.overlays.borrow().len()
    }

    pub fn overlay_set_position(&self, id: OverlayId, x: f32, y: f32) -> bool {
        let ok = self.inner.overlays.borrow_mut().set_offset(id, (x, y));
        if ok {
            self.request_redraw();
        }
        ok
    }

    pub fn begin_pending(
        &self,
        source: ElementId,
        payload: DragPayload,
        origin: Point,
        source_rect: Rect,
        options: DragOptions,
    ) {
        if self.inner.active_drag.borrow().is_some() || !self.is_alive(source) {
            return;
        }
        *self.inner.pending_drag.borrow_mut() = Some(PendingDrag {
            source,
            origin,
            source_rect,
            payload,
            preview: options.preview,
            threshold: options.threshold,
        });
    }

    pub fn current_drag(&self) -> Option<DragSession> {
        self.inner
            .active_drag
            .borrow()
            .as_ref()
            .map(ActiveDrag::snapshot)
    }

    pub fn has_pending_drag(&self) -> bool {
        self.inner.pending_drag.borrow().is_some()
    }

    pub fn drag_target_contains(&self, el: ElementId) -> bool {
        let Some(target) = self
            .inner
            .active_drag
            .borrow()
            .as_ref()
            .and_then(|a| a.target)
        else {
            return false;
        };
        let mut cur = Some(el);
        while let Some(e) = cur {
            if e == target {
                return true;
            }
            cur = self.parent_of(e);
        }
        false
    }

    pub fn drop_target_at(&self, pos: Point, payload: &DragPayload) -> Option<ElementId> {
        let mut cur = self.hit_test(pos);
        while let Some(el) = cur {
            if self.is_alive(el)
                && self
                    .view_rc(el)
                    .is_some_and(|v| v.accepts_drop_erased(payload))
            {
                return Some(el);
            }
            cur = self.parent_of(el);
        }
        None
    }

    pub fn cancel_drag(&self, text: &mut TextSystem) {
        let (source, current) = {
            let guard = self.inner.active_drag.borrow();
            let Some(act) = guard.as_ref() else {
                drop(guard);
                self.inner.pending_drag.borrow_mut().take();
                return;
            };
            (act.source, act.current)
        };
        if let Some(src) = source.filter(|s| self.is_alive(*s)) {
            self.call_drag_end(
                src,
                DragEnd {
                    dropped: false,
                    cancelled: true,
                },
                current,
                text,
            );
        }
        let active = self.inner.active_drag.borrow_mut().take();
        self.inner.pending_drag.borrow_mut().take();
        if let Some(act) = active
            && let Some(pv) = act.preview
        {
            self.overlay_hide(pv);
        }
        let mut st = self.inner.interact.get();
        st.pressed = None;
        st.captured = None;
        self.inner.interact.set(st);
        self.request_redraw();
    }

    fn poll_pending(&self, current: Point, modifiers: Modifiers, text: &mut TextSystem) {
        if self.inner.active_drag.borrow().is_some() {
            self.update_active(current, modifiers, text);
            return;
        }
        let pending = self.inner.pending_drag.borrow_mut().take();
        let Some(p) = pending else { return };
        if !self.is_alive(p.source) {
            return;
        }
        if !exceeded_threshold(p.origin, current, p.threshold) {
            *self.inner.pending_drag.borrow_mut() = Some(p);
            return;
        }
        let grab = grab_offset(p.origin, p.source_rect);
        let id = DragId(self.inner.next_drag_id.get());
        self.inner.next_drag_id.set(id.0 + 1);
        let preview = p.preview.map(|view| {
            let at = preview_origin(current, grab);
            self.overlay_show(
                OverlayEntry::new(view)
                    .at(at.x, at.y)
                    .layer(OverlayLayer::DragPreview)
                    .input_mode(InputMode::Passthrough),
            )
        });
        let target = self.drop_target_at(current, &p.payload);
        *self.inner.active_drag.borrow_mut() = Some(ActiveDrag {
            id,
            payload: p.payload,
            source: Some(p.source),
            origin: p.origin,
            grab_offset: grab,
            current,
            modifiers,
            target,
            preview,
        });

        let mut st = self.inner.interact.get();
        st.captured = Some(p.source);
        self.inner.interact.set(st);
        self.call_drag_start(p.source, current, text);
        if let Some(t) = target {
            self.call_drag_over(t, current, modifiers, text);
        }
        self.request_redraw();
    }

    fn update_active(&self, current: Point, modifiers: Modifiers, text: &mut TextSystem) {
        let (target, preview_move) = {
            let mut guard = self.inner.active_drag.borrow_mut();
            let Some(act) = guard.as_mut() else {
                return;
            };
            act.current = current;
            act.modifiers = modifiers;
            let t = self.drop_target_at(current, &act.payload);
            act.target = t;
            let mv = act
                .preview
                .map(|id| (id, preview_origin(current, act.grab_offset)));
            (t, mv)
        };
        if let Some((id, at)) = preview_move {
            self.overlay_set_position(id, at.x, at.y);
        }
        if let Some(t) = target {
            self.call_drag_over(t, current, modifiers, text);
        }
        self.request_redraw();
    }

    fn finish_drop(&self, current: Point, modifiers: Modifiers, text: &mut TextSystem) -> bool {
        let payload = {
            let guard = self.inner.active_drag.borrow();
            let Some(act) = guard.as_ref() else {
                return false;
            };
            act.payload.clone()
        };

        let target = self.drop_target_at(current, &payload);
        let mut dropped = false;
        if let Some(t) = target {
            let rect = self.rect_of(t).unwrap_or(Rect::ZERO);
            let pos = DropPosition::in_rect(current, rect, modifiers);
            dropped = self.call_drop(t, pos, text);
        }
        let (source, preview) = {
            let mut guard = self.inner.active_drag.borrow_mut();
            match guard.take() {
                Some(act) => (act.source, act.preview),

                None => return true,
            }
        };
        self.inner.pending_drag.borrow_mut().take();
        if let Some(pv) = preview {
            self.overlay_hide(pv);
        }
        let mut st = self.inner.interact.get();
        st.pressed = None;
        st.captured = None;
        st.hovered = self.hit_test(current);
        self.inner.interact.set(st);
        if let Some(src) = source.filter(|s| self.is_alive(*s)) {
            self.call_drag_end(
                src,
                DragEnd {
                    dropped,
                    cancelled: false,
                },
                current,
                text,
            );
        }
        self.request_redraw();
        true
    }

    fn drag_ctx_parts(&self, el: ElementId) -> (VisualState, bool, Rect, Environment) {
        let (state, hover_self) = self.view_state(el);
        let rect = {
            let arena = self.inner.arena.borrow();
            arena.get(el).map(|e| e.rect.get()).unwrap_or(Rect::ZERO)
        };
        let env = self.inner.environment.borrow().clone();
        (state, hover_self, rect, env)
    }

    fn call_drag_start(&self, el: ElementId, pos: Point, text: &mut TextSystem) {
        let Some(v) = self.view_rc(el) else { return };
        let cells = DispatchCells::new();
        let (state, hover_self, rect, env) = self.drag_ctx_parts(el);
        let mut ctx = EventCtx {
            phase: EventPhase::Target,
            element: el,
            state,
            hover_self,
            pos,
            rect,
            environment: &env,
            window: self.inner.window.get(),
            stopped: &cells.stopped,
            handled: &cells.handled,
            capture_req: &cells.capture_req,
            release_req: &cells.release_req,
            focus_req: &cells.focus_req,
            text,
            runtime: self,
        };
        let type_name = v.type_name().to_string();
        if std::panic::catch_unwind(AssertUnwindSafe(|| {
            v.on_drag_start_erased(&mut ctx);
        }))
        .is_err()
        {
            self.record_error(el, type_name, "on_drag_start panicked".to_string());
        }
    }

    fn call_drag_over(
        &self,
        el: ElementId,
        current: Point,
        modifiers: Modifiers,
        text: &mut TextSystem,
    ) {
        let Some(v) = self.view_rc(el) else { return };
        let cells = DispatchCells::new();
        let (state, hover_self, rect, env) = self.drag_ctx_parts(el);
        let mut ctx = EventCtx {
            phase: EventPhase::Target,
            element: el,
            state,
            hover_self,
            pos: current,
            rect,
            environment: &env,
            window: self.inner.window.get(),
            stopped: &cells.stopped,
            handled: &cells.handled,
            capture_req: &cells.capture_req,
            release_req: &cells.release_req,
            focus_req: &cells.focus_req,
            text,
            runtime: self,
        };
        let pos = DropPosition::in_rect(current, rect, modifiers);
        let type_name = v.type_name().to_string();
        if std::panic::catch_unwind(AssertUnwindSafe(|| {
            v.on_drag_over_erased(pos, &mut ctx);
        }))
        .is_err()
        {
            self.record_error(el, type_name, "on_drag_over panicked".to_string());
        }
    }

    fn call_drop(&self, el: ElementId, pos: DropPosition, text: &mut TextSystem) -> bool {
        let Some(v) = self.view_rc(el) else {
            return false;
        };
        let cells = DispatchCells::new();
        let (state, hover_self, rect, env) = self.drag_ctx_parts(el);
        let mut ctx = EventCtx {
            phase: EventPhase::Target,
            element: el,
            state,
            hover_self,
            pos: pos.global,
            rect,
            environment: &env,
            window: self.inner.window.get(),
            stopped: &cells.stopped,
            handled: &cells.handled,
            capture_req: &cells.capture_req,
            release_req: &cells.release_req,
            focus_req: &cells.focus_req,
            text,
            runtime: self,
        };
        let type_name = v.type_name().to_string();
        if std::panic::catch_unwind(AssertUnwindSafe(|| {
            v.on_drop_erased(pos, &mut ctx);
        }))
        .is_err()
        {
            self.record_error(el, type_name, "on_drop panicked".to_string());
            return false;
        }
        cells.handled.get()
    }

    fn call_drag_end(&self, el: ElementId, end: DragEnd, current: Point, text: &mut TextSystem) {
        let Some(v) = self.view_rc(el) else { return };
        let cells = DispatchCells::new();
        let (state, hover_self, rect, env) = self.drag_ctx_parts(el);
        let mut ctx = EventCtx {
            phase: EventPhase::Target,
            element: el,
            state,
            hover_self,
            pos: current,
            rect,
            environment: &env,
            window: self.inner.window.get(),
            stopped: &cells.stopped,
            handled: &cells.handled,
            capture_req: &cells.capture_req,
            release_req: &cells.release_req,
            focus_req: &cells.focus_req,
            text,
            runtime: self,
        };
        let type_name = v.type_name().to_string();
        if std::panic::catch_unwind(AssertUnwindSafe(|| {
            v.on_drag_end_erased(end, &mut ctx);
        }))
        .is_err()
        {
            self.record_error(el, type_name, "on_drag_end panicked".to_string());
        }
    }

    fn layout_overlays(&self, viewport: Rect, ctx: &mut LayoutCtx) {
        for id in self.inner.overlays.borrow().ordered_ids() {
            self.layout_one_overlay(id, viewport, ctx);
        }
    }

    fn layout_one_overlay(&self, id: OverlayId, viewport: Rect, ctx: &mut LayoutCtx) {
        let (anchor, placement, position, offset, overflow, el) = {
            let store = self.inner.overlays.borrow();
            let Some(e) = store.get(id) else { return };
            let Some(el) = e.element.get() else { return };
            (e.anchor, e.placement, e.position, e.offset, e.overflow, el)
        };

        let anchor_rect = match anchor {
            Anchor::View(aid) => match self.rect_of(aid) {
                Some(r) => r,

                None => {
                    self.record_error(
                        el,
                        "Overlay".to_string(),
                        "dead Anchor::View: overlay falls back to (0,0); recreate entry with live ctx.element".to_string(),
                    );
                    Rect::ZERO
                }
            },
            _ => self.resolve_anchor(anchor, viewport),
        };

        let measured = self.measure(el, Constraints::UNBOUNDED, ctx);

        let pos = compute_position(
            position,
            placement,
            anchor_rect,
            measured,
            viewport,
            offset,
            overflow,
        );
        let rect = Rect::new(pos, measured);

        self.arrange(el, rect, ctx);

        if let Some(e) = self.inner.overlays.borrow().get(id) {
            e.rect.set(rect);
        }
    }

    fn resolve_anchor(&self, anchor: Anchor, viewport: Rect) -> Rect {
        match anchor {
            Anchor::View(id) => self.rect_of(id).unwrap_or(Rect::ZERO),
            Anchor::Rect(r) => r,
            Anchor::Point(p) => Rect::new(p, Size::ZERO),
            Anchor::Cursor => Rect::new(self.inner.interact.get().cursor, Size::ZERO),
            Anchor::Viewport => viewport,
        }
    }

    fn hit_overlays(&self, pos: Point) -> Option<ElementId> {
        for id in self.inner.overlays.borrow().ordered_ids().into_iter().rev() {
            let (mode, el, rect) = {
                let store = self.inner.overlays.borrow();
                let Some(e) = store.get(id) else { continue };
                let Some(el) = e.element.get() else { continue };
                (e.input_mode, el, e.rect.get())
            };
            match mode {
                InputMode::Passthrough => continue,
                InputMode::Modal => {
                    if rect.contains(pos)
                        && let Some(hit) = self.hit_rec(el, pos)
                    {
                        return Some(hit);
                    }
                    return Some(el);
                }
                InputMode::Interactive => {
                    if rect.contains(pos)
                        && let Some(hit) = self.hit_rec(el, pos)
                    {
                        return Some(hit);
                    }
                }
            }
        }
        None
    }

    fn hover_in_subtree(&self, el: ElementId) -> bool {
        let st = self.inner.interact.get();
        let Some(h) = st.hovered else { return false };
        let mut cur = Some(h);
        while let Some(e) = cur {
            if e == el {
                return true;
            }
            cur = self.parent_of(e);
        }
        false
    }

    pub fn focus_in_subtree(&self, el: ElementId) -> bool {
        let st = self.inner.interact.get();
        let Some(f) = st.focus else { return false };
        if f == el {
            return true;
        }
        let mut cur = self.parent_of(f);
        while let Some(e) = cur {
            if e == el {
                return true;
            }
            cur = self.parent_of(e);
        }
        false
    }

    pub fn parent_of(&self, el: ElementId) -> Option<ElementId> {
        self.inner.arena.borrow().get(el)?.parent
    }

    fn focused_accepts_tab(&self) -> bool {
        let st = self.inner.interact.get();
        let Some(f) = st.focus else { return false };
        if !self.is_alive(f) {
            return false;
        }
        self.view_rc(f).is_some_and(|v| v.accepts_tab_erased())
    }

    fn nearest_focusable(&self, mut el: ElementId) -> Option<ElementId> {
        loop {
            if let Some(v) = self.view_rc(el)
                && v.focusable_erased()
            {
                return Some(el);
            }
            {
                let p = self.parent_of(el)?;
                el = p
            }
        }
    }

    fn nearest_trap_scope(&self, mut el: ElementId) -> Option<ElementId> {
        loop {
            if let Some(v) = self.view_rc(el)
                && v.focus_trap_erased()
            {
                return Some(el);
            }
            {
                let p = self.parent_of(el)?;
                el = p
            }
        }
    }

    fn collect_focusables(&self, el: ElementId, out: &mut Vec<ElementId>) {
        if let Some(v) = self.view_rc(el)
            && v.focusable_erased()
        {
            out.push(el);
        }
        for c in self.children(el) {
            self.collect_focusables(c, out);
        }
    }

    pub fn focus_next(&self, backward: bool) {
        let mut st = self.inner.interact.get();

        let mut f = Vec::new();
        let scope = st.focus.and_then(|c| self.nearest_trap_scope(c));
        match scope {
            Some(s) => self.collect_focusables(s, &mut f),
            None => {
                if let Some(root) = self.inner.root.get() {
                    self.collect_focusables(root, &mut f);
                }
            }
        }
        if f.is_empty() {
            return;
        }
        let len = f.len();
        let next = match st.focus.and_then(|c| f.iter().position(|&e| e == c)) {
            Some(i) if backward => (i + len - 1) % len,
            Some(i) => (i + 1) % len,
            None => 0,
        };
        st.focus = Some(f[next]);
        self.inner.interact.set(st);
        self.dismiss_focus_lost(st.focus);
    }

    pub fn set_focus(&self, el: Option<ElementId>) {
        let mut st = self.inner.interact.get();
        st.focus = el;
        self.inner.interact.set(st);
        self.dismiss_focus_lost(el);
    }

    fn target_in_overlay(&self, target: ElementId, overlay_el: Option<ElementId>) -> bool {
        let Some(root) = overlay_el else { return false };
        let mut cur = Some(target);
        while let Some(el) = cur {
            if el == root {
                return true;
            }
            cur = self.parent_of(el);
        }
        false
    }

    fn dismiss_focus_lost(&self, target: Option<ElementId>) {
        let ids = self.inner.overlays.borrow().focus_lost_ids(target, |el| {
            target.is_some_and(|t| self.target_in_overlay(t, el))
        });
        for id in ids {
            self.overlay_hide(id);
        }
    }

    fn auto_capture(&self, target: ElementId) -> Option<ElementId> {
        let mut cur = Some(target);
        while let Some(el) = cur {
            if let Some(v) = self.view_rc(el)
                && v.captures_pointer_ctx_erased(self, el)
            {
                return Some(el);
            }
            cur = self.parent_of(el);
        }
        None
    }

    fn view_state(&self, el: ElementId) -> (VisualState, bool) {
        let st = self.inner.interact.get();
        let mut vs = VisualState::default();
        vs.hovered = st.hovered == Some(el);
        vs.pressed = st.pressed == Some(el);
        vs.focused = st.focus == Some(el);
        (vs, st.hovered == Some(el))
    }

    pub fn visual_state(&self, el: ElementId) -> VisualState {
        self.view_state(el).0
    }

    fn call_on_event(
        &self,
        el: ElementId,
        event: &Event,
        phase: EventPhase,
        pos: Point,
        cells: &DispatchCells,
        text: &mut TextSystem,
    ) {
        let Some(v) = self.view_rc(el) else { return };
        let (state, hover_self) = self.view_state(el);
        let rect = {
            let arena = self.inner.arena.borrow();
            arena.get(el).map(|e| e.rect.get()).unwrap_or(Rect::ZERO)
        };

        let env = self.inner.environment.borrow().clone();
        let mut ctx = EventCtx {
            phase,
            element: el,
            state,
            hover_self,
            pos,
            rect,
            environment: &env,
            window: self.inner.window.get(),
            stopped: &cells.stopped,
            handled: &cells.handled,
            capture_req: &cells.capture_req,
            release_req: &cells.release_req,
            focus_req: &cells.focus_req,
            text,
            runtime: self,
        };

        let type_name = v.type_name().to_string();
        if std::panic::catch_unwind(AssertUnwindSafe(|| {
            v.on_event_erased(event, &mut ctx);
        }))
        .is_err()
        {
            let msg = format!("on_event panicked ({phase:?})");
            self.record_error(el, type_name, msg);
        }
    }

    fn dispatch_chain(&self, target: ElementId, event: &Event, pos: Point, text: &mut TextSystem) {
        let mut path = vec![target];
        while let Some(last) = path.last() {
            let Some(p) = self.parent_of(*last) else {
                break;
            };
            path.push(p);
        }
        path.reverse();
        let n = path.len();
        let cells = DispatchCells::new();

        for &el in &path[..n - 1] {
            if cells.stopped.get() || cells.handled.get() {
                self.apply_dispatch_requests(target, &cells);
                return;
            }
            self.call_on_event(el, event, EventPhase::Capture, pos, &cells, text);
        }
        if !(cells.stopped.get() || cells.handled.get()) {
            self.call_on_event(target, event, EventPhase::Target, pos, &cells, text);
        }
        for &el in path[..n - 1].iter().rev() {
            if cells.stopped.get() || cells.handled.get() {
                break;
            }
            self.call_on_event(el, event, EventPhase::Bubble, pos, &cells, text);
        }
        self.apply_dispatch_requests(target, &cells);
    }

    fn dispatch_single(
        &self,
        el: ElementId,
        event: &Event,
        pos: Point,
        text: &mut TextSystem,
    ) -> bool {
        let cells = DispatchCells::new();
        self.call_on_event(el, event, EventPhase::Target, pos, &cells, text);
        self.apply_dispatch_requests(el, &cells);
        cells.handled.get() || cells.stopped.get()
    }

    fn apply_dispatch_requests(&self, el: ElementId, cells: &DispatchCells) {
        let mut st = self.inner.interact.get();
        if cells.release_req.get() {
            if st.captured == Some(el) {
                st.captured = None;
            }
        } else if let Some(req) = cells.capture_req.get() {
            st.captured = Some(req);
        }
        if cells.focus_req.get() {
            st.focus = Some(el);
        }
        self.inner.interact.set(st);
    }

    pub fn pointer_cancel(&self, text: &mut TextSystem) {
        let captured = self.inner.interact.get().captured;
        if let Some(c) = captured {
            let pos = self.inner.interact.get().cursor;
            let ev = Event::Pointer(PointerEvent::Cancel);
            let _ = self.dispatch_single(c, &ev, pos, text);
        }
        self.inner.pending_drag.borrow_mut().take();
        if self.inner.active_drag.borrow().is_some() {
            self.cancel_drag(text);
            return;
        }
        let mut st = self.inner.interact.get();
        st.pressed = None;
        st.captured = None;
        st.hovered = None;
        self.inner.interact.set(st);
        self.request_redraw();
    }

    pub(crate) fn measure_node(
        &self,
        el: ElementId,
        constraints: Constraints,
        ctx: &mut LayoutCtx,
    ) -> Size {
        self.measure(el, constraints, ctx)
    }

    pub(crate) fn arrange_node(&self, el: ElementId, rect: Rect, ctx: &mut LayoutCtx) {
        self.arrange(el, rect, ctx)
    }

    pub(crate) fn layout_epoch(&self) -> u64 {
        self.inner.layout_epoch.get()
    }

    pub(crate) fn translate_subtree(&self, root: ElementId, delta: Vec2) {
        if delta == Vec2::ZERO {
            return;
        }
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            let children = {
                let mut arena = self.inner.arena.borrow_mut();
                let Some(e) = arena.get_mut(id) else {
                    continue;
                };
                e.rect.set(e.rect.get().translate(delta));
                e.children.clone()
            };
            stack.extend(children);
        }
    }

    pub(crate) fn child_layout_style(&self, el: ElementId) -> LayoutStyle {
        self.view_rc(el)
            .map(|v| v.layout_style_ctx_erased(self, el))
            .unwrap_or_default()
    }

    pub fn check_root_fill(&self, root: ElementId, bounds: Rect, measured: Size) -> bool {
        let style = self.child_layout_style(root);
        let fill_w = style.width.is_fill();
        let fill_h = style.height.is_fill();
        if fill_w && fill_h {
            return true;
        }
        let gap_w = bounds.size.width - measured.width;
        let gap_h = bounds.size.height - measured.height;

        if gap_w <= 1.0 && gap_h <= 1.0 {
            return true;
        }
        let type_name = self.type_name_of(root).unwrap_or_else(|| "?".into());
        log::warn!(
            "sequoia-view: root `{type_name}` has no fill_size (width_fill={fill_w}, height_fill={fill_h}); \
             measured {measured:?} with window {:?} — unpainted background. Add `.fill_size()` to the root.",
            bounds.size
        );
        false
    }

    fn warn_if_root_not_fill(&self, root: ElementId, bounds: Rect, measured: Size) {
        let _ = self.check_root_fill(root, bounds, measured);
    }

    fn measure(&self, el: ElementId, constraints: Constraints, ctx: &mut LayoutCtx) -> Size {
        match std::panic::catch_unwind(AssertUnwindSafe(|| {
            self.measure_inner(el, constraints, ctx)
        })) {
            Ok(s) => s,
            Err(_) => {
                let type_name = self.type_name_of(el).unwrap_or_else(|| "?".into());
                self.record_error(el, type_name, "measure panicked".to_string());
                Size::ZERO
            }
        }
    }

    fn measure_inner(&self, el: ElementId, constraints: Constraints, ctx: &mut LayoutCtx) -> Size {
        {
            let arena = self.inner.arena.borrow();
            if arena.get(el).map(|e| e.dirty.is_empty()).unwrap_or(false) {
                drop(arena);
                let measures = self.inner.measures.borrow();
                if let Some(e) = measures.get(&el)
                    && e.constraints == constraints
                {
                    return e.size;
                }
            }
        }
        let Some(v) = self.view_rc(el) else {
            return Size::ZERO;
        };
        let style = v.layout_style_ctx_erased(self, el);
        let container = v.container();
        let grid = v.grid();

        let dock = if grid.is_some() { None } else { v.dock() };

        let wrap = if grid.is_some() || dock.is_some() {
            None
        } else {
            v.wrap()
        };

        let flex = v.flex_weights().unwrap_or_default();
        let pre_placements = v.grid_placements().unwrap_or_default();
        let children_len = self.children_len(el);

        {
            let measures = self.inner.measures.borrow();
            if let Some(e) = measures.get(&el)
                && e.constraints == constraints
                && e.style == style
                && e.container == container
                && e.grid == grid
                && e.placements == pre_placements
                && e.dock == dock
                && e.wrap == wrap
                && e.children_len == children_len
            {
                return e.size;
            }
        }

        let children = self.children(el);

        let intrinsic = v.intrinsic_size(ctx);
        let avail = constraints.max;
        let mut w = match style.width {
            Length::Px(p) => p,
            Length::Percent(p) => {
                if avail.width.is_finite() && p.is_finite() {
                    (avail.width * p / 100.0).max(0.0)
                } else {
                    intrinsic.map(|i| i.width).unwrap_or(0.0)
                }
            }
            Length::Fill(_) => {
                if avail.width.is_finite() {
                    avail.width
                } else {
                    intrinsic.map(|i| i.width).unwrap_or(0.0)
                }
            }
            Length::Fit => intrinsic.map(|i| i.width).unwrap_or(0.0),
        };
        let mut h = match style.height {
            Length::Px(p) => p,
            Length::Percent(p) => {
                if avail.height.is_finite() && p.is_finite() {
                    (avail.height * p / 100.0).max(0.0)
                } else {
                    intrinsic.map(|i| i.height).unwrap_or(0.0)
                }
            }
            Length::Fill(_) => {
                if avail.height.is_finite() {
                    avail.height
                } else {
                    intrinsic.map(|i| i.height).unwrap_or(0.0)
                }
            }
            Length::Fit => intrinsic.map(|i| i.height).unwrap_or(0.0),
        };

        let content_width = matches!(style.width, Length::Fit)
            || (style.width.is_fill() && !avail.width.is_finite());
        let content_height = matches!(style.height, Length::Fit)
            || (style.height.is_fill() && !avail.height.is_finite());

        let apply_aspect = |mut w: f32, mut h: f32| -> (f32, f32) {
            if let Some(r) = style.aspect_ratio
                && r.is_finite()
                && r > 0.0
            {
                match (content_width, content_height) {
                    (true, false) => w = (h * r).max(0.0),
                    (false, true) => h = (w / r).max(0.0),
                    (true, true) => {
                        if w <= 0.0 && h > 0.0 {
                            w = (h * r).max(0.0);
                        } else if h <= 0.0 && w > 0.0 {
                            h = (w / r).max(0.0);
                        }
                    }
                    (false, false) => {}
                }
            }
            (w, h)
        };

        let eff_container: Option<ContainerStyle> = if grid.is_some()
            || dock.is_some()
            || wrap.is_some()
        {
            None
        } else {
            container.or_else(|| {
                if children.is_empty() || intrinsic.is_some() || (!content_width && !content_height)
                {
                    None
                } else {
                    Some(ContainerStyle::new(Axis::Vertical, 0.0))
                }
            })
        };

        if let Some(cstyle) = eff_container
            && !children.is_empty()
            && !(intrinsic.is_some() && !content_width && !content_height)
            && !(v.contain_size_erased() && !content_width && !content_height)
        {
            let pad = style.padding;
            let inner_avail = Size::new(
                ((if content_width { avail.width } else { w }) - pad.horizontal_sum()).max(0.0),
                ((if content_height { avail.height } else { h }) - pad.vertical_sum()).max(0.0),
            );
            let (total_main, cross_max) = self.measure_children(
                &children,
                &cstyle,
                inner_avail,
                &flex,
                ctx,
                match cstyle.axis {
                    Axis::Horizontal => content_height,
                    Axis::Vertical => content_width,
                },
            );
            let (mw, mh) = match cstyle.axis {
                Axis::Horizontal => (total_main, cross_max),
                Axis::Vertical => (cross_max, total_main),
            };
            if content_width && intrinsic.is_none() {
                w = mw + pad.horizontal_sum();
            }
            if content_height && intrinsic.is_none() {
                h = mh + pad.vertical_sum();
            }
        }

        let placements = v.grid_placements().unwrap_or_default();
        if let Some(ref gstyle) = grid
            && !children.is_empty()
            && !(intrinsic.is_some() && !content_width && !content_height)
            && !(v.contain_size_erased() && !content_width && !content_height)
        {
            let pad = style.padding;
            let inner_avail = Size::new(
                ((if content_width { avail.width } else { w }) - pad.horizontal_sum()).max(0.0),
                ((if content_height { avail.height } else { h }) - pad.vertical_sum()).max(0.0),
            );
            let (gw, gh) = self.measure_grid(&children, gstyle, &placements, inner_avail, ctx);
            if content_width && intrinsic.is_none() {
                w = gw + pad.horizontal_sum();
            }
            if content_height && intrinsic.is_none() {
                h = gh + pad.vertical_sum();
            }
        }

        if let Some(ref dstyle) = dock
            && !children.is_empty()
            && !(intrinsic.is_some() && !content_width && !content_height)
            && !(v.contain_size_erased() && !content_width && !content_height)
        {
            let pad = style.padding;
            let inner_avail = Size::new(
                ((if content_width { avail.width } else { w }) - pad.horizontal_sum()).max(0.0),
                ((if content_height { avail.height } else { h }) - pad.vertical_sum()).max(0.0),
            );
            let (dw, dh) = self.measure_dock(&children, dstyle, inner_avail, ctx);
            if content_width && intrinsic.is_none() {
                w = dw + pad.horizontal_sum();
            }
            if content_height && intrinsic.is_none() {
                h = dh + pad.vertical_sum();
            }
        }

        if let Some(wstyle) = wrap
            && !children.is_empty()
            && !(intrinsic.is_some() && !content_width && !content_height)
            && !(v.contain_size_erased() && !content_width && !content_height)
        {
            let pad = style.padding;
            let inner_avail = Size::new(
                ((if content_width { avail.width } else { w }) - pad.horizontal_sum()).max(0.0),
                ((if content_height { avail.height } else { h }) - pad.vertical_sum()).max(0.0),
            );
            let (ww, wh) = self.measure_wrap(&children, &wstyle, inner_avail, ctx);
            if content_width && intrinsic.is_none() {
                w = ww + pad.horizontal_sum();
            }
            if content_height && intrinsic.is_none() {
                h = wh + pad.vertical_sum();
            }
        }

        let (w, h) = apply_aspect(w, h);
        let size = clamp_to(style.clamp_size(Size::new(w, h)), constraints);
        self.inner.measures.borrow_mut().insert(
            el,
            MeasureEntry {
                constraints,
                style,
                container,
                grid,
                placements,
                dock,
                wrap,
                children_len: children.len(),
                size,
            },
        );
        size
    }

    fn measure_children(
        &self,
        children: &[ElementId],
        cstyle: &ContainerStyle,
        inner_avail: Size,
        flex: &[f32],
        ctx: &mut LayoutCtx,
        need_cross: bool,
    ) -> (f32, f32) {
        let flow: Vec<(usize, ElementId)> = children
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, c)| !self.child_layout_style(*c).is_absolute())
            .collect();
        if flow.is_empty() {
            return (0.0, 0.0);
        }

        if cstyle.wrap {
            return self.measure_children_wrapped(
                &flow,
                cstyle,
                inner_avail,
                flex,
                ctx,
                need_cross,
            );
        }
        let (avail_main, avail_cross) = match cstyle.axis {
            Axis::Horizontal => (inner_avail.width, inner_avail.height),
            Axis::Vertical => (inner_avail.height, inner_avail.width),
        };

        let mut bases = Vec::with_capacity(flow.len());
        let mut crosses = if need_cross {
            Vec::with_capacity(flow.len())
        } else {
            Vec::new()
        };
        let mut items = Vec::with_capacity(flow.len());
        let mut child_styles = Vec::with_capacity(flow.len());

        for &(orig_i, child) in &flow {
            let extra = flex.get(orig_i).copied().unwrap_or(0.0);
            let (base, grow, shrink, cross, mn, mx) =
                self.child_layout_info_full(child, cstyle, avail_main, avail_cross, extra, ctx);
            bases.push(base);
            if need_cross {
                crosses.push(cross);
            }
            items.push(FlexItem::new(base, grow, shrink, mn, mx));
            child_styles.push(self.child_layout_style(child));
        }
        let gaps = Self::indents_main_total(&child_styles, cstyle.axis, cstyle.gap);

        {
            let mut sizes = bases.clone();
            distribute_flex(&items, &mut sizes, gaps, avail_main);
            bases = sizes;
        }

        if need_cross && avail_main.is_finite() {
            for (k, &(_, child)) in flow.iter().enumerate() {
                if self.subtree_has_wrap_axis(child, cstyle.axis) {
                    crosses[k] =
                        self.constrained_cross(child, cstyle.axis, bases[k], avail_cross, ctx);
                }
            }
        }

        let total_main = bases.iter().sum::<f32>() + gaps;
        let cross_max = if need_cross {
            crosses.iter().cloned().fold(0.0_f32, f32::max)
        } else {
            0.0
        };
        (total_main, cross_max)
    }

    fn measure_children_wrapped(
        &self,
        flow: &[(usize, ElementId)],
        cstyle: &ContainerStyle,
        inner_avail: Size,
        flex: &[f32],
        ctx: &mut LayoutCtx,
        need_cross: bool,
    ) -> (f32, f32) {
        let (avail_main, avail_cross) = match cstyle.axis {
            Axis::Horizontal => (inner_avail.width, inner_avail.height),
            Axis::Vertical => (inner_avail.height, inner_avail.width),
        };

        let mut mains = Vec::with_capacity(flow.len());
        let mut crosses = Vec::with_capacity(flow.len());
        let mut grows = Vec::with_capacity(flow.len());
        let mut shrinks = Vec::with_capacity(flow.len());
        let mut mins = Vec::with_capacity(flow.len());
        let mut maxs = Vec::with_capacity(flow.len());
        for &(orig_i, child) in flow {
            let extra = flex.get(orig_i).copied().unwrap_or(0.0);
            let (base, grow, shrink, cross, mn, mx) =
                self.child_layout_info_full(child, cstyle, avail_main, avail_cross, extra, ctx);
            mains.push(base);
            crosses.push(cross);
            grows.push(grow);
            shrinks.push(shrink);
            mins.push(mn);
            maxs.push(mx);
        }
        let lines = wrap_lines(&mains, avail_main, cstyle.gap);
        if lines.is_empty() {
            return (0.0, 0.0);
        }
        let mut main_max = 0.0f32;
        let mut cross_total = 0.0f32;
        for (li, &(s, e)) in lines.iter().enumerate() {
            let mut line_sizes = mains[s..e].to_vec();
            let items: Vec<FlexItem> = (s..e)
                .map(|i| FlexItem::new(mains[i], grows[i], shrinks[i], mins[i], maxs[i]))
                .collect();
            let line_gaps = cstyle.main_gaps_total(e - s);
            distribute_flex(&items, &mut line_sizes, line_gaps, avail_main);
            let used: f32 = line_sizes.iter().sum::<f32>() + line_gaps;
            main_max = main_max.max(used);
            if need_cross {
                let line_cross = crosses[s..e].iter().cloned().fold(0.0f32, f32::max);
                if li > 0 {
                    cross_total += cstyle.cross_gap;
                }
                cross_total += line_cross;
            }
        }

        (main_max.max(0.0), cross_total.max(0.0))
    }

    fn subtree_has_wrap_axis(&self, root: ElementId, axis: Axis) -> bool {
        let axis_u8 = match axis {
            Axis::Horizontal => 0,
            Axis::Vertical => 1,
        };
        let key = (root.index, root.generation, axis_u8);
        if let Some(hit) = self.inner.wrap_cache.borrow().get(&key).copied() {
            return hit;
        }
        let mut stack = vec![root];
        let mut found = false;
        while let Some(id) = stack.pop() {
            if let Some(v) = self.view_rc(id) {
                let grid = v.grid();
                let dock = if grid.is_some() { None } else { v.dock() };
                let wrap = if grid.is_some() || dock.is_some() {
                    None
                } else {
                    v.wrap()
                };
                if let Some(w) = wrap
                    && w.axis == axis
                {
                    found = true;
                    break;
                }
            }
            {
                let arena = self.inner.arena.borrow();
                if let Some(e) = arena.get(id) {
                    stack.extend(e.children.iter().copied());
                }
            }
        }
        self.inner.wrap_cache.borrow_mut().insert(key, found);
        found
    }

    fn subtree_redistributes(&self, root: ElementId) -> bool {
        let key = (root.index, root.generation, 2u8);
        if let Some(hit) = self.inner.wrap_cache.borrow().get(&key).copied() {
            return hit;
        }
        let mut found = false;
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            let Some(v) = self.view_rc(id) else {
                continue;
            };
            if v.flex_weights().is_some_and(|w| w.iter().any(|f| *f > 0.0)) {
                found = true;
                break;
            }
            let cs = v.layout_style_ctx_erased(self, id);
            if cs.flex_weight() > 0.0
                || matches!(cs.width, Length::Fill(_))
                || matches!(cs.height, Length::Fill(_))
            {
                found = true;
                break;
            }
            {
                let arena = self.inner.arena.borrow();
                if let Some(e) = arena.get(id) {
                    stack.extend(e.children.iter().copied());
                }
            }
        }
        self.inner.wrap_cache.borrow_mut().insert(key, found);
        found
    }

    fn constrained_cross(
        &self,
        child: ElementId,
        container_axis: Axis,
        allocated_main: f32,
        avail_cross: f32,
        ctx: &mut LayoutCtx,
    ) -> f32 {
        let Some(v) = self.view_rc(child) else {
            return 0.0;
        };
        let cs = v.layout_style_ctx_erased(self, child);

        let margin_main = cs.margin_main(container_axis);
        let allocated = (allocated_main - margin_main).max(0.0);
        let max = match container_axis {
            Axis::Horizontal => Size::new(allocated, avail_cross),
            Axis::Vertical => Size::new(avail_cross, allocated),
        };
        let m = self.measure(
            child,
            Constraints {
                min: Size::ZERO,
                max,
            },
            ctx,
        );
        let cross_margin = cs.margin_cross(container_axis);
        match container_axis {
            Axis::Horizontal => {
                cs.clamp_height(resolve_cross(cs.height, m.height, avail_cross))
                    .max(0.0)
                    + cross_margin
            }
            Axis::Vertical => {
                cs.clamp_width(resolve_cross(cs.width, m.width, avail_cross))
                    .max(0.0)
                    + cross_margin
            }
        }
    }

    fn child_limits_full(&self, child: ElementId, axis: Axis) -> (f32, f32, f32, f32, f32) {
        match self.view_rc(child) {
            Some(v) => {
                let cs = v.layout_style_ctx_erased(self, child);
                let margin_main = cs.margin_main(axis);
                let margin_cross = cs.margin_cross(axis);
                let px_floor_main = match axis {
                    Axis::Horizontal => match cs.width {
                        Length::Px(w) => w.max(0.0),
                        _ => 0.0,
                    },
                    Axis::Vertical => match cs.height {
                        Length::Px(h) => h.max(0.0),
                        _ => 0.0,
                    },
                };
                let px_floor_cross = match axis {
                    Axis::Horizontal => match cs.height {
                        Length::Px(h) => h.max(0.0),
                        _ => 0.0,
                    },
                    Axis::Vertical => match cs.width {
                        Length::Px(w) => w.max(0.0),
                        _ => 0.0,
                    },
                };
                let (min_c, max_c) = match axis {
                    Axis::Horizontal => (cs.min_height, cs.max_height),
                    Axis::Vertical => (cs.min_width, cs.max_width),
                };
                let min_c = {
                    let m = if min_c.is_finite() {
                        min_c.max(0.0)
                    } else {
                        0.0
                    };
                    m.max(px_floor_cross)
                };
                let max_c = if max_c.is_nan() {
                    f32::INFINITY
                } else if max_c.is_infinite() {
                    if max_c > 0.0 { f32::INFINITY } else { 0.0 }
                } else {
                    max_c.max(0.0)
                };
                (
                    cs.min_main(axis).max(px_floor_main) + margin_main,
                    cs.max_main(axis) + margin_main,
                    min_c + margin_cross,
                    max_c + margin_cross,
                    cs.flex_shrink_weight(),
                )
            }
            None => (0.0, f32::INFINITY, 0.0, f32::INFINITY, 1.0),
        }
    }

    fn child_layout_info_full(
        &self,
        child: ElementId,
        cstyle: &ContainerStyle,
        avail_main: f32,
        avail_cross: f32,
        flex_extra: f32,
        ctx: &mut LayoutCtx,
    ) -> (f32, f32, f32, f32, f32, f32) {
        let Some(v) = self.view_rc(child) else {
            return (0.0, 0.0, 1.0, 0.0, 0.0, f32::INFINITY);
        };
        let cs = v.layout_style_ctx_erased(self, child);
        let (main_len, cross_len) = match cstyle.axis {
            Axis::Horizontal => (cs.width, cs.height),
            Axis::Vertical => (cs.height, cs.width),
        };

        let (child_max_w, child_max_h) = match cstyle.axis {
            Axis::Horizontal => (f32::INFINITY, avail_cross),
            Axis::Vertical => (avail_cross, f32::INFINITY),
        };
        let m = self.measure(
            child,
            Constraints {
                min: Size::ZERO,
                max: Size::new(child_max_w, child_max_h),
            },
            ctx,
        );
        let (main_intrinsic, cross_intrinsic) = match cstyle.axis {
            Axis::Horizontal => (m.width, m.height),
            Axis::Vertical => (m.height, m.width),
        };

        let (mut base, fill_grow) = resolve_main(main_len, main_intrinsic, avail_main);

        let (basis_base, basis_grow) =
            resolve_flex_basis(cs.flex_basis, (base, fill_grow), main_intrinsic, avail_main);
        base = basis_base;
        let mut grow_basis = fill_grow.max(basis_grow);
        let mut cross = resolve_cross(cross_len, cross_intrinsic, avail_cross);

        if matches!(cross_len, Length::Fit) && !self.subtree_redistributes(child) {
            let m_true = self.measure(
                child,
                Constraints {
                    min: Size::ZERO,
                    max: Size::new(f32::INFINITY, f32::INFINITY),
                },
                ctx,
            );
            let true_cross_intrinsic = match cstyle.axis {
                Axis::Horizontal => m_true.height,
                Axis::Vertical => m_true.width,
            };
            cross = cross.max(resolve_cross(cross_len, true_cross_intrinsic, avail_cross));
        }

        let main_auto =
            matches!(main_len, Length::Fit) || (main_len.is_fill() && !avail_main.is_finite());
        let cross_auto =
            matches!(cross_len, Length::Fit) || (cross_len.is_fill() && !avail_cross.is_finite());

        let (bw, bh) = match cstyle.axis {
            Axis::Horizontal => cs.apply_aspect(base, cross, main_auto, cross_auto),
            Axis::Vertical => {
                let (h, w) = (base, cross);
                let (w2, h2) = cs.apply_aspect(w, h, cross_auto, main_auto);
                (h2, w2)
            }
        };
        base = bw;
        cross = bh;

        let extra = if flex_extra.is_finite() && flex_extra > 0.0 {
            flex_extra
        } else {
            0.0
        };
        let grow = grow_basis.max(cs.flex_weight()).max(extra);
        grow_basis = grow;
        let shrink = cs.flex_shrink_weight();

        let (mut mn, mx, _, _, _) = self.child_limits_full(child, cstyle.axis);
        let margin_main = cs.margin_main(cstyle.axis);
        let margin_cross = cs.margin_cross(cstyle.axis);
        let (base, cross) = match cstyle.axis {
            Axis::Horizontal => (
                cs.clamp_width(base).max(0.0) + margin_main,
                cs.clamp_height(cross).max(0.0) + margin_cross,
            ),
            Axis::Vertical => (
                cs.clamp_height(base).max(0.0) + margin_main,
                cs.clamp_width(cross).max(0.0) + margin_cross,
            ),
        };

        if grow <= 0.0 && matches!(main_len, Length::Fit) {
            mn = mn.max(base);
        }
        let _ = grow_basis;
        (base.max(0.0), grow, shrink, cross.max(0.0), mn, mx)
    }

    fn child_grid_placement(&self, child: ElementId) -> GridPlacement {
        self.view_rc(child)
            .and_then(|v| v.grid_placement())
            .unwrap_or(GridPlacement::AUTO)
    }

    fn grid_cells(
        &self,
        children: &[ElementId],
        grid: &GridStyle,
        placements: &[GridPlacement],
    ) -> Vec<(usize, usize, usize, usize)> {
        let ncols = grid.columns.len().max(1);
        let mut cells = Vec::with_capacity(children.len());
        let mut occupied: Vec<Vec<bool>> = Vec::new();
        let ensure_row = |occupied: &mut Vec<Vec<bool>>, r: usize| {
            while occupied.len() <= r {
                occupied.push(vec![false; ncols]);
            }
        };
        let mut cursor = (0usize, 0usize);
        for (idx, &child) in children.iter().enumerate() {
            if self.child_layout_style(child).is_absolute() {
                cells.push((0, 0, 0, 0));
                continue;
            }
            let p = placements
                .get(idx)
                .copied()
                .filter(|p| !p.is_auto())
                .or_else(|| Some(self.child_grid_placement(child)))
                .unwrap_or(GridPlacement::AUTO);
            let cs = p.col_span.max(1).min(ncols);
            let rs = p.row_span.max(1);
            if let (Some(c), Some(r)) = (p.column, p.row) {
                let c = c.min(ncols.saturating_sub(1));
                ensure_row(&mut occupied, r + rs - 1);
                for row in occupied.iter_mut().skip(r).take(rs) {
                    for cell in row.iter_mut().take((c + cs).min(ncols)).skip(c) {
                        *cell = true;
                    }
                }
                cells.push((c, r, cs, rs));
                continue;
            }

            match grid.auto_flow {
                sequoia_layout::GridFlow::Row => {
                    let (mut c, mut r) = cursor;
                    loop {
                        ensure_row(&mut occupied, r + rs - 1);

                        let mut placed: Option<usize> = None;
                        let start_c = if r == cursor.1 { c } else { 0 };
                        for cc in start_c..ncols {
                            if cc + cs > ncols {
                                break;
                            }
                            let free =
                                (cc..cc + cs).all(|x| (r..r + rs).all(|rr| !occupied[rr][x]));
                            if free {
                                placed = Some(cc);
                                break;
                            }
                        }
                        if let Some(cc) = placed {
                            for row in occupied.iter_mut().skip(r).take(rs) {
                                for cell in row.iter_mut().skip(cc).take(cs) {
                                    *cell = true;
                                }
                            }
                            cells.push((cc, r, cs, rs));
                            cursor = (cc + cs, r);
                            if cursor.0 >= ncols {
                                cursor = (0, r + 1);
                            }
                            break;
                        }

                        c = 0;
                        r += 1;
                    }
                }
                sequoia_layout::GridFlow::Column => {
                    let idx = cells.iter().filter(|(_, _, cs, _)| *cs > 0).count();
                    let nrows_est = children.len().div_ceil(ncols).max(1);
                    let col = (idx / nrows_est) % ncols;
                    let row = idx % nrows_est;
                    ensure_row(&mut occupied, row + rs - 1);
                    cells.push((col.min(ncols - 1), row, cs, rs));
                }
            }
        }
        cells
    }

    fn track_cap(&self, track: Option<&Track>, avail: f32) -> f32 {
        match track {
            Some(Track::Px(v)) => v.max(0.0),
            Some(Track::Percent(p)) if avail.is_finite() && p.is_finite() => {
                (avail * p / 100.0).max(0.0)
            }
            Some(Track::MinMax { max, .. }) => self.track_cap(Some(max), avail),
            _ => f32::INFINITY,
        }
    }

    fn grid_content_sizes(
        &self,
        children: &[ElementId],
        grid: &GridStyle,
        placements: &[GridPlacement],
        inner_avail: Size,
        ctx: &mut LayoutCtx,
    ) -> (Vec<f32>, Vec<f32>, Vec<(usize, usize, usize, usize)>, usize) {
        let ncols = grid.columns.len().max(1);
        let cells = self.grid_cells(children, grid, placements);
        let nrows_needed = cells
            .iter()
            .map(|(_, r, _, rs)| if *rs > 0 { r + rs } else { 0 })
            .max()
            .unwrap_or(0)
            .max(grid.rows.len())
            .max(1);

        let mut content_col = vec![0.0_f32; ncols];
        let mut content_row = vec![0.0_f32; nrows_needed];

        let tracks_fixed = grid.columns.iter().all(|t| {
            matches!(
                t,
                sequoia_layout::Track::Px(_)
                    | sequoia_layout::Track::Percent(_)
                    | sequoia_layout::Track::Fill(_)
            )
        }) && grid.rows.len() >= nrows_needed
            && grid.rows.iter().all(|t| {
                matches!(
                    t,
                    sequoia_layout::Track::Px(_)
                        | sequoia_layout::Track::Percent(_)
                        | sequoia_layout::Track::Fill(_)
                )
            });
        if tracks_fixed && inner_avail.width.is_finite() && inner_avail.height.is_finite() {
            return (content_col, content_row, cells, nrows_needed);
        }

        for (i, &child) in children.iter().enumerate() {
            let (col, row, cs, _rs) = cells[i];
            if cs == 0 {
                continue;
            }
            let col_fixed = matches!(
                grid.columns.get(col),
                Some(sequoia_layout::Track::Px(_)) | Some(sequoia_layout::Track::Percent(_))
            );
            let row_fixed = matches!(
                grid.rows.get(row),
                Some(sequoia_layout::Track::Px(_)) | Some(sequoia_layout::Track::Percent(_))
            );
            if col_fixed && row_fixed {
                continue;
            }
            let col_max = self.track_cap(grid.columns.get(col), inner_avail.width);
            let m = self.measure(
                child,
                Constraints {
                    min: Size::ZERO,
                    max: Size::new(col_max, f32::INFINITY),
                },
                ctx,
            );

            let style = self.child_layout_style(child);
            let outer = style.outer_size(m);
            content_col[col] = content_col[col].max(outer.width);
            if row < content_row.len() {
                content_row[row] = content_row[row].max(outer.height);
            }
        }

        (content_col, content_row, cells, nrows_needed)
    }

    fn grid_rows(grid: &GridStyle, nrows_needed: usize) -> Vec<Track> {
        let mut rows: Vec<Track> = grid.rows.clone();
        rows.resize(nrows_needed, Track::Fit);
        rows
    }

    fn measure_grid(
        &self,
        children: &[ElementId],
        grid: &GridStyle,
        placements: &[GridPlacement],
        inner_avail: Size,
        ctx: &mut LayoutCtx,
    ) -> (f32, f32) {
        let (content_col, content_row, _cells, nrows_needed) =
            self.grid_content_sizes(children, grid, placements, inner_avail, ctx);
        let widths = resolve_tracks(
            &grid.columns,
            &content_col,
            inner_avail.width,
            grid.column_gap,
        );
        let rows = Self::grid_rows(grid, nrows_needed);
        let heights = resolve_tracks(&rows, &content_row, inner_avail.height, grid.row_gap);

        let gaps_x = grid.column_gap * widths.len().saturating_sub(1) as f32;
        let gaps_y = grid.row_gap * heights.len().saturating_sub(1) as f32;
        (
            widths.iter().sum::<f32>() + gaps_x,
            heights.iter().sum::<f32>() + gaps_y,
        )
    }

    fn measure_wrap(
        &self,
        children: &[ElementId],
        wstyle: &WrapStyle,
        inner_avail: Size,
        ctx: &mut LayoutCtx,
    ) -> (f32, f32) {
        let avail_main = match wstyle.axis {
            Axis::Horizontal => inner_avail.width,
            Axis::Vertical => inner_avail.height,
        };
        let (mains, crosses, _) = self.wrap_child_sizes(children, wstyle, ctx);
        let lines = wrap_lines(&mains, avail_main, wstyle.main_gap());

        let mut main_max = 0.0_f32;
        let mut cross_total = 0.0_f32;
        for (line_idx, &(start, end)) in lines.iter().enumerate() {
            let mut used = 0.0_f32;
            let mut line_cross = 0.0_f32;
            for i in start..end {
                if i > start {
                    used += wstyle.main_gap();
                }
                used += mains[i];
                line_cross = line_cross.max(crosses[i]);
            }
            main_max = main_max.max(used);
            if line_idx > 0 {
                cross_total += wstyle.cross_gap();
            }
            cross_total += line_cross;
        }

        match wstyle.axis {
            Axis::Horizontal => (main_max, cross_total),
            Axis::Vertical => (cross_total, main_max),
        }
    }

    fn wrap_child_sizes(
        &self,
        children: &[ElementId],
        wstyle: &WrapStyle,
        ctx: &mut LayoutCtx,
    ) -> (Vec<f32>, Vec<f32>, Vec<ElementId>) {
        let mut mains = Vec::with_capacity(children.len());
        let mut crosses = Vec::with_capacity(children.len());
        let mut ids = Vec::with_capacity(children.len());
        for &child in children {
            if self.child_layout_style(child).is_absolute() {
                continue;
            }
            let m = self.measure(
                child,
                Constraints {
                    min: Size::ZERO,
                    max: Size::new(f32::INFINITY, f32::INFINITY),
                },
                ctx,
            );
            let cs = self.child_layout_style(child);
            let (main, cross) = match wstyle.axis {
                Axis::Horizontal => (
                    m.width + cs.margin.horizontal_sum(),
                    m.height + cs.margin.vertical_sum(),
                ),
                Axis::Vertical => (
                    m.height + cs.margin.vertical_sum(),
                    m.width + cs.margin.horizontal_sum(),
                ),
            };
            mains.push(main.max(0.0));
            crosses.push(cross.max(0.0));
            ids.push(child);
        }
        (mains, crosses, ids)
    }

    fn arrange(&self, el: ElementId, rect: Rect, ctx: &mut LayoutCtx) {
        if std::panic::catch_unwind(AssertUnwindSafe(|| self.arrange_inner(el, rect, ctx))).is_err()
        {
            let type_name = self.type_name_of(el).unwrap_or_else(|| "?".into());
            self.record_error(el, type_name, "arrange panicked".to_string());
        }
    }

    fn arrange_inner(&self, el: ElementId, rect: Rect, ctx: &mut LayoutCtx) {
        if let Some(e) = self.inner.arena.borrow_mut().get_mut(el) {
            e.rect.set(rect);
        }

        if let Some(v) = self.view_rc(el)
            && v.arrange_override_erased(self, el, rect, ctx)
        {
            return;
        }

        let Some(v) = self.view_rc(el) else { return };
        let style = v.layout_style_ctx_erased(self, el);
        let content = Rect::from_xywh(
            rect.left() + style.padding.left,
            rect.top() + style.padding.top,
            (rect.size.width - style.padding.horizontal_sum()).max(0.0),
            (rect.size.height - style.padding.vertical_sum()).max(0.0),
        );

        if let Some(gstyle) = v.grid() {
            let children = self.children(el);
            if !children.is_empty() {
                let placements = v.grid_placements().unwrap_or_default();
                self.arrange_grid(&children, &gstyle, &placements, content, ctx);
            } else {
                for &child in &self.children(el) {
                    if self.child_layout_style(child).is_absolute() {
                        self.arrange_absolute(child, content, ctx);
                    }
                }
            }
            return;
        }

        if let Some(dstyle) = v.dock() {
            let children = self.children(el);
            if !children.is_empty() {
                self.arrange_dock(&children, &dstyle, content, ctx);
            }
            return;
        }

        if let Some(wstyle) = v.wrap() {
            let children = self.children(el);
            if !children.is_empty() {
                self.arrange_wrap(&children, &wstyle, content, ctx);
            }
            return;
        }

        if let Some(cstyle) = v.container() {
            let children = self.children(el);
            if !children.is_empty() {
                let flex = v.flex_weights().unwrap_or_default();
                self.arrange_children(&children, &cstyle, content, &flex, ctx);
                return;
            }
        }

        let children = self.children(el);
        if children.len() == 1 {
            let child = children[0];
            let cs = self.child_layout_style(child);
            if cs.is_absolute() {
                self.arrange_absolute(child, content, ctx);
                return;
            }
            let avail = Size::new(
                (content.size.width - cs.margin.horizontal_sum()).max(0.0),
                (content.size.height - cs.margin.vertical_sum()).max(0.0),
            );
            let m = self.measure(
                child,
                Constraints {
                    min: Size::ZERO,
                    max: avail,
                },
                ctx,
            );
            let (cw, ch) = match self.view_rc(child) {
                Some(v) => {
                    let cs = v.layout_style_ctx_erased(self, child);
                    let w_auto = matches!(cs.width, Length::Fit)
                        || (cs.width.is_fill() && !avail.width.is_finite());
                    let h_auto = matches!(cs.height, Length::Fit)
                        || (cs.height.is_fill() && !avail.height.is_finite());
                    let w = resolve_cross(cs.width, m.width, avail.width);
                    let h = resolve_cross(cs.height, m.height, avail.height);
                    let (w, h) = cs.apply_aspect(w, h, w_auto, h_auto);
                    (cs.clamp_width(w).max(0.0), cs.clamp_height(h).max(0.0))
                }
                None => (avail.width, avail.height),
            };
            self.arrange(
                child,
                Rect::from_xywh(
                    content.left() + cs.margin.left,
                    content.top() + cs.margin.top,
                    cw,
                    ch,
                ),
                ctx,
            );
        } else if children.len() > 1 {
            let flow: Vec<ElementId> = children
                .iter()
                .copied()
                .filter(|c| !self.child_layout_style(*c).is_absolute())
                .collect();
            if flow.len() == 1 {
                let child = flow[0];
                let cs = self.child_layout_style(child);
                let avail = Size::new(
                    (content.size.width - cs.margin.horizontal_sum()).max(0.0),
                    (content.size.height - cs.margin.vertical_sum()).max(0.0),
                );
                let m = self.measure(child, Constraints::loose(avail), ctx);
                let w = cs
                    .clamp_width(resolve_cross(cs.width, m.width, avail.width))
                    .max(0.0);
                let h = cs
                    .clamp_height(resolve_cross(cs.height, m.height, avail.height))
                    .max(0.0);
                self.arrange(
                    child,
                    Rect::from_xywh(
                        content.left() + cs.margin.left,
                        content.top() + cs.margin.top,
                        w,
                        h,
                    ),
                    ctx,
                );
            } else if !flow.is_empty() {
                let cs = ContainerStyle::new(Axis::Vertical, 0.0);
                self.arrange_children(&flow, &cs, content, &[], ctx);
            }
            for &child in &children {
                if self.child_layout_style(child).is_absolute() {
                    self.arrange_absolute(child, content, ctx);
                }
            }
        }
    }

    fn arrange_wrap(
        &self,
        children: &[ElementId],
        wstyle: &WrapStyle,
        content: Rect,
        ctx: &mut LayoutCtx,
    ) {
        let avail_main = match wstyle.axis {
            Axis::Horizontal => content.size.width,
            Axis::Vertical => content.size.height,
        };
        let (mains, crosses, ids) = self.wrap_child_sizes(children, wstyle, ctx);
        let lines = wrap_lines(&mains, avail_main, wstyle.main_gap());
        let (main_gap, cross_gap) = (wstyle.main_gap(), wstyle.cross_gap());

        let mut cross_cursor = 0.0_f32;
        for (line_idx, &(start, end)) in lines.iter().enumerate() {
            let line_cross = crosses[start..end].iter().copied().fold(0.0_f32, f32::max);
            if line_idx > 0 {
                cross_cursor += cross_gap;
            }
            let mut main_cursor = 0.0_f32;
            for i in start..end {
                let child = ids[i];
                if i > start {
                    main_cursor += main_gap;
                }
                let cs = self.child_layout_style(child);
                let (m_main_start, m_main_end, m_cross_start, m_cross_end) = match wstyle.axis {
                    Axis::Horizontal => (
                        cs.margin.left,
                        cs.margin.right,
                        cs.margin.top,
                        cs.margin.bottom,
                    ),
                    Axis::Vertical => (
                        cs.margin.top,
                        cs.margin.bottom,
                        cs.margin.left,
                        cs.margin.right,
                    ),
                };
                let main_outer = mains[i];
                let cross_outer = crosses[i];
                let main_inner = (main_outer - m_main_start - m_main_end).max(0.0);
                let mut cross_inner = (cross_outer - m_cross_start - m_cross_end).max(0.0);
                let cross_off = match wstyle.align {
                    Align::Start | Align::Stretch => 0.0,
                    Align::Center => ((line_cross - cross_outer) * 0.5).max(0.0),
                    Align::End => (line_cross - cross_outer).max(0.0),
                };
                if wstyle.align == Align::Stretch {
                    let cross_len = match wstyle.axis {
                        Axis::Horizontal => cs.height,
                        Axis::Vertical => cs.width,
                    };
                    if matches!(cross_len, Length::Fit) {
                        cross_inner = (line_cross - m_cross_start - m_cross_end).max(0.0);
                    }
                }
                let rect = match wstyle.axis {
                    Axis::Horizontal => Rect::from_xywh(
                        content.left() + main_cursor + m_main_start,
                        content.top() + cross_cursor + cross_off + m_cross_start,
                        main_inner,
                        cross_inner,
                    ),
                    Axis::Vertical => Rect::from_xywh(
                        content.left() + cross_cursor + cross_off + m_cross_start,
                        content.top() + main_cursor + m_main_start,
                        cross_inner,
                        main_inner,
                    ),
                };
                self.arrange(child, rect, ctx);
                main_cursor += main_outer;
            }
            cross_cursor += line_cross;
        }

        for &child in children {
            if self.child_layout_style(child).is_absolute() {
                self.arrange_absolute(child, content, ctx);
            }
        }
    }

    fn arrange_children(
        &self,
        children: &[ElementId],
        cstyle: &ContainerStyle,
        content: Rect,
        flex: &[f32],
        ctx: &mut LayoutCtx,
    ) {
        let flow_idx: Vec<usize> = children
            .iter()
            .enumerate()
            .filter(|(_, c)| !self.child_layout_style(**c).is_absolute())
            .map(|(i, _)| i)
            .collect();
        if cstyle.wrap {
            self.arrange_children_wrapped(children, &flow_idx, cstyle, content, flex, ctx);
        } else {
            self.arrange_children_linear(children, &flow_idx, cstyle, content, flex, ctx);
        }

        for &child in children {
            if self.child_layout_style(child).is_absolute() {
                self.arrange_absolute(child, content, ctx);
            }
        }
    }

    fn arrange_children_linear(
        &self,
        children: &[ElementId],
        flow_idx: &[usize],
        cstyle: &ContainerStyle,
        content: Rect,
        flex: &[f32],
        ctx: &mut LayoutCtx,
    ) {
        let (avail_main, avail_cross) = match cstyle.axis {
            Axis::Horizontal => (content.size.width, content.size.height),
            Axis::Vertical => (content.size.height, content.size.width),
        };

        let mut infos: Vec<(ElementId, f32, f32, f32, f32, f32, f32, LayoutStyle)> =
            Vec::with_capacity(flow_idx.len());
        for &orig in flow_idx {
            let child = children[orig];
            let extra = flex.get(orig).copied().unwrap_or(0.0);
            let (base, grow, shrink, cross, mn, mx) =
                self.child_layout_info_full(child, cstyle, avail_main, avail_cross, extra, ctx);
            let cs = self.child_layout_style(child);
            infos.push((child, base, grow, shrink, cross, mn, mx, cs));
        }
        let child_styles: Vec<LayoutStyle> = infos.iter().map(|info| info.7).collect();
        let gaps = Self::indents_main_total(&child_styles, cstyle.axis, cstyle.gap);

        {
            let items: Vec<FlexItem> = infos
                .iter()
                .map(|(_, b, g, s, _, mn, mx, _)| FlexItem::new(*b, *g, *s, *mn, *mx))
                .collect();
            let mut bases: Vec<f32> = infos.iter().map(|(_, b, ..)| *b).collect();
            distribute_flex(&items, &mut bases, gaps, avail_main);
            for (info, base) in infos.iter_mut().zip(bases) {
                info.1 = base;
            }
        }

        if avail_main.is_finite() {
            for info in infos.iter_mut() {
                if self.subtree_has_wrap_axis(info.0, cstyle.axis) {
                    info.4 = self.constrained_cross(info.0, cstyle.axis, info.1, avail_cross, ctx);
                }
            }
        }

        let used_main: f32 = infos.iter().map(|(_, b, ..)| *b).sum::<f32>() + gaps;
        let extra = Self::extra_main(avail_main, used_main);
        let (mut cursor, gap_step) = cstyle.justify.distribute(infos.len(), cstyle.gap, extra);
        for (i, info) in infos.iter().enumerate() {
            let (child, base, _, _, cross, _, _, cs) = *info;
            if i > 0 {
                cursor += Self::indent_leading(&cs, cstyle.axis, cstyle.gap, false) + gap_step;
            }
            let (margin_main_start, margin_cross_start) = match cstyle.axis {
                Axis::Horizontal => (cs.margin.left, cs.margin.top),
                Axis::Vertical => (cs.margin.top, cs.margin.left),
            };
            let (cross_off, cross_size) = self.place_cross(child, cstyle, cross, avail_cross);
            let shift = Self::indent_cross_shift(&cs, cstyle.axis);
            let base_inner = (base - cs.margin_main(cstyle.axis)).max(0.0);
            let (rx, ry, rw, rh) = match cstyle.axis {
                Axis::Horizontal => (
                    content.left() + cursor + margin_main_start,
                    content.top() + cross_off + shift,
                    base_inner,
                    cross_size,
                ),
                Axis::Vertical => (
                    content.left() + cross_off + shift,
                    content.top() + cursor + margin_main_start,
                    cross_size,
                    base_inner,
                ),
            };
            let _ = margin_cross_start;
            cursor += base + Self::indent_trailing(&cs, cstyle.axis);
            self.arrange(
                child,
                Rect::from_xywh(rx, ry, rw.max(0.0), rh.max(0.0)),
                ctx,
            );
        }
    }

    fn arrange_children_wrapped(
        &self,
        children: &[ElementId],
        flow_idx: &[usize],
        cstyle: &ContainerStyle,
        content: Rect,
        flex: &[f32],
        ctx: &mut LayoutCtx,
    ) {
        let (avail_main, avail_cross) = match cstyle.axis {
            Axis::Horizontal => (content.size.width, content.size.height),
            Axis::Vertical => (content.size.height, content.size.width),
        };
        let n = flow_idx.len();
        if n == 0 {
            return;
        }
        let mut bases = Vec::with_capacity(n);
        let mut crosses = Vec::with_capacity(n);
        let mut items = Vec::with_capacity(n);
        let mut ids = Vec::with_capacity(n);
        for &orig in flow_idx {
            let child = children[orig];
            let extra = flex.get(orig).copied().unwrap_or(0.0);
            let (base, grow, shrink, cross, mn, mx) =
                self.child_layout_info_full(child, cstyle, avail_main, avail_cross, extra, ctx);
            ids.push(child);
            bases.push(base);
            crosses.push(cross);
            items.push(FlexItem::new(base, grow, shrink, mn, mx));
        }
        let lines = wrap_lines(&bases, avail_main, cstyle.gap);

        let mut line_crosses: Vec<f32> = Vec::with_capacity(lines.len());
        for &(s, e) in &lines {
            let mut line_sizes = bases[s..e].to_vec();
            distribute_flex(
                &items[s..e],
                &mut line_sizes,
                cstyle.main_gaps_total(e - s),
                avail_main,
            );
            for (k, v) in (s..e).zip(line_sizes) {
                bases[k] = v;
            }

            if avail_main.is_finite() {
                for k in s..e {
                    if self.subtree_has_wrap_axis(ids[k], cstyle.axis) {
                        crosses[k] =
                            self.constrained_cross(ids[k], cstyle.axis, bases[k], avail_cross, ctx);
                    }
                }
            }
            line_crosses.push(crosses[s..e].iter().cloned().fold(0.0f32, f32::max));
        }
        let total_cross: f32 = line_crosses.iter().sum::<f32>()
            + cstyle.cross_gap * lines.len().saturating_sub(1) as f32;
        let cross_extra = if avail_cross.is_finite() {
            (avail_cross - total_cross).max(0.0)
        } else {
            0.0
        };
        let (mut cross_cursor, cross_step) = match cstyle.align_content {
            Align::Start => (0.0, 0.0),
            Align::Center => (cross_extra * 0.5, 0.0),
            Align::End => (cross_extra, 0.0),
            Align::Stretch if line_crosses.len() > 1 => {
                (0.0, cross_extra / (line_crosses.len() - 1) as f32)
            }
            Align::Stretch => (0.0, 0.0),
        };

        for (li, &(s, e)) in lines.iter().enumerate() {
            let line_cross = line_crosses[li];
            if li > 0 {
                cross_cursor += cstyle.cross_gap + cross_step;
            }
            let used: f32 = bases[s..e].iter().sum::<f32>() + cstyle.main_gaps_total(e - s);
            let line_extra = if avail_main.is_finite() {
                (avail_main - used).max(0.0)
            } else {
                0.0
            };
            let (mut cursor, gap_step) = cstyle.justify.distribute(e - s, cstyle.gap, line_extra);
            for k in s..e {
                if k > s {
                    cursor += cstyle.gap + gap_step;
                }
                let child = ids[k];
                let cs = self.child_layout_style(child);
                let margin_main_start = match cstyle.axis {
                    Axis::Horizontal => cs.margin.left,
                    Axis::Vertical => cs.margin.top,
                };
                let (cross_off, cross_size) =
                    self.place_cross(child, cstyle, crosses[k], line_cross);
                let base_inner = (bases[k] - cs.margin_main(cstyle.axis)).max(0.0);
                let (rx, ry, rw, rh) = match cstyle.axis {
                    Axis::Horizontal => (
                        content.left() + cursor + margin_main_start,
                        content.top() + cross_cursor + cross_off,
                        base_inner,
                        cross_size,
                    ),
                    Axis::Vertical => (
                        content.left() + cross_cursor + cross_off,
                        content.top() + cursor + margin_main_start,
                        cross_size,
                        base_inner,
                    ),
                };
                cursor += bases[k];
                self.arrange(
                    child,
                    Rect::from_xywh(rx, ry, rw.max(0.0), rh.max(0.0)),
                    ctx,
                );
            }
            cross_cursor += line_cross;
        }
    }

    fn arrange_grid(
        &self,
        children: &[ElementId],
        grid: &GridStyle,
        placements: &[GridPlacement],
        content: Rect,
        ctx: &mut LayoutCtx,
    ) {
        let (content_col, content_row, cells, nrows_needed) =
            self.grid_content_sizes(children, grid, placements, content.size, ctx);
        let widths = resolve_tracks(
            &grid.columns,
            &content_col,
            content.size.width,
            grid.column_gap,
        );
        let rows = Self::grid_rows(grid, nrows_needed);
        let heights = resolve_tracks(&rows, &content_row, content.size.height, grid.row_gap);
        let xs = track_offsets(&widths, grid.column_gap);
        let ys = track_offsets(&heights, grid.row_gap);

        for (i, &child) in children.iter().enumerate() {
            let cs = self.child_layout_style(child);
            if cs.is_absolute() {
                self.arrange_absolute(child, content, ctx);
                continue;
            }
            let (col, row, col_span, row_span) = cells.get(i).copied().unwrap_or((0, 0, 1, 1));
            if col_span == 0 {
                continue;
            }

            let cw = (col..(col + col_span).min(widths.len()))
                .map(|c| widths[c])
                .sum::<f32>()
                + grid.column_gap * col_span.saturating_sub(1) as f32;
            let ch = (row..(row + row_span).min(heights.len()))
                .map(|r| heights[r])
                .sum::<f32>()
                + grid.row_gap * row_span.saturating_sub(1) as f32;
            let col_max = self.track_cap(grid.columns.get(col), content.size.width);
            let m = self.measure(
                child,
                Constraints {
                    min: Size::ZERO,
                    max: Size::new(col_max, f32::INFINITY),
                },
                ctx,
            );
            let eff_h = cs.effective_align(grid.align_items);
            let eff_w = cs.effective_align(grid.justify_items);

            let mut w = cs
                .clamp_width(resolve_cross(cs.width, m.width, cw))
                .max(0.0);
            let mut h = cs
                .clamp_height(resolve_cross(cs.height, m.height, ch))
                .max(0.0);
            if eff_w == Align::Stretch && matches!(cs.width, Length::Fit) && cw.is_finite() {
                w = (cw - cs.margin.horizontal_sum())
                    .max(0.0)
                    .min(cs.max_width)
                    .max(cs.min_width);
            }
            if eff_h == Align::Stretch && matches!(cs.height, Length::Fit) && ch.is_finite() {
                h = (ch - cs.margin.vertical_sum())
                    .max(0.0)
                    .min(cs.max_height)
                    .max(cs.min_height);
            }
            let cell_x = content.left() + xs.get(col).copied().unwrap_or(0.0);
            let cell_y = content.top() + ys.get(row).copied().unwrap_or(0.0);
            let avail_w = (cw - cs.margin.horizontal_sum()).max(0.0);
            let avail_h = (ch - cs.margin.vertical_sum()).max(0.0);
            let ox = match eff_w {
                Align::Start | Align::Stretch => 0.0,
                Align::Center => ((avail_w - w).max(0.0)) * 0.5,
                Align::End => (avail_w - w).max(0.0),
            };
            let oy = match eff_h {
                Align::Start | Align::Stretch => 0.0,
                Align::Center => ((avail_h - h).max(0.0)) * 0.5,
                Align::End => (avail_h - h).max(0.0),
            };
            self.arrange(
                child,
                Rect::from_xywh(
                    cell_x + cs.margin.left + ox,
                    cell_y + cs.margin.top + oy,
                    w,
                    h,
                ),
                ctx,
            );
        }
    }

    fn measure_dock(
        &self,
        children: &[ElementId],
        dstyle: &DockStyle,
        inner_avail: Size,
        ctx: &mut LayoutCtx,
    ) -> (f32, f32) {
        let flow: Vec<ElementId> = children
            .iter()
            .copied()
            .filter(|c| !self.child_layout_style(*c).is_absolute())
            .collect();
        let n = flow.len();
        if n == 0 {
            return (0.0, 0.0);
        }
        let gap = sanitize_gap(dstyle.gap);

        let order = dock_layout_order(&dstyle.docks, dstyle.last_child_fill, dstyle.ordered, n);
        let mut remaining = Size::new(inner_avail.width.max(0.0), inner_avail.height.max(0.0));
        let mut measured = vec![Size::ZERO; n];
        let mut thick = vec![0.0_f32; n];

        for (pos, &i) in order.iter().enumerate() {
            let child = flow[i];
            let d = resolve_dock(&dstyle.docks, dstyle.last_child_fill, dstyle.ordered, i, n);
            let max = match d {
                Dock::Left | Dock::Right => Size::new(f32::INFINITY, remaining.height),
                Dock::Top | Dock::Bottom => Size::new(remaining.width, f32::INFINITY),
                Dock::Fill => remaining,
            };
            let m = self.measure(child, Constraints::loose(max), ctx);
            let cs = self.child_layout_style(child);
            let t = match d {
                Dock::Left | Dock::Right => Self::clamp_dock_thickness(
                    cs.clamp_width(dock_thickness(cs.width, m.width, remaining.width)),
                    remaining.width,
                ),
                Dock::Top | Dock::Bottom => Self::clamp_dock_thickness(
                    cs.clamp_height(dock_thickness(cs.height, m.height, remaining.height)),
                    remaining.height,
                ),
                Dock::Fill => 0.0,
            };
            measured[i] = m;
            thick[i] = t;
            remaining = Self::shrink_dock_remaining(remaining, d, t, gap, pos, n);
        }

        let mut cw = 0.0_f32;
        let mut ch = 0.0_f32;
        for &i in order.iter().rev() {
            let m = measured[i];
            match resolve_dock(&dstyle.docks, dstyle.last_child_fill, dstyle.ordered, i, n) {
                Dock::Fill => {
                    cw = cw.max(m.width);
                    ch = ch.max(m.height);
                }
                Dock::Left | Dock::Right => {
                    cw += thick[i];

                    if order.iter().position(|&x| x == i).unwrap_or(0) + 1 < n {
                        cw += gap;
                    }
                    ch = ch.max(m.height);
                }
                Dock::Top | Dock::Bottom => {
                    cw = cw.max(m.width);
                    ch += thick[i];
                    if order.iter().position(|&x| x == i).unwrap_or(0) + 1 < n {
                        ch += gap;
                    }
                }
            }
        }
        (cw.max(0.0), ch.max(0.0))
    }

    fn arrange_dock(
        &self,
        children: &[ElementId],
        dstyle: &DockStyle,
        content: Rect,
        ctx: &mut LayoutCtx,
    ) {
        let flow: Vec<ElementId> = children
            .iter()
            .copied()
            .filter(|c| !self.child_layout_style(*c).is_absolute())
            .collect();
        let n = flow.len();
        let gap = sanitize_gap(dstyle.gap);
        let order = dock_layout_order(&dstyle.docks, dstyle.last_child_fill, dstyle.ordered, n);
        let mut remaining = content;
        for (pos, &i) in order.iter().enumerate() {
            let child = flow[i];
            let d = resolve_dock(&dstyle.docks, dstyle.last_child_fill, dstyle.ordered, i, n);
            let max = match d {
                Dock::Left | Dock::Right => {
                    Size::new(f32::INFINITY, remaining.size.height.max(0.0))
                }
                Dock::Top | Dock::Bottom => Size::new(remaining.size.width.max(0.0), f32::INFINITY),
                Dock::Fill => Size::new(
                    remaining.size.width.max(0.0),
                    remaining.size.height.max(0.0),
                ),
            };
            let m = self.measure(child, Constraints::loose(max), ctx);
            let cs = self.child_layout_style(child);
            let rect = match d {
                Dock::Left => {
                    let t = Self::clamp_dock_thickness(
                        cs.clamp_width(dock_thickness(cs.width, m.width, remaining.size.width)),
                        remaining.size.width,
                    );
                    let h = cs.clamp_height(remaining.size.height.max(0.0)).max(0.0);
                    Rect::from_xywh(remaining.left(), remaining.top(), t, h)
                }
                Dock::Right => {
                    let t = Self::clamp_dock_thickness(
                        cs.clamp_width(dock_thickness(cs.width, m.width, remaining.size.width)),
                        remaining.size.width,
                    );
                    let h = cs.clamp_height(remaining.size.height.max(0.0)).max(0.0);
                    Rect::from_xywh(remaining.right() - t, remaining.top(), t, h)
                }
                Dock::Top => {
                    let t = Self::clamp_dock_thickness(
                        cs.clamp_height(dock_thickness(cs.height, m.height, remaining.size.height)),
                        remaining.size.height,
                    );
                    let w = cs.clamp_width(remaining.size.width.max(0.0)).max(0.0);
                    Rect::from_xywh(remaining.left(), remaining.top(), w, t)
                }
                Dock::Bottom => {
                    let t = Self::clamp_dock_thickness(
                        cs.clamp_height(dock_thickness(cs.height, m.height, remaining.size.height)),
                        remaining.size.height,
                    );
                    let w = cs.clamp_width(remaining.size.width.max(0.0)).max(0.0);
                    Rect::from_xywh(remaining.left(), remaining.bottom() - t, w, t)
                }
                Dock::Fill => {
                    let w = cs.clamp_width(remaining.size.width.max(0.0)).max(0.0);
                    let h = cs.clamp_height(remaining.size.height.max(0.0)).max(0.0);
                    Rect::from_xywh(remaining.left(), remaining.top(), w, h)
                }
            };
            self.arrange(
                child,
                Rect::from_xywh(
                    rect.left(),
                    rect.top(),
                    rect.size.width.max(0.0),
                    rect.size.height.max(0.0),
                ),
                ctx,
            );
            remaining = Self::shrink_dock_rect(
                remaining,
                d,
                Self::dock_rect_thickness(rect, d),
                gap,
                pos,
                n,
            );
        }
        for &child in children {
            if self.child_layout_style(child).is_absolute() {
                self.arrange_absolute(child, content, ctx);
            }
        }
    }

    fn dock_rect_thickness(rect: Rect, d: Dock) -> f32 {
        match d {
            Dock::Left | Dock::Right => rect.size.width.max(0.0),
            Dock::Top | Dock::Bottom => rect.size.height.max(0.0),
            Dock::Fill => 0.0,
        }
    }

    fn clamp_dock_thickness(thickness: f32, remaining: f32) -> f32 {
        let t = if thickness.is_finite() {
            thickness.max(0.0)
        } else {
            0.0
        };
        if remaining.is_finite() {
            t.min(remaining.max(0.0))
        } else {
            t
        }
    }

    fn shrink_dock_remaining(
        remaining: Size,
        d: Dock,
        thickness: f32,
        gap: f32,
        index: usize,
        count: usize,
    ) -> Size {
        let extra = if d.is_side() && index + 1 < count {
            gap
        } else {
            0.0
        };
        let sub = thickness + extra;
        match d {
            Dock::Left | Dock::Right => {
                Size::new(shrink_dock_axis(remaining.width, sub), remaining.height)
            }
            Dock::Top | Dock::Bottom => {
                Size::new(remaining.width, shrink_dock_axis(remaining.height, sub))
            }
            Dock::Fill => Size::ZERO,
        }
    }

    fn shrink_dock_rect(
        remaining: Rect,
        d: Dock,
        thickness: f32,
        gap: f32,
        index: usize,
        count: usize,
    ) -> Rect {
        let extra = if d.is_side() && index + 1 < count {
            gap
        } else {
            0.0
        };
        let sub = thickness + extra;
        match d {
            Dock::Left => Rect::from_xywh(
                remaining.left() + sub,
                remaining.top(),
                shrink_dock_axis(remaining.size.width, sub),
                remaining.size.height.max(0.0),
            ),
            Dock::Right => Rect::from_xywh(
                remaining.left(),
                remaining.top(),
                shrink_dock_axis(remaining.size.width, sub),
                remaining.size.height.max(0.0),
            ),
            Dock::Top => Rect::from_xywh(
                remaining.left(),
                remaining.top() + sub,
                remaining.size.width.max(0.0),
                shrink_dock_axis(remaining.size.height, sub),
            ),
            Dock::Bottom => Rect::from_xywh(
                remaining.left(),
                remaining.top(),
                remaining.size.width.max(0.0),
                shrink_dock_axis(remaining.size.height, sub),
            ),
            Dock::Fill => Rect::from_xywh(remaining.left(), remaining.top(), 0.0, 0.0),
        }
    }

    fn extra_main(avail_main: f32, used_main: f32) -> f32 {
        if avail_main.is_finite() {
            (avail_main - used_main).max(0.0)
        } else {
            0.0
        }
    }

    fn sanitize_gap_value(v: f32) -> f32 {
        if v.is_finite() { v.max(0.0) } else { 0.0 }
    }

    fn indent_leading(cs: &LayoutStyle, axis: Axis, default_gap: f32, is_first: bool) -> f32 {
        if is_first {
            return 0.0;
        }
        match cs.indent_main_start(axis) {
            Some(v) => Self::sanitize_gap_value(v),
            None => Self::sanitize_gap_value(default_gap),
        }
    }

    fn indent_trailing(cs: &LayoutStyle, axis: Axis) -> f32 {
        match cs.indent_main_end(axis) {
            Some(v) => Self::sanitize_gap_value(v),
            None => 0.0,
        }
    }

    fn indents_main_total(styles: &[LayoutStyle], axis: Axis, default_gap: f32) -> f32 {
        let mut total = 0.0;
        for (i, cs) in styles.iter().enumerate() {
            total += Self::indent_leading(cs, axis, default_gap, i == 0);
            total += Self::indent_trailing(cs, axis);
        }
        total
    }

    fn indent_cross_shift(cs: &LayoutStyle, axis: Axis) -> f32 {
        let start = cs.indent_cross_start(axis).map(Self::sanitize_gap_value);
        let end = cs.indent_cross_end(axis).map(Self::sanitize_gap_value);
        start.unwrap_or(0.0) - end.unwrap_or(0.0)
    }

    fn place_cross(
        &self,
        child: ElementId,
        cstyle: &ContainerStyle,
        cross: f32,
        avail_cross: f32,
    ) -> (f32, f32) {
        let Some(v) = self.view_rc(child) else {
            return (0.0, 0.0);
        };
        let cs = v.layout_style_ctx_erased(self, child);
        let eff = cs.effective_align(cstyle.align);
        let (m_start, m_end) = match cstyle.axis {
            Axis::Horizontal => (cs.margin.top, cs.margin.bottom),
            Axis::Vertical => (cs.margin.left, cs.margin.right),
        };
        let m_total = m_start + m_end;
        let cross_inner = (cross - m_total).max(0.0);

        let floor = {
            let (cross_len, explicit_min, explicit_max) = match cstyle.axis {
                Axis::Horizontal => (cs.height, cs.min_height, cs.max_height),
                Axis::Vertical => (cs.width, cs.min_width, cs.max_width),
            };
            let px = match cross_len {
                Length::Px(v) if v.is_finite() => v.max(0.0),
                _ => 0.0,
            };
            let explicit_min = if explicit_min.is_finite() {
                explicit_min.max(0.0)
            } else {
                0.0
            };
            let mx = if explicit_max.is_nan() || explicit_max == f32::INFINITY {
                f32::INFINITY
            } else {
                explicit_max.max(0.0)
            };

            let content_floor = match cross_len {
                Length::Fit => cross_inner,
                _ => 0.0,
            };
            px.max(explicit_min).max(content_floor).min(mx)
        };

        let avail_inner = if avail_cross.is_finite() {
            (avail_cross - m_total).max(0.0)
        } else {
            f32::INFINITY
        };
        let placed_inner = cross_inner.min(avail_inner).max(floor);
        let slack = if avail_inner.is_finite() {
            (avail_inner - placed_inner).max(0.0)
        } else {
            0.0
        };
        match eff {
            Align::Start => (m_start, placed_inner),
            Align::Center => (m_start + slack * 0.5, placed_inner),
            Align::End => (m_start + slack, placed_inner),
            Align::Stretch => {
                let (auto_cross, max_cross) = match cstyle.axis {
                    Axis::Horizontal => (matches!(cs.height, Length::Fit), cs.max_height),
                    Axis::Vertical => (matches!(cs.width, Length::Fit), cs.max_width),
                };

                if auto_cross && avail_inner.is_finite() {
                    let stretched = avail_inner.min(max_cross).max(floor).max(0.0);
                    (m_start, stretched)
                } else {
                    (m_start, placed_inner)
                }
            }
        }
    }

    pub(crate) fn arrange_absolute(&self, child: ElementId, content: Rect, ctx: &mut LayoutCtx) {
        let cs = self.child_layout_style(child);
        let Some(inset) = cs.absolute_inset_of() else {
            let m = self.measure(child, Constraints::loose(content.size), ctx);
            let w = cs
                .clamp_width(resolve_cross(cs.width, m.width, content.size.width))
                .max(0.0);
            let h = cs
                .clamp_height(resolve_cross(cs.height, m.height, content.size.height))
                .max(0.0);
            self.arrange(
                child,
                Rect::from_xywh(
                    content.left() + cs.margin.left,
                    content.top() + cs.margin.top,
                    w,
                    h,
                ),
                ctx,
            );
            return;
        };
        let ml = cs.margin.left;
        let mt = cs.margin.top;
        let mr = cs.margin.right;
        let mb = cs.margin.bottom;
        let avail_w = (content.size.width - ml - mr).max(0.0);
        let avail_h = (content.size.height - mt - mb).max(0.0);
        let m = self.measure(child, Constraints::loose(Size::new(avail_w, avail_h)), ctx);

        let mut w = resolve_cross(cs.width, m.width, avail_w);
        let mut h = resolve_cross(cs.height, m.height, avail_h);
        if let (Some(l), Some(r)) = (inset.left, inset.right)
            && (matches!(cs.width, Length::Fit) || matches!(cs.width, Length::Fill(_)))
        {
            w = (content.size.width - l.max(0.0) - r.max(0.0) - ml - mr).max(0.0);
        }
        if let (Some(t), Some(b)) = (inset.top, inset.bottom)
            && (matches!(cs.height, Length::Fit) || matches!(cs.height, Length::Fill(_)))
        {
            h = (content.size.height - t.max(0.0) - b.max(0.0) - mt - mb).max(0.0);
        }
        w = cs.clamp_width(w).max(0.0);
        h = cs.clamp_height(h).max(0.0);
        let x = match (inset.left, inset.right) {
            (Some(l), _) => content.left() + l + ml,
            (None, Some(r)) => content.right() - r - mr - w,
            (None, None) => content.left() + ml,
        };
        let y = match (inset.top, inset.bottom) {
            (Some(t), _) => content.top() + t + mt,
            (None, Some(b)) => content.bottom() - b - mb - h,
            (None, None) => content.top() + mt,
        };
        self.arrange(child, Rect::from_xywh(x, y, w, h), ctx);
    }

    fn paint_rec(
        &self,
        id: ElementId,
        parent_offset: Vec2,
        parent_scale: f32,
        parent_transform: Transform,
        list: &mut RenderList,
        text: &mut TextSystem,
    ) {
        let depth = list.clip_depth();
        let opacity_depth = list.opacity_depth();
        let transform_depth = list.transform_depth();
        match std::panic::catch_unwind(AssertUnwindSafe(|| {
            self.paint_rec_inner(
                id,
                parent_offset,
                parent_scale,
                parent_transform,
                &mut *list,
                &mut *text,
            )
        })) {
            Ok(()) => {}
            Err(_) => {
                list.truncate_clips(depth);
                list.truncate_opacity(opacity_depth);
                list.truncate_transforms(transform_depth);
                let type_name = self.type_name_of(id).unwrap_or_else(|| "?".into());
                self.record_error(id, type_name, "paint panicked".to_string());
            }
        }
    }

    fn paint_rec_inner(
        &self,
        id: ElementId,
        parent_offset: Vec2,
        parent_scale: f32,
        parent_transform: Transform,
        list: &mut RenderList,
        text: &mut TextSystem,
    ) {
        let Some(v) = self.view_rc(id) else { return };
        let base_rect = {
            let arena = self.inner.arena.borrow();
            arena.get(id).map(|e| e.rect.get()).unwrap_or(Rect::ZERO)
        };

        let own = v.paint_offset_ctx_erased(self, id);
        let acc = Vec2::new(parent_offset.x + own.x, parent_offset.y + own.y);
        let rect = if acc.x == 0.0 && acc.y == 0.0 {
            base_rect
        } else {
            base_rect.translate(acc)
        };

        let scale = {
            let own = v.paint_scale_ctx_erased(self, id);
            let own = if own.is_finite() && own > 0.0 {
                own
            } else {
                1.0
            };
            (parent_scale * own).max(0.0)
        };
        let rect = if (scale - 1.0).abs() > f32::EPSILON {
            let cx = rect.origin.x + rect.size.width * 0.5;
            let cy = rect.origin.y + rect.size.height * 0.5;
            let w = (rect.size.width * scale).max(0.0);
            let h = (rect.size.height * scale).max(0.0);
            Rect::from_xywh(cx - w * 0.5, cy - h * 0.5, w, h)
        } else {
            rect
        };

        let own_rotation = {
            let d = v.paint_rotation_ctx_erased(self, id);
            if d.is_finite() { d } else { 0.0 }
        };
        let own_transform = if own_rotation.abs() > 0.001 {
            Transform::rotate_around_degrees(own_rotation, rect.center())
        } else {
            Transform::IDENTITY
        };
        let total_transform = parent_transform.then(own_transform);
        let pushes_transform = !own_transform.is_identity();
        if pushes_transform {
            list.push_transform(own_transform);
        }

        let paint_bounds = total_transform.apply_rect_aabb(rect);

        let opacity = v
            .paint_opacity_ctx_erased(self, id)
            .map(|o| {
                if o.is_finite() {
                    o.clamp(0.0, 1.0)
                } else {
                    1.0
                }
            })
            .unwrap_or(1.0);
        let pushes_opacity = opacity < 1.0 - f32::EPSILON;
        if pushes_opacity {
            list.push_opacity(opacity);
        }

        const CULL_MARGIN: f32 = 32.0;
        let culled = match list.active_clip() {
            Some(c) => !c.intersects(paint_bounds.inflate(CULL_MARGIN, CULL_MARGIN)),
            None => false,
        };
        if culled {
            if v.clip_self_erased() {
                if pushes_opacity {
                    list.pop_opacity();
                }
                if pushes_transform {
                    list.pop_transform();
                }
                return;
            }
            for c in self.children(id) {
                self.paint_rec(c, acc, scale, total_transform, list, text);
            }
            if pushes_opacity {
                list.pop_opacity();
            }
            if pushes_transform {
                list.pop_transform();
            }
            return;
        }

        let should_clip = v.clip_self_erased()
            || v.layout_style_ctx_erased(self, id).overflow == Overflow::Hidden;
        if should_clip {
            let clip_rect = if pushes_transform { paint_bounds } else { rect };
            let r = v.clip_radius_erased();
            if r > 0.0 {
                list.push_rounded_clip(clip_rect, r);
            } else {
                list.push_clip(clip_rect);
            }
        }

        if let Some(target) = v.surface_target_erased() {
            let clipped = match list.active_clip() {
                Some(c) => c.intersect(rect),
                None => rect,
            };
            if !clipped.is_empty() {
                self.inner.surfaces.borrow_mut().push(SurfaceDraw {
                    rect: clipped,
                    full: rect,
                    target,
                });
            }
        }

        let filter_handle = v
            .foreground_filter_erased()
            .filter(|f| !f.is_empty())
            .and_then(|f| {
                let r = if pushes_transform { paint_bounds } else { rect };
                list.push_filter_begin(r, 0.0, f.blur_px(), f.span_mask())
            });

        let depth = list.clip_depth();

        let (state, _) = self.view_state(id);
        let env = self.inner.environment.borrow().clone();
        {
            let type_name = v.type_name().to_string();

            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                let mut ctx = PaintCtx {
                    list: &mut *list,
                    text: &mut *text,
                    environment: &env,
                    rect,
                    element: id,
                    state,
                    window: self.inner.window.get(),
                    hovered_subtree: self.hover_in_subtree(id),
                    runtime: self,
                    shapes: &self.inner.shapes,
                    draw_offset: Vec2::ZERO,
                    offset_stack: Vec::new(),
                };
                v.paint_self_erased(&mut ctx);
            }));
            if res.is_err() {
                self.record_error(id, type_name, "paint_self panicked".to_string());
            }
        }
        for c in self.children(id) {
            self.paint_rec(c, acc, scale, total_transform, list, text);
        }

        {
            let type_name = v.type_name().to_string();
            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                let mut ctx = PaintCtx {
                    list: &mut *list,
                    text: &mut *text,
                    environment: &env,
                    rect,
                    element: id,
                    state,
                    window: self.inner.window.get(),
                    hovered_subtree: self.hover_in_subtree(id),
                    runtime: self,
                    shapes: &self.inner.shapes,
                    draw_offset: Vec2::ZERO,
                    offset_stack: Vec::new(),
                };
                v.paint_foreground_erased(&mut ctx);
            }));
            if res.is_err() {
                self.record_error(id, type_name, "paint_foreground panicked".to_string());
            }
        }
        if let Some(h) = filter_handle {
            list.push_filter_end(h);
        }

        list.truncate_clips(depth);
        if should_clip {
            list.pop_clip();
        }
        if pushes_opacity {
            list.pop_opacity();
        }
        if pushes_transform {
            list.pop_transform();
        }
    }

    fn view_rc(&self, id: ElementId) -> Option<Rc<dyn ErasedView>> {
        self.inner.arena.borrow().get(id)?.view.clone()
    }

    fn invalidate_measure_chain(&self, id: ElementId) {
        self.inner
            .layout_epoch
            .set(self.inner.layout_epoch.get().wrapping_add(1));
        let mut measures = self.inner.measures.borrow_mut();
        let arena = self.inner.arena.borrow();
        let mut cur = Some(id);
        let mut n = 0u32;
        while let Some(c) = cur {
            if measures.remove(&c).is_some() {
                n += 1;
            }
            cur = arena.get(c).and_then(|e| e.parent);
            if n > 4096 {
                break;
            }
        }
        drop(arena);
        drop(measures);
        if n > 0 {
            let mut s = self.inner.stats.get();
            s.measure_invalidations += u64::from(n);
            self.inner.stats.set(s);
        }
    }

    fn alloc(
        &self,
        value: Rc<dyn ErasedView>,
        parent: Option<ElementId>,
        index: usize,
        key: u64,
    ) -> ElementId {
        let (slot, generation) = { self.inner.arena.borrow_mut().alloc_slot() };
        let id = ElementId {
            index: slot,
            generation,
        };
        let weak: Weak<Inner> = Rc::downgrade(&self.inner);
        let watcher = Watcher::new(move || {
            if let Some(inner) = weak.upgrade() {
                inner.schedule(id);
            }
        });
        {
            let mut arena = self.inner.arena.borrow_mut();
            arena.slots[slot as usize] = Some(Element {
                parent,
                key,
                children: Vec::new(),
                view: Some(value),
                watcher: Some(watcher),
                reads: ReadSet::empty(),
                dirty: Dirty::empty(),
                rect: Cell::new(Rect::ZERO),
            });
            if let Some(p) = parent
                && let Some(pe) = arena.get_mut(p)
            {
                let at = index.min(pe.children.len());
                pe.children.insert(at, id);
            }
        }
        let mut s = self.inner.stats.get();
        s.mounts += 1;
        self.inner.stats.set(s);
        id
    }

    fn build_with(&self, el: ElementId, value: Rc<dyn ErasedView>) {
        self.invalidate_measure_chain(el);
        let v = value;

        let type_name = v.type_name().to_string();
        let fail = |rt: &Self| {
            rt.rebind_reads(el, sequoia_reactive::ReadSet::empty());
            rt.truncate_children(el, 0);
            rt.record_error(el, type_name.clone(), "body panicked".to_string());
        };

        let overridden_res =
            std::panic::catch_unwind(AssertUnwindSafe(|| capture_reads(|| v.children_override())));
        let Ok((overridden, override_reads)) = overridden_res else {
            fail(self);
            return;
        };
        if let Some(children) = overridden {
            let reads = match std::panic::catch_unwind(AssertUnwindSafe(|| {
                capture_reads(|| v.call_body())
            })) {
                Ok((_, body_reads)) => override_reads.union(body_reads),
                Err(_) => override_reads,
            };
            self.rebind_reads(el, reads);
            let list = children
                .into_iter()
                .enumerate()
                .map(|(i, av)| (i as u64, av.into_inner()))
                .collect();
            self.reconcile_keyed(el, list);
            return;
        }

        {
            let keyed_res =
                std::panic::catch_unwind(AssertUnwindSafe(|| capture_reads(|| v.keyed_children())));
            let Ok((keyed, reads)) = keyed_res else {
                fail(self);
                return;
            };
            if let Some(children) = keyed {
                let reads = match std::panic::catch_unwind(AssertUnwindSafe(|| {
                    capture_reads(|| v.call_body())
                })) {
                    Ok((_, body_reads)) => reads.union(body_reads),
                    Err(_) => reads,
                };
                self.rebind_reads(el, reads);
                let list = children
                    .into_iter()
                    .map(|(k, av)| (k, av.into_inner()))
                    .collect();
                self.reconcile_keyed(el, list);
                return;
            }
        }

        let is_leaf = match std::panic::catch_unwind(AssertUnwindSafe(|| v.is_leaf())) {
            Ok(b) => b,
            Err(_) => {
                self.record_error(el, type_name.clone(), "is_leaf panicked".to_string());
                false
            }
        };
        if is_leaf {
            let leaf_res =
                std::panic::catch_unwind(AssertUnwindSafe(|| capture_reads(|| v.call_body())));
            let Ok((_, reads)) = leaf_res else {
                fail(self);
                return;
            };
            self.rebind_reads(el, reads);
            self.truncate_children(el, 0);
            return;
        }

        let body_res =
            std::panic::catch_unwind(AssertUnwindSafe(|| capture_reads(|| v.call_body())));
        let Ok((body, reads)) = body_res else {
            fail(self);
            return;
        };
        self.rebind_reads(el, reads);
        self.reconcile_keyed(el, vec![(0, body.into_inner())]);
    }

    fn reconcile_keyed(&self, parent: ElementId, new: Vec<(u64, Rc<dyn ErasedView>)>) {
        let old: Vec<(u64, ElementId)> = {
            let arena = self.inner.arena.borrow();
            arena
                .get(parent)
                .map(|p| {
                    p.children
                        .iter()
                        .filter_map(|&c| arena.get(c).map(|e| (e.key, c)))
                        .collect()
                })
                .unwrap_or_default()
        };

        let new_keys: HashSet<u64> = new.iter().map(|(k, _)| *k).collect();

        for (k, id) in &old {
            if !new_keys.contains(k) {
                self.unmount_subtree(*id);
            }
        }

        let mut buckets: HashMap<u64, Vec<ElementId>> = HashMap::new();
        for (k, id) in old {
            if new_keys.contains(&k) {
                buckets.entry(k).or_default().push(id);
            }
        }
        for ids in buckets.values_mut() {
            ids.reverse();
        }

        let mut order: Vec<ElementId> = Vec::with_capacity(new.len());
        for (key, view) in new {
            let view = unwrap_any(view);
            let reused = buckets.get_mut(&key).and_then(|ids| ids.pop());
            match reused {
                Some(el) => {
                    let same_type = {
                        let arena = self.inner.arena.borrow();
                        arena
                            .get(el)
                            .and_then(|e| e.view.as_ref())
                            .map(|v| v.view_type())
                            == Some(view.view_type())
                    };
                    if same_type {
                        let old_view = self
                            .inner
                            .arena
                            .borrow()
                            .get(el)
                            .and_then(|e| e.view.clone());
                        if let Some(old) = old_view {
                            view.reuse_state_erased(&*old);
                        }
                        if let Some(e) = self.inner.arena.borrow_mut().get_mut(el) {
                            e.key = key;
                            e.view = Some(view);
                        }
                        self.invalidate_measure_chain(el);
                        if let Some(v) = self.view_rc(el) {
                            self.build_with(el, v);
                        }
                        order.push(el);
                    } else {
                        self.unmount_subtree(el);
                        order.push(self.mount_fresh_detached(view, key));
                    }
                }
                None => order.push(self.mount_fresh_detached(view, key)),
            }
        }

        for (_, leftover) in buckets {
            for id in leftover {
                self.unmount_subtree(id);
            }
        }

        self.invalidate_measure_chain(parent);
        self.set_children(parent, order);
    }

    fn mount_fresh_detached(&self, value: Rc<dyn ErasedView>, key: u64) -> ElementId {
        let id = self.alloc(value, None, 0, key);
        if let Some(v) = self.view_rc(id) {
            self.build_with(id, v);
        }
        if let Some(v) = self.view_rc(id) {
            let type_name = v.type_name().to_string();
            if std::panic::catch_unwind(AssertUnwindSafe(|| {
                v.on_mount();
            }))
            .is_err()
            {
                self.record_error(id, type_name, "on_mount panicked".to_string());
            }
        }
        id
    }

    fn set_children(&self, parent: ElementId, order: Vec<ElementId>) {
        {
            let mut arena = self.inner.arena.borrow_mut();
            for &c in &order {
                if let Some(e) = arena.get_mut(c) {
                    e.parent = Some(parent);
                }
            }
            if let Some(p) = arena.get_mut(parent) {
                p.children = order;
            }
        }
        self.invalidate_measure_chain(parent);
    }

    fn truncate_children(&self, el: ElementId, keep: usize) {
        let doomed: Vec<ElementId> = {
            let arena = self.inner.arena.borrow();
            arena
                .get(el)
                .map(|e| e.children.iter().skip(keep).copied().collect())
                .unwrap_or_default()
        };
        for d in doomed {
            self.unmount_subtree(d);
        }
        if let Some(e) = self.inner.arena.borrow_mut().get_mut(el) {
            e.children.truncate(keep);
        }
    }

    fn rebind_reads(&self, el: ElementId, reads: ReadSet) {
        if let Some(e) = self.inner.arena.borrow_mut().get_mut(el) {
            if let Some(w) = e.watcher.clone() {
                e.reads.unbind(&w);
            }
            e.reads = reads;
            if let Some(w) = e.watcher.clone() {
                e.reads.bind(&w);
            }
        }
    }

    fn unmount_subtree(&self, id: ElementId) {
        self.invalidate_measure_chain(id);
        let children: Vec<ElementId> = {
            let arena = self.inner.arena.borrow();
            arena
                .get(id)
                .map(|e| e.children.clone())
                .unwrap_or_default()
        };
        for c in children {
            self.unmount_subtree(c);
        }
        let (parent, slot, view) = {
            let mut arena = self.inner.arena.borrow_mut();
            let Some(e) = arena.get_mut(id) else { return };
            if let Some(w) = e.watcher.take() {
                e.reads.unbind(&w);
            }
            (e.parent, id.index, e.view.take())
        };
        if let Some(v) = view {
            let type_name = v.type_name().to_string();
            if std::panic::catch_unwind(AssertUnwindSafe(|| {
                v.on_unmount();
            }))
            .is_err()
            {
                self.record_error(id, type_name, "on_unmount panicked".to_string());
            }
        }
        if let Some(p) = parent {
            let mut arena = self.inner.arena.borrow_mut();
            if let Some(pe) = arena.get_mut(p)
                && let Some(pos) = pe.children.iter().position(|&c| c == id)
            {
                pe.children.remove(pos);
            }
        }
        self.inner.measures.borrow_mut().remove(&id);
        self.inner
            .live_anims
            .borrow_mut()
            .retain(|(eid, _), _| *eid != id);

        self.inner.states.borrow_mut().remove(&id);
        if let Some(cancels) = self.inner.scoped_cancels.borrow_mut().remove(&id) {
            for cancel in cancels {
                let _ = std::panic::catch_unwind(AssertUnwindSafe(cancel));
            }
        }
        self.inner.arena.borrow_mut().free_slot(slot);

        let anchored = self.inner.overlays.borrow().anchored_to(id);
        for oid in anchored {
            self.overlay_hide(oid);
        }

        if self
            .inner
            .pending_drag
            .borrow()
            .as_ref()
            .is_some_and(|p| p.source == id)
        {
            self.inner.pending_drag.borrow_mut().take();
        }
        {
            let mut guard = self.inner.active_drag.borrow_mut();
            if let Some(act) = guard.as_mut() {
                if act.source == Some(id) {
                    act.source = None;
                }
                if act.target == Some(id) {
                    act.target = None;
                }
            }
        }
        let mut s = self.inner.stats.get();
        s.unmounts += 1;
        self.inner.stats.set(s);
    }
}

fn shrink_dock_axis(remaining: f32, used: f32) -> f32 {
    if remaining.is_finite() {
        (remaining - used).max(0.0)
    } else {
        remaining
    }
}

fn unwrap_any(mut v: Rc<dyn ErasedView>) -> Rc<dyn ErasedView> {
    for _ in 0..64 {
        let any: Rc<dyn Any> = v.clone().as_any_rc();
        match any.downcast::<AnyView>() {
            Ok(av) => {
                let inner = av.clone_inner();
                if Rc::ptr_eq(&inner, &v) {
                    return v;
                }
                v = inner;
            }
            Err(_) => return v,
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(index: u32) -> ElementId {
        ElementId {
            index,
            generation: 0,
        }
    }

    #[test]
    fn with_state_allows_reentrant_access() {
        let rt = Runtime::new();
        let (a, b) = (id(0), id(1));
        rt.with_state::<u32, _>(a, |x| *x += 1);
        let out = rt.with_state::<u32, _>(a, |x| {
            *x += 10;
            let inner = rt.with_state::<String, _>(b, |s| {
                s.push_str("inner");
                s.clone()
            });
            format!("{x}:{inner}")
        });
        assert_eq!(out, "11:inner");
        assert_eq!(rt.with_state::<u32, _>(a, |x| *x), 11);
        assert_eq!(rt.state_count(), 2);
        assert!(rt.has_state(a) && rt.has_state(b));
    }

    #[test]
    fn with_state_survives_panic_and_keeps_slot() {
        let rt = Runtime::new();
        let a = id(0);
        rt.with_state::<u32, _>(a, |x| *x = 7);
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let r = std::panic::catch_unwind(AssertUnwindSafe(|| {
            rt.with_state::<u32, _>(a, |x| {
                *x = 99;
                panic!("boom");
            });
        }));
        std::panic::set_hook(hook);
        assert!(r.is_err(), "panic went through with_state");
        assert_eq!(
            rt.with_state::<u32, _>(a, |x| *x),
            99,
            "the slot is back after the panic"
        );
        assert_eq!(rt.state_count(), 1);
    }
}
