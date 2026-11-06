//! Recursive layout: AST tree → flattened list of rectangles to draw.
//! Handles flex (row/col), gap, padding, align, and absolute/z-stack.

use sequoia_ui::prelude::{Color, Rect};

use super::svelte_ast::*;

#[derive(Clone, Debug, PartialEq)]
pub struct FlatNode {
    pub id: u64,
    pub name: String,
    pub rect: Rect,
    pub depth: u8,
    pub visible: bool,
    pub locked: bool,
    pub kind: FlatKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FlatKind {
    Element {
        tag: Tag,
        style: Style,
        component: bool,
        instance: bool,
    },
    Text {
        content: String,
        style: TextStyle,
        color: Color,
    },
}

impl FlatNode {
    pub fn icon(&self) -> &'static str {
        match &self.kind {
            FlatKind::Element { tag, .. } => tag.icon(),
            FlatKind::Text { .. } => "tool/text",
        }
    }
    pub fn is_element(&self) -> bool {
        matches!(self.kind, FlatKind::Element { .. })
    }
    pub fn style(&self) -> Option<&Style> {
        match &self.kind {
            FlatKind::Element { style, .. } => Some(style),
            _ => None,
        }
    }
}

/// Measures text (approximate). Sufficient for layout.
fn measure_text(text: &str, font_size: f32) -> (f32, f32) {
    let line_h = font_size * 1.3;
    let w = text.chars().count() as f32 * font_size * 0.55;
    (w.max(1.0), line_h)
}

pub fn compute_layout(root: &Option<SvelteNode>) -> Vec<FlatNode> {
    let mut out = Vec::new();
    if let Some(r) = root {
        let origin = (0.0f32, 0.0f32);
        walk(r, origin.0, origin.1, 0, None, &mut out);
    }
    out
}

fn walk(
    node: &SvelteNode,
    ax: f32,
    ay: f32,
    depth: u8,
    parent_size: Option<(f32, f32)>,
    out: &mut Vec<FlatNode>,
) {
    if !node.visible() {
        return;
    }
    match node {
        SvelteNode::Element(e) => {
            let (x, y, w, h) = place_element(e, ax, ay, parent_size);
            out.push(FlatNode {
                id: e.id,
                name: e.name.clone(),
                rect: Rect::from_xywh(x, y, w, h),
                depth,
                visible: e.visible,
                locked: e.locked,
                kind: FlatKind::Element {
                    tag: e.tag,
                    style: e.style.clone(),
                    component: e.component,
                    instance: e.instance,
                },
            });
            layout_children(&e.children, &e.style, x, y, w, h, depth + 1, out);
        }
        SvelteNode::Text(t) => {
            let (tw, th) = measure_text(&t.content, t.style.font_size);
            let w = t.width.unwrap_or(tw);
            let h = t.height.unwrap_or(th);
            out.push(FlatNode {
                id: t.id,
                name: t.name.clone(),
                rect: Rect::from_xywh(ax, ay, w, h),
                depth,
                visible: t.visible,
                locked: t.locked,
                kind: FlatKind::Text {
                    content: t.content.clone(),
                    style: t.style.clone(),
                    color: t.style.color,
                },
            });
        }
    }
}

fn place_element(
    e: &Element,
    ax: f32,
    ay: f32,
    parent_size: Option<(f32, f32)>,
) -> (f32, f32, f32, f32) {
    let s = &e.style;

    // Position: absolute uses top/left, the rest (relative/static) is set by the parent.
    let x = if s.position == Position::Absolute {
        s.left.unwrap_or(ax)
    } else {
        ax
    };
    let y = if s.position == Position::Absolute {
        s.top.unwrap_or(ay)
    } else {
        ay
    };

    // Dimensions
    let (pw, ph) = parent_size.unwrap_or((0.0, 0.0));
    let w = match s.width {
        Dimension::Fixed(v) => v,
        Dimension::Fill => (pw - s.padding.left - s.padding.right).max(1.0),
        Dimension::Auto => intrinsic_width(e),
    };
    let h = match s.height {
        Dimension::Fixed(v) => v,
        Dimension::Fill => (ph - s.padding.top - s.padding.bottom).max(1.0),
        Dimension::Auto => intrinsic_height(e),
    };
    (x, y, w, h)
}

fn intrinsic_width(e: &Element) -> f32 {
    // Auto: sum children (row) or max (col)
    let s = &e.style;
    if e.children.is_empty() {
        return 40.0;
    }
    match s.display {
        Display::Flex if s.flex_dir == FlexDir::Row => {
            let mut w = s.padding.left + s.padding.right;
            let mut n = 0;
            for c in &e.children {
                w += node_width(c);
                n += 1;
            }
            if n > 1 {
                w += s.gap * (n - 1) as f32;
            }
            w
        }
        _ => {
            let mut w = s.padding.left + s.padding.right;
            for c in &e.children {
                w = w.max(node_width(c) + s.padding.left + s.padding.right);
            }
            w
        }
    }
}

fn intrinsic_height(e: &Element) -> f32 {
    let s = &e.style;
    if e.children.is_empty() {
        return 40.0;
    }
    match s.display {
        Display::Flex if s.flex_dir == FlexDir::Col => {
            let mut h = s.padding.top + s.padding.bottom;
            let mut n = 0;
            for c in &e.children {
                h += node_height(c);
                n += 1;
            }
            if n > 1 {
                h += s.gap * (n - 1) as f32;
            }
            h
        }
        _ => {
            let mut h = s.padding.top + s.padding.bottom;
            for c in &e.children {
                h = h.max(node_height(c) + s.padding.top + s.padding.bottom);
            }
            h
        }
    }
}

fn node_width(n: &SvelteNode) -> f32 {
    match n {
        SvelteNode::Element(e) => match e.style.width {
            Dimension::Fixed(v) => v,
            _ => intrinsic_width(e),
        },
        SvelteNode::Text(t) => {
            let (w, _) = measure_text(&t.content, t.style.font_size);
            t.width.unwrap_or(w)
        }
    }
}

fn node_height(n: &SvelteNode) -> f32 {
    match n {
        SvelteNode::Element(e) => match e.style.height {
            Dimension::Fixed(v) => v,
            _ => intrinsic_height(e),
        },
        SvelteNode::Text(t) => {
            let (_, h) = measure_text(&t.content, t.style.font_size);
            t.height.unwrap_or(h)
        }
    }
}

fn layout_children(
    children: &[SvelteNode],
    parent_style: &Style,
    px: f32,
    py: f32,
    pw: f32,
    ph: f32,
    depth: u8,
    out: &mut Vec<FlatNode>,
) {
    let s = parent_style;
    let ix = px + s.padding.left;
    let iy = py + s.padding.top;
    let iw = (pw - s.padding.left - s.padding.right).max(1.0);
    let ih = (ph - s.padding.top - s.padding.bottom).max(1.0);

    // Absolute children are positioned on their own
    for c in children {
        if let SvelteNode::Element(e) = c {
            if e.style.position == Position::Absolute {
                walk(c, ix, iy, depth, Some((iw, ih)), out);
            }
        }
    }

    // Flow children
    let flow: Vec<&SvelteNode> = children
        .iter()
        .filter(|c| match c {
            SvelteNode::Element(e) => e.style.position != Position::Absolute,
            _ => true,
        })
        .collect();

    if flow.is_empty() {
        return;
    }

    match s.display {
        Display::Flex if s.flex_dir == FlexDir::Row => {
            let total_w: f32 = flow.iter().map(|c| node_width(c)).sum::<f32>()
                + s.gap * (flow.len().saturating_sub(1)) as f32;
            let extra = (iw - total_w).max(0.0);
            let (start_x, gap_extra) = match s.justify_content {
                Align::Center => (ix + extra * 0.5, 0.0),
                Align::End => (ix + extra, 0.0),
                Align::SpaceBetween if flow.len() > 1 => (ix, extra / (flow.len() - 1) as f32),
                _ => (ix, 0.0),
            };
            let mut cx = start_x;
            for c in &flow {
                let cw = node_width(c);
                let ch = match c {
                    SvelteNode::Element(e) => match e.style.height {
                        Dimension::Fixed(v) => v,
                        Dimension::Fill => ih,
                        Dimension::Auto => node_height(c),
                    },
                    _ => node_height(c),
                };
                let cy = match s.align_items {
                    Align::Center => iy + (ih - ch) * 0.5,
                    Align::End => iy + ih - ch,
                    Align::Stretch => iy,
                    _ => iy,
                };
                walk(c, cx, cy, depth, Some((cw, ch)), out);
                cx += cw + s.gap + gap_extra;
            }
        }
        Display::Flex if s.flex_dir == FlexDir::Col => {
            let total_h: f32 = flow.iter().map(|c| node_height(c)).sum::<f32>()
                + s.gap * (flow.len().saturating_sub(1)) as f32;
            let extra = (ih - total_h).max(0.0);
            let (start_y, gap_extra) = match s.justify_content {
                Align::Center => (iy + extra * 0.5, 0.0),
                Align::End => (iy + extra, 0.0),
                Align::SpaceBetween if flow.len() > 1 => (iy, extra / (flow.len() - 1) as f32),
                _ => (iy, 0.0),
            };
            let mut cy = start_y;
            for c in &flow {
                let ch = node_height(c);
                let cw = match c {
                    SvelteNode::Element(e) => match e.style.width {
                        Dimension::Fixed(v) => v,
                        Dimension::Fill => iw,
                        Dimension::Auto => node_width(c),
                    },
                    _ => node_width(c),
                };
                let cx = match s.align_items {
                    Align::Center => ix + (iw - cw) * 0.5,
                    Align::End => ix + iw - cw,
                    Align::Stretch => ix,
                    _ => ix,
                };
                walk(c, cx, cy, depth, Some((cw, ch)), out);
                cy += ch + s.gap + gap_extra;
            }
        }
        Display::Grid => {
            let cols = s.grid_cols.unwrap_or(2).max(1) as usize;
            let cell_w = (iw - s.gap * (cols as f32 - 1.0)) / cols as f32;
            let mut cx = ix;
            let mut cy = iy;
            let mut row_h = 0.0f32;
            let mut i = 0;
            for c in &flow {
                if i > 0 && i % cols == 0 {
                    cx = ix;
                    cy += row_h + s.gap;
                    row_h = 0.0;
                }
                let ch = node_height(c);
                walk(c, cx, cy, depth, Some((cell_w, ch)), out);
                row_h = row_h.max(ch);
                cx += cell_w + s.gap;
                i += 1;
            }
        }
        _ => {
            // Display::None — treat as z-stack (free children, absolute-like)
            for c in &flow {
                walk(c, ix, iy, depth, Some((iw, ih)), out);
            }
        }
    }
}
