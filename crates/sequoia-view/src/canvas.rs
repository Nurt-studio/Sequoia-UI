#![forbid(unsafe_code)]

use std::rc::Rc;

use sequoia_core::{Color, Fill, Length, Point, Rect, Size, Vec2};
use sequoia_input::{
    CursorIcon, Event, EventPhase, Key, KeyEvent, Modifiers, PointerButton, PointerEvent,
};
use sequoia_layout::LayoutStyle;

use crate::painter::Painter;
use crate::view::{EventCtx, PaintCtx, View};

#[derive(Clone)]
pub struct KeyBinding {
    pub key: Key,

    pub modifiers: Modifiers,
    action: Rc<dyn Fn()>,
}

impl KeyBinding {
    pub fn new(key: Key, modifiers: Modifiers, action: impl Fn() + 'static) -> Self {
        Self {
            key,
            modifiers,
            action: Rc::new(action),
        }
    }

    pub fn fire(&self) {
        (self.action)();
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub pos: Vec2,
    pub zoom: f32,
}

impl Camera {
    pub fn new(pos: Vec2, zoom: f32) -> Self {
        let zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        };
        Self { pos, zoom }
    }

    pub fn identity() -> Self {
        Self {
            pos: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

#[derive(Clone)]
pub struct Canvas {
    background: Option<Fill>,
    focusable: bool,
    capturable: bool,
    cursor: CursorIcon,
    style: LayoutStyle,
    draw: Rc<dyn Fn(&mut Painter)>,
    on_pointer: Option<Rc<dyn Fn(Point, &PointerEvent, &mut EventCtx)>>,
    bindings: Vec<KeyBinding>,
    on_key: Option<Rc<dyn Fn(&KeyEvent) -> bool>>,
    track: Option<Rc<dyn Fn()>>,
    clip: bool,

    overflow: f32,

    grid: Option<(f32, Color)>,

    axes: Option<Color>,

    snap: Option<f32>,

    camera: Option<Camera>,
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            background: None,
            focusable: true,
            capturable: true,
            cursor: CursorIcon::Default,
            style: LayoutStyle {
                width: Length::Fill(1.0),
                height: Length::Fill(1.0),
                ..LayoutStyle::default()
            },
            draw: Rc::new(|_| {}),
            on_pointer: None,
            bindings: Vec::new(),
            on_key: None,
            track: None,
            clip: true,
            overflow: 0.0,
            grid: None,
            axes: None,
            snap: None,
            camera: None,
        }
    }

    pub fn background(mut self, fill: impl Into<Fill>) -> Self {
        self.background = Some(fill.into());
        self
    }

    pub fn no_background(mut self) -> Self {
        self.background = None;
        self
    }

    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    pub fn capture(mut self, capturable: bool) -> Self {
        self.capturable = capturable;
        self
    }

    pub fn cursor(mut self, cursor: CursorIcon) -> Self {
        self.cursor = cursor;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.style = self.style.width(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.style = self.style.height(height.into());
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.style = self.style.min_width(v);
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.style = self.style.max_width(v);
        self
    }

    pub fn min_height(mut self, v: f32) -> Self {
        self.style = self.style.min_height(v);
        self
    }

    pub fn max_height(mut self, v: f32) -> Self {
        self.style = self.style.max_height(v);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.style = self.style.flex(weight);
        self
    }

    pub fn expand(self) -> Self {
        self.flex(1.0)
    }

    pub fn on_draw(mut self, draw: impl Fn(&mut Painter) + 'static) -> Self {
        self.draw = Rc::new(draw);
        self
    }

    pub fn on_pointer(
        mut self,
        handler: impl Fn(Point, &PointerEvent, &mut EventCtx) + 'static,
    ) -> Self {
        self.on_pointer = Some(Rc::new(handler));
        self
    }

    pub fn key(mut self, key: Key, modifiers: Modifiers, action: impl Fn() + 'static) -> Self {
        self.bindings.push(KeyBinding::new(key, modifiers, action));
        self
    }

    pub fn on_key(mut self, handler: impl Fn(&KeyEvent) -> bool + 'static) -> Self {
        self.on_key = Some(Rc::new(handler));
        self
    }

    pub fn track(mut self, track: impl Fn() + 'static) -> Self {
        self.track = Some(Rc::new(track));
        self
    }

    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    pub fn allow_overflow(mut self, px: f32) -> Self {
        self.overflow = if px.is_finite() { px.max(0.0) } else { 0.0 };
        self
    }

    pub fn overflow_px(self, px: f32) -> Self {
        self.allow_overflow(px)
    }

    pub fn paint_overflow(&self) -> f32 {
        self.overflow
    }

    pub fn grid(mut self, step: f32) -> Self {
        self.grid = Some((step, Color::rgba(0.5, 0.5, 0.5, 0.08)));
        self
    }

    pub fn grid_colored(mut self, step: f32, color: Color) -> Self {
        self.grid = Some((step, color));
        self
    }

    pub fn axes(mut self) -> Self {
        self.axes = Some(Color::rgba(0.5, 0.5, 0.5, 0.25));
        self
    }

    pub fn axes_colored(mut self, color: Color) -> Self {
        self.axes = Some(color);
        self
    }

    pub fn snap_every(mut self, step: f32) -> Self {
        self.snap = Some(step);
        self
    }

    pub fn snap_step(&self) -> Option<f32> {
        self.snap
    }

    pub fn snap_value(&self, value: f32) -> f32 {
        match self.snap {
            Some(step) => Self::snap(value, step),
            None => value,
        }
    }

    pub fn camera(mut self, pos: Vec2, zoom: f32) -> Self {
        self.camera = Some(Camera::new(pos, zoom));
        self
    }

    pub fn no_camera(mut self) -> Self {
        self.camera = None;
        self
    }

    pub fn camera_value(&self) -> Option<Camera> {
        self.camera
    }

    pub fn background_fill(&self) -> Option<&Fill> {
        self.background.as_ref()
    }

    pub fn is_focusable(&self) -> bool {
        self.focusable
    }

    pub fn cursor_icon_value(&self) -> CursorIcon {
        self.cursor
    }

    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    pub fn world_to_local(world: Point, cam: Vec2, zoom: f32) -> Point {
        let zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        };
        Point::new((world.x - cam.x) * zoom, (world.y - cam.y) * zoom)
    }

    pub fn local_to_world(local: Point, cam: Vec2, zoom: f32) -> Point {
        let zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        };
        Point::new(cam.x + local.x / zoom, cam.y + local.y / zoom)
    }

    pub fn view_bounds(size: Size, cam: Vec2, zoom: f32) -> Rect {
        let zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        };
        Rect::from_xywh(cam.x, cam.y, size.width / zoom, size.height / zoom)
    }

    pub fn fit_view(content: Rect, size: Size, padding: f32) -> (Vec2, f32) {
        if size.width <= 0.0 || size.height <= 0.0 {
            return (Vec2::new(content.left(), content.top()), 1.0);
        }
        let pad = padding.max(0.0);
        let cw = (content.size.width + pad * 2.0).max(1.0);
        let ch = (content.size.height + pad * 2.0).max(1.0);
        let zoom = (size.width / cw).min(size.height / ch).clamp(0.01, 100.0);
        let cx = content.left() + content.size.width * 0.5;
        let cy = content.top() + content.size.height * 0.5;
        let cam = Vec2::new(
            cx - size.width / (2.0 * zoom),
            cy - size.height / (2.0 * zoom),
        );
        (cam, zoom)
    }

    pub fn snap(value: f32, step: f32) -> f32 {
        if !(step > 0.0) || !value.is_finite() {
            return value;
        }
        (value / step).round() * step
    }

    pub fn snap_point(p: Point, step: f32) -> Point {
        Point::new(Self::snap(p.x, step), Self::snap(p.y, step))
    }

    pub fn lod_banded(zoom: f32, breaks: &[f32]) -> usize {
        if !(zoom > 0.0) {
            return breaks.len();
        }
        for (i, b) in breaks.iter().enumerate() {
            if zoom >= *b {
                return i;
            }
        }
        breaks.len()
    }

    pub fn show_detail(zoom: f32, world: f32, min_px: f32) -> bool {
        if !(zoom > 0.0) || !(world > 0.0) || !(min_px > 0.0) {
            return false;
        }
        world * zoom >= min_px
    }

    pub fn detail_alpha(zoom: f32, world: f32, min_px: f32, fade_px: f32) -> f32 {
        if !(zoom > 0.0) || !(world > 0.0) || !(min_px > 0.0) {
            return 0.0;
        }
        let px = world * zoom;
        if px <= min_px {
            return 0.0;
        }
        if !(fade_px > 0.0) {
            return 1.0;
        }
        ((px - min_px) / fade_px).clamp(0.0, 1.0)
    }
}

impl View for Canvas {
    fn body(&self) -> impl View + 'static {
        if let Some(track) = &self.track {
            track();
        }
        crate::views::Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn focusable(&self) -> bool {
        self.focusable
    }

    fn captures_pointer(&self) -> bool {
        self.capturable
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn cursor_icon(&self, _state: sequoia_style::VisualState) -> sequoia_input::CursorIcon {
        self.cursor
    }

    fn layout_style(&self) -> LayoutStyle {
        self.style
    }

    fn clip_self(&self) -> bool {
        self.clip && self.overflow <= 0.0
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase != EventPhase::Target {
            return;
        }
        match event {
            Event::Pointer(ev) => {
                if self.focusable
                    && let PointerEvent::Down {
                        button: PointerButton::Left,
                        ..
                    } = ev
                {
                    ctx.request_focus();
                }
                if let Some(handler) = &self.on_pointer {
                    let screen = match *ev {
                        PointerEvent::Down { pos, .. }
                        | PointerEvent::Move { pos, .. }
                        | PointerEvent::Up { pos, .. }
                        | PointerEvent::Wheel { pos, .. }
                        | PointerEvent::PenMove { pos, .. }
                        | PointerEvent::PenDown { pos, .. }
                        | PointerEvent::PenUp { pos, .. }
                        | PointerEvent::TouchDown { pos, .. }
                        | PointerEvent::TouchMove { pos, .. }
                        | PointerEvent::TouchUp { pos, .. }
                        | PointerEvent::Pinch { pos, .. } => pos,

                        _ => ctx.pos,
                    };

                    let mut local =
                        Point::new(screen.x - ctx.rect.left(), screen.y - ctx.rect.top());
                    if let Some(step) = self.snap {
                        local = Self::snap_point(local, step);
                    }
                    handler(local, ev, ctx);
                }
            }
            Event::Key(key) => {
                for b in &self.bindings {
                    if b.key == key.key && b.modifiers == key.modifiers {
                        b.fire();
                        ctx.handled();
                        return;
                    }
                }
                if let Some(handler) = &self.on_key
                    && handler(key)
                {
                    ctx.handled();
                }
            }
            Event::Ime(_) => {}
            Event::FilesHovered { .. } | Event::FilesDropped { .. } | Event::FilesCancelled => {}

            _ => {}
        }
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        if self.clip && self.overflow > 0.0 && !ctx.rect.is_empty() {
            ctx.push_clip(ctx.rect.inflate(self.overflow, self.overflow));
        }
        let mut g = Painter::new(ctx);
        if let Some(bg) = &self.background {
            let rect = g.rect();
            g.fill(rect, bg.clone());
        }

        if let Some((step, color)) = self.grid {
            match self.camera {
                Some(cam) => {
                    let s = step * cam.zoom;
                    let off = Vec2::new(-cam.pos.x * cam.zoom, -cam.pos.y * cam.zoom);
                    g.grid_offset(s, off, color);
                }
                None => {
                    g.grid(step, color);
                }
            }
        }
        if let Some(color) = self.axes {
            let origin = match self.camera {
                Some(cam) => Self::world_to_local(Point::ZERO, cam.pos, cam.zoom),
                None => Point::new(g.size().width * 0.5, g.size().height * 0.5),
            };
            g.axes(origin, color, 1.0);
        }
        (self.draw)(&mut g);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vp(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_xywh(x, y, w, h)
    }

    #[test]
    fn overflow_disables_exact_clip() {
        use crate::view::View;
        assert_eq!(Canvas::new().paint_overflow(), 0.0);
        assert!(Canvas::new().clip_self());
        let c = Canvas::new().allow_overflow(7.0);
        assert!((c.paint_overflow() - 7.0).abs() < 1e-6);
        assert!(!c.clip_self(), "z overflow clip idzie w paint_self");
        assert!(Canvas::new().allow_overflow(f32::NAN).clip_self());
        assert!(Canvas::new().allow_overflow(-5.0).clip_self());
        assert!(!Canvas::new().clip(false).clip_self());
    }

    #[test]
    fn camera_roundtrip_local() {
        let cam = Vec2::new(100.0, 200.0);
        let world = Point::new(150.0, 260.0);
        let local = Canvas::world_to_local(world, cam, 2.0);
        assert_eq!(local, Point::new(100.0, 120.0));
        assert_eq!(Canvas::local_to_world(local, cam, 2.0), world);
        assert_eq!(
            Canvas::local_to_world(local, cam, 0.0),
            Point::new(cam.x + local.x, cam.y + local.y)
        );
    }

    #[test]
    fn view_bounds_matches_camera() {
        let size = Size::new(200.0, 100.0);
        let cam = Vec2::new(0.0, 0.0);
        let world = Canvas::view_bounds(size, cam, 2.0);
        assert_eq!(world.size, Size::new(100.0, 50.0));
        assert!((world.left() - cam.x).abs() < 1e-4);
        assert!((world.top() - cam.y).abs() < 1e-4);
        let flat = Canvas::view_bounds(size, cam, 0.0);
        assert!((flat.size.width - 200.0).abs() < 1e-4);
    }

    #[test]
    fn fit_view_centers_content() {
        let content = vp(0.0, 0.0, 400.0, 200.0);
        let size = Size::new(800.0, 600.0);
        let (cam, zoom) = Canvas::fit_view(content, size, 0.0);
        assert_eq!(zoom, 2.0);
        let center = Canvas::world_to_local(Point::new(200.0, 100.0), cam, zoom);
        assert_eq!(center, Point::new(400.0, 300.0));
        assert_eq!(
            Canvas::fit_view(content, Size::ZERO, 0.0),
            (Vec2::new(0.0, 0.0), 1.0)
        );
    }

    #[test]
    fn snap_rounds_to_grid() {
        assert_eq!(Canvas::snap(13.0, 10.0), 10.0);
        assert_eq!(Canvas::snap(17.0, 10.0), 20.0);
        assert_eq!(Canvas::snap(-3.0, 10.0), 0.0);
        assert_eq!(Canvas::snap(13.0, 0.0), 13.0);
        assert_eq!(Canvas::snap(13.0, -5.0), 13.0);
        let p = Canvas::snap_point(Point::new(13.0, 27.0), 10.0);
        assert_eq!(p, Point::new(10.0, 30.0));
    }

    #[test]
    fn lod_bands_from_full_to_coarse() {
        let breaks = [1.0, 0.55, 0.3];
        assert_eq!(Canvas::lod_banded(2.0, &breaks), 0);
        assert_eq!(Canvas::lod_banded(1.0, &breaks), 0);
        assert_eq!(Canvas::lod_banded(0.7, &breaks), 1);
        assert_eq!(Canvas::lod_banded(0.55, &breaks), 1);
        assert_eq!(Canvas::lod_banded(0.4, &breaks), 2);
        assert_eq!(Canvas::lod_banded(0.1, &breaks), 3);
        assert_eq!(Canvas::lod_banded(0.1, &[]), 0);
        assert_eq!(Canvas::lod_banded(0.0, &breaks), 3);
        assert_eq!(Canvas::lod_banded(-1.0, &breaks), 3);
        assert_eq!(Canvas::lod_banded(f32::NAN, &breaks), 3);
    }

    #[test]
    fn show_detail_hides_subpixel_stuff() {
        assert!(Canvas::show_detail(1.0, 100.0, 6.0));
        assert!(Canvas::show_detail(0.06, 100.0, 6.0));
        assert!(!Canvas::show_detail(0.05, 100.0, 6.0));
        assert!(!Canvas::show_detail(1.0, 100.0, 0.0));
        assert!(!Canvas::show_detail(0.0, 100.0, 6.0));
        assert!(!Canvas::show_detail(f32::NAN, 100.0, 6.0));
    }

    #[test]
    fn detail_alpha_ramps_without_popping() {
        assert_eq!(Canvas::detail_alpha(0.05, 100.0, 6.0, 4.0), 0.0);
        assert_eq!(Canvas::detail_alpha(0.08, 100.0, 6.0, 4.0), 0.5);
        assert_eq!(Canvas::detail_alpha(1.0, 100.0, 6.0, 4.0), 1.0);
        assert_eq!(Canvas::detail_alpha(1.0, 100.0, 6.0, 0.0), 1.0);
        assert_eq!(Canvas::detail_alpha(0.0, 100.0, 6.0, 4.0), 0.0);
    }

    #[test]
    fn default_cursor_is_default() {
        assert_eq!(
            Canvas::new().cursor_icon_value(),
            sequoia_input::CursorIcon::Default
        );
        assert_eq!(
            Canvas::new()
                .cursor(sequoia_input::CursorIcon::Crosshair)
                .cursor_icon_value(),
            sequoia_input::CursorIcon::Crosshair
        );
    }
}
