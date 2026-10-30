use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use sequoia_ui::prelude::*;
use sequoia_ui::view::{BrushStroke, Fill, Painter};

use crate::colors;
use crate::state::{EditorState, Layer};

const GRID_SIZE: f32 = 24.0;
const GRID_COLOR: Color = Color::hexa(0xFFFFFF09);
const SEL_COLOR: Color = Color::hex(0x0D99FF);
const SEL_FILL: Color = Color::hexa(0x0D99FF29);
const GUIDE_COLOR: Color = Color::hex(0xFF3B30);
const HANDLE_PX: f32 = 7.0;
const HANDLE_TOL_PX: f32 = 6.0;
const ZOOM_MIN: f32 = 0.10;
const ZOOM_MAX: f32 = 8.0;
const ZOOM_STEP: f32 = 1.15;
const GUIDE_TOL_PX: f32 = 5.0;

fn snap(v: f32) -> f32 {
    (v / GRID_SIZE).round() * GRID_SIZE
}

fn find_parent_frame(ls: &[Layer], wx: f32, wy: f32) -> Option<u64> {
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

#[derive(Default, Clone)]
struct Guides {
    v: Vec<f32>,
    h: Vec<f32>,
}

fn compute_snap_and_guides(
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

fn draw_grid_screen(p: &mut Painter, pan_x: f32, pan_y: f32, zoom: f32) {
    let size = p.size();
    let step = GRID_SIZE * zoom;
    if step < 8.0 {
        return;
    }
    let stroke = BrushStroke::new(1.0);
    let off_x = (-pan_x * zoom).rem_euclid(step);
    let off_y = (-pan_y * zoom).rem_euclid(step);
    let mut x = off_x;
    while x <= size.width {
        p.line(
            Point::new(x, 0.0),
            Point::new(x, size.height),
            stroke,
            GRID_COLOR,
        );
        x += step;
    }
    let mut y = off_y;
    while y <= size.height {
        p.line(
            Point::new(0.0, y),
            Point::new(size.width, y),
            stroke,
            GRID_COLOR,
        );
        y += step;
    }
}

fn draw_layer(p: &mut Painter, l: &Layer) {
    if !l.visible || l.opacity <= 0.0 {
        return;
    }
    let rect = Rect::from_xywh(l.x, l.y, l.w, l.h);
    let fill = Fill::solid(l.fill);
    match l.kind {
        "FRAME" | "RECTANGLE" => {
            p.rounded(rect, l.radius, fill);
            if l.stroke_width > 0.0 {
                p.rect_outline(rect, l.radius, BrushStroke::new(l.stroke_width), l.stroke);
            }
        }
        "ELLIPSE" => {
            let cx = l.x + l.w * 0.5;
            let cy = l.y + l.h * 0.5;
            let rx = l.w * 0.5;
            let ry = l.h * 0.5;
            p.ellipse(Point::new(cx, cy), rx, ry, l.fill);
            if l.stroke_width > 0.0 {
                p.ellipse_outline(
                    Point::new(cx, cy),
                    rx,
                    ry,
                    BrushStroke::new(l.stroke_width),
                    l.stroke,
                );
            }
        }
        "TEXT" => {
            p.text_tl(&l.text, Point::new(l.x, l.y), l.font_size, l.fill);
        }
        _ => {}
    }
}

fn draw_selection(p: &mut Painter, l: &Layer, zoom: f32) {
    let hs = HANDLE_PX / zoom;
    let rect = Rect::from_xywh(l.x, l.y, l.w, l.h);
    p.rect_outline(rect, 0.0, BrushStroke::screen(1.5), SEL_COLOR);
    let h2 = hs * 0.5;
    let pts = [
        (l.x, l.y),
        (l.x + l.w * 0.5, l.y),
        (l.x + l.w, l.y),
        (l.x + l.w, l.y + l.h * 0.5),
        (l.x + l.w, l.y + l.h),
        (l.x + l.w * 0.5, l.y + l.h),
        (l.x, l.y + l.h),
        (l.x, l.y + l.h * 0.5),
    ];
    for (hx, hy) in pts {
        let r = Rect::from_xywh(hx - h2, hy - h2, hs, hs);
        p.fill(r, Fill::solid(Color::WHITE));
        p.rect_outline(r, 0.0, BrushStroke::screen(1.2), SEL_COLOR);
    }
}

fn draw_marquee(p: &mut Painter, x: f32, y: f32, w: f32, h: f32) {
    p.fill(Rect::from_xywh(x, y, w, h), Fill::solid(SEL_FILL));
    p.rect_outline(
        Rect::from_xywh(x, y, w, h),
        0.0,
        BrushStroke::screen(1.0),
        SEL_COLOR,
    );
}

fn draw_ghost(p: &mut Painter, tool: &str, x: f32, y: f32, w: f32, h: f32) {
    let stroke = BrushStroke::screen(1.5);
    let fill = Fill::solid(SEL_FILL);
    let rect = Rect::from_xywh(x, y, w, h);
    match tool {
        "O" | "ELLIPSE" => {
            let cx = x + w * 0.5;
            let cy = y + h * 0.5;
            let rx = w * 0.5;
            let ry = h * 0.5;
            p.ellipse(Point::new(cx, cy), rx, ry, SEL_FILL);
            p.ellipse_outline(Point::new(cx, cy), rx, ry, stroke, SEL_COLOR);
        }
        "R" | "RECTANGLE" => {
            p.rounded(rect, 8.0, fill);
            p.rect_outline(rect, 8.0, stroke, SEL_COLOR);
        }
        _ => {
            p.fill(rect, fill);
            p.rect_outline(rect, 0.0, stroke, SEL_COLOR);
        }
    }
}

fn draw_guides(p: &mut Painter, guides: &Guides, wx0: f32, wx1: f32, wy0: f32, wy1: f32) {
    let stroke = BrushStroke::screen(1.0);
    for &gx in &guides.v {
        p.line(
            Point::new(gx, wy0),
            Point::new(gx, wy1),
            stroke,
            GUIDE_COLOR,
        );
    }
    for &gy in &guides.h {
        p.line(
            Point::new(wx0, gy),
            Point::new(wx1, gy),
            stroke,
            GUIDE_COLOR,
        );
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Handle {
    Nw,
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
}

fn hit_handle(l: &Layer, wx: f32, wy: f32, zoom: f32) -> Option<Handle> {
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

fn hit_layer(layers: &[Layer], wx: f32, wy: f32) -> Option<u64> {
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

#[derive(Clone)]
enum DragMode {
    Move,
    Resize(Handle),
}

struct DragSession {
    mode: DragMode,
    start_wx: f32,
    start_wy: f32,
    orig: Vec<(u64, f32, f32, f32, f32)>,
    layers_before: Vec<Layer>,
}
struct DrawSession {
    tool: &'static str,
    start_wx: f32,
    start_wy: f32,
    cur_wx: f32,
    cur_wy: f32,
    layers_before: Vec<Layer>,
}
struct MarqueeSession {
    start_wx: f32,
    start_wy: f32,
    cur_wx: f32,
    cur_wy: f32,
}
struct PanSession {
    start_screen_x: f32,
    start_screen_y: f32,
    start_pan_x: f32,
    start_pan_y: f32,
}

fn apply_resize(
    h: Handle,
    x: f32,
    y: f32,
    w: f32,
    hh: f32,
    dx: f32,
    dy: f32,
    snap_on: bool,
) -> (f32, f32, f32, f32) {
    let (mut nx, mut ny, mut nw, mut nh) = (x, y, w, hh);
    match h {
        Handle::Nw => {
            nx = x + dx;
            ny = y + dy;
            nw = w - dx;
            nh = hh - dy;
        }
        Handle::N => {
            ny = y + dy;
            nh = hh - dy;
        }
        Handle::Ne => {
            ny = y + dy;
            nw = w + dx;
            nh = hh - dy;
        }
        Handle::E => {
            nw = w + dx;
        }
        Handle::Se => {
            nw = w + dx;
            nh = hh + dy;
        }
        Handle::S => {
            nh = hh + dy;
        }
        Handle::Sw => {
            nx = x + dx;
            nw = w - dx;
            nh = hh + dy;
        }
        Handle::W => {
            nx = x + dx;
            nw = w - dx;
        }
    }
    if snap_on {
        let right = nx + nw;
        let bottom = ny + nh;
        match h {
            Handle::Nw => {
                nx = snap(nx);
                ny = snap(ny);
                nw = right - nx;
                nh = bottom - ny;
            }
            Handle::N => {
                ny = snap(ny);
                nh = bottom - ny;
            }
            Handle::Ne => {
                ny = snap(ny);
                nw = snap(right) - nx;
                nh = bottom - ny;
            }
            Handle::E => {
                nw = snap(right) - nx;
            }
            Handle::Se => {
                nw = snap(right) - nx;
                nh = snap(bottom) - ny;
            }
            Handle::S => {
                nh = snap(bottom) - ny;
            }
            Handle::Sw => {
                nx = snap(nx);
                nw = right - nx;
                nh = snap(bottom) - ny;
            }
            Handle::W => {
                nx = snap(nx);
                nw = right - nx;
            }
        }
    }
    (nx, ny, nw.max(4.0), nh.max(4.0))
}

fn next_id(layers: &[Layer]) -> u64 {
    layers.iter().map(|l| l.id).max().unwrap_or(0) + 1
}

fn pick_hex_for(kind: &str) -> (&'static str, Color) {
    match kind {
        "FRAME" => ("#1B1B1B", Color::hex(0x1B1B1B)),
        "RECTANGLE" => ("#0D99FF", Color::hex(0x0D99FF)),
        "ELLIPSE" => ("#42BE65", Color::hex(0x42BE65)),
        "TEXT" => ("#FFFFFF", Color::WHITE),
        _ => ("#0D99FF", Color::hex(0x0D99FF)),
    }
}

pub(crate) fn canvas(state: &EditorState) -> impl View + 'static {
    let layers = state.layers.clone();
    let selected = state.selected_layer.clone();
    let tool = state.tool.clone();
    let st_hist = state.clone();
    let grid = state.grid_on.clone();
    let zoom = state.zoom.clone();
    let pan_x = state.pan_x.clone();
    let pan_y = state.pan_y.clone();
    let viewport_w = state.viewport_w.clone();
    let viewport_h = state.viewport_h.clone();
    let ctx_menu_open = state.ctx_menu_open.clone();
    let ctx_menu_x = state.ctx_menu_x.clone();
    let ctx_menu_y = state.ctx_menu_y.clone();

    let drag: Rc<RefCell<Option<DragSession>>> = Rc::new(RefCell::new(None));
    let draw: Rc<RefCell<Option<DrawSession>>> = Rc::new(RefCell::new(None));
    let marquee: Rc<RefCell<Option<MarqueeSession>>> = Rc::new(RefCell::new(None));
    let pan: Rc<RefCell<Option<PanSession>>> = Rc::new(RefCell::new(None));
    let guides: Rc<RefCell<Guides>> = Rc::new(RefCell::new(Guides::default()));
    let repaint: Signal<u64> = Signal::new(0);

    let lc_t = layers.clone();
    let sc_t = selected.clone();
    let tc_t = tool.clone();
    let rp_t = repaint.clone();
    let g_t = grid.clone();
    let z_t = zoom.clone();
    let px_t = pan_x.clone();
    let py_t = pan_y.clone();

    let lc_d = layers.clone();
    let sc_d = selected.clone();
    let dw_d = draw.clone();
    let mq_d = marquee.clone();
    let gd_d = guides.clone();
    let g_d = grid.clone();
    let z_d = zoom.clone();
    let px_d = pan_x.clone();
    let py_d = pan_y.clone();
    let vpw_d = viewport_w.clone();
    let vph_d = viewport_h.clone();

    let lc_p = layers.clone();
    let sc_p = selected.clone();
    let tc_p = tool.clone();
    let tl_p = state.tool.clone();
    let dr_p = drag.clone();
    let dw_p = draw.clone();
    let mq_p = marquee.clone();
    let pn_p = pan.clone();
    let gd_p = guides.clone();
    let rp_p = repaint.clone();
    let g_p = grid.clone();
    let sh_p = st_hist.clone();
    let z_p = zoom.clone();
    let px_p = pan_x.clone();
    let cm_p = ctx_menu_open.clone();
    let py_p = pan_y.clone();
    let st_te = state.clone();

    Canvas::new()
        .background(colors::BG_DARKER)
        .track(move || {
            lc_t.get();
            sc_t.get();
            tc_t.get();
            rp_t.get();
            g_t.get();
            z_t.get();
            px_t.get();
            py_t.get();
        })
        .on_draw(move |p| {
            let size = p.size();
            if (vpw_d.get_untracked() - size.width).abs() > 0.5 {
                vpw_d.set(size.width);
            }
            if (vph_d.get_untracked() - size.height).abs() > 0.5 {
                vph_d.set(size.height);
            }

            let zoom_v = z_d.get_untracked();
            let pan_vx = px_d.get_untracked();
            let pan_vy = py_d.get_untracked();

            if g_d.get_untracked() {
                draw_grid_screen(p, pan_vx, pan_vy, zoom_v);
            }

            let wx0 = pan_vx - 100.0;
            let wy0 = pan_vy - 100.0;
            let wx1 = pan_vx + size.width / zoom_v + 100.0;
            let wy1 = pan_vy + size.height / zoom_v + 100.0;

            let transform = Transform::translate(-pan_vx, -pan_vy).then(Transform::uniform(zoom_v));

            p.with_transform(transform, |p| {
                lc_d.with_untracked(|ls| {
                    for l in ls {
                        draw_layer(p, l);
                    }
                    if let Some(d) = dw_d.borrow().as_ref() {
                        let x = d.start_wx.min(d.cur_wx);
                        let y = d.start_wy.min(d.cur_wy);
                        let w = (d.cur_wx - d.start_wx).abs();
                        let h = (d.cur_wy - d.start_wy).abs();
                        draw_ghost(p, d.tool, x, y, w, h);
                    }
                    if let Some(m) = mq_d.borrow().as_ref() {
                        let x = m.start_wx.min(m.cur_wx);
                        let y = m.start_wy.min(m.cur_wy);
                        let w = (m.cur_wx - m.start_wx).abs();
                        let h = (m.cur_wy - m.start_wy).abs();
                        draw_marquee(p, x, y, w, h);
                    }
                    if let Some(id) = sc_d.get_untracked()
                        && let Some(l) = ls.iter().find(|l| l.id == id)
                    {
                        draw_selection(p, l, zoom_v);
                    }
                    let g = gd_d.borrow();
                    if !g.v.is_empty() || !g.h.is_empty() {
                        draw_guides(p, &g, wx0, wx1, wy0, wy1);
                    }
                });
            });
        })
        .on_pointer(move |pos, ev, ctx| {
            let t = tc_p.get_untracked();
            let z = z_p.get_untracked();
            let px = px_p.get_untracked();
            let py = py_p.get_untracked();
            let wx = pos.x / z + px;
            let wy = pos.y / z + py;
            let cm_p_x = ctx_menu_x.clone();
            let cm_p_y = ctx_menu_y.clone();

            match ev {
                PointerEvent::Wheel {
                    delta, modifiers, ..
                } => {
                    let ctrl = modifiers.contains(Modifiers::CONTROL)
                        || modifiers.contains(Modifiers::SUPER);
                    if ctrl {
                        let factor = if delta.y > 0.0 {
                            ZOOM_STEP
                        } else {
                            1.0 / ZOOM_STEP
                        };
                        let new_z = (z * factor).clamp(ZOOM_MIN, ZOOM_MAX);
                        if (new_z - z).abs() > 1e-4 {
                            let npx = wx - pos.x / new_z;
                            let npy = wy - pos.y / new_z;
                            px_p.set(npx);
                            py_p.set(npy);
                            z_p.set(new_z);
                        }
                    } else if modifiers.contains(Modifiers::SHIFT) {
                        px_p.update(|v| *v += delta.y / z);
                    } else {
                        px_p.update(|v| *v += delta.x / z);
                        py_p.update(|v| *v += delta.y / z);
                    }
                    ctx.handled();
                }

                PointerEvent::Down {
                    button, modifiers, ..
                } => {
                    if *button == PointerButton::Middle
                        || (*button == PointerButton::Left && t == "H")
                    {
                        *pn_p.borrow_mut() = Some(PanSession {
                            start_screen_x: pos.x,
                            start_screen_y: pos.y,
                            start_pan_x: px,
                            start_pan_y: py,
                        });
                        ctx.handled();
                        return;
                    }

                    if *button == PointerButton::Right {
                        if dr_p.borrow().is_some()
                            || dw_p.borrow().is_some()
                            || mq_p.borrow().is_some()
                        {
                            return;
                        }
                        let ls = lc_p.get_untracked();
                        if let Some(id) = hit_layer(&ls, wx, wy) {
                            sc_p.set(Some(id));
                        } else {
                            sc_p.set(None);
                        }
                        cm_p_x.set(pos.x + 272.0);
                        cm_p_y.set(pos.y + 64.0);
                        cm_p.set(true);
                        ctx.handled();
                        return;
                    }

                    if *button != PointerButton::Left {
                        return;
                    }

                    let snapping = !modifiers.contains(Modifiers::ALT);
                    let grid_snap = snapping && g_p.get_untracked();
                    *gd_p.borrow_mut() = Guides::default();

                    if t == "V"
                        && let Some(id) = sc_p.get_untracked()
                        && let Some(l) = lc_p.get_untracked().iter().find(|l| l.id == id)
                        && !l.locked
                        && let Some(h) = hit_handle(l, wx, wy, z)
                    {
                        *dr_p.borrow_mut() = Some(DragSession {
                            mode: DragMode::Resize(h),
                            start_wx: wx,
                            start_wy: wy,
                            orig: vec![(l.id, l.x, l.y, l.w, l.h)],
                            layers_before: lc_p.get_untracked(),
                        });
                        ctx.handled();
                        return;
                    }

                    if t == "F" || t == "R" || t == "O" {
                        let sx = if grid_snap { snap(wx) } else { wx };
                        let sy = if grid_snap { snap(wy) } else { wy };
                        *dw_p.borrow_mut() = Some(DrawSession {
                            tool: t,
                            start_wx: sx,
                            start_wy: sy,
                            cur_wx: sx,
                            cur_wy: sy,
                            layers_before: lc_p.get_untracked(),
                        });
                        ctx.handled();
                        return;
                    }

                    if t == "T" {
                        let ls = lc_p.get_untracked();
                        let id = next_id(&ls);
                        let (hex, fill) = pick_hex_for("TEXT");
                        let lx = if grid_snap { snap(wx) } else { wx };
                        let ly = if grid_snap { snap(wy) } else { wy };
                        let parent = find_parent_frame(&ls, lx, ly);
                        let level = if parent.is_some() { 1 } else { 0 };
                        sh_p.push_history();
                        let l = Layer {
                            id,
                            name: "Text".into(),
                            icon: "tool/text",
                            kind: "TEXT",
                            level,
                            parent,
                            visible: true,
                            locked: false,
                            component: false,
                            instance: false,
                            x: lx,
                            y: ly,
                            w: 200.0,
                            h: 32.0,
                            opacity: 100.0,
                            radius: 0.0,
                            fill,
                            fill_hex: hex.into(),
                            stroke: Color::TRANSPARENT,
                            stroke_hex: "#00000000".into(),
                            stroke_width: 0.0,
                            text: "Text".into(),
                            font_size: 16.0,
                            layout: "NONE",
                            gap: 0.0,
                            pad_l: 0.0,
                            pad_t: 0.0,
                            pad_r: 0.0,
                            pad_b: 0.0,
                            align_h: "start",
                            align_v: "start",
                        };
                        lc_p.update(|ls| ls.push(l));
                        sc_p.set(Some(id));
                        tl_p.set("V");
                        st_te.start_text_edit(false);
                        ctx.handled();
                        return;
                    }

                    if t == "V" {
                        let ls = lc_p.get_untracked();
                        if let Some(id) = hit_layer(&ls, wx, wy) {
                            sc_p.set(Some(id));
                            let l = ls.iter().find(|l| l.id == id).unwrap();
                            let now = Instant::now();
                            let lcc = st_te.last_canvas_click.clone();
                            let is_dbl = {
                                let c = lcc.borrow();
                                c.0 == Some(id) && now.duration_since(c.1).as_millis() < 400
                            };
                            if is_dbl && l.kind == "TEXT" {
                                *lcc.borrow_mut() = (None, now);
                                st_te.start_text_edit(true);
                                ctx.handled();
                                return;
                            }
                            *lcc.borrow_mut() = (Some(id), now);
                            if !l.locked {
                                *dr_p.borrow_mut() = Some(DragSession {
                                    mode: DragMode::Move,
                                    start_wx: wx,
                                    start_wy: wy,
                                    orig: vec![(l.id, l.x, l.y, l.w, l.h)],
                                    layers_before: ls.clone(),
                                });
                            }
                        } else {
                            sc_p.set(None);
                            *mq_p.borrow_mut() = Some(MarqueeSession {
                                start_wx: wx,
                                start_wy: wy,
                                cur_wx: wx,
                                cur_wy: wy,
                            });
                        }
                        ctx.handled();
                    }
                }

                PointerEvent::Move { modifiers, .. } => {
                    if let Some(p) = pn_p.borrow().as_ref() {
                        let dx = pos.x - p.start_screen_x;
                        let dy = pos.y - p.start_screen_y;
                        px_p.set(p.start_pan_x - dx / z);
                        py_p.set(p.start_pan_y - dy / z);
                        ctx.handled();
                        return;
                    }

                    let snapping = !modifiers.contains(Modifiers::ALT);
                    let grid_snap = snapping && g_p.get_untracked();

                    if let Some(d) = dr_p.borrow().as_ref() {
                        let dx = wx - d.start_wx;
                        let dy = wy - d.start_wy;
                        let orig = d.orig.clone();
                        let mode = d.mode.clone();
                        match mode {
                            DragMode::Move if orig.len() == 1 => {
                                let (id, ox, oy, ow, oh) = orig[0];
                                let nx_raw = ox + dx;
                                let ny_raw = oy + dy;
                                let (sgdx, sgdy, g) = if snapping {
                                    lc_p.with_untracked(|ls| {
                                        compute_snap_and_guides(
                                            &[id],
                                            (nx_raw, ny_raw, ow, oh),
                                            ls,
                                            GUIDE_TOL_PX / z,
                                        )
                                    })
                                } else {
                                    (0.0, 0.0, Guides::default())
                                };
                                let use_guide_x = sgdx.abs() > 0.01;
                                let use_guide_y = sgdy.abs() > 0.01;
                                let final_x = if use_guide_x {
                                    nx_raw + sgdx
                                } else if grid_snap {
                                    snap(nx_raw)
                                } else {
                                    nx_raw
                                };
                                let final_y = if use_guide_y {
                                    ny_raw + sgdy
                                } else if grid_snap {
                                    snap(ny_raw)
                                } else {
                                    ny_raw
                                };
                                *gd_p.borrow_mut() = g;
                                lc_p.update(|ls| {
                                    if let Some(l) = ls.iter_mut().find(|l| l.id == id) {
                                        l.x = final_x;
                                        l.y = final_y;
                                    }
                                });
                                rp_p.update(|v| *v = v.wrapping_add(1));
                            }
                            DragMode::Move => {
                                *gd_p.borrow_mut() = Guides::default();
                                lc_p.update(|ls| {
                                    for (id, ox, oy, _, _) in &orig {
                                        if let Some(l) = ls.iter_mut().find(|l| l.id == *id) {
                                            let nx = ox + dx;
                                            let ny = oy + dy;
                                            l.x = if grid_snap { snap(nx) } else { nx };
                                            l.y = if grid_snap { snap(ny) } else { ny };
                                        }
                                    }
                                });
                            }
                            DragMode::Resize(h) => {
                                *gd_p.borrow_mut() = Guides::default();
                                lc_p.update(|ls| {
                                    for (id, ox, oy, ow, oh) in &orig {
                                        if let Some(l) = ls.iter_mut().find(|l| l.id == *id) {
                                            let (nx, ny, nw, nh) = apply_resize(
                                                h, *ox, *oy, *ow, *oh, dx, dy, grid_snap,
                                            );
                                            l.x = nx;
                                            l.y = ny;
                                            l.w = nw;
                                            l.h = nh;
                                        }
                                    }
                                });
                            }
                        }
                        ctx.handled();
                        return;
                    }

                    if let Some(d) = dw_p.borrow_mut().as_mut() {
                        d.cur_wx = if grid_snap { snap(wx) } else { wx };
                        d.cur_wy = if grid_snap { snap(wy) } else { wy };
                        rp_p.update(|v| *v = v.wrapping_add(1));
                        ctx.handled();
                        return;
                    }

                    if let Some(m) = mq_p.borrow_mut().as_mut() {
                        m.cur_wx = wx;
                        m.cur_wy = wy;
                        rp_p.update(|v| *v = v.wrapping_add(1));
                        ctx.handled();
                    }
                }

                PointerEvent::Up { .. } => {
                    if pn_p.borrow_mut().take().is_some() {
                        ctx.handled();
                        return;
                    }
                    if let Some(d) = dr_p.borrow_mut().take() {
                        *gd_p.borrow_mut() = Guides::default();
                        sh_p.push_snapshot(d.layers_before);
                        sh_p.propagate_layout();
                        ctx.handled();
                    }
                    if let Some(d) = dw_p.borrow_mut().take() {
                        let x = d.start_wx.min(d.cur_wx);
                        let y = d.start_wy.min(d.cur_wy);
                        let w = (d.cur_wx - d.start_wx).abs();
                        let h = (d.cur_wy - d.start_wy).abs();
                        if w >= 4.0 && h >= 4.0 {
                            let (kind, icon, name, radius) = match d.tool {
                                "F" => ("FRAME", "tool/frame", "Frame", 0.0),
                                "R" => ("RECTANGLE", "tool/rect", "Rectangle", 8.0),
                                "O" => ("ELLIPSE", "tool/ellipse", "Ellipse", 60.0),
                                _ => ("RECTANGLE", "tool/rect", "Shape", 0.0),
                            };
                            let (hex, fill) = pick_hex_for(kind);
                            let ls = lc_p.get_untracked();
                            let id = next_id(&ls);
                            let parent = if kind == "FRAME" {
                                None
                            } else {
                                find_parent_frame(&ls, x, y)
                            };
                            let level = if parent.is_some() { 1 } else { 0 };
                            let l = Layer {
                                id,
                                name: name.into(),
                                icon,
                                kind,
                                level,
                                parent,
                                visible: true,
                                locked: false,
                                component: false,
                                instance: false,
                                x,
                                y,
                                w,
                                h,
                                opacity: 100.0,
                                radius,
                                fill,
                                fill_hex: hex.into(),
                                stroke: Color::TRANSPARENT,
                                stroke_hex: "#00000000".into(),
                                stroke_width: 0.0,
                                text: String::new(),
                                font_size: 14.0,
                                layout: "NONE",
                                gap: 0.0,
                                pad_l: 0.0,
                                pad_t: 0.0,
                                pad_r: 0.0,
                                pad_b: 0.0,
                                align_h: "start",
                                align_v: "start",
                            };
                            lc_p.update(|ls| ls.push(l));
                            sc_p.set(Some(id));
                            tl_p.set("V");
                        }
                        sh_p.push_snapshot(d.layers_before);
                        sh_p.propagate_layout();
                        ctx.handled();
                    }
                    if let Some(m) = mq_p.borrow_mut().take() {
                        let x1 = m.start_wx.min(m.cur_wx);
                        let y1 = m.start_wy.min(m.cur_wy);
                        let x2 = m.start_wx.max(m.cur_wx);
                        let y2 = m.start_wy.max(m.cur_wy);
                        let ls = lc_p.get_untracked();
                        let hit: Vec<u64> = ls
                            .iter()
                            .filter(|l| {
                                l.visible
                                    && l.x < x2
                                    && l.x + l.w > x1
                                    && l.y < y2
                                    && l.y + l.h > y1
                            })
                            .map(|l| l.id)
                            .collect();
                        sc_p.set(hit.first().copied());
                        ctx.handled();
                    }
                }
                _ => {}
            }
        })
}
