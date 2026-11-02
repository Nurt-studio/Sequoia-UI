//! Hit tests for layers and handles.
use crate::core::geometry::{HANDLE_TOL_PX, Handle};
use crate::core::layer::Layer;

pub(crate) fn find_parent_frame(ls: &[Layer], wx: f32, wy: f32) -> Option<u64> {
    let mut found = None;
    for l in ls {
        if l.kind != "FRAME" || !l.visible {
            continue;
        }
        if wx >= l.x && wx <= l.x + l.w && wy >= l.y && wy <= l.y + l.h {
            found = Some(l.id);
        }
    }
    found
}

pub(crate) fn hit_handle(l: &Layer, wx: f32, wy: f32, zoom: f32) -> Option<Handle> {
    let tol = HANDLE_TOL_PX / zoom;
    let items = [
        (Handle::Nw, l.x, l.y),
        (Handle::N, l.x + l.w * 0.5, l.y),
        (Handle::Ne, l.x + l.w, l.y),
        (Handle::E, l.x + l.w, l.y + l.h * 0.5),
        (Handle::Se, l.x + l.w, l.y + l.h),
        (Handle::S, l.x + l.w * 0.5, l.y + l.h),
        (Handle::Sw, l.x, l.y + l.h),
        (Handle::W, l.x, l.y + l.h * 0.5),
    ];
    for (h, hx, hy) in items {
        if (wx - hx).abs() <= tol && (wy - hy).abs() <= tol {
            return Some(h);
        }
    }
    None
}

pub(crate) fn hit_layer(layers: &[Layer], wx: f32, wy: f32) -> Option<u64> {
    for l in layers.iter().rev() {
        if !l.visible {
            continue;
        }
        let inside = wx >= l.x && wx <= l.x + l.w && wy >= l.y && wy <= l.y + l.h;
        if !inside {
            continue;
        }
        if l.kind == "ELLIPSE" {
            let cx = l.x + l.w * 0.5;
            let cy = l.y + l.h * 0.5;
            let nx = (wx - cx) / (l.w * 0.5).max(1.0);
            let ny = (wy - cy) / (l.h * 0.5).max(1.0);
            if nx * nx + ny * ny > 1.0 {
                continue;
            }
        }
        return Some(l.id);
    }
    None
}