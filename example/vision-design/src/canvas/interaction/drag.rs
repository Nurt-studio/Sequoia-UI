use std::cell::RefCell;

use super::CanvasSignals;
use crate::canvas::GUIDE_TOL_PX;
use crate::core::geometry::{Handle, apply_resize, snap};
use crate::core::layer::Layer;
use crate::core::snap::{Guides, compute_snap_and_guides};
use crate::model::svelte_ast::SvelteNode;

#[derive(Clone)]
pub(crate) enum DragMode {
    Move,
    Resize(Handle),
}

/// Dragging the selection: moving or resizing with a handle.
pub(crate) struct DragSession {
    pub(crate) mode: DragMode,
    pub(crate) start_wx: f32,
    pub(crate) start_wy: f32,
    pub(crate) orig: Vec<(u64, f32, f32, f32, f32)>,
    pub(crate) before: Option<SvelteNode>,
}

impl DragSession {
    pub(crate) fn moving(layer: &Layer, wx: f32, wy: f32, before: Option<SvelteNode>) -> Self {
        Self {
            mode: DragMode::Move,
            start_wx: wx,
            start_wy: wy,
            orig: vec![(layer.id, layer.x, layer.y, layer.w, layer.h)],
            before,
        }
    }

    pub(crate) fn resizing(
        layer: &Layer,
        handle: Handle,
        wx: f32,
        wy: f32,
        before: Option<SvelteNode>,
    ) -> Self {
        Self {
            mode: DragMode::Resize(handle),
            start_wx: wx,
            start_wy: wy,
            orig: vec![(layer.id, layer.x, layer.y, layer.w, layer.h)],
            before,
        }
    }
}

/// Applies the cursor delta to the session, updating layers and guides.
pub(crate) fn apply(
    signals: &CanvasSignals,
    session: &DragSession,
    dx: f32,
    dy: f32,
    snapping: bool,
    grid_snap: bool,
    guides: &RefCell<Guides>,
) {
    let zoom = signals.zoom.get_untracked();
    let orig = &session.orig;

    match &session.mode {
        DragMode::Move if orig.len() == 1 => {
            let (id, ox, oy, ow, oh) = orig[0];
            let nx_raw = ox + dx;
            let ny_raw = oy + dy;
            let (snap_dx, snap_dy, found) = if snapping {
                signals.layers.with_untracked(|ls| {
                    compute_snap_and_guides(
                        &[id],
                        (nx_raw, ny_raw, ow, oh),
                        ls,
                        GUIDE_TOL_PX / zoom,
                    )
                })
            } else {
                (0.0, 0.0, Guides::default())
            };
            let final_x = if snap_dx.abs() > 0.01 {
                nx_raw + snap_dx
            } else if grid_snap {
                snap(nx_raw)
            } else {
                nx_raw
            };
            let final_y = if snap_dy.abs() > 0.01 {
                ny_raw + snap_dy
            } else if grid_snap {
                snap(ny_raw)
            } else {
                ny_raw
            };
            *guides.borrow_mut() = found;
            signals.state.move_layer(id, final_x, final_y);
            signals.bump();
        }
        DragMode::Move => {
            *guides.borrow_mut() = Guides::default();
            for (id, ox, oy, _, _) in orig {
                let nx = ox + dx;
                let ny = oy + dy;
                signals.state.move_layer(*id, if grid_snap { snap(nx) } else { nx }, if grid_snap { snap(ny) } else { ny });
            }
        }
        DragMode::Resize(handle) => {
            *guides.borrow_mut() = Guides::default();
            for (id, ox, oy, ow, oh) in orig {
                let (nx, ny, nw, nh) = apply_resize(*handle, *ox, *oy, *ow, *oh, dx, dy, grid_snap);
                signals.state.resize_layer(*id, nx, ny, nw, nh);
            }
        }
    }
}