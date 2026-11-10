//! Layer list as a collapsible tree (page layers + components).
//!
//! The page section lists everything that lives on the page; the components
//! section lists each component master as a root with its own subtree. Both
//! sections share the collapse state and the search filter, so the drag & drop
//! row math and the rendered list always see the same rows.
use std::collections::{HashMap, HashSet};

use sequoia_ui::prelude::*;

use crate::core::layer::Layer;
use crate::state::EditorState;

use super::layer_row::LayerRow;

/// One visible row of the tree: the layer, its indent and whether its children
/// are currently shown (a chevron is only drawn when it has any).
#[derive(Clone)]
pub(super) struct TreeRow {
    pub layer: Layer,
    pub depth: usize,
    pub has_children: bool,
    pub expanded: bool,
}

/// Row height shared by the list and the drag math.
pub(super) const ROW_H: f32 = 30.0;

fn by_id(layers: &[Layer]) -> HashMap<u64, &Layer> {
    layers.iter().map(|l| (l.id, l)).collect()
}

/// Ids that live inside a component master (the master itself excluded), so
/// the page section can skip internals that the COMPONENTS tree shows.
fn master_internals(layers: &[Layer]) -> HashSet<u64> {
    let masters: HashSet<u64> = layers
        .iter()
        .filter(|l| l.component)
        .map(|l| l.id)
        .collect();
    let ids = by_id(layers);
    let mut out = HashSet::new();
    for l in layers {
        let mut cur = l.parent;
        let mut guard = 0;
        while let Some(pid) = cur {
            if masters.contains(&pid) {
                out.insert(l.id);
                break;
            }
            guard += 1;
            if guard > 256 {
                break;
            }
            cur = ids.get(&pid).and_then(|p| p.parent);
        }
    }
    out
}

fn matches_query(l: &Layer, q: &str) -> bool {
    q.is_empty() || l.name.to_lowercase().contains(q)
}

fn subtree_matches(
    l: &Layer,
    q: &str,
    children: &HashMap<Option<u64>, Vec<&Layer>>,
    depth: usize,
) -> bool {
    if matches_query(l, q) {
        return true;
    }
    if depth > 128 {
        return false;
    }
    children.get(&Some(l.id)).is_some_and(|cs| {
        cs.iter()
            .any(|c| subtree_matches(c, q, children, depth + 1))
    })
}

/// Rows of one section in display order, honouring the search query, the
/// "hide hidden" filter and the collapsed containers.
pub(super) fn tree_rows(state: &EditorState, only_components: bool) -> Vec<TreeRow> {
    let layers = state.layers.get();
    let q = state.layer_query.get().to_lowercase();
    let show_hidden = state.show_hidden_layers.get();
    let collapsed = state.collapsed_layers.get();

    let internals = master_internals(&layers);
    let section: Vec<&Layer> = layers
        .iter()
        .filter(|l| {
            if only_components {
                l.component || internals.contains(&l.id)
            } else {
                !l.component && !internals.contains(&l.id)
            }
        })
        .collect();
    let ids: HashSet<u64> = section.iter().map(|l| l.id).collect();

    // Children keyed by their parent inside the section. A layer whose parent
    // is not part of the section becomes a root (the page root frame, a master
    // whose parent is another master's child, ...).
    let mut children: HashMap<Option<u64>, Vec<&Layer>> = HashMap::new();
    for l in &section {
        let key = l.parent.filter(|p| ids.contains(p));
        children.entry(key).or_default().push(l);
    }
    let roots: Vec<&Layer> = section
        .iter()
        .copied()
        .filter(|l| l.parent.is_none_or(|p| !ids.contains(&p)))
        .collect();

    fn emit(
        l: &Layer,
        depth: usize,
        out: &mut Vec<TreeRow>,
        q: &str,
        show_hidden: bool,
        collapsed: &[u64],
        children: &HashMap<Option<u64>, Vec<&Layer>>,
    ) {
        if !show_hidden && !l.visible {
            return;
        }
        if !subtree_matches(l, q, children, depth) {
            return;
        }
        let kids = children.get(&Some(l.id));
        let has_children = kids.is_some_and(|c| !c.is_empty());
        let expanded = !q.is_empty() || !collapsed.contains(&l.id);
        out.push(TreeRow {
            layer: l.clone(),
            depth,
            has_children,
            expanded,
        });
        if has_children && expanded {
            for c in kids.unwrap() {
                emit(c, depth + 1, out, q, show_hidden, collapsed, children);
            }
        }
    }

    let mut out = Vec::new();
    for r in roots {
        emit(r, 0, &mut out, &q, show_hidden, &collapsed, &children);
    }
    out
}

pub(super) fn toggle_collapsed(state: &EditorState, id: u64) {
    state.collapsed_layers.update(|v| {
        if let Some(pos) = v.iter().position(|x| *x == id) {
            v.remove(pos);
        } else {
            v.push(id);
        }
    });
}

/// Expands a container (used after dropping a layer into it).
pub(super) fn expand(state: &EditorState, id: u64) {
    state.collapsed_layers.update(|v| v.retain(|x| *x != id));
}

pub(super) fn layer_list(state: &EditorState, only_components: bool) -> impl View + 'static {
    let st = state.clone();
    VirtualList::new({
        let st = st.clone();
        move || tree_rows(&st, only_components)
    })
    .key(|r: &TreeRow| r.layer.id)
    .item_height(ROW_H)
    .item({
        let st = st.clone();
        move |r: &TreeRow| LayerRow {
            row: r.clone(),
            state: st.clone(),
            only_components,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_are_a_tree_and_collapse_hides_children() {
        let state = EditorState::with_seed(Some(crate::document::demo::demo_tree()), None);
        let rows = tree_rows(&state, true);
        let master = rows.first().expect("COMPONENTS lists the Overlay master");
        assert_eq!(master.layer.name, "Overlay");
        assert!(master.has_children, "master row is expandable");
        let child = rows
            .iter()
            .find(|r| r.depth > 0)
            .expect("master children are nested rows");
        assert_eq!(child.depth, 1);

        state.collapsed_layers.set(vec![master.layer.id]);
        let collapsed = tree_rows(&state, true);
        assert_eq!(collapsed.len(), 1, "collapsing hides the whole subtree");

        let page = tree_rows(&state, false);
        assert!(
            page.iter().any(|r| r.layer.name == "Rectangle 2"),
            "page layers stay listed"
        );
        assert!(
            !page.iter().any(|r| r.layer.id == master.layer.id),
            "the master itself lives in COMPONENTS"
        );
        assert!(
            !page.iter().any(|r| r.layer.id == child.layer.id),
            "master internals are not duplicated on the page"
        );
    }
}
