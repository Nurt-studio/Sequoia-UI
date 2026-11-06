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
                st.nodes.get();
                st.connections.get();
                st.selected_node.get();
                st.selected_conn.get();
                st.zoom.get();
                st.pan_x.get();
                st.pan_y.get();
                st.repaint.get();
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

    // dot grid
    draw_dot_grid(p, pan_x, pan_y, zoom, size);

    // world transform
    let transform = Transform::translate(-pan_x, -pan_y).then(Transform::uniform(zoom));

    p.with_transform(transform, |p| {
        let nodes = st.nodes.get_untracked();
        let conns = st.connections.get_untracked();
        let sel_node = st.selected_node.get_untracked();
        let sel_conn = st.selected_conn.get_untracked();

        // connections
        for (ci, conn) in conns.iter().enumerate() {
            let from = nodes.iter().find(|n| n.id == conn.from_node);
            let to = nodes.iter().find(|n| n.id == conn.to_node);
            if let (Some(fn_), Some(tn)) = (from, to) {
                let p1 = fn_.port_pos(PortSide::Output, conn.from_port);
                let p2 = tn.port_pos(PortSide::Input, conn.to_port);
                let selected = sel_conn == Some(ci);
                let color = if selected {
                    colors::active().sel
                } else {
                    fn_.kind.accent()
                };
                draw_bezier(p, p1, p2, color, selected);
            }
        }

        // preview of the connection being created
        if let Some(GraphDrag::Connect {
            from_node,
            from_side,
            from_port,
            cur_wx,
            cur_wy,
        }) = st.drag.borrow().as_ref()
            && let Some(fn_) = nodes.iter().find(|n| n.id == *from_node)
        {
            let anchor = fn_.port_pos(*from_side, *from_port);
            let cursor = Point::new(*cur_wx, *cur_wy);
            let (p1, p2) = match from_side {
                PortSide::Output => (anchor, cursor),
                PortSide::Input => (cursor, anchor),
            };
            draw_bezier(p, p1, p2, colors::active().sel, true);
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

    // zoom indicator
    let label = format!("{:.0}%", zoom * 100.0);
    p.text_tl(
        &label,
        Point::new(size.width - 60.0, size.height - 24.0),
        11.0,
        colors::active().icon_inactive,
    );
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

    // title
    p.text_tl(
        &node.title,
        Point::new(node.x + 10.0, node.y + 8.0),
        12.0,
        Color::WHITE,
    );

    // selection outline (under the ports)
    if selected {
        p.rect_outline(rect, 8.0, BrushStroke::screen(2.0), colors::active().sel);
    } else {
        p.rect_outline(rect, 8.0, BrushStroke::screen(1.0), pal.stroke);
    }

    // ports + labels
    for (i, label) in node.inputs.iter().enumerate() {
        let pos = node.port_pos(PortSide::Input, i);
        p.circle(pos, PORT_R, Fill::solid(pal.bg_darker));
        p.circle(pos, PORT_R - 1.5, Fill::solid(node.kind.accent()));
        p.text_tl(label, Point::new(pos.x + 10.0, pos.y - 6.0), 10.0, pal.text);
    }
    for (i, label) in node.outputs.iter().enumerate() {
        let pos = node.port_pos(PortSide::Output, i);
        p.circle(pos, PORT_R, Fill::solid(pal.bg_darker));
        p.circle(pos, PORT_R - 1.5, Fill::solid(node.kind.accent()));
        let tw = label.len() as f32 * 5.5;
        p.text_tl(
            label,
            Point::new(pos.x - 10.0 - tw, pos.y - 6.0),
            10.0,
            pal.text,
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

            // 1) port → start a connection (from input or output)
            if let Some((nid, side, pi)) = find_port(&st.nodes.get_untracked(), wx, wy) {
                *st.drag.borrow_mut() = Some(GraphDrag::Connect {
                    from_node: nid,
                    from_side: side,
                    from_port: pi,
                    cur_wx: wx,
                    cur_wy: wy,
                });
                st.selected_node.set(Some(nid));
                st.selected_conn.set(None);
                st.bump();
                ctx.handled();
                return;
            }

            // 2) node → select + drag
            if let Some(nid) = st.hit_node(wx, wy) {
                st.selected_node.set(Some(nid));
                st.selected_conn.set(None);
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
                st.bump();
                ctx.handled();
                return;
            }

            // 3) connection → select
            if let Some(ci) = hit_connection(st, wx, wy) {
                st.selected_conn.set(Some(ci));
                st.selected_node.set(None);
                st.bump();
                ctx.handled();
                return;
            }

            // 4) empty → deselect + pan with the left button
            st.selected_node.set(None);
            st.selected_conn.set(None);
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
                    st.nodes.update(|ns| {
                        if let Some(n) = ns.iter_mut().find(|n| n.id == nid) {
                            n.x = nx;
                            n.y = ny;
                        }
                    });
                    st.bump();
                    ctx.handled();
                }
                Some(GraphDrag::Connect { cur_wx, cur_wy, .. }) => {
                    *cur_wx = wx;
                    *cur_wy = wy;
                    drop(drag);
                    st.bump();
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
            let drag = st.drag.borrow_mut().take();
            match drag {
                Some(GraphDrag::Connect {
                    from_node,
                    from_side,
                    from_port,
                    cur_wx,
                    cur_wy,
                }) => {
                    // drop on a port on the opposite side
                    if let Some((tid, to_side, ti)) =
                        find_port(&st.nodes.get_untracked(), cur_wx, cur_wy)
                        && tid != from_node
                        && to_side != from_side
                    {
                        let conn = match from_side {
                            PortSide::Output => GraphConnection {
                                from_node,
                                from_port,
                                to_node: tid,
                                to_port: ti,
                            },
                            PortSide::Input => GraphConnection {
                                from_node: tid,
                                from_port: ti,
                                to_node: from_node,
                                to_port: from_port,
                            },
                        };
                        // do not add duplicates
                        let exists = st.connections.get_untracked().contains(&conn);
                        if !exists {
                            st.connections.update(|cs| cs.push(conn));
                        }
                    }
                    st.bump();
                    ctx.handled();
                }
                Some(GraphDrag::MoveNode { .. }) | Some(GraphDrag::Pan { .. }) => {
                    ctx.handled();
                }
                None => {}
            }
        }

        _ => {}
    }
}

/// Simple connection hit test (checks distance from the curve)
fn hit_connection(st: &NodeGraphState, wx: f32, wy: f32) -> Option<usize> {
    let nodes = st.nodes.get_untracked();
    let conns = st.connections.get_untracked();
    let tol = 8.0 / st.zoom.get_untracked();

    for (ci, conn) in conns.iter().enumerate() {
        let from = nodes.iter().find(|n| n.id == conn.from_node)?;
        let to = nodes.iter().find(|n| n.id == conn.to_node)?;
        let p1 = from.port_pos(PortSide::Output, conn.from_port);
        let p2 = to.port_pos(PortSide::Input, conn.to_port);

        let dx = ((p2.x - p1.x).abs() * 0.5).max(50.0);
        let cp1 = Point::new(p1.x + dx, p1.y);
        let cp2 = Point::new(p2.x - dx, p2.y);

        // sample the curve and check the distance
        for i in 0..=20 {
            let t = i as f32 / 20.0;
            let pt = bezier_eval(t, p1, cp1, cp2, p2);
            if (wx - pt.x).abs() < tol && (wy - pt.y).abs() < tol {
                return Some(ci);
            }
        }
    }
    None
}
