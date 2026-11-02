//! Snapping to the edges of other layers (guides).
use crate::core::layer::Layer;

#[derive(Default, Clone)]
pub(crate) struct Guides {
    pub(crate) v: Vec<f32>,
    pub(crate) h: Vec<f32>,
}

pub(crate) fn compute_snap_and_guides(
    moved_ids: &[u64],
    proposed: (f32, f32, f32, f32),
    others: &[Layer],
    threshold: f32,
) -> (f32, f32, Guides) {
    let (bx, by, bw, bh) = proposed;
    let moved_x = [bx, bx + bw * 0.5, bx + bw];
    let moved_y = [by, by + bh * 0.5, by + bh];

    let mut best_dx = 0.0f32;
    let mut best_dx_dist = f32::INFINITY;
    let mut best_dy = 0.0f32;
    let mut best_dy_dist = f32::INFINITY;

    for l in others {
        if !l.visible || moved_ids.contains(&l.id) {
            continue;
        }
        let other_x = [l.x, l.x + l.w * 0.5, l.x + l.w];
        let other_y = [l.y, l.y + l.h * 0.5, l.y + l.h];
        for &mx in &moved_x {
            for &ox in &other_x {
                let d = ox - mx;
                if d.abs() <= threshold && d.abs() < best_dx_dist {
                    best_dx_dist = d.abs();
                    best_dx = d;
                }
            }
        }
        for &my in &moved_y {
            for &oy in &other_y {
                let d = oy - my;
                if d.abs() <= threshold && d.abs() < best_dy_dist {
                    best_dy_dist = d.abs();
                    best_dy = d;
                }
            }
        }
    }

    let final_bx = bx + best_dx;
    let final_by = by + best_dy;
    let final_moved_x = [final_bx, final_bx + bw * 0.5, final_bx + bw];
    let final_moved_y = [final_by, final_by + bh * 0.5, final_by + bh];

    let mut guides = Guides::default();
    let eps = 0.5;
    for l in others {
        if !l.visible || moved_ids.contains(&l.id) {
            continue;
        }
        let other_x = [l.x, l.x + l.w * 0.5, l.x + l.w];
        let other_y = [l.y, l.y + l.h * 0.5, l.y + l.h];
        for &mx in &final_moved_x {
            for &ox in &other_x {
                if (ox - mx).abs() < eps && !guides.v.iter().any(|g| (g - ox).abs() < eps) {
                    guides.v.push(ox);
                }
            }
        }
        for &my in &final_moved_y {
            for &oy in &other_y {
                if (oy - my).abs() < eps && !guides.h.iter().any(|g| (g - oy).abs() < eps) {
                    guides.h.push(oy);
                }
            }
        }
    }
    (best_dx, best_dy, guides)
}