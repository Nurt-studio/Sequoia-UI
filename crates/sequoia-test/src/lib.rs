#![forbid(unsafe_code)]

use std::rc::Rc;

use sequoia_core::{Point, Rect};
use sequoia_input::{ImeEvent, Key, Modifiers, PointerButton};
use sequoia_reactive::{Environment, Signal};
use sequoia_view::Text;
use sequoia_view::prelude::*;
use sequoia_view::{DragPayload, DragSession};
use sequoia_view::{OverlayEntry, OverlayId};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaintStats {
    pub items: usize,
    pub quads: u64,
    pub texts: u64,
    pub glyphs: u64,
    pub images: u64,
    pub shadows: u64,
    pub waveform_bars: u64,
    pub backdrops: usize,
}

impl PaintStats {
    pub fn gpu_instances(&self) -> u64 {
        self.quads + self.glyphs + self.images + self.waveform_bars
    }
}

pub struct Harness {
    rt: Runtime,
}

impl Harness {
    pub fn new(root: impl View + 'static) -> Self {
        let rt = Runtime::new();
        rt.mount(root);
        Self { rt }
    }

    pub fn runtime(&self) -> Runtime {
        self.rt.clone()
    }

    pub fn layout(&self, width: f32, height: f32) {
        let bounds = Rect::from_xywh(0.0, 0.0, width, height);
        let mut text = sequoia_text::TextSystem::new();
        self.rt.layout(bounds, &mut text);
    }

    pub fn paint(&self) -> sequoia_core::RenderList {
        let mut list = sequoia_core::RenderList::new();
        let mut text = sequoia_text::TextSystem::new();
        self.rt.paint(&mut list, &mut text);
        list
    }

    pub fn paint_stats(&self, width: f32, height: f32) -> PaintStats {
        use sequoia_core::RenderItem;
        self.layout(width, height);
        let list = self.paint();
        let mut stats = PaintStats {
            items: list.len(),
            backdrops: list.backdrops().len(),
            ..PaintStats::default()
        };
        for item in list.items() {
            match item {
                RenderItem::Quad(_) => stats.quads += 1,
                RenderItem::Text(t) => {
                    stats.texts += 1;
                    stats.glyphs += t.glyphs.len() as u64;
                }
                RenderItem::Image(_) => stats.images += 1,
                RenderItem::Shadow(_) => stats.shadows += 1,
                RenderItem::Waveform(w) => stats.waveform_bars += w.bars.len() as u64,

                _ => {}
            }
        }
        stats
    }

    pub fn overlay_show(&self, entry: OverlayEntry) -> OverlayId {
        self.rt.overlay_show(entry)
    }

    pub fn overlay_hide(&self, id: OverlayId) -> bool {
        self.rt.overlay_hide(id)
    }

    pub fn overlay_rect(&self, id: OverlayId) -> Option<Rect> {
        self.rt.overlay_rect(id)
    }

    pub fn overlay_ids(&self) -> Vec<OverlayId> {
        self.rt.overlay_ids()
    }

    pub fn overlay_element(&self, id: OverlayId) -> Option<ElementId> {
        self.rt.overlay_element(id)
    }

    pub fn rect(&self, id: ElementId) -> Option<Rect> {
        self.rt.rect_of(id)
    }

    pub fn tick(&self) {
        self.rt.tick();
    }

    pub fn set<T: Clone + PartialEq + 'static>(&self, signal: &Signal<T>, value: T) {
        signal.set(value);
        self.rt.tick();
    }

    pub fn root(&self) -> Option<ElementId> {
        self.rt.root()
    }

    pub fn children(&self, id: ElementId) -> Vec<ElementId> {
        self.rt.children(id)
    }

    pub fn key_of(&self, id: ElementId) -> Option<u64> {
        self.rt.key_of(id)
    }

    pub fn is_alive(&self, id: ElementId) -> bool {
        self.rt.is_alive(id)
    }

    pub fn element_count(&self) -> usize {
        self.rt.element_count()
    }

    pub fn state_count(&self) -> usize {
        self.rt.state_count()
    }

    pub fn scoped_count(&self) -> usize {
        self.rt.scoped_count()
    }

    pub fn stats(&self) -> Stats {
        self.rt.stats()
    }

    pub fn interact(&self) -> InteractState {
        self.rt.interact()
    }

    pub fn type_name_of(&self, id: ElementId) -> Option<String> {
        self.rt.type_name_of(id)
    }

    pub fn pointer_move(&self, x: f32, y: f32) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt.pointer_move(Point::new(x, y), &mut text);
    }

    pub fn pointer_down(&self, x: f32, y: f32) {
        self.pointer_down_at(x, y, PointerButton::Left);
    }

    pub fn pointer_down_at(&self, x: f32, y: f32, button: PointerButton) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt
            .pointer_down(Point::new(x, y), button, Modifiers::NONE, &mut text);
    }

    pub fn pointer_down_mods(&self, x: f32, y: f32, mods: Modifiers) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt
            .pointer_down(Point::new(x, y), PointerButton::Left, mods, &mut text);
    }

    pub fn pointer_up(&self, x: f32, y: f32) {
        self.pointer_up_at(x, y, PointerButton::Left);
    }

    pub fn pointer_up_at(&self, x: f32, y: f32, button: PointerButton) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt
            .pointer_up(Point::new(x, y), button, Modifiers::NONE, &mut text);
    }

    pub fn pointer_up_mods(&self, x: f32, y: f32, mods: Modifiers) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt
            .pointer_up(Point::new(x, y), PointerButton::Left, mods, &mut text);
    }

    pub fn pen_down(&self, x: f32, y: f32, pressure: f32) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt.pen_down(
            Point::new(x, y),
            PointerButton::Left,
            pressure,
            sequoia_core::Vec2::ZERO,
            sequoia_view::PenTool::Pen,
            Modifiers::NONE,
            &mut text,
        );
    }

    pub fn pen_move(&self, x: f32, y: f32, pressure: f32) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt.pen_move(
            Point::new(x, y),
            pressure,
            sequoia_core::Vec2::ZERO,
            sequoia_view::PenTool::Pen,
            &mut text,
        );
    }

    pub fn pen_up(&self, x: f32, y: f32) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt.pen_up(
            Point::new(x, y),
            PointerButton::Left,
            0.0,
            Modifiers::NONE,
            &mut text,
        );
    }

    pub fn touch_down(&self, id: u64, x: f32, y: f32, force: Option<f32>) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt
            .touch_down(id, Point::new(x, y), force, Modifiers::NONE, &mut text);
    }

    pub fn touch_move(&self, id: u64, x: f32, y: f32, force: Option<f32>) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt.touch_move(id, Point::new(x, y), force, &mut text);
    }

    pub fn touch_up(&self, id: u64, x: f32, y: f32) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt
            .touch_up(id, Point::new(x, y), Modifiers::NONE, &mut text);
    }

    pub fn pinch(&self, x: f32, y: f32, delta: f32) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt
            .pinch(Point::new(x, y), delta, Modifiers::NONE, &mut text);
    }

    pub fn pointer_pressure(&self) -> f32 {
        self.rt.pointer_pressure()
    }

    pub fn active_touch_count(&self) -> usize {
        self.rt.active_touches().len()
    }

    pub fn drag(&self, from: (f32, f32), to: (f32, f32)) {
        self.drag_steps(from, to, 5);
    }

    pub fn drag_steps(&self, from: (f32, f32), to: (f32, f32), steps: usize) {
        self.pointer_down(from.0, from.1);
        let steps = steps.max(1);
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            self.pointer_move(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
        }
        self.pointer_up(to.0, to.1);
    }

    pub fn drag_cancel(&self) {
        self.key(Key::ESCAPE);
    }

    pub fn drag_session(&self) -> Option<DragSession> {
        self.rt.current_drag()
    }

    pub fn has_pending_drag(&self) -> bool {
        self.rt.has_pending_drag()
    }

    pub fn drop_target_at(&self, x: f32, y: f32, payload: &DragPayload) -> Option<ElementId> {
        self.rt.drop_target_at(Point::new(x, y), payload)
    }

    pub fn overlay_count(&self) -> usize {
        self.rt.overlay_count()
    }

    pub fn cursor_icon(&self) -> sequoia_input::CursorIcon {
        self.rt.cursor_icon()
    }

    pub fn click(&self, x: f32, y: f32) {
        self.pointer_down(x, y);
        self.pointer_up(x, y);
    }

    pub fn key(&self, key: Key) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt.key(key, Modifiers::NONE, &mut text);
    }

    pub fn key_with_shift(&self, key: Key) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt.key(key, Modifiers::SHIFT, &mut text);
    }

    pub fn set_environment(&self, env: Environment) {
        self.rt.set_environment(env);
    }

    pub fn key_with_modifiers(&self, key: Key, mods: Modifiers) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt.key(key, mods, &mut text);
    }

    pub fn wheel(&self, x: f32, y: f32, dy: f32) {
        self.wheel_xy(x, y, 0.0, dy);
    }

    pub fn wheel_xy(&self, x: f32, y: f32, dx: f32, dy: f32) {
        self.wheel_with_modifiers(x, y, dx, dy, Modifiers::NONE);
    }

    pub fn wheel_with_modifiers(&self, x: f32, y: f32, dx: f32, dy: f32, mods: Modifiers) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt.wheel(
            Point::new(x, y),
            sequoia_core::Vec2::new(dx, dy),
            mods,
            &mut text,
        );
    }

    pub fn ime(&self, event: ImeEvent) {
        let mut text = sequoia_text::TextSystem::new();
        self.rt.ime(event, &mut text);
    }

    pub fn ime_preedit(&self, value: &str, cursor: Option<(usize, usize)>) {
        self.ime(ImeEvent::Preedit {
            value: value.to_owned(),
            cursor,
        });
    }

    pub fn ime_commit(&self, value: &str) {
        self.ime(ImeEvent::Commit(value.to_owned()));
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<ElementId> {
        self.rt.hit_test(Point::new(x, y))
    }

    pub fn sync(&self, width: f32, height: f32) {
        self.tick();
        self.layout(width, height);
        self.paint();
        self.tick();
    }

    pub fn dump(&self) -> String {
        let mut out = String::new();
        if let Some(root) = self.rt.root() {
            self.dump_rec(root, 0, &mut out);
        }
        out
    }

    fn dump_rec(&self, id: ElementId, depth: usize, out: &mut String) {
        let name = self.rt.type_name_of(id).unwrap_or_else(|| "?".into());
        out.push_str(&"  ".repeat(depth));
        out.push_str(&name);
        if let Some(r) = self.rt.rect_of(id) {
            out.push_str(&format!(
                "  l={:.0} t={:.0} w={:.0} h={:.0}",
                r.left(),
                r.top(),
                r.size.width,
                r.size.height
            ));
        }
        out.push_str(&format!("  ({id:?})\n"));
        for c in self.rt.children(id) {
            self.dump_rec(c, depth + 1, out);
        }
    }

    pub fn view<T: View + 'static>(&self, id: ElementId) -> Option<Rc<T>> {
        self.rt.view::<T>(id)
    }

    pub fn text(&self, id: ElementId) -> Option<String> {
        self.rt.view::<Text>(id).map(|t| t.content())
    }
}
