//! AST tree operations.

use super::svelte_ast::*;

pub fn find<'a>(node: &'a SvelteNode, id: u64) -> Option<&'a SvelteNode> {
    if node.id() == id {
        return Some(node);
    }
    for c in node.children() {
        if let Some(f) = find(c, id) {
            return Some(f);
        }
    }
    None
}

pub fn find_mut<'a>(node: &'a mut SvelteNode, id: u64) -> Option<&'a mut SvelteNode> {
    if node.id() == id {
        return Some(node);
    }
    if let Some(children) = node.children_mut() {
        for c in children.iter_mut() {
            if let Some(f) = find_mut(c, id) {
                return Some(f);
            }
        }
    }
    None
}

pub fn parent_of<'a>(node: &'a SvelteNode, id: u64) -> Option<&'a SvelteNode> {
    for c in node.children() {
        if c.id() == id {
            return Some(node);
        }
        if let Some(p) = parent_of(c, id) {
            return Some(p);
        }
    }
    None
}

pub fn remove(root: &mut Option<SvelteNode>, id: u64) -> Option<SvelteNode> {
    if let Some(r) = root {
        if r.id() == id {
            let taken = r.clone();
            *root = None;
            return Some(taken);
        }
        return remove_in(r, id);
    }
    None
}

fn remove_in(node: &mut SvelteNode, id: u64) -> Option<SvelteNode> {
    if let Some(children) = node.children_mut() {
        if let Some(pos) = children.iter().position(|c| c.id() == id) {
            return Some(children.remove(pos));
        }
        for c in children.iter_mut() {
            if let Some(found) = remove_in(c, id) {
                return Some(found);
            }
        }
    }
    None
}

/// Inserts `child` into the given parent (at the end).
pub fn insert_child(root: &mut Option<SvelteNode>, parent_id: u64, child: SvelteNode) -> bool {
    let Some(r) = root else {
        if parent_id == 0 {
            *root = Some(child);
            return true;
        }
        return false;
    };
    if let Some(p) = find_mut(r, parent_id) {
        if let Some(ch) = p.children_mut() {
            ch.push(child);
            return true;
        }
    }
    false
}

/// Finds the deepest container at a point (world coords). Returns its id.
pub fn deepest_container_at(
    flat: &[super::layout_engine::FlatNode],
    wx: f32,
    wy: f32,
) -> Option<u64> {
    let mut best: Option<(u8, u64)> = None;
    for f in flat {
        let Style { display, .. } = match f.style() {
            Some(s) => s,
            None => continue,
        };
        let is_container = !matches!(display, Display::None) || f.depth == 0;
        if !is_container {
            continue;
        }
        let r = f.rect;
        if wx >= r.left()
            && wx <= r.left() + r.size.width
            && wy >= r.top()
            && wy <= r.top() + r.size.height
        {
            if best.map_or(true, |(d, _)| f.depth >= d) {
                best = Some((f.depth, f.id));
            }
        }
    }
    best.map(|(_, id)| id)
}
