use super::graph_state::{GraphDrag, NodeGraphState};
use super::model::*;
use crate::theme::colors;
use sequoia_ui::core::Length;
use sequoia_ui::prelude::*;
use sequoia_ui::view::{BrushStroke, Fill, Painter};
// ── view ─────────────────────────────────────────────────
#[derive(Clone)]
pub(crate) struct GraphCanvas {
    pub st: NodeGraphState,
}

impl View for GraphCanvas {
    fn body(&self) -> impl View + 'static {
        let st = self.st.clone();
        let st2 = self.st.clone();
        let st3 = self.st.clone();

        Canvas::new()
            .background(colors::active().bg_darker)
            .track(move || {
                // The document drives the graph: any edit (history push)
                // lands here and `sync` rebuilds when the stamp changed.
                st.editor.root.get();
                st.scope_id.get();
                st.nodes.get();
                st.connections.get();
                st.selected_node.get();
                st.issues.get();
                st.zoom.get();
                st.pan_x.get();
                st.pan_y.get();
                st.repaint.get();
                st.sync();
            })
            .on_draw(move |p| {
                draw_graph(p, &st2);
            })
            .on_pointer(move |pos, ev, ctx| {
                handle_pointer(&st3, pos, ev, ctx);
            })
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fill(1.0),
            ..LayoutStyle::default()
        }
    }
}

// ── drawing ──────────────────────────────────────────────
fn draw_graph(p: &mut Painter, st: &NodeGraphState) {
    let zoom = st.zoom.get_untracked();
    let pan_x = st.pan_x.get_untracked();
    let pan_y = st.pan_y.get_untracked();
    let size = p.size();
    let nodes = st.nodes.get_untracked();

    // dot grid
    draw_dot_grid(p, pan_x, pan_y, zoom, size);

    if nodes.is_empty() {
        draw_empty_state(p, size);
        return;
    }

    // world transform
    let transform = Transform::translate(-pan_x, -pan_y).then(Transform::uniform(zoom));

    p.with_transform(transform, |p| {
        let conns = st.connections.get_untracked();
        let sel_node = st.selected_node.get_untracked();

        // connections: execution edges take the source accent, data edges
        // (state ⇄ action) the blue of the State kind.
        for conn in conns.iter() {
            let from = nodes.iter().find(|n| n.id == conn.from_node);
            let to = nodes.iter().find(|n| n.id == conn.to_node);
            if let (Some(fn_), Some(tn)) = (from, to) {
                let p1 = fn_.port_pos(PortSide::Output, conn.from_port);
                let p2 = tn.port_pos(PortSide::Input, conn.to_port);
                let color = if conn.is_data(&nodes) {
                    NodeKind::State.accent()
                } else {
                    fn_.kind.accent()
                };
                let in_cycle = fn_.in_cycle && tn.in_cycle;
                draw_bezier(
                    p,
                    p1,
                    p2,
                    if in_cycle {
                        colors::active().danger
                    } else {
                        color
                    },
                    in_cycle,
                );
            }
        }

        // wire being dragged: from the grabbed socket to the cursor.
        if let Some(GraphDrag::Wire { node, side, port }) = st.drag.borrow().as_ref() {
            if let Some(p1) = st.port_world_pos(*node, *side, *port) {
                let p2 = *st.wire_mouse.borrow();
                draw_bezier(p, p1, p2, colors::active().sel, false);
                // Aim assist: ring the socket under the cursor.
                if let Some((hn, hside, hport)) = *st.wire_hover.borrow() {
                    if let Some(hn_node) = nodes.iter().find(|n| n.id == hn) {
                        let hp = hn_node.port_pos(hside, hport);
                        p.ellipse_outline(
                            hp,
                            PORT_R + 3.0,
                            PORT_R + 3.0,
                            BrushStroke::screen(2.0),
                            colors::active().sel,
                        );
                    }
                }
            }
        }

        // nodes (the selected one is drawn last)
        let mut order: Vec<&GraphNode> = nodes.iter().collect();
        if let Some(sid) = sel_node {
            order.sort_by_key(|n| if n.id == sid { 1 } else { 0 });
        }
        for node in &order {
            let selected = sel_node == Some(node.id);
            draw_node(p, node, selected);
        }
    });

    draw_legend(p, size);
}

fn draw_dot_grid(p: &mut Painter, pan_x: f32, pan_y: f32, zoom: f32, size: Size) {
    let step = GRID_STEP * zoom;
    if step < 8.0 {
        return;
    }
    let dot_r = (1.0 * zoom).min(1.5);
    let off_x = (-pan_x * zoom).rem_euclid(step);
    let off_y = (-pan_y * zoom).rem_euclid(step);
    let color = Color::hexa(0xFFFFFF12);

    let mut x = off_x;
    while x <= size.width {
        let mut y = off_y;
        while y <= size.height {
            p.circle(Point::new(x, y), dot_r, Fill::solid(color));
            y += step;
        }
        x += step;
    }
}

fn draw_empty_state(p: &mut Painter, size: Size) {
    let pal = colors::active();
    let line1 = "No rules or variables in this scope yet.";
    let line2 = "Select an element and add rules in the Interact tab.";
    let w1 = line1.len() as f32 * 7.2;
    let w2 = line2.len() as f32 * 6.1;
    p.text_tl(
        line1,
        Point::new((size.width - w1) / 2.0, size.height / 2.0 - 24.0),
        13.0,
        pal.text,
    );
    p.text_tl(
        line2,
        Point::new((size.width - w2) / 2.0, size.height / 2.0 - 2.0),
        11.0,
        pal.icon_inactive,
    );
}

fn draw_legend(p: &mut Painter, size: Size) {
    let pal = colors::active();
    let y = size.height - 20.0;
    let mut x = 14.0;
    for (kind, label) in [
        (NodeKind::Trigger, "Trigger"),
        (NodeKind::State, "State"),
        (NodeKind::Action, "Action"),
    ] {
        p.circle(Point::new(x, y), 4.0, Fill::solid(kind.accent()));
        p.text_tl(label, Point::new(x + 9.0, y - 6.0), 10.0, pal.icon_inactive);
        x += 9.0 + label.len() as f32 * 6.0 + 18.0;
    }
}

fn draw_node(p: &mut Painter, node: &GraphNode, selected: bool) {
    let h = node.height();
    let rect = Rect::from_xywh(node.x, node.y, NODE_W, h);
    let pal = colors::active();

    // shadow
    let shadow_rect = Rect::from_xywh(node.x + 2.0, node.y + 2.0, NODE_W, h);
    p.rounded(shadow_rect, 8.0, Fill::solid(Color::hexa(0x00000040)));

    // background
    p.rounded(rect, 8.0, Fill::solid(pal.bg_lighter));

    // header
    let header_rect = Rect::from_xywh(node.x, node.y, NODE_W, HEADER_H);
    p.rounded(header_rect, 8.0, Fill::solid(node.kind.header_color()));
    // mask the header's bottom corners with a rectangle
    let mask = Rect::from_xywh(node.x, node.y + HEADER_H - 8.0, NODE_W, 8.0);
    p.fill(mask, Fill::solid(node.kind.header_color()));

    // title + optional second line (element name / type + initial)
    match &node.subtitle {
        Some(sub) => {
            p.text_tl(
                &node.title,
                Point::new(node.x + 10.0, node.y + 5.0),
                11.5,
                Color::WHITE,
            );
            p.text_tl(
                sub,
                Point::new(node.x + 10.0, node.y + 18.0),
                8.5,
                Color::hexa(0xFFFFFFB0),
            );
        }
        None => {
            p.text_tl(
                &node.title,
                Point::new(node.x + 10.0, node.y + 8.0),
                12.0,
                Color::WHITE,
            );
        }
    }

    // selection outline (under the ports); cycles pulse red
    if node.in_cycle {
        p.rect_outline(rect, 8.0, BrushStroke::screen(2.5), pal.danger);
    } else if selected {
        p.rect_outline(rect, 8.0, BrushStroke::screen(2.0), pal.sel);
    } else {
        p.rect_outline(rect, 8.0, BrushStroke::screen(1.0), pal.stroke);
    }

    // ports + labels
    for (i, label) in node.inputs.iter().enumerate() {
        let pos = node.port_pos(PortSide::Input, i);
        p.circle(pos, PORT_R, Fill::solid(pal.bg_darker));
        p.circle(pos, PORT_R - 1.5, Fill::solid(node.kind.accent()));
        if !label.is_empty() {
            p.text_tl(label, Point::new(pos.x + 10.0, pos.y - 6.0), 10.0, pal.text);
        }
    }
    for (i, label) in node.outputs.iter().enumerate() {
        let pos = node.port_pos(PortSide::Output, i);
        p.circle(pos, PORT_R, Fill::solid(pal.bg_darker));
        p.circle(pos, PORT_R - 1.5, Fill::solid(node.kind.accent()));
        if !label.is_empty() {
            let tw = label.len() as f32 * 5.5;
            p.text_tl(
                label,
                Point::new(pos.x - 10.0 - tw, pos.y - 6.0),
                10.0,
                pal.text,
            );
        }
    }

    // issue dot for a node that breaks validation (cycles, dangling refs)
    if node.in_cycle {
        p.circle(
            Point::new(node.x + NODE_W - 10.0, node.y + HEADER_H / 2.0),
            4.5,
            Fill::solid(pal.danger),
        );
    }
}

fn draw_bezier(p: &mut Painter, from: Point, to: Point, color: Color, thick: bool) {
    let dx = ((to.x - from.x).abs() * 0.5).max(50.0);
    let cp1 = Point::new(from.x + dx, from.y);
    let cp2 = Point::new(to.x - dx, to.y);
    let w = if thick { 2.5 } else { 1.8 };
    let stroke = BrushStroke::screen(w);

    let segments = 28;
    let mut prev = from;
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let pt = bezier_eval(t, from, cp1, cp2, to);
        p.line(prev, pt, stroke, color);
        prev = pt;
    }
}

fn bezier_eval(t: f32, p0: Point, p1: Point, p2: Point, p3: Point) -> Point {
    let u = 1.0 - t;
    let uu = u * u;
    let uuu = uu * u;
    let tt = t * t;
    let ttt = tt * t;
    Point::new(
        uuu * p0.x + 3.0 * uu * t * p1.x + 3.0 * u * tt * p2.x + ttt * p3.x,
        uuu * p0.y + 3.0 * uu * t * p1.y + 3.0 * u * tt * p2.y + ttt * p3.y,
    )
}

// ── pointer events ───────────────────────────────────────
fn handle_pointer(st: &NodeGraphState, pos: Point, ev: &PointerEvent, ctx: &mut EventCtx) {
    let zoom = st.zoom.get_untracked();
    let (wx, wy) = st.screen_to_world(pos.x, pos.y);

    match ev {
        // ── scroll = zoom / pan ──────────────────────────
        PointerEvent::Wheel {
            delta, modifiers, ..
        } => {
            let ctrl =
                modifiers.contains(Modifiers::CONTROL) || modifiers.contains(Modifiers::SUPER);
            if ctrl {
                let factor = if delta.y > 0.0 { 1.15 } else { 1.0 / 1.15 };
                let new_z = (zoom * factor).clamp(0.15, 5.0);
                if (new_z - zoom).abs() > 1e-4 {
                    let pan_x = st.pan_x.get_untracked();
                    let pan_y = st.pan_y.get_untracked();
                    let world_x = pos.x / zoom + pan_x;
                    let world_y = pos.y / zoom + pan_y;
                    st.pan_x.set(world_x - pos.x / new_z);
                    st.pan_y.set(world_y - pos.y / new_z);
                    st.zoom.set(new_z);
                }
            } else if modifiers.contains(Modifiers::SHIFT) {
                st.pan_x.update(|v| *v -= delta.y / zoom);
            } else {
                // The view follows the fingers, as when dragging the canvas.
                st.pan_x.update(|v| *v -= delta.x / zoom);
                st.pan_y.update(|v| *v -= delta.y / zoom);
            }
            ctx.handled();
        }

        // ── press ────────────────────────────────────────
        PointerEvent::Down { button, .. } => {
            // pan: middle button
            if *button == PointerButton::Middle {
                *st.drag.borrow_mut() = Some(GraphDrag::Pan {
                    start_sx: pos.x,
                    start_sy: pos.y,
                    orig_pan_x: st.pan_x.get_untracked(),
                    orig_pan_y: st.pan_y.get_untracked(),
                });
                ctx.handled();
                return;
            }

            if *button != PointerButton::Left {
                return;
            }

            // 1) socket → start a wire drag (checked before the node, ports
            //    sit on the node's edge). Dragging FROM a socket connects it.
            if let Some((nid, side, port)) = st.hit_port(wx, wy) {
                *st.drag.borrow_mut() = Some(GraphDrag::Wire {
                    node: nid,
                    side,
                    port,
                });
                *st.wire_mouse.borrow_mut() = Point::new(wx, wy);
                *st.wire_hover.borrow_mut() = None;
                st.bump();
                ctx.handled();
                return;
            }

            // 2) node → select + drag (the editor selection follows)
            if let Some(nid) = st.hit_node(wx, wy) {
                st.select_node(nid);
                let nodes = st.nodes.get_untracked();
                if let Some(n) = nodes.iter().find(|n| n.id == nid) {
                    *st.drag.borrow_mut() = Some(GraphDrag::MoveNode {
                        id: nid,
                        start_wx: wx,
                        start_wy: wy,
                        orig_x: n.x,
                        orig_y: n.y,
                    });
                }
                ctx.handled();
                return;
            }

            // 3) empty → deselect + pan with the left button
            st.selected_node.set(None);
            *st.drag.borrow_mut() = Some(GraphDrag::Pan {
                start_sx: pos.x,
                start_sy: pos.y,
                orig_pan_x: st.pan_x.get_untracked(),
                orig_pan_y: st.pan_y.get_untracked(),
            });
            st.bump();
            ctx.handled();
        }

        // ── move ─────────────────────────────────────────
        PointerEvent::Move { .. } => {
            let mut drag = st.drag.borrow_mut();
            match drag.as_mut() {
                Some(GraphDrag::Wire { .. }) => {
                    let hover = st.hit_port(wx, wy);
                    *st.wire_mouse.borrow_mut() = Point::new(wx, wy);
                    *st.wire_hover.borrow_mut() = hover;
                    drop(drag);
                    st.bump();
                    ctx.handled();
                }
                Some(GraphDrag::MoveNode {
                    id,
                    start_wx,
                    start_wy,
                    orig_x,
                    orig_y,
                }) => {
                    let dx = wx - *start_wx;
                    let dy = wy - *start_wy;
                    let nx = *orig_x + dx;
                    let ny = *orig_y + dy;
                    let nid = *id;
                    drop(drag);
                    st.move_node(nid, nx, ny);
                    ctx.handled();
                }
                Some(GraphDrag::Pan {
                    start_sx,
                    start_sy,
                    orig_pan_x,
                    orig_pan_y,
                }) => {
                    let dx = (pos.x - *start_sx) / zoom;
                    let dy = (pos.y - *start_sy) / zoom;
                    st.pan_x.set(*orig_pan_x - dx);
                    st.pan_y.set(*orig_pan_y - dy);
                    ctx.handled();
                }
                None => {}
            }
        }

        // ── release ──────────────────────────────────────
        PointerEvent::Up { .. } => {
            match st.drag.borrow_mut().take() {
                Some(GraphDrag::Wire { node, side, port }) => {
                    // Drop on a compatible socket rewires the rules; dropping
                    // on empty space or back on the source socket cancels
                    // silently.
                    let target = *st.wire_hover.borrow();
                    if let Some(t) = target
                        && t != (node, side, port)
                    {
                        st.finish_wire((node, side, port), t);
                    }
                    *st.wire_hover.borrow_mut() = None;
                    st.bump();
                    ctx.handled();
                }
                Some(_) => {
                    ctx.handled();
                }
                None => {}
            }
        }

        _ => {}
    }
}
