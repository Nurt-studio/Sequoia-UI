use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use sequoia_ui::prelude::*;
use sequoia_ui::view::Painter;

use crate::files;
use crate::layout::{self, DialogLayout, Hit, Layout};
use crate::model::Dialog;
use crate::model::*;
use crate::raster;
use crate::text;
use crate::ui;

pub struct PaintApp {
    state: Rc<RefCell<State>>,
}

impl PaintApp {
    pub fn new() -> Self {
        Self::with_state(Rc::new(RefCell::new(State::new())))
    }

    pub fn with_state(state: Rc<RefCell<State>>) -> Self {
        Self { state }
    }

    pub fn state(&self) -> Rc<RefCell<State>> {
        self.state.clone()
    }
}

impl Default for PaintApp {
    fn default() -> Self {
        Self::new()
    }
}

impl View for PaintApp {
    fn body(&self) -> impl View + 'static {
        self.state.borrow().rev.track();
        Unit
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn focusable(&self) -> bool {
        true
    }

    fn captures_pointer(&self) -> bool {
        true
    }

    fn wants_hover_updates(&self) -> bool {
        true
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle::default()
            .width(Length::Fill(1.0))
            .height(Length::Fill(1.0))
    }

    fn cursor_icon(&self, _state: VisualState) -> CursorIcon {
        CursorIcon::Crosshair
    }

    fn paint_self(&self, ctx: &mut PaintCtx) {
        let state = self.state.borrow();
        let l = layout::compute(&state, ctx.rect.size);
        let mut g = Painter::new(ctx);
        ui::paint(&mut g, &state, &l, state.mouse);
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase != EventPhase::Target {
            return;
        }
        let local = ctx.local_pos();
        match event {
            Event::Pointer(ev) => self.pointer(ev, local, ctx),
            Event::Key(key) => {
                if self.key(key, ctx.rect.size) {
                    ctx.handled();
                }
            }
            Event::Ime(ime) => {
                if self.ime(ime) {
                    ctx.handled();
                }
            }
            Event::FilesDropped { paths, .. } => {
                if let Some(path) = paths.first() {
                    let path = path.clone();
                    let mut s = self.state.borrow_mut();
                    files::open_into(&mut s, path);
                    s.bump();
                }
            }
            _ => {}
        }
    }
}

impl PaintApp {
    fn pointer(&self, ev: &PointerEvent, local: Point, ctx: &mut EventCtx) {
        let size = ctx.rect.size;
        let l = {
            let s = self.state.borrow();
            layout::compute(&s, size)
        };
        match ev {
            PointerEvent::Down { button, pos, .. } => {
                let _ = pos;
                ctx.request_focus();
                ctx.capture_pointer();
                self.pointer_down(local, *button, &l, ctx);
            }
            PointerEvent::Move { .. } => {
                self.pointer_move(local, &l, ctx);
            }
            PointerEvent::Up { .. } => {
                self.pointer_up(local, &l, ctx);
            }
            PointerEvent::Wheel {
                delta, modifiers, ..
            } => {
                self.pointer_wheel(local, *delta, *modifiers, &l);
            }
            PointerEvent::Cancel => {
                let mut s = self.state.borrow_mut();
                s.drag = Drag::None;
                s.mouse_down = false;
                s.bump();
            }
            PointerEvent::PenDown { button, pos, .. } => {
                let _ = pos;
                ctx.request_focus();
                ctx.capture_pointer();
                self.pointer_down(local, *button, &l, ctx);
            }
            PointerEvent::PenMove { .. } => {
                self.pointer_move(local, &l, ctx);
            }
            PointerEvent::PenUp { .. } => {
                self.pointer_up(local, &l, ctx);
            }
            PointerEvent::TouchDown { .. } => {
                ctx.request_focus();
                ctx.capture_pointer();
                self.pointer_down(local, PointerButton::Left, &l, ctx);
            }
            PointerEvent::TouchMove { .. } => {
                self.pointer_move(local, &l, ctx);
            }
            PointerEvent::TouchUp { .. } => {
                self.pointer_up(local, &l, ctx);
            }
            _ => {}
        }
    }

    fn pointer_down(&self, local: Point, button: PointerButton, l: &Layout, ctx: &mut EventCtx) {
        let size = l.size;
        if self.state.borrow().dialog.is_some() {
            let dl = {
                let s = self.state.borrow();
                layout::dialog_layout(&s, size)
            };
            if let Some(dl) = dl {
                let hit = {
                    let s = self.state.borrow();
                    dl.hit_at(&s, local)
                };
                if let Some(hit) = hit {
                    self.dialog_click(hit, &dl, local);
                }
            }
            return;
        }
        let windows = ctx.environment.get::<Windows>();
        let hit = {
            let s = self.state.borrow();
            l.hit_at(&s, local)
        };
        match hit {
            Hit::CaptionClose => {
                if let Some(windows) = windows {
                    windows.close(ctx.window);
                }
            }
            Hit::CaptionMinimize => {
                if let Some(windows) = windows {
                    windows.minimize(ctx.window);
                }
            }
            Hit::CaptionMaximize => {
                if let Some(windows) = windows {
                    windows.toggle_maximize(ctx.window);
                }
            }
            Hit::Qat => {
                if let Some(windows) = windows {
                    windows.begin_drag(ctx.window);
                }
            }
            Hit::BackstageClose => {
                let mut s = self.state.borrow_mut();
                s.backstage = false;
                s.bump();
            }
            Hit::BackstageItem(action) => {
                let mut s = self.state.borrow_mut();
                s.backstage = false;
                drop(s);
                perform(&self.state, action, size);
            }
            Hit::FileTab => {
                let mut s = self.state.borrow_mut();
                s.backstage = true;
                s.bump();
            }
            Hit::Tab(tab) => {
                let mut s = self.state.borrow_mut();
                s.backstage = false;
                s.tab = tab;
                s.bump();
            }
            Hit::Control(action) => perform(&self.state, action, size),
            Hit::ZoomOut => perform(&self.state, Action::ZoomOut, size),
            Hit::ZoomIn => perform(&self.state, Action::ZoomIn, size),
            Hit::ZoomSlider => {
                let idx = layout::zoom_from_x(l, local.x);
                let mut s = self.state.borrow_mut();
                s.zoom_idx = idx;
                s.drag = Drag::ZoomSlider;
                s.bump();
            }
            Hit::Scroll(axis) => {
                let bar = match axis {
                    layout::ScrollAxis::Vertical => l.vscroll,
                    layout::ScrollAxis::Horizontal => l.hscroll,
                };
                if let Some(bar) = bar {
                    let mut s = self.state.borrow_mut();
                    let grab = scroll_grab(&s, l, bar, axis, local);
                    s.drag = match axis {
                        layout::ScrollAxis::Vertical => Drag::ScrollV { grab },
                        layout::ScrollAxis::Horizontal => Drag::ScrollH { grab },
                    };
                    apply_scroll_drag(&mut s, l, axis, local, grab);
                }
            }
            Hit::Canvas => self.canvas_down(local, button, l),
            Hit::None => {}
        }
    }

    fn canvas_down(&self, local: Point, button: PointerButton, l: &Layout) {
        let doc = screen_to_doc(l, &self.state.borrow(), local);
        let tool = self.state.borrow().tool;
        if tool == Tool::Text {
            let hit = {
                let s = self.state.borrow();
                s.text.as_ref().and_then(|t| {
                    let transform = layout::CanvasTransform::new(l, &s);
                    let zoom = transform.zoom;
                    let size = (t.size * zoom).clamp(1.0, 1600.0);
                    let m = text::metrics(&t.text, size);
                    let p = transform.to_screen(t.pos);
                    let center = text::box_rect(p, &m).center();
                    let handle = text::handle_point(center, m.box_h, t.angle_deg);
                    (local.distance(handle) <= text::HANDLE_HIT).then_some((center, t.angle_deg))
                })
            };
            if let Some((center, base)) = hit {
                let mut s = self.state.borrow_mut();
                s.drag = Drag::TextRotate {
                    center,
                    start: (local.y - center.y).atan2(local.x - center.x),
                    base,
                };
                s.bump();
                return;
            }
        }
        let mut s = self.state.borrow_mut();
        if s.text.is_some() && tool != Tool::Text {
            text::commit(&mut s);
        }
        let pix = (doc.x.floor() as i32, doc.y.floor() as i32);
        match tool {
            Tool::Pencil | Tool::Brush | Tool::Eraser => {
                s.clear_selection_committing();
                s.text = None;
                s.push_undo();
                s.drag = Drag::Draw { last: doc, button };
                let (width, brush, color) = if tool == Tool::Pencil {
                    (1.0, BrushKind::Brush, s.stroke_color(button))
                } else if tool == Tool::Eraser {
                    ((s.width() * 2.0).max(8.0), BrushKind::Brush, s.color2)
                } else {
                    (s.width(), s.brush, s.stroke_color(button))
                };
                raster::stamp(&mut s.doc, doc.x, doc.y, width, color, brush);
                s.touch_doc();
            }
            Tool::Fill => {
                s.clear_selection_committing();
                s.text = None;
                s.push_undo();
                let color = s.stroke_color(button);
                raster::flood_fill(&mut s.doc, pix.0, pix.1, color);
                s.touch_doc();
            }
            Tool::Picker => {
                if let Some(c) = s.doc.get(pix.0, pix.1) {
                    if button == PointerButton::Right {
                        s.color2 = c;
                    } else {
                        s.color1 = c;
                    }
                    s.set_status("Color picked");
                }
            }
            Tool::Magnifier => {
                if button == PointerButton::Right {
                    s.zoom_idx = s.zoom_idx.saturating_sub(1);
                } else {
                    s.zoom_idx = (s.zoom_idx + 1).min(ZOOMS.len() - 1);
                }
                s.scroll = Vec2::ZERO;
                s.bump();
            }
            Tool::Text => {
                if s.text.is_some() {
                    text::commit(&mut s);
                }
                s.clear_selection_committing();
                s.text = Some(TextEditor {
                    text: String::new(),
                    pos: doc,
                    size: 24.0,
                    angle_deg: 0.0,
                });
                s.bump();
            }
            Tool::Shape => {
                s.clear_selection_committing();
                s.text = None;
                s.drag = Drag::Shape {
                    start: doc,
                    current: doc,
                    button,
                };
                s.bump();
            }
            Tool::Select => {
                let inside = s
                    .selection
                    .as_ref()
                    .map(|sel| sel.rect.contains(doc))
                    .unwrap_or(false);
                if inside {
                    let sel = s.selection.as_ref().unwrap();
                    let origin = sel.rect;
                    let content = sel
                        .floating
                        .clone()
                        .unwrap_or_else(|| s.doc.crop(origin.as_rect()));
                    let lifted = sel.floating.is_some();
                    s.drag = Drag::MoveSelection {
                        start: doc,
                        lifted,
                        content,
                        origin,
                        undo_pushed: false,
                    };
                    s.bump();
                } else {
                    s.clear_selection_committing();
                    s.drag = Drag::Select { start: doc };
                    s.selection = None;
                    s.bump();
                }
            }
        }
    }

    fn pointer_move(&self, local: Point, l: &Layout, _ctx: &mut EventCtx) {
        let mut s = self.state.borrow_mut();
        s.mouse = local;
        let doc = layout::CanvasTransform::new(l, &s).to_doc(local);
        if l.viewport.contains(local) {
            s.pointer = doc;
        }
        let drag = std::mem::replace(&mut s.drag, Drag::None);
        match drag {
            Drag::Draw { last, button } => {
                if s.doc.width() > 0 && s.doc.height() > 0 {
                    let (width, brush, color) = if s.tool == Tool::Pencil {
                        (1.0, BrushKind::Brush, s.stroke_color(button))
                    } else if s.tool == Tool::Eraser {
                        ((s.width() * 2.0).max(8.0), BrushKind::Brush, s.color2)
                    } else {
                        (s.width(), s.brush, s.stroke_color(button))
                    };
                    let clamped = Point::new(
                        doc.x.clamp(0.0, s.doc.width() as f32 - 1.0),
                        doc.y.clamp(0.0, s.doc.height() as f32 - 1.0),
                    );
                    raster::stroke_segment(&mut s.doc, last, clamped, width, color, brush);
                    s.touch_doc();
                    s.drag = Drag::Draw {
                        last: clamped,
                        button,
                    };
                } else {
                    s.drag = drag;
                }
            }
            Drag::Shape {
                start,
                button,
                current: _,
            } => {
                let clamped = Point::new(
                    doc.x.clamp(0.0, s.doc.width() as f32),
                    doc.y.clamp(0.0, s.doc.height() as f32),
                );
                s.pointer = clamped;
                s.drag = Drag::Shape {
                    start,
                    current: clamped,
                    button,
                };
                s.bump();
            }
            Drag::Select { start } => {
                s.pointer = doc;
                s.drag = Drag::Select { start };
                s.bump();
            }
            Drag::MoveSelection {
                start,
                mut lifted,
                content,
                origin,
                mut undo_pushed,
            } => {
                let dx = doc.x - start.x;
                let dy = doc.y - start.y;
                let moved = dx.abs() + dy.abs() > 0.5;
                let mut new_rect = origin;
                new_rect.x = (origin.x + dx.round() as i32)
                    .clamp(0, (s.doc.width() as i32 - origin.w).max(0));
                new_rect.y = (origin.y + dy.round() as i32)
                    .clamp(0, (s.doc.height() as i32 - origin.h).max(0));
                if moved {
                    if !lifted {
                        if !undo_pushed {
                            let fill = s.color2;
                            s.push_undo();
                            s.doc.fill_rect(origin.as_rect(), fill);
                            s.touch_doc();
                            undo_pushed = true;
                        }
                        lifted = true;
                    }
                    s.selection = Some(Selection {
                        rect: new_rect,
                        floating: Some(content.clone()),
                    });
                    s.bump();
                }
                s.drag = Drag::MoveSelection {
                    start,
                    lifted,
                    content,
                    origin,
                    undo_pushed,
                };
            }
            Drag::TextRotate {
                center,
                start,
                base,
            } => {
                let now = (local.y - center.y).atan2(local.x - center.x);
                let deg = base + (now - start).to_degrees();
                if let Some(t) = s.text.as_mut() {
                    t.angle_deg = text::normalize_angle(deg);
                }
                s.drag = Drag::TextRotate {
                    center,
                    start,
                    base,
                };
                s.bump();
            }
            Drag::Pan { start, scroll } => {
                let dx = local.x - start.x;
                let dy = local.y - start.y;
                s.scroll.x = (scroll.x - dx).clamp(0.0, l.max_scroll.x);
                s.scroll.y = (scroll.y - dy).clamp(0.0, l.max_scroll.y);
                s.bump();
                s.drag = Drag::Pan { start, scroll };
            }
            Drag::ScrollH { grab } => {
                apply_scroll_drag(&mut s, l, layout::ScrollAxis::Horizontal, local, grab);
                s.drag = Drag::ScrollH { grab };
            }
            Drag::ScrollV { grab } => {
                apply_scroll_drag(&mut s, l, layout::ScrollAxis::Vertical, local, grab);
                s.drag = Drag::ScrollV { grab };
            }
            Drag::ZoomSlider => {
                s.zoom_idx = layout::zoom_from_x(l, local.x);
                s.bump();
                s.drag = Drag::ZoomSlider;
            }
            Drag::DialogSlider(ch) => {
                let dl = layout::dialog_layout(&s, l.size);
                if let Some(dl) = dl {
                    if let Some((_, track)) = dl.sliders.iter().find(|(c, _)| *c == ch) {
                        let v = ((local.x - track.left()) / track.size.width).clamp(0.0, 1.0);
                        set_editor_channel(&mut s, ch, v);
                    }
                }
                s.drag = Drag::DialogSlider(ch);
            }
            Drag::None => {
                s.drag = Drag::None;
                s.bump();
            }
        }
    }

    fn pointer_up(&self, local: Point, _l: &Layout, ctx: &mut EventCtx) {
        let mut s = self.state.borrow_mut();
        let drag = std::mem::replace(&mut s.drag, Drag::None);
        match drag {
            Drag::Shape {
                start,
                current,
                button,
            } => {
                let (_, closed) = raster::shape_points(s.shape, start, current);
                let dist = (current.x - start.x).abs() + (current.y - start.y).abs();
                if dist > 1.0 && (closed || dist > 2.0) {
                    let shape = s.shape;
                    let fill = s.fill;
                    let outline = s.stroke_color(button);
                    let fill_color = s.color2;
                    let width = s.width();
                    s.push_undo();
                    raster::raster_shape(
                        &mut s.doc, shape, start, current, fill, outline, fill_color, width,
                    );
                    s.touch_doc();
                }
            }
            Drag::Select { start } => {
                let rect =
                    IRect::from_points(start, s.pointer).clamp(s.doc.width(), s.doc.height());
                if rect.w > 2 && rect.h > 2 {
                    s.selection = Some(Selection {
                        rect,
                        floating: None,
                    });
                    s.set_status("Area selected");
                } else {
                    s.selection = None;
                }
                s.bump();
            }
            Drag::MoveSelection {
                lifted,
                undo_pushed,
                ..
            } => {
                if lifted {
                    if !undo_pushed {
                        s.push_undo();
                    }
                    s.commit_floating();
                    s.set_status("Selection moved");
                }
                s.bump();
            }
            _ => {
                s.bump();
            }
        }
        let _ = local;
        s.mouse_down = false;
        ctx.release_pointer();
    }

    fn pointer_wheel(&self, local: Point, delta: Vec2, modifiers: Modifiers, l: &Layout) {
        let mut s = self.state.borrow_mut();
        if !l.viewport.contains(local) {
            return;
        }
        if modifiers.contains(Modifiers::CONTROL) {
            if delta.y > 0.0 {
                s.zoom_idx = (s.zoom_idx + 1).min(ZOOMS.len() - 1);
            } else if delta.y < 0.0 {
                s.zoom_idx = s.zoom_idx.saturating_sub(1);
            }
            s.bump();
            return;
        }
        if modifiers.contains(Modifiers::SHIFT) {
            s.scroll.x = (s.scroll.x - delta.y).clamp(0.0, l.max_scroll.x);
        } else {
            s.scroll.y = (s.scroll.y - delta.y).clamp(0.0, l.max_scroll.y);
            s.scroll.x = (s.scroll.x - delta.x).clamp(0.0, l.max_scroll.x);
        }
        s.bump();
    }

    fn key(&self, key: &KeyEvent, size: Size) -> bool {
        let ctrl = key.modifiers.contains(Modifiers::CONTROL);
        let alt = key.modifiers.contains(Modifiers::ALT);
        let shift = key.modifiers.contains(Modifiers::SHIFT);
        let mut s = self.state.borrow_mut();
        if matches!(s.dialog, Some(Dialog::Resize(_))) {
            let mut changed = false;
            let mut close = false;
            let mut accept = false;
            if let Some(Dialog::Resize(r)) = s.dialog.as_mut() {
                match key.key {
                    Key::BACKSPACE => {
                        let field = if r.focus == 0 {
                            &mut r.width
                        } else {
                            &mut r.height
                        };
                        field.pop();
                        changed = true;
                    }
                    Key::TAB => {
                        r.focus = (r.focus + 1) % 2;
                        changed = true;
                    }
                    Key::ENTER => accept = true,
                    Key::ESCAPE => close = true,
                    _ => {
                        if let Some(c) = key.key.char() {
                            if c.is_ascii_digit() {
                                let field = if r.focus == 0 {
                                    &mut r.width
                                } else {
                                    &mut r.height
                                };
                                if field.len() < 4 {
                                    field.push(c);
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
            if changed {
                s.bump();
            }
            if close {
                s.dialog = None;
                s.pending = None;
                s.bump();
            }
            if accept {
                let dl = layout::dialog_layout(&s, size);
                drop(s);
                if let Some(dl) = dl {
                    self.dialog_click(DialogHit::Ok, &dl, dl.ok.center());
                }
            }
            return true;
        }
        if s.dialog.is_some() {
            match key.key {
                Key::ESCAPE => {
                    s.dialog = None;
                    s.pending = None;
                    s.bump();
                }
                Key::ENTER => {
                    let dl = layout::dialog_layout(&s, size);
                    let Some(dl) = dl else { return true };
                    let hit = dl.hit_at(&s, dl.ok.center()).unwrap_or(DialogHit::Ok);
                    drop(s);
                    self.dialog_click(hit, &dl, dl.ok.center());
                }
                _ => {}
            }
            return true;
        }
        if s.text.is_some() {
            if alt {
                let step = if shift { 1.0 } else { 5.0 };
                let delta = match key.key {
                    Key::ARROW_LEFT => -step,
                    Key::ARROW_RIGHT => step,
                    Key::ARROW_UP => 15.0,
                    Key::ARROW_DOWN => -15.0,
                    _ => 0.0,
                };
                if delta != 0.0 {
                    if let Some(t) = s.text.as_mut() {
                        t.angle_deg = text::normalize_angle(t.angle_deg + delta);
                    }
                    s.bump();
                    return true;
                }
            }
            if ctrl {
                return false;
            }
            let mut changed = false;
            let mut commit = false;
            let mut cancel = false;
            {
                let editor = s.text.as_mut().unwrap();
                match key.key {
                    Key::BACKSPACE => {
                        editor.text.pop();
                        changed = true;
                    }
                    Key::ENTER => commit = true,
                    Key::ESCAPE => cancel = true,
                    Key::SPACE => {
                        editor.text.push(' ');
                        changed = true;
                    }
                    _ => {
                        if let Some(c) = key.key.char() {
                            if !c.is_control() {
                                editor.text.push(c);
                                changed = true;
                            }
                        }
                    }
                }
            }
            if changed {
                s.bump();
            }
            if cancel {
                s.text = None;
                s.bump();
            }
            if commit {
                text::commit(&mut s);
            }
            return changed || commit || cancel;
        }
        match key.key {
            Key::DELETE => {
                if let Some(sel) = s.selection.take() {
                    let fill = s.color2;
                    s.push_undo();
                    s.doc.fill_rect(sel.rect.as_rect(), fill);
                    s.touch_doc();
                }
                true
            }
            Key::ESCAPE => {
                s.clear_selection_committing();
                s.text = None;
                s.bump();
                true
            }
            Key::F11 => {
                s.view.fullscreen = !s.view.fullscreen;
                s.bump();
                true
            }
            _ => false,
        }
    }

    fn ime(&self, ime: &ImeEvent) -> bool {
        if let ImeEvent::Commit(text) = ime {
            let mut s = self.state.borrow_mut();
            if let Some(editor) = s.text.as_mut() {
                editor.text.push_str(text);
                s.bump();
                return true;
            }
        }
        false
    }

    fn dialog_click(&self, hit: DialogHit, dl: &DialogLayout, local: Point) {
        let mut s = self.state.borrow_mut();
        match hit {
            DialogHit::Cancel => {
                s.dialog = None;
                s.pending = None;
                s.bump();
            }
            DialogHit::Ok => {
                let dialog = s.dialog.clone();
                match dialog {
                    Some(Dialog::Resize(r)) => {
                        let w = r.width.trim().parse::<u32>().ok();
                        let h = r.height.trim().parse::<u32>().ok();
                        match (w, h) {
                            (Some(w), Some(h))
                                if (1..=8192).contains(&w) && (1..=8192).contains(&h) =>
                            {
                                s.push_undo();
                                s.doc = raster::resize_canvas(&s.doc, w, h);
                                s.dirty = true;
                                s.selection = None;
                                s.text = None;
                                s.dialog = None;
                                s.pending = None;
                                s.set_status("Image resized");
                            }
                            _ => {
                                s.set_status("Enter dimensions from 1 to 8192");
                            }
                        }
                    }
                    Some(Dialog::EditColors(e)) => {
                        let color = Color::rgb(e.r, e.g, e.b);
                        if let Some(idx) = e.target {
                            if idx < s.palette.len() {
                                s.palette[idx] = color;
                            }
                        }
                        s.color1 = color;
                        s.dialog = None;
                        s.bump();
                    }
                    Some(Dialog::Properties) | Some(Dialog::About) => {
                        s.dialog = None;
                        s.bump();
                    }
                    Some(Dialog::ConfirmNew) => {
                        self.confirm_save_or_discard(&mut s, true);
                    }
                    None => {}
                }
            }
            DialogHit::Field(i) => {
                if let Some(Dialog::Resize(r)) = s.dialog.as_mut() {
                    r.focus = i.min(1);
                    s.bump();
                }
            }
            DialogHit::Palette(i) => {
                let chosen = s.palette.get(i).copied();
                if let (Some(c), Some(Dialog::EditColors(e))) = (chosen, s.dialog.as_mut()) {
                    e.target = Some(i);
                    e.r = c.r;
                    e.g = c.g;
                    e.b = c.b;
                    s.bump();
                }
            }
            DialogHit::Preset(i) => {
                if let Some(Dialog::EditColors(e)) = s.dialog.as_mut() {
                    if let Some(c) = CUSTOM_COLORS.get(i).copied() {
                        e.r = c.r;
                        e.g = c.g;
                        e.b = c.b;
                        s.bump();
                    }
                }
            }
            DialogHit::Slider(ch) => {
                if let Some(Dialog::EditColors(e)) = s.dialog.as_mut() {
                    if let Some((_, track)) = dl.sliders.iter().find(|(c, _)| *c == ch) {
                        let v = ((local.x - track.left()) / track.size.width).clamp(0.0, 1.0);
                        match ch {
                            SliderChannel::R => e.r = v,
                            SliderChannel::G => e.g = v,
                            SliderChannel::B => e.b = v,
                        }
                        e.slider = Some(ch);
                        s.drag = Drag::DialogSlider(ch);
                        s.bump();
                    }
                }
            }
            DialogHit::ConfirmSave => {
                self.confirm_save_or_discard(&mut s, true);
            }
            DialogHit::ConfirmDontSave => {
                self.confirm_save_or_discard(&mut s, false);
            }
        }
    }

    fn confirm_save_or_discard(&self, s: &mut State, save: bool) {
        let pending = s.pending.take();
        s.dialog = None;
        if save {
            if !files::save_current(s, false) {
                return;
            }
        }
        if matches!(pending, Some(Action::New)) {
            new_document(s);
        } else {
            std::process::exit(0);
        }
    }
}

fn set_editor_channel(s: &mut State, ch: SliderChannel, v: f32) {
    if let Some(Dialog::EditColors(e)) = s.dialog.as_mut() {
        match ch {
            SliderChannel::R => e.r = v,
            SliderChannel::G => e.g = v,
            SliderChannel::B => e.b = v,
        }
        e.slider = Some(ch);
        s.bump();
    }
}

fn screen_to_doc(l: &Layout, s: &State, p: Point) -> Point {
    layout::CanvasTransform::new(l, s).to_doc(p)
}

fn scroll_grab(s: &State, l: &Layout, bar: Rect, axis: layout::ScrollAxis, local: Point) -> f32 {
    let thumb = thumb_of(s, l, bar, axis);
    let vertical = axis == layout::ScrollAxis::Vertical;
    if thumb.contains(local) {
        if vertical {
            local.y - thumb.top()
        } else {
            local.x - thumb.left()
        }
    } else if vertical {
        thumb.size.height * 0.5
    } else {
        thumb.size.width * 0.5
    }
}

fn thumb_of(s: &State, l: &Layout, bar: Rect, axis: layout::ScrollAxis) -> Rect {
    layout::scroll_thumb(bar, axis, layout::scroll_metrics(s, l, axis))
}

fn apply_scroll_drag(s: &mut State, l: &Layout, axis: layout::ScrollAxis, local: Point, grab: f32) {
    let vertical = axis == layout::ScrollAxis::Vertical;
    let bar = if vertical { l.vscroll } else { l.hscroll };
    let Some(bar) = bar else { return };
    let metrics = layout::scroll_metrics(s, l, axis);
    let len = if vertical {
        bar.size.height
    } else {
        bar.size.width
    };
    let tlen = metrics.thumb_len(len);
    let range = (len - tlen).max(1.0);
    let pos = if vertical {
        local.y - grab - bar.top()
    } else {
        local.x - grab - bar.left()
    };
    let t = (pos / range).clamp(0.0, 1.0);
    let offset = t * metrics.max_offset();
    if vertical {
        s.scroll.y = offset;
    } else {
        s.scroll.x = offset;
    }
    s.bump();
}

fn new_document(s: &mut State) {
    s.doc = Pixels::filled(DEFAULT_DOC_W, DEFAULT_DOC_H, Color::WHITE);
    s.path = None;
    s.dirty = false;
    s.undo.clear();
    s.redo.clear();
    s.selection = None;
    s.clipboard = None;
    s.text = None;
    s.tool = Tool::Pencil;
    s.zoom_idx = 3;
    s.scroll = Vec2::ZERO;
    s.dialog = None;
    s.pending = None;
    s.backstage = false;
    s.set_status("New image");
}

pub fn perform(state: &Rc<RefCell<State>>, action: Action, size: Size) {
    let mut s = state.borrow_mut();
    match action {
        Action::Save => {
            files::save_current(&mut s, false);
        }
        Action::SaveAs => {
            files::save_current(&mut s, true);
        }
        Action::Open => {
            s.clear_selection_committing();
            if let Some(path) = files::choose_open() {
                files::open_into(&mut s, path);
            }
        }
        Action::New => {
            if s.dirty {
                s.pending = Some(Action::New);
                s.dialog = Some(Dialog::ConfirmNew);
                s.bump();
            } else {
                new_document(&mut s);
            }
        }
        Action::Undo => {
            s.text = None;
            s.selection = None;
            s.undo();
        }
        Action::Redo => {
            s.text = None;
            s.selection = None;
            s.redo();
        }
        Action::Paste => {
            if let Some(clip) = s.clipboard.clone() {
                s.commit_floating();
                let x = (s.scroll.x / s.zoom()).round() as i32 + 10;
                let y = (s.scroll.y / s.zoom()).round() as i32 + 10;
                let rect = IRect {
                    x,
                    y,
                    w: clip.width() as i32,
                    h: clip.height() as i32,
                };
                s.selection = Some(Selection {
                    rect,
                    floating: Some(clip),
                });
                s.tool = Tool::Select;
                s.set_status("Image pasted");
            }
        }
        Action::Cut => {
            if let Some(sel) = s.selection.clone() {
                let pix = sel
                    .floating
                    .clone()
                    .unwrap_or_else(|| s.doc.crop(sel.rect.as_rect()));
                s.clipboard = Some(pix);
                let fill = s.color2;
                s.push_undo();
                s.doc.fill_rect(sel.rect.as_rect(), fill);
                s.selection = None;
                s.touch_doc();
                s.set_status("Selection cut");
            }
        }
        Action::Copy => {
            if let Some(sel) = s.selection.as_ref() {
                let pix = sel
                    .floating
                    .clone()
                    .unwrap_or_else(|| s.doc.crop(sel.rect.as_rect()));
                s.clipboard = Some(pix);
                s.set_status("Selection copied");
            }
        }
        Action::SelectTool => {
            if s.tool == Tool::Text {
                text::commit(&mut s);
            }
            s.tool = Tool::Select;
            s.bump();
        }
        Action::SelectAll => {
            s.clear_selection_committing();
            s.tool = Tool::Select;
            s.selection = Some(Selection {
                rect: IRect {
                    x: 0,
                    y: 0,
                    w: s.doc.width() as i32,
                    h: s.doc.height() as i32,
                },
                floating: None,
            });
            s.set_status("Entire image selected");
        }
        Action::Crop => {
            s.commit_floating();
            if let Some(sel) = s.selection.take() {
                let rect = sel.rect.clamp(s.doc.width(), s.doc.height());
                if rect.w > 0 && rect.h > 0 {
                    s.push_undo();
                    s.doc = s.doc.crop(rect.as_rect());
                    s.dirty = true;
                    s.set_status("Image cropped");
                }
            }
        }
        Action::ResizeDialog => {
            s.text = None;
            s.dialog = Some(Dialog::Resize(ResizeDialog {
                width: s.doc.width().to_string(),
                height: s.doc.height().to_string(),
                focus: 0,
            }));
            s.bump();
        }
        Action::RotateLeft => rotate(&mut s, false),
        Action::RotateRight => rotate(&mut s, true),
        Action::FlipH => flip(&mut s, true),
        Action::FlipV => flip(&mut s, false),
        Action::SetTool(tool) => {
            if s.tool == Tool::Text && tool != Tool::Text {
                text::commit(&mut s);
            }
            if tool != Tool::Select {
                s.clear_selection_committing();
            }
            s.tool = tool;
            s.bump();
        }
        Action::SetBrush(kind) => {
            s.brush = kind;
            s.tool = Tool::Brush;
            s.bump();
        }
        Action::SetShape(kind) => {
            s.shape = kind;
            s.tool = Tool::Shape;
            s.bump();
        }
        Action::SetFill(mode) => {
            s.fill = mode;
            s.bump();
        }
        Action::SetWidth(i) => {
            s.width_idx = i.min(LINE_WIDTHS.len() - 1);
            s.bump();
        }
        Action::SetColor1 => {
            s.bump();
        }
        Action::SetColor2 => {
            s.bump();
        }
        Action::Palette(i) => {
            if let Some(c) = s.palette.get(i).copied() {
                s.color1 = c;
                s.set_status("Color picked");
            }
        }
        Action::EditColors => {
            s.dialog = Some(Dialog::EditColors(ColorEditor {
                r: s.color1.r,
                g: s.color1.g,
                b: s.color1.b,
                target: None,
                slider: None,
            }));
            s.bump();
        }
        Action::CycleBrush => {
            let index = BrushKind::ALL
                .iter()
                .position(|kind| *kind == s.brush)
                .unwrap_or(0);
            s.brush = BrushKind::ALL[(index + 1) % BrushKind::ALL.len()];
            s.tool = Tool::Brush;
            s.bump();
        }
        Action::CycleWidth => {
            s.width_idx = (s.width_idx + 1) % LINE_WIDTHS.len();
            s.bump();
        }
        Action::CycleFill => {
            s.fill = match s.fill {
                FillMode::Outline => FillMode::Filled,
                FillMode::Filled => FillMode::Both,
                FillMode::Both => FillMode::Outline,
            };
            s.bump();
        }
        Action::None => {}
        Action::ZoomIn => {
            s.zoom_idx = (s.zoom_idx + 1).min(ZOOMS.len() - 1);
            s.bump();
        }
        Action::ZoomOut => {
            s.zoom_idx = s.zoom_idx.saturating_sub(1);
            s.bump();
        }
        Action::Zoom100 => {
            s.zoom_idx = 3;
            s.scroll = Vec2::ZERO;
            s.bump();
        }
        Action::FitWindow => {
            let avail_w = (size.width - 60.0).max(100.0);
            let avail_h = (size.height - 220.0).max(100.0);
            let fit = (avail_w / s.doc.width() as f32).min(avail_h / s.doc.height() as f32);
            let pct = (fit * 100.0).clamp(12.5, 800.0);
            let mut best = 0;
            let mut best_d = f32::INFINITY;
            for (i, z) in ZOOMS.iter().enumerate() {
                let d = (z - pct).abs();
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            s.zoom_idx = best;
            s.scroll = Vec2::ZERO;
            s.bump();
        }
        Action::ToggleRulers => {
            s.view.rulers = !s.view.rulers;
            s.bump();
        }
        Action::ToggleGrid => {
            s.view.grid = !s.view.grid;
            s.bump();
        }
        Action::ToggleStatus => {
            s.view.status = !s.view.status;
            s.bump();
        }
        Action::ToggleThumbnail => {
            s.view.thumbnail = !s.view.thumbnail;
            s.bump();
        }
        Action::ToggleFullscreen => {
            s.view.fullscreen = !s.view.fullscreen;
            s.bump();
        }
        Action::Print => {
            s.set_status("Printing is not available in this version");
        }
        Action::SetWallpaper => {
            s.set_status("Set as desktop background (demo)");
        }
        Action::Properties => {
            s.dialog = Some(Dialog::Properties);
            s.bump();
        }
        Action::About => {
            s.dialog = Some(Dialog::About);
            s.bump();
        }
        Action::CloseApp => {
            if s.dirty {
                s.pending = Some(Action::CloseApp);
                s.dialog = Some(Dialog::ConfirmNew);
                s.bump();
            } else {
                std::process::exit(0);
            }
        }
        Action::ConfirmSave | Action::ConfirmDontSave | Action::ConfirmCancel => {}
    }
    drop(s);
    let s = state.borrow_mut();
    s.bump();
}

fn rotate(s: &mut State, cw: bool) {
    s.commit_floating();
    s.text = None;
    s.push_undo();
    s.doc = raster::rotate_90(&s.doc, cw);
    s.selection = None;
    s.scroll = Vec2::ZERO;
    s.touch_doc();
    s.set_status(if cw { "Rotated right" } else { "Rotated left" });
}

fn flip(s: &mut State, horizontal: bool) {
    s.commit_floating();
    s.text = None;
    s.push_undo();
    s.doc = raster::flip(&s.doc, horizontal);
    s.selection = None;
    s.touch_doc();
    s.set_status(if horizontal {
        "Flipped horizontally"
    } else {
        "Flipped vertically"
    });
}

pub fn open_dropped(state: &Rc<RefCell<State>>, path: PathBuf) {
    let mut s = state.borrow_mut();
    files::open_into(&mut s, path);
}
