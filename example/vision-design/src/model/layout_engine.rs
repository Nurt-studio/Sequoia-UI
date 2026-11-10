//! Recursive layout: AST tree → flattened list of rectangles to draw.
//!
//! Flex layout follows the same rules as the CSS that `codegen` generates, so
//! the canvas and the exported Svelte agree: item and line spacing, wrapping,
//! fill (`flex-grow`, `align-self: stretch`) and hug (auto) sizing, alignment
//! of items and wrapped lines, plus absolutely positioned children.

use sequoia_ui::prelude::{Color, Rect};

use super::behavior::Value;
use super::component_ops::{inject_slots, slot_container_names};
use super::svelte_ast::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub struct FlatNode {
    pub id: u64,
    pub name: String,
    pub rect: Rect,
    pub depth: u8,
    pub visible: bool,
    pub locked: bool,
    pub parent: Option<u64>,
    /// Slot container of a master (◇) — drawn as the dashed "hole".
    pub slot: Option<String>,
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
        /// Positioned on its own instead of in the parent's flow.
        absolute: bool,
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

    /// Element or text opacity in percent (100 = opaque).
    pub fn opacity(&self) -> f32 {
        match &self.kind {
            FlatKind::Element { style, .. } => style.opacity,
            FlatKind::Text { style, .. } => style.opacity,
        }
    }
}

/// Measures text (approximate). Sufficient for layout.
fn measure_text(text: &str, font_size: f32) -> (f32, f32) {
    let line_h = font_size * 1.3;
    let w = text.chars().count() as f32 * font_size * 0.55;
    (w.max(1.0), line_h)
}

fn text_size(t: &TextNode) -> (f32, f32) {
    let (tw, th) = measure_text(&t.content, t.style.font_size);
    (t.width.unwrap_or(tw), t.height.unwrap_or(th))
}

/// Runtime variable store of the Present interpreter: (owner id, name) → value.
pub type PresentVars = HashMap<(u64, String), Value>;

/// Everything the layout needs from the Present interpreter: live variable
/// values plus the elements that are animating OUT (they stay rendered until
/// their leave transition finishes).
pub struct PresentRuntime {
    pub vars: PresentVars,
    pub keep_visible: std::collections::HashSet<u64>,
    /// Screen the presenter navigated to. Its ancestors and subtree are the
    /// only layers that stay rendered; `None` shows the whole document.
    pub focus: Option<u64>,
    /// Overlay frames shown above the focused screen.
    pub overlays: Vec<u64>,
    /// Screen animating out — kept rendered until its transition finishes.
    pub leaving: Option<u64>,
}

impl Default for PresentRuntime {
    fn default() -> Self {
        Self {
            vars: PresentVars::new(),
            keep_visible: std::collections::HashSet::new(),
            focus: None,
            overlays: Vec::new(),
            leaving: None,
        }
    }
}

pub fn compute_layout(root: &Option<SvelteNode>) -> Vec<FlatNode> {
    compute_layout_with_runtime(root, &PresentRuntime::default())
}

/// Layout with the Present interpreter's runtime: bindings resolve against
/// the live variable values (design doc §7), slots are materialized and
/// hidden subtrees stay out of the flat list.
pub fn compute_layout_with_runtime(
    root: &Option<SvelteNode>,
    runtime: &PresentRuntime,
) -> Vec<FlatNode> {
    let mut out = Vec::new();
    let Some(r) = root else {
        return out;
    };
    // Slots first (ids are preserved), then visibility: the runtime decides
    // per binding, and a hidden element hides its whole subtree (like `{#if}`).
    let mut tree = inject_slots(r);
    let mut stack: Vec<u64> = Vec::new();
    let keep = focus_keep(&tree, runtime);
    apply_runtime_visibility(&mut tree, &mut stack, runtime, keep.as_ref());
    let slots = slot_container_names(r);
    walk(&tree, 0.0, 0.0, 0, (0.0, 0.0), None, &mut out);
    // Mark the slot containers ("hole" overlay on the canvas).
    if !slots.is_empty() {
        for f in out.iter_mut() {
            if let Some(name) = slots.get(&f.id) {
                f.slot = Some(name.clone());
            }
        }
    }
    out
}

/// Bakes visibility into the prepared copy: bindings evaluate against the
/// runtime values, and `visible = false` cascades to the whole subtree.
/// `stack` carries the ancestor element ids (outermost first).
fn apply_runtime_visibility(
    node: &mut SvelteNode,
    stack: &mut Vec<u64>,
    runtime: &PresentRuntime,
    keep: Option<&HashSet<u64>>,
) {
    let forced = runtime.keep_visible.contains(&node.id());
    let off_screen = keep.is_some_and(|keep| !keep.contains(&node.id()));
    let hidden = match node {
        SvelteNode::Element(e) if !e.visible => true,
        SvelteNode::Text(t) if !t.visible => true,
        SvelteNode::Element(e) => {
            off_screen
                || match &e.binding {
                    Some(b) => {
                        !forced && !runtime_value_in_scope(stack, e, &b.var, &runtime.vars).truthy()
                    }
                    None => false,
                }
        }
        _ => off_screen,
    };
    if hidden {
        hide_subtree(node);
        return;
    }
    stack.push(node.id());
    if let Some(children) = node.children_mut() {
        for c in children {
            apply_runtime_visibility(c, stack, runtime, keep);
        }
    }
    if let SvelteNode::Element(e) = node {
        for sc in &mut e.slot_content {
            for n in sc.nodes.iter_mut() {
                apply_runtime_visibility(n, stack, runtime, keep);
            }
        }
    }
    stack.pop();
}

/// Nodes that stay rendered while a screen is focused: the path from the
/// root to each target, the target's subtree, and the same for overlays and
/// the screen animating out.
fn focus_keep(root: &SvelteNode, runtime: &PresentRuntime) -> Option<HashSet<u64>> {
    let mut targets: Vec<u64> = Vec::new();
    if let Some(id) = runtime.focus {
        targets.push(id);
    }
    if let Some(id) = runtime.leaving
        && runtime.focus != Some(id)
    {
        targets.push(id);
    }
    for id in &runtime.overlays {
        if !targets.contains(id) {
            targets.push(*id);
        }
    }
    if targets.is_empty() {
        return None;
    }
    let mut keep = HashSet::new();
    for id in targets {
        let mut path = Vec::new();
        if !find_path(root, id, &mut path) {
            continue;
        }
        keep.extend(path.iter().copied());
        if let Some(node) = crate::model::tree_ops::find(root, id) {
            mark_subtree(node, &mut keep);
        }
    }
    (!keep.is_empty()).then_some(keep)
}

fn find_path(node: &SvelteNode, id: u64, out: &mut Vec<u64>) -> bool {
    out.push(node.id());
    if node.id() == id {
        return true;
    }
    for c in node.children() {
        if find_path(c, id, out) {
            return true;
        }
    }
    out.pop();
    false
}

fn mark_subtree(node: &SvelteNode, out: &mut HashSet<u64>) {
    out.insert(node.id());
    for c in node.children() {
        mark_subtree(c, out);
    }
}

/// Runtime value of a binding variable: the innermost element on the path
/// that declares it wins (same shadowing as `behavior::scope_vars`); unknown
/// variables read as `false`, like an unset `{#if}`.
fn runtime_value_in_scope(
    stack: &[u64],
    self_el: &Element,
    name: &str,
    vars: &PresentVars,
) -> Value {
    // The element's own behavior first.
    if let Some(beh) = &self_el.behavior
        && beh.var(name).is_some()
    {
        return vars
            .get(&(self_el.id, name.to_string()))
            .cloned()
            .unwrap_or_else(|| {
                beh.var(name)
                    .map(|v| v.initial.clone())
                    .unwrap_or(Value::Bool(false))
            });
    }
    // Then the ancestors, innermost first.
    for id in stack.iter().rev() {
        if let Some(v) = vars.get(&(*id, name.to_string())) {
            return v.clone();
        }
    }
    Value::Bool(false)
}

fn hide_subtree(node: &mut SvelteNode) {
    match node {
        SvelteNode::Element(e) => {
            e.visible = false;
            for c in e.children.iter_mut() {
                hide_subtree(c);
            }
            for sc in &mut e.slot_content {
                for n in sc.nodes.iter_mut() {
                    hide_subtree(n);
                }
            }
        }
        SvelteNode::Text(t) => t.visible = false,
    }
}

pub(crate) fn is_absolute(node: &SvelteNode) -> bool {
    match node {
        SvelteNode::Element(e) => e.style.position == Position::Absolute,
        SvelteNode::Text(t) => t.absolute,
    }
}

/// Natural (hug) width of a node.
fn base_width(n: &SvelteNode) -> f32 {
    match n {
        SvelteNode::Element(e) => match e.style.width {
            Dimension::Fixed(v) => v,
            _ => intrinsic_width(e),
        },
        SvelteNode::Text(t) => text_size(t).0,
    }
}

/// Natural (hug) height of a node.
fn base_height(n: &SvelteNode) -> f32 {
    match n {
        SvelteNode::Element(e) => match e.style.height {
            Dimension::Fixed(v) => v,
            _ => intrinsic_height(e),
        },
        SvelteNode::Text(t) => text_size(t).1,
    }
}

fn main_fills(n: &SvelteNode, row: bool) -> bool {
    match n {
        SvelteNode::Element(e) => {
            if row {
                e.style.width.is_fill()
            } else {
                e.style.height.is_fill()
            }
        }
        SvelteNode::Text(_) => false,
    }
}

fn cross_fills(n: &SvelteNode, row: bool) -> bool {
    match n {
        SvelteNode::Element(e) => {
            if row {
                e.style.height.is_fill()
            } else {
                e.style.width.is_fill()
            }
        }
        SvelteNode::Text(_) => false,
    }
}

fn align_self(n: &SvelteNode) -> Option<Align> {
    match n {
        SvelteNode::Element(e) => e.style.align_self,
        SvelteNode::Text(_) => None,
    }
}

/// A fixed cross size wins over `align-items: stretch`, like in CSS.
fn cross_fixed(n: &SvelteNode, row: bool) -> bool {
    match n {
        SvelteNode::Element(e) => {
            if row {
                e.style.height.value().is_some()
            } else {
                e.style.width.value().is_some()
            }
        }
        SvelteNode::Text(t) => {
            if row {
                t.height.is_some()
            } else {
                t.width.is_some()
            }
        }
    }
}

/// Places a node at a position the parent already resolved (flow placement).
fn walk(
    node: &SvelteNode,
    ax: f32,
    ay: f32,
    depth: u8,
    avail: (f32, f32),
    parent: Option<u64>,
    out: &mut Vec<FlatNode>,
) {
    match node {
        SvelteNode::Element(e) => {
            let absolute = e.style.position == Position::Absolute;
            let x = if absolute {
                e.style.left.unwrap_or(ax)
            } else {
                ax
            };
            let y = if absolute {
                e.style.top.unwrap_or(ay)
            } else {
                ay
            };
            let w = match e.style.width {
                Dimension::Fixed(v) => v,
                Dimension::Fill => avail.0.max(1.0),
                Dimension::Auto => intrinsic_width(e),
            };
            let h = match e.style.height {
                Dimension::Fixed(v) => v,
                Dimension::Fill => avail.1.max(1.0),
                Dimension::Auto => intrinsic_height(e),
            };
            place(node, x, y, w, h, depth, parent, out);
        }
        SvelteNode::Text(t) => {
            let (x, y) = if t.absolute { (t.x, t.y) } else { (ax, ay) };
            let (w, h) = text_size(t);
            place(node, x, y, w, h, depth, parent, out);
        }
    }
}

/// Emits the node at an exact rectangle and lays out its children inside it.
fn place(
    node: &SvelteNode,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    depth: u8,
    parent: Option<u64>,
    out: &mut Vec<FlatNode>,
) {
    match node {
        SvelteNode::Element(e) => {
            out.push(FlatNode {
                id: e.id,
                name: e.name.clone(),
                rect: Rect::from_xywh(x, y, w, h),
                depth,
                visible: e.visible,
                locked: e.locked,
                parent,
                slot: None,
                kind: FlatKind::Element {
                    tag: e.tag,
                    style: e.style.clone(),
                    component: e.component,
                    instance: e.instance,
                },
            });
            layout_children(&e.children, &e.style, e.id, x, y, w, h, depth + 1, out);
        }
        SvelteNode::Text(t) => {
            out.push(FlatNode {
                id: t.id,
                name: t.name.clone(),
                rect: Rect::from_xywh(x, y, w, h),
                depth,
                visible: t.visible,
                locked: t.locked,
                parent,
                slot: None,
                kind: FlatKind::Text {
                    content: t.content.clone(),
                    style: t.style.clone(),
                    color: t.style.color,
                    absolute: t.absolute,
                },
            });
        }
    }
}

/// Content box of an element.
fn inner_box(px: f32, py: f32, pw: f32, ph: f32, s: &Style) -> (f32, f32, f32, f32) {
    (
        px + s.padding.left,
        py + s.padding.top,
        (pw - s.padding.left - s.padding.right).max(0.0),
        (ph - s.padding.top - s.padding.bottom).max(0.0),
    )
}

fn layout_children(
    children: &[SvelteNode],
    parent_style: &Style,
    parent_id: u64,
    px: f32,
    py: f32,
    pw: f32,
    ph: f32,
    depth: u8,
    out: &mut Vec<FlatNode>,
) {
    match parent_style.display {
        Display::Flex => layout_flex(
            children,
            parent_style,
            parent_id,
            px,
            py,
            pw,
            ph,
            depth,
            out,
        ),
        Display::Grid => layout_grid(
            children,
            parent_style,
            parent_id,
            px,
            py,
            pw,
            ph,
            depth,
            out,
        ),
        // Z-stack: children keep their own position, pinning is applied by the
        // editor's constraints, not by layout.
        Display::None => {
            let (ix, iy, iw, ih) = inner_box(px, py, pw, ph, parent_style);
            for c in children {
                walk(c, ix, iy, depth, (iw, ih), Some(parent_id), out);
            }
        }
    }
}

fn layout_flex(
    children: &[SvelteNode],
    s: &Style,
    parent_id: u64,
    px: f32,
    py: f32,
    pw: f32,
    ph: f32,
    depth: u8,
    out: &mut Vec<FlatNode>,
) {
    let (ix, iy, iw, ih) = inner_box(px, py, pw, ph, s);
    let row = s.flex_dir == FlexDir::Row;
    let main = if row { iw } else { ih };
    let cross = if row { ih } else { iw };
    let gap = s.gap.max(0.0);
    let line_gap = s.row_gap.max(0.0);

    // Absolute children are taken out of the flow.
    for c in children.iter().filter(|c| is_absolute(c)) {
        walk(c, ix, iy, depth, (iw, ih), Some(parent_id), out);
    }

    let items: Vec<&SvelteNode> = children.iter().filter(|c| !is_absolute(c)).collect();
    if items.is_empty() {
        return;
    }

    let base_main: Vec<f32> = items
        .iter()
        .map(|c| if row { base_width(c) } else { base_height(c) })
        .collect();
    let natural_cross: Vec<f32> = items
        .iter()
        .map(|c| if row { base_height(c) } else { base_width(c) })
        .collect();
    let fill_main: Vec<bool> = items.iter().map(|c| main_fills(c, row)).collect();
    let fill_cross: Vec<bool> = items.iter().map(|c| cross_fills(c, row)).collect();
    let fixed_cross: Vec<bool> = items.iter().map(|c| cross_fixed(c, row)).collect();

    // ── line breaking, based on flex base sizes like CSS does ──
    let mut lines: Vec<Vec<usize>> = Vec::new();
    if s.flex_wrap == Wrap::Wrap && main > 0.0 {
        let mut line: Vec<usize> = Vec::new();
        let mut used = 0.0;
        for (i, base) in base_main.iter().enumerate() {
            let extra = if line.is_empty() { *base } else { gap + *base };
            if !line.is_empty() && used + extra > main + 0.01 {
                lines.push(std::mem::take(&mut line));
                used = *base;
            } else {
                used += extra;
            }
            line.push(i);
        }
        if !line.is_empty() {
            lines.push(line);
        }
    } else {
        lines.push((0..items.len()).collect());
    }

    // ── cross size of each line, then distribute the lines ──
    let mut line_cross: Vec<f32> = lines
        .iter()
        .map(|line| line.iter().map(|&i| natural_cross[i]).fold(0.0, f32::max))
        .collect();
    if lines.len() == 1 {
        // A single line spans the container, so item alignment and stretching
        // are relative to the container, as in CSS.
        line_cross[0] = cross;
        lines[0].shrink_to_fit();
    }
    let total_cross =
        line_cross.iter().sum::<f32>() + line_gap * (lines.len() as f32 - 1.0).max(0.0);
    let free_cross = (cross - total_cross).max(0.0);
    let mut start_cross = 0.0;
    let mut extra_cross = 0.0;
    match s.align_content {
        Align::Center => start_cross = free_cross * 0.5,
        Align::End => start_cross = free_cross,
        Align::SpaceBetween if lines.len() > 1 => {
            extra_cross = free_cross / (lines.len() - 1) as f32;
        }
        Align::Stretch if lines.len() > 1 => {
            let each = free_cross / lines.len() as f32;
            for v in &mut line_cross {
                *v += each;
            }
        }
        _ => {}
    }

    let mut line_offset = start_cross;
    for (li, line) in lines.iter().enumerate() {
        let n = line.len();
        let gaps = gap * (n.saturating_sub(1)) as f32;
        let base_used: f32 = line.iter().map(|&i| base_main[i]).sum::<f32>() + gaps;
        let free = (main - base_used).max(0.0);
        let flex_count = line.iter().filter(|&&i| fill_main[i]).count();
        let share = if flex_count > 0 {
            free / flex_count as f32
        } else {
            0.0
        };
        let sizes: Vec<f32> = line
            .iter()
            .map(|&i| if fill_main[i] { share } else { base_main[i] })
            .collect();
        let used: f32 = sizes.iter().sum::<f32>() + gaps;
        let leftover = (main - used).max(0.0);
        let (mut pos, extra_gap) = match s.justify_content {
            Align::Center => (leftover * 0.5, 0.0),
            Align::End => (leftover, 0.0),
            Align::SpaceBetween if n > 1 => (0.0, leftover / (n - 1) as f32),
            _ => (0.0, 0.0),
        };

        for (k, &i) in line.iter().enumerate() {
            let cross_align = align_self(items[i]).unwrap_or(s.align_items);
            let stretches = fill_cross[i] || (!fixed_cross[i] && cross_align == Align::Stretch);
            let cross_size = if stretches {
                line_cross[li]
            } else {
                natural_cross[i]
            };
            let cross_off = match cross_align {
                Align::Center => (line_cross[li] - cross_size) * 0.5,
                Align::End => line_cross[li] - cross_size,
                _ => 0.0,
            };
            let (x, y, w, h) = if row {
                (ix + pos, iy + line_offset + cross_off, sizes[k], cross_size)
            } else {
                (ix + line_offset + cross_off, iy + pos, cross_size, sizes[k])
            };
            place(items[i], x, y, w, h, depth, Some(parent_id), out);
            pos += sizes[k] + gap + extra_gap;
        }

        line_offset += line_cross[li] + line_gap + extra_cross;
    }
}

fn layout_grid(
    children: &[SvelteNode],
    s: &Style,
    parent_id: u64,
    px: f32,
    py: f32,
    pw: f32,
    ph: f32,
    depth: u8,
    out: &mut Vec<FlatNode>,
) {
    let (ix, iy, iw, ih) = inner_box(px, py, pw, ph, s);
    for c in children.iter().filter(|c| is_absolute(c)) {
        walk(c, ix, iy, depth, (iw, ih), Some(parent_id), out);
    }

    let cols = s.grid_cols.unwrap_or(2).max(1) as usize;
    let explicit_rows = s.grid_rows.unwrap_or(0) as usize;
    let gap = s.gap.max(0.0);
    let row_gap = if s.row_gap > 0.0 { s.row_gap } else { gap };
    let cell_w = ((iw - gap * (cols as f32 - 1.0)) / cols as f32).max(0.0);
    let track_h = if explicit_rows > 0 && ih > 0.0 {
        ((ih - row_gap * (explicit_rows as f32 - 1.0)) / explicit_rows as f32).max(0.0)
    } else {
        0.0
    };
    let items: Vec<&SvelteNode> = children.iter().filter(|c| !is_absolute(c)).collect();
    let mut cy = iy;
    for (ri, row) in items.chunks(cols).enumerate() {
        let natural_h = row.iter().map(|c| base_height(c)).fold(0.0, f32::max);
        let row_h = if explicit_rows > 0 && ri < explicit_rows && ih > 0.0 {
            track_h
        } else {
            natural_h
        };
        for (i, c) in row.iter().enumerate() {
            let v_align = align_self(c).unwrap_or(s.align_items);
            let (cw, ch) = grid_child_size(c, cell_w, row_h, s, v_align);
            let x_off = match s.justify_content {
                Align::Center => (cell_w - cw) * 0.5,
                Align::End => cell_w - cw,
                _ => 0.0,
            };
            let y_off = match v_align {
                Align::Center => (row_h - ch) * 0.5,
                Align::End => row_h - ch,
                _ => 0.0,
            };
            place(
                c,
                ix + i as f32 * (cell_w + gap) + x_off,
                cy + y_off,
                cw,
                ch,
                depth,
                Some(parent_id),
                out,
            );
        }
        cy += row_h + row_gap;
    }
}

/// Size of a grid item inside its cell. Fill takes the cell, stretch takes the
/// track when the item has no explicit size, everything else stays intrinsic.
fn grid_child_size(
    n: &SvelteNode,
    cell_w: f32,
    row_h: f32,
    s: &Style,
    v_align: Align,
) -> (f32, f32) {
    let stretch_w = s.justify_content == Align::Stretch;
    let stretch_h = v_align == Align::Stretch;
    match n {
        SvelteNode::Element(e) => {
            let w = match e.style.width {
                Dimension::Fixed(v) => v,
                Dimension::Fill => cell_w,
                Dimension::Auto if stretch_w => cell_w,
                Dimension::Auto => intrinsic_width(e),
            };
            let h = match e.style.height {
                Dimension::Fixed(v) => v,
                Dimension::Fill => row_h,
                Dimension::Auto if stretch_h => row_h,
                Dimension::Auto => intrinsic_height(e),
            };
            (w, h)
        }
        SvelteNode::Text(t) => {
            let (tw, th) = text_size(t);
            (
                t.width.unwrap_or(if stretch_w { cell_w } else { tw }),
                t.height.unwrap_or(if stretch_h { row_h } else { th }),
            )
        }
    }
}

fn flow_children(e: &Element) -> Vec<&SvelteNode> {
    e.children.iter().filter(|c| !is_absolute(c)).collect()
}

fn intrinsic_width(e: &Element) -> f32 {
    let s = &e.style;
    let flow = flow_children(e);
    let pad = s.padding.left + s.padding.right;
    let gap = s.gap.max(0.0);
    match s.display {
        Display::Flex if s.flex_dir == FlexDir::Row => {
            let sum: f32 = flow.iter().map(|c| base_width(c)).sum();
            pad + sum + gap * (flow.len() as f32 - 1.0).max(0.0)
        }
        Display::Grid => {
            let cols = s.grid_cols.unwrap_or(2).max(1) as f32;
            let cell = flow.iter().map(|c| base_width(c)).fold(0.0, f32::max);
            pad + cell * cols + gap * (cols - 1.0)
        }
        _ => pad + flow.iter().map(|c| base_width(c)).fold(0.0, f32::max),
    }
}

fn intrinsic_height(e: &Element) -> f32 {
    let s = &e.style;
    let flow = flow_children(e);
    let pad = s.padding.top + s.padding.bottom;
    let gap = s.gap.max(0.0);
    match s.display {
        Display::Flex if s.flex_dir == FlexDir::Col => {
            let sum: f32 = flow.iter().map(|c| base_height(c)).sum();
            pad + sum + gap * (flow.len() as f32 - 1.0).max(0.0)
        }
        Display::Grid => {
            let cols = s.grid_cols.unwrap_or(2).max(1) as f32;
            let rows = (flow.len() as f32 / cols).ceil().max(1.0);
            let cell = flow.iter().map(|c| base_height(c)).fold(0.0, f32::max);
            pad + cell * rows + gap * (rows - 1.0)
        }
        _ => pad + flow.iter().map(|c| base_height(c)).fold(0.0, f32::max),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(id: u64, w: Option<f32>, h: Option<f32>, dir: FlexDir) -> Element {
        Element {
            id,
            name: "Frame".into(),
            tag: Tag::Div,
            style: Style {
                display: Display::Flex,
                flex_dir: dir,
                width: w.map(Dimension::Fixed).unwrap_or(Dimension::Auto),
                height: h.map(Dimension::Fixed).unwrap_or(Dimension::Auto),
                ..Default::default()
            },
            children: Vec::new(),
            visible: true,
            locked: false,
            component: false,
            instance: false,
            interface: None,
            overrides: Vec::new(),
            slot_content: Vec::new(),
            behavior: None,
            binding: None,
            hover: None,
            prototype: None,
        }
    }

    /// A fixed-size box in the parent's flow (`new_rect` is absolute).
    fn rect(id: u64, w: f32, h: f32) -> SvelteNode {
        let mut node = new_rect(id, 0.0, 0.0, w, h, 0x0D99FF);
        if let SvelteNode::Element(e) = &mut node {
            e.style.position = Position::Relative;
        }
        node
    }

    /// A flow box that hugs its contents on the vertical axis.
    fn hug_rect(id: u64, w: f32) -> SvelteNode {
        let mut node = rect(id, w, 0.0);
        if let SvelteNode::Element(e) = &mut node {
            e.style.height = Dimension::Auto;
        }
        node
    }

    fn flat_rect(root: &SvelteNode, id: u64) -> Rect {
        let flat = compute_layout(&Some(root.clone()));
        flat.iter()
            .find(|f| f.id == id)
            .map(|f| f.rect)
            .unwrap_or_else(|| panic!("node {id} not laid out"))
    }

    #[test]
    fn present_focus_hides_the_other_screens() {
        let mut a = frame(2, Some(100.0), Some(100.0), FlexDir::Col);
        a.name = "Home".into();
        let mut b = frame(3, Some(100.0), Some(100.0), FlexDir::Col);
        b.name = "Details".into();
        let mut root = frame(1, Some(300.0), Some(200.0), FlexDir::Row);
        root.children = vec![SvelteNode::Element(a), SvelteNode::Element(b)];
        let root = Some(SvelteNode::Element(root));

        let runtime = PresentRuntime {
            focus: Some(2),
            ..Default::default()
        };
        let flat = compute_layout_with_runtime(&root, &runtime);
        assert!(
            flat.iter().any(|f| f.id == 2 && f.visible),
            "focused screen is laid out"
        );
        assert!(
            !flat.iter().any(|f| f.id == 3 && f.visible),
            "sibling screens stay hidden while focused"
        );
    }

    #[test]
    fn row_places_children_with_gap() {
        let mut f = frame(1, Some(400.0), Some(100.0), FlexDir::Row);
        f.style.gap = 10.0;
        f.children = vec![rect(2, 100.0, 20.0), rect(3, 50.0, 20.0)];
        let root = SvelteNode::Element(f);

        assert_eq!(flat_rect(&root, 2).left(), 0.0);
        assert_eq!(flat_rect(&root, 3).left(), 110.0);
    }

    #[test]
    fn grid_honors_columns_gaps_and_cell_alignment() {
        let mut f = frame(1, Some(320.0), Some(200.0), FlexDir::Row);
        f.style.display = Display::Grid;
        f.style.grid_cols = Some(2);
        f.style.gap = 10.0;
        f.style.row_gap = 20.0;
        f.style.justify_content = Align::End;
        f.style.align_items = Align::End;
        f.children = vec![
            rect(2, 40.0, 30.0),
            rect(3, 60.0, 50.0),
            rect(4, 40.0, 30.0),
        ];
        let root = SvelteNode::Element(f);

        let a = flat_rect(&root, 2);
        let b = flat_rect(&root, 3);
        let c = flat_rect(&root, 4);
        assert_eq!((a.left(), a.top()), (115.0, 20.0), "cell 1, bottom right");
        assert_eq!((b.left(), b.top()), (260.0, 0.0), "cell 2, top right");
        assert_eq!((c.left(), c.top()), (115.0, 70.0), "second row");
    }

    #[test]
    fn grid_rows_distribute_the_container_height() {
        let mut f = frame(1, Some(320.0), Some(240.0), FlexDir::Row);
        f.style.display = Display::Grid;
        f.style.grid_cols = Some(2);
        f.style.grid_rows = Some(2);
        f.style.gap = 10.0;
        f.style.row_gap = 20.0;
        f.children = vec![
            rect(2, 40.0, 30.0),
            rect(3, 60.0, 50.0),
            rect(4, 40.0, 30.0),
        ];
        let root = SvelteNode::Element(f);

        assert_eq!(flat_rect(&root, 2).top(), 0.0);
        assert_eq!(flat_rect(&root, 3).top(), 0.0);
        assert_eq!(
            flat_rect(&root, 4).top(),
            130.0,
            "second track is 110px plus a 20px gap"
        );
    }

    #[test]
    fn align_self_overrides_cross_alignment_per_item() {
        let mut f = frame(1, Some(400.0), Some(100.0), FlexDir::Row);
        f.style.align_items = Align::Start;
        let mut centered = rect(2, 100.0, 20.0);
        if let SvelteNode::Element(e) = &mut centered {
            e.style.align_self = Some(Align::Center);
        }
        f.children = vec![centered, rect(3, 100.0, 20.0)];
        let root = SvelteNode::Element(f);

        assert_eq!(flat_rect(&root, 2).top(), 40.0);
        assert_eq!(flat_rect(&root, 3).top(), 0.0);
    }

    #[test]
    fn grid_align_self_overrides_row_alignment() {
        let mut f = frame(1, Some(200.0), Some(200.0), FlexDir::Row);
        f.style.display = Display::Grid;
        f.style.grid_cols = Some(1);
        f.style.grid_rows = Some(1);
        f.style.align_items = Align::Start;
        let mut end = rect(2, 40.0, 30.0);
        if let SvelteNode::Element(e) = &mut end {
            e.style.align_self = Some(Align::End);
        }
        f.children = vec![end];
        let root = SvelteNode::Element(f);

        assert_eq!(flat_rect(&root, 2).top(), 170.0);
    }

    #[test]
    fn wrap_moves_overflowing_items_to_the_next_line() {
        let mut f = frame(1, Some(300.0), Some(200.0), FlexDir::Row);
        f.style.gap = 10.0;
        f.style.row_gap = 10.0;
        f.style.flex_wrap = Wrap::Wrap;
        f.children = vec![
            rect(2, 120.0, 40.0),
            rect(3, 120.0, 40.0),
            rect(4, 120.0, 40.0),
        ];
        let root = SvelteNode::Element(f);

        assert_eq!(flat_rect(&root, 2).top(), 0.0);
        assert_eq!(flat_rect(&root, 3).left(), 130.0);
        let third = flat_rect(&root, 4);
        assert_eq!(third.left(), 0.0, "third item starts a new line");
        assert_eq!(
            third.top(),
            50.0,
            "new line follows the first line + row gap"
        );
    }

    #[test]
    fn fill_child_takes_the_remaining_main_space() {
        let mut f = frame(1, Some(400.0), Some(100.0), FlexDir::Row);
        let mut filled = rect(3, 0.0, 20.0);
        if let SvelteNode::Element(e) = &mut filled {
            e.style.width = Dimension::Fill;
        }
        f.children = vec![rect(2, 100.0, 20.0), filled];
        let root = SvelteNode::Element(f);

        let r = flat_rect(&root, 3);
        assert_eq!(r.left(), 100.0);
        assert!((r.size.width - 300.0).abs() < 0.01, "fill width: {:?}", r);
    }

    #[test]
    fn stretch_grows_hug_children_on_the_cross_axis() {
        let mut f = frame(1, Some(400.0), Some(100.0), FlexDir::Row);
        f.style.align_items = Align::Stretch;
        f.children = vec![hug_rect(2, 100.0)];
        let root = SvelteNode::Element(f);

        assert!((flat_rect(&root, 2).size.height - 100.0).abs() < 0.01);
    }

    #[test]
    fn hug_frame_sizes_itself_from_children() {
        let mut f = frame(1, None, None, FlexDir::Row);
        f.style.gap = 10.0;
        f.style.padding = sequoia_ui::prelude::Edges::all(8.0);
        f.children = vec![rect(2, 50.0, 20.0), rect(3, 60.0, 20.0)];
        let root = SvelteNode::Element(f);

        let r = flat_rect(&root, 1);
        assert!(
            (r.size.width - (16.0 + 50.0 + 10.0 + 60.0)).abs() < 0.01,
            "{r:?}"
        );
        assert!((r.size.height - (16.0 + 20.0)).abs() < 0.01, "{r:?}");
    }

    #[test]
    fn center_aligns_a_single_line_on_the_cross_axis() {
        let mut f = frame(1, Some(400.0), Some(100.0), FlexDir::Row);
        f.style.align_items = Align::Center;
        f.children = vec![rect(2, 100.0, 20.0)];
        let root = SvelteNode::Element(f);

        assert_eq!(flat_rect(&root, 2).top(), 40.0);
    }
}
