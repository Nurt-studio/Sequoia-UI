//! Canvas pointer state machine: pan, drag/resize, drawing, marquee and zoom.

mod drag;
mod draw;
mod marquee;
mod pan;

use std::cell::RefCell;
use std::time::Instant;

use sequoia_ui::prelude::*;

use crate::canvas::{ZOOM_MAX, ZOOM_MIN, ZOOM_STEP};
use crate::core::hit_test::{hit_handle, hit_layer};
use crate::core::layer::Layer;
use crate::core::snap::Guides;
use crate::state::EditorState;

use drag::DragSession;
use draw::DrawSession;
use marquee::MarqueeSession;
use pan::PanSession;

/// Signals the canvas interaction operates on.
#[derive(Clone)]
pub(crate) struct CanvasSignals {
    pub(crate) layers: Signal<Vec<Layer>>,
    pub(crate) selected: Signal<Option<u64>>,
    pub(crate) selected_layers: Signal<Vec<u64>>,
    pub(crate) tool: Signal<&'static str>,
    pub(crate) grid: Signal<bool>,
    pub(crate) zoom: Signal<f32>,
    pub(crate) pan_x: Signal<f32>,
    pub(crate) pan_y: Signal<f32>,
    pub(crate) viewport_w: Signal<f32>,
    pub(crate) viewport_h: Signal<f32>,
    pub(crate) repaint: Signal<u64>,
    pub(crate) state: EditorState,
}

impl CanvasSignals {
    pub(crate) fn from_state(state: &EditorState) -> Self {
        Self {
            layers: state.layers.clone(),
            selected: state.selected_layer.clone(),
            selected_layers: state.selected_layers.clone(),
            tool: state.tool.clone(),
            grid: state.grid_on.clone(),
            zoom: state.zoom.clone(),
            pan_x: state.pan_x.clone(),
            pan_y: state.pan_y.clone(),
            viewport_w: state.viewport_w.clone(),
            viewport_h: state.viewport_h.clone(),
            repaint: Signal::new(0),
            state: state.clone(),
        }
    }

    /// Forces a canvas repaint (signal tracked by `track`).
    pub(crate) fn bump(&self) {
        self.repaint.update(|v| *v = v.wrapping_add(1));
    }
}

/// Gesture sessions kept between pointer events.
#[derive(Default)]
pub(crate) struct Interaction {
    pub(crate) drag: RefCell<Option<DragSession>>,
    pub(crate) draw: RefCell<Option<DrawSession>>,
    pub(crate) marquee: RefCell<Option<MarqueeSession>>,
    pan: RefCell<Option<PanSession>>,
    pub(crate) guides: RefCell<Guides>,
}

impl Interaction {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Zooms the view around a canvas point, keeping the position under the cursor.
    pub(crate) fn zoom_at(&self, signals: &CanvasSignals, pos: Point, factor: f32) {
        let zoom = signals.zoom.get_untracked();
        let pan_x = signals.pan_x.get_untracked();
        let pan_y = signals.pan_y.get_untracked();
        let next = (zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        if (next - zoom).abs() <= 1e-4 {
            return;
        }
        let world_x = pos.x / zoom + pan_x;
        let world_y = pos.y / zoom + pan_y;
        signals.pan_x.set(world_x - pos.x / next);
        signals.pan_y.set(world_y - pos.y / next);
        signals.zoom.set(next);
    }

    pub(crate) fn pointer(
        &self,
        signals: &CanvasSignals,
        pos: Point,
        ev: &PointerEvent,
        ctx: &mut EventCtx,
    ) {
        let tool = signals.tool.get_untracked();
        let zoom = signals.zoom.get_untracked();
        let pan_x = signals.pan_x.get_untracked();
        let pan_y = signals.pan_y.get_untracked();
        let wx = pos.x / zoom + pan_x;
        let wy = pos.y / zoom + pan_y;
        let state = &signals.state;

        // ── Present mode (Phase 4): the canvas is live. Clicks and hovers run
        // the behavior interpreter; navigation (wheel, pinch, middle-drag pan)
        // still goes through the normal path below.
        if state.present.on.get_untracked() {
            match ev {
                PointerEvent::Wheel { .. } | PointerEvent::Pinch { .. } => {}
                PointerEvent::Down {
                    button: PointerButton::Middle,
                    ..
                } => {}
                PointerEvent::Down {
                    button: PointerButton::Left,
                    ..
                } => {
                    state.present_click(wx, wy);
                    ctx.handled();
                    return;
                }
                PointerEvent::Down { .. } => {
                    // No context menu while presenting.
                    ctx.handled();
                    return;
                }
                PointerEvent::Move { .. } => {
                    state.present_hover(wx, wy);
                    ctx.handled();
                    return;
                }
                _ => {}
            }
        }

        match ev {
            PointerEvent::Wheel {
                delta, modifiers, ..
            } => {
                let ctrl =
                    modifiers.contains(Modifiers::CONTROL) || modifiers.contains(Modifiers::SUPER);
                if ctrl {
                    let factor = if delta.y > 0.0 {
                        ZOOM_STEP
                    } else {
                        1.0 / ZOOM_STEP
                    };
                    self.zoom_at(signals, pos, factor);
                } else if modifiers.contains(Modifiers::SHIFT) {
                    signals.pan_x.update(|v| *v -= delta.y / zoom);
                } else {
                    // The view follows the fingers, as when dragging the canvas.
                    signals.pan_x.update(|v| *v -= delta.x / zoom);
                    signals.pan_y.update(|v| *v -= delta.y / zoom);
                }
                ctx.handled();
            }

            // Two-finger trackpad pinch (macOS/iOS, Linux via winit).
            PointerEvent::Pinch { delta, .. } => {
                if delta.is_finite() {
                    let factor = (1.0 + *delta).clamp(0.2, 5.0);
                    self.zoom_at(signals, pos, factor);
                }
                ctx.handled();
            }

            PointerEvent::Down {
                button, modifiers, ..
            } => {
                if *button == PointerButton::Middle
                    || (*button == PointerButton::Left && tool == "H")
                {
                    *self.pan.borrow_mut() = Some(PanSession {
                        start_screen_x: pos.x,
                        start_screen_y: pos.y,
                        start_pan_x: pan_x,
                        start_pan_y: pan_y,
                    });
                    ctx.handled();
                    return;
                }

                if *button == PointerButton::Right {
                    if self.drag.borrow().is_some()
                        || self.draw.borrow().is_some()
                        || self.marquee.borrow().is_some()
                    {
                        return;
                    }
                    let layers = signals.layers.get_untracked();
                    let hit = hit_layer(&layers, wx, wy);
                    signals.selected.set(hit);
                    signals
                        .selected_layers
                        .set(hit.map(|id| vec![id]).unwrap_or_default());
                    state.ctx_menu_x.set(pos.x + 272.0);
                    state.ctx_menu_y.set(pos.y + 64.0);
                    state.ctx_menu_open.set(true);
                    ctx.handled();
                    return;
                }

                if *button != PointerButton::Left {
                    return;
                }

                // Clicking away from the text being edited commits the edit;
                // clicking it again keeps the caret where it is.
                if state.editing_text.get_untracked() {
                    let inside = state
                        .selected_layer
                        .get_untracked()
                        .and_then(|id| {
                            signals
                                .layers
                                .get_untracked()
                                .iter()
                                .find(|l| l.id == id)
                                .map(|l| (l.x, l.y, l.w, l.h))
                        })
                        .is_some_and(|(x, y, w, h)| {
                            wx >= x && wx <= x + w && wy >= y && wy <= y + h
                        });
                    if inside {
                        ctx.handled();
                        return;
                    }
                    state.end_text_edit(true);
                }

                let snapping = !modifiers.contains(Modifiers::ALT);
                let grid_snap = snapping && signals.grid.get_untracked();
                *self.guides.borrow_mut() = Guides::default();

                if tool == "V"
                    && let Some(id) = signals.selected.get_untracked()
                    && let Some(l) = signals.layers.get_untracked().iter().find(|l| l.id == id)
                    && !l.locked
                    && let Some(handle) = hit_handle(l, wx, wy, zoom)
                {
                    *self.drag.borrow_mut() = Some(DragSession::resizing(
                        l,
                        handle,
                        wx,
                        wy,
                        signals.state.root.get_untracked().clone(),
                    ));
                    ctx.handled();
                    return;
                }

                if tool == "F" || tool == "R" || tool == "O" {
                    *self.draw.borrow_mut() =
                        Some(DrawSession::begin(signals, tool, wx, wy, grid_snap));
                    ctx.handled();
                    return;
                }

                if tool == "T" {
                    // Clicking an existing text with the T tool edits it in
                    // place; clicking anywhere else creates a new text node.
                    let layers = signals.layers.get_untracked();
                    if let Some(id) = hit_layer(&layers, wx, wy) {
                        let is_text = layers
                            .iter()
                            .find(|l| l.id == id)
                            .map(|l| l.kind == "TEXT")
                            .unwrap_or(false);
                        if is_text {
                            signals.selected.set(Some(id));
                            signals.selected_layers.set(vec![id]);
                            state.start_text_edit(true);
                            ctx.handled();
                            return;
                        }
                    }
                    draw::create_text(signals, wx, wy, grid_snap);
                    ctx.handled();
                    return;
                }

                if tool == "V" {
                    let layers = signals.layers.get_untracked();
                    if let Some(id) = hit_layer(&layers, wx, wy) {
                        signals.selected.set(Some(id));
                        signals.selected_layers.set(vec![id]);
                        let layer = layers.iter().find(|l| l.id == id).unwrap();
                        let now = Instant::now();
                        let last = state.last_canvas_click.clone();
                        let is_dbl = {
                            let c = last.borrow();
                            c.0 == Some(id) && now.duration_since(c.1).as_millis() < 400
                        };
                        if is_dbl && layer.kind == "TEXT" {
                            *last.borrow_mut() = (None, now);
                            state.start_text_edit(true);
                            ctx.handled();
                            return;
                        }
                        *last.borrow_mut() = (Some(id), now);
                        if !layer.locked {
                            *self.drag.borrow_mut() = Some(DragSession::moving(
                                layer,
                                wx,
                                wy,
                                signals.state.root.get_untracked().clone(),
                            ));
                        }
                    } else {
                        signals.selected.set(None);
                        signals.selected_layers.set(Vec::new());
                        *self.marquee.borrow_mut() = Some(MarqueeSession::begin(wx, wy));
                    }
                    ctx.handled();
                }
            }

            PointerEvent::Move { modifiers, .. } => {
                if let Some(session) = self.pan.borrow().as_ref() {
                    session.apply(signals, pos, zoom);
                    ctx.handled();
                    return;
                }

                let snapping = !modifiers.contains(Modifiers::ALT);
                let grid_snap = snapping && signals.grid.get_untracked();

                if let Some(session) = self.drag.borrow().as_ref() {
                    drag::apply(
                        signals,
                        session,
                        wx - session.start_wx,
                        wy - session.start_wy,
                        snapping,
                        grid_snap,
                        &self.guides,
                    );
                    ctx.handled();
                    return;
                }

                if let Some(session) = self.draw.borrow_mut().as_mut() {
                    session.update(wx, wy, grid_snap);
                    signals.bump();
                    ctx.handled();
                    return;
                }

                if let Some(session) = self.marquee.borrow_mut().as_mut() {
                    session.update(wx, wy);
                    signals.bump();
                    ctx.handled();
                }
            }

            PointerEvent::Up { .. } => {
                if self.pan.borrow_mut().take().is_some() {
                    ctx.handled();
                    return;
                }
                if let Some(mut session) = self.drag.borrow_mut().take() {
                    *self.guides.borrow_mut() = Guides::default();
                    // Drag & drop onto a container: dropping a layer over a
                    // different frame reparents it (position is preserved).
                    if matches!(session.mode, drag::DragMode::Move) && session.orig.len() == 1 {
                        let (id, _, _, _, _) = session.orig[0];
                        // The grab point (start_wx/wy), not the layer origin,
                        // decides whether the pointer really moved.
                        state.reparent_on_drop(id, session.start_wx, session.start_wy, wx, wy);
                    }
                    state.push_snapshot(session.before.take());
                    state.propagate_layout();
                    ctx.handled();
                }
                if let Some(session) = self.draw.borrow_mut().take() {
                    session.commit(signals);
                    state.propagate_layout();
                    ctx.handled();
                }
                if let Some(session) = self.marquee.borrow_mut().take() {
                    let ids = session.commit(signals);
                    // Primary first, so the syncing effect leaves `selected_layers`
                    // alone once the full marquee selection is written.
                    signals.selected.set(ids.first().copied());
                    signals.selected_layers.set(ids);
                    ctx.handled();
                }
            }

            _ => {}
        }
    }
}
