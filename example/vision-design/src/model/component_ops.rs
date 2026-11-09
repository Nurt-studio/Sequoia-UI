//! Component mechanics shared by the demo, the panels and codegen.
//!
//! An instance is a **materialized copy** of its master: the instance subtree
//! in the AST carries its own ids, so hit-testing, selection and layout work
//! without special cases. Overrides are keyed by prop *name* and applied at
//! the structural path of the prop's target inside the master — the copy has
//! the same shape, so the path transfers 1:1.

use std::collections::HashMap;

use sequoia_ui::prelude::Color;

use super::svelte_ast::*;
use super::svelte_ast::{SlotContent, SlotDef};

// ─────────────────────────────────────────────────────────── lookups

/// Finds the master component with the given name (first match, document order).
pub fn find_master<'a>(root: &'a SvelteNode, name: &str) -> Option<&'a Element> {
    if let SvelteNode::Element(e) = root {
        if e.component && e.name == name {
            return Some(e);
        }
    }
    for c in root.children() {
        if let Some(m) = find_master(c, name) {
            return Some(m);
        }
    }
    None
}

/// Child-index path from a master element down to the node with `target`.
/// An empty path addresses the master root itself. Text nodes are regular
/// path steps (they are children of elements).
pub fn prop_path(master: &Element, target: u64) -> Option<Vec<usize>> {
    if master.id == target {
        return Some(Vec::new());
    }
    walk_children(&master.children, target)
}

fn walk_children(children: &[SvelteNode], target: u64) -> Option<Vec<usize>> {
    for (i, c) in children.iter().enumerate() {
        if c.id() == target {
            return Some(vec![i]);
        }
        if let SvelteNode::Element(e) = c
            && let Some(mut sub) = walk_children(&e.children, target)
        {
            sub.insert(0, i);
            return Some(sub);
        }
    }
    None
}

/// Follows a child-index path from `root`; an empty path addresses `root`.
fn node_at_path_mut<'a>(root: &'a mut SvelteNode, path: &[usize]) -> Option<&'a mut SvelteNode> {
    let mut cur = root;
    for &i in path {
        cur = cur.children_mut()?.get_mut(i)?;
    }
    Some(cur)
}

/// Child-index path from the master down to the container of `slot_name`.
pub fn slot_path(master: &Element, slot_name: &str) -> Option<Vec<usize>> {
    let def = master
        .interface
        .as_ref()?
        .slots
        .iter()
        .find(|s| s.name == slot_name)?;
    if master.id == def.container {
        return Some(Vec::new());
    }
    walk_children(&master.children, def.container)
}

/// Slot definitions of the master an instance refers to (by name).
pub fn master_slots<'a>(root: &'a SvelteNode, instance: &Element) -> Vec<SlotDef> {
    find_master(root, &instance.name)
        .and_then(|m| m.interface.clone())
        .map(|it| it.slots)
        .unwrap_or_default()
}

// ───────────────────────────────────────────────────── overrides

/// Resolves the current effective value of a prop on the instance copy:
/// text content for `Text`, `fill_hex` for `Fill`, `color_hex` for `TextColor`.
pub fn effective_value(instance: &Element, master: &Element, def: &PropDef) -> String {
    let Some(path) = prop_path(master, def.target) else {
        return def.default.clone();
    };
    if path.is_empty() {
        // The prop drives the master root itself.
        return match def.kind {
            PropKind::Fill => instance.style.fill_hex.clone(),
            _ => def.default.clone(),
        };
    }
    let mut cur: &SvelteNode = match instance.children.get(path[0]) {
        Some(n) => n,
        None => return def.default.clone(),
    };
    for &i in &path[1..] {
        cur = match cur.children().get(i) {
            Some(n) => n,
            None => return def.default.clone(),
        };
    }
    match (def.kind, cur) {
        (PropKind::Text, SvelteNode::Text(t)) => t.content.clone(),
        (PropKind::Fill, SvelteNode::Element(e)) => e.style.fill_hex.clone(),
        (PropKind::TextColor, SvelteNode::Text(t)) => t.style.color_hex.clone(),
        _ => def.default.clone(),
    }
}

/// Applies one override to a node at the master-derived path.
/// Returns `true` when the value was applied.
pub fn apply_override_at(
    node: &mut SvelteNode,
    path: &[usize],
    kind: PropKind,
    value: &str,
) -> bool {
    let Some(target) = node_at_path_mut(node, path) else {
        return false;
    };
    match (kind, target) {
        (PropKind::Text, SvelteNode::Text(t)) => {
            t.content = value.to_string();
            true
        }
        (PropKind::Fill, SvelteNode::Element(e)) => match Color::parse_hex(value) {
            Some(c) => {
                e.style.fill = Some(c);
                e.style.fill_hex = value.to_string();
                true
            }
            None => false,
        },
        (PropKind::TextColor, SvelteNode::Text(t)) => match Color::parse_hex(value) {
            Some(c) => {
                t.style.color = c;
                t.style.color_hex = value.to_string();
                true
            }
            None => false,
        },
        _ => false,
    }
}

// ───────────────────────────────────────────── instantiate

/// Deep copy of `master` with fresh ids. Keeps the master's name and marks
/// the copy as an instance; `overrides` are applied through the master's
/// structural paths, `slots` replace the slot containers' fallback children,
/// and the (prop, value) pairs are recorded for the panel and the multi-file
/// export.
pub fn instantiate(
    master: &Element,
    overrides: Vec<Override>,
    slots: Vec<SlotContent>,
    next: &mut u64,
) -> SvelteNode {
    let mut id_map: HashMap<u64, u64> = HashMap::new();
    let mut copy = clone_element_with_ids(master, &mut id_map, next);
    if let SvelteNode::Element(e) = &mut copy {
        e.component = false;
        e.instance = true;
        e.interface = None;
        e.overrides = overrides.clone();
        e.slot_content = slots.clone();
    }
    // Apply the overrides through the master's paths.
    for o in &overrides {
        let Some(def) = master
            .interface
            .as_ref()
            .and_then(|it| it.prop(&o.prop))
            .cloned()
        else {
            continue;
        };
        let Some(path) = prop_path(master, def.target) else {
            continue;
        };
        apply_override_at(&mut copy, &path, def.kind, &o.value);
    }
    // Materialize the slot content: the copy's container (same structural
    // path as in the master) shows the parent's nodes instead of the fallback.
    for sc in &slots {
        let Some(path) = slot_path(master, &sc.slot) else {
            continue;
        };
        apply_slot_content_at(&mut copy, &path, &sc.nodes);
    }
    copy
}

/// Replaces the children of the node at `path` with copies of `nodes`.
fn apply_slot_content_at(node: &mut SvelteNode, path: &[usize], nodes: &[SvelteNode]) -> bool {
    let Some(target) = node_at_path_mut(node, path) else {
        return false;
    };
    match target.children_mut() {
        Some(children) => {
            *children = nodes.to_vec();
            true
        }
        None => false,
    }
}

/// Applies the slot content of every instance in the tree (Phase 4).
/// Returns a materialized copy: slot containers hold the parent's nodes
/// instead of their fallback children. Used by the layout engine and the
/// single-file codegen; the multi-file export reads `slot_content` directly.
pub fn inject_slots(root: &SvelteNode) -> SvelteNode {
    if !tree_has_slot_content(root) {
        return root.clone();
    }
    let mut copy = root.clone();
    fill_slots(&mut copy, root);
    copy
}

fn tree_has_slot_content(node: &SvelteNode) -> bool {
    if let SvelteNode::Element(e) = node
        && !e.slot_content.is_empty()
    {
        return true;
    }
    node.children().iter().any(tree_has_slot_content)
}

fn fill_slots(node: &mut SvelteNode, root: &SvelteNode) {
    // Compute the fills first (cloned), so the immutable borrow of `node`
    // ends before the mutation.
    let fills: Vec<(Vec<usize>, Vec<SvelteNode>)> = match node {
        SvelteNode::Element(e) if e.instance && !e.slot_content.is_empty() => {
            match find_master(root, &e.name) {
                Some(master) => e
                    .slot_content
                    .iter()
                    .filter_map(|sc| slot_path(master, &sc.slot).map(|p| (p, sc.nodes.clone())))
                    .collect(),
                None => Vec::new(),
            }
        }
        _ => Vec::new(),
    };
    for (path, nodes) in &fills {
        apply_slot_content_at(node, path, nodes);
    }
    // Slot content can carry nested instances — recurse everywhere.
    if let Some(children) = node.children_mut() {
        for c in children.iter_mut() {
            fill_slots(c, root);
        }
    }
    if let SvelteNode::Element(e) = node {
        for sc in &mut e.slot_content {
            for n in sc.nodes.iter_mut() {
                fill_slots(n, root);
            }
        }
    }
}

/// Map: slot container id → slot name, across every master in the tree.
/// Used to mark the "hole" in the flat tree (dashed canvas overlay).
pub fn slot_container_names(root: &SvelteNode) -> HashMap<u64, String> {
    let mut out = HashMap::new();
    collect_slot_containers(root, &mut out);
    out
}

fn collect_slot_containers(node: &SvelteNode, out: &mut HashMap<u64, String>) {
    if let SvelteNode::Element(e) = node
        && e.component
        && let Some(it) = &e.interface
    {
        for s in &it.slots {
            out.entry(s.container).or_insert_with(|| s.name.clone());
        }
    }
    for c in node.children() {
        collect_slot_containers(c, out);
    }
}

/// Re-id walk that records an old→new id map so interface targets (and slot
/// containers) keep pointing at the right nodes.
pub fn clone_with_ids(
    node: &SvelteNode,
    map: &mut HashMap<u64, u64>,
    next: &mut u64,
) -> SvelteNode {
    let new_id = *next;
    *next += 1;
    map.insert(node.id(), new_id);
    match node {
        SvelteNode::Element(e) => {
            let children: Vec<SvelteNode> = e
                .children
                .iter()
                .map(|c| clone_with_ids(c, map, next))
                .collect();
            let slot_content: Vec<SlotContent> = e
                .slot_content
                .iter()
                .map(|sc| SlotContent {
                    slot: sc.slot.clone(),
                    nodes: sc
                        .nodes
                        .iter()
                        .map(|n| clone_with_ids(n, map, next))
                        .collect(),
                })
                .collect();
            let mut el = e.clone();
            el.id = new_id;
            el.children = children;
            el.slot_content = slot_content;
            if let Some(it) = el.interface.as_mut() {
                remap_interface(it, map);
            }
            SvelteNode::Element(el)
        }
        SvelteNode::Text(t) => {
            let mut tn = t.clone();
            tn.id = new_id;
            SvelteNode::Text(tn)
        }
    }
}

/// Element-rooted variant: no wrapper node is created, the master element
/// itself gets the first fresh id.
fn clone_element_with_ids(
    master: &Element,
    map: &mut HashMap<u64, u64>,
    next: &mut u64,
) -> SvelteNode {
    let new_id = *next;
    *next += 1;
    map.insert(master.id, new_id);
    let children: Vec<SvelteNode> = master
        .children
        .iter()
        .map(|c| clone_with_ids(c, map, next))
        .collect();
    let mut el = master.clone();
    el.id = new_id;
    el.children = children;
    el.interface = None;
    el.overrides = Vec::new();
    SvelteNode::Element(el)
}

/// Remaps interface targets in place using an old→new id map.
pub fn remap_interface(interface: &mut ComponentInterface, map: &HashMap<u64, u64>) {
    for p in interface.props.iter_mut() {
        if let Some(n) = map.get(&p.target) {
            p.target = *n;
        }
    }
    for s in interface.slots.iter_mut() {
        if let Some(n) = map.get(&s.container) {
            s.container = *n;
        }
    }
}

// ──────────────────────────────────────────────────── tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::svelte_ast::new_text;
    use crate::model::tree_ops;

    /// master: div(1) { text "A"(2), div(3) { text "B"(4) } }
    fn demo_master() -> Element {
        let inner_text = new_text(4, 0.0, 0.0, "B");
        let mut inner = new_rect(3, 0.0, 0.0, 40.0, 20.0, 0xFA4D56);
        if let SvelteNode::Element(e) = &mut inner {
            e.children = vec![inner_text];
        }
        let title = new_text(2, 0.0, 0.0, "A");
        let mut root = new_frame(1, 0.0, 0.0, 100.0, 60.0);
        if let SvelteNode::Element(e) = &mut root {
            e.component = true;
            e.name = "Card".into();
            e.children = vec![title, inner];
            e.interface = Some(ComponentInterface {
                props: vec![
                    PropDef {
                        name: "title".into(),
                        default: "A".into(),
                        target: 2,
                        kind: PropKind::Text,
                    },
                    PropDef {
                        name: "accent".into(),
                        default: "#FA4D56".into(),
                        target: 3,
                        kind: PropKind::Fill,
                    },
                    PropDef {
                        name: "note".into(),
                        default: "B".into(),
                        target: 4,
                        kind: PropKind::Text,
                    },
                ],
                events: vec!["confirm".into()],
                slots: vec![],
            });
        }
        match root {
            SvelteNode::Element(e) => e,
            _ => unreachable!(),
        }
    }

    #[test]
    fn prop_path_finds_nested_targets() {
        let m = demo_master();
        assert_eq!(prop_path(&m, 2), Some(vec![0]));
        assert_eq!(prop_path(&m, 4), Some(vec![1, 0]));
        assert_eq!(prop_path(&m, 99), None);
        assert_eq!(prop_path(&m, 1), Some(vec![]), "root addresses itself");
    }

    #[test]
    fn instantiate_applies_overrides_and_marks_the_copy() {
        let m = demo_master();
        let mut next = 100u64;
        let inst = instantiate(
            &m,
            vec![
                Override {
                    prop: "title".into(),
                    value: "Hello".into(),
                },
                Override {
                    prop: "accent".into(),
                    value: "#42BE65".into(),
                },
            ],
            vec![],
            &mut next,
        );
        assert!(inst.is_instance());
        assert!(!inst.is_component());
        assert_eq!(inst.name(), "Card");
        // Fresh ids starting from 100: root 100, title text 101, div 102, note 103.
        assert_eq!(inst.id(), 100);
        // Text override landed through the path.
        match &inst {
            SvelteNode::Element(e) => match &e.children[0] {
                SvelteNode::Text(t) => assert_eq!(t.content, "Hello"),
                _ => panic!("expected text"),
            },
            _ => panic!("expected element"),
        }
        // Fill override landed on the nested element.
        let flat = crate::model::layout_engine::compute_layout(&Some(inst.clone()));
        let inner = flat.iter().find(|f| f.id == 102).expect("inner laid out");
        assert_eq!(inner.style().unwrap().fill_hex, "#42BE65");
        // Overrides recorded for codegen / panel.
        match &inst {
            SvelteNode::Element(e) => assert_eq!(e.overrides.len(), 2),
            _ => panic!("expected element"),
        }
    }

    #[test]
    fn effective_value_reads_the_instance_copy() {
        let m = demo_master();
        let mut next = 10u64;
        let inst = match instantiate(
            &m,
            vec![Override {
                prop: "note".into(),
                value: "Changed".into(),
            }],
            vec![],
            &mut next,
        ) {
            SvelteNode::Element(e) => e,
            _ => unreachable!(),
        };
        let def = PropDef {
            name: "note".into(),
            default: "B".into(),
            target: 4,
            kind: PropKind::Text,
        };
        assert_eq!(
            effective_value(&inst, &m, &def),
            "Changed",
            "the copy carries the overridden value"
        );
        // A prop without an override reads the master's default from the copy.
        let def_title = PropDef {
            name: "title".into(),
            default: "A".into(),
            target: 2,
            kind: PropKind::Text,
        };
        assert_eq!(effective_value(&inst, &m, &def_title), "A");
    }

    #[test]
    fn inject_slots_materializes_the_container_children() {
        let mut m = demo_master();
        // Slot "default" -> container is the inner rect (id 3), i.e. child
        // index 1 of the master root (the structural path survives the copy).
        if let Some(it) = m.interface.as_mut() {
            it.slots.push(SlotDef {
                name: "default".into(),
                container: 3,
            });
        }
        assert_eq!(slot_path(&m, "default"), Some(vec![1]));
        // Parent content: a fresh rect used as slot content.
        let content = new_rect(900, 0.0, 0.0, 10.0, 10.0, 0x0D99FF);

        // instantiate materializes the content: the container shows it.
        let mut next = 100u64;
        let inst = instantiate(
            &m,
            vec![],
            vec![SlotContent {
                slot: "default".into(),
                nodes: vec![content],
            }],
            &mut next,
        );
        let ids: Vec<u64> = inst.children()[1]
            .children()
            .iter()
            .map(|c| c.id())
            .collect();
        assert_eq!(ids, vec![900], "content replaced the fallback");

        // Without content the fallback children stay (single source of
        // truth; the copy remaps their ids, so compare the shape/name).
        let mut next = 100u64;
        let plain = instantiate(&m, vec![], vec![], &mut next);
        let names: Vec<String> = plain.children()[1]
            .children()
            .iter()
            .map(|c| c.name().to_string())
            .collect();
        let fallback_name = tree_ops::find(&SvelteNode::Element(m.clone()), 4)
            .map(|n| n.name().to_string())
            .unwrap_or_default();
        assert_eq!(
            names,
            vec![fallback_name],
            "fallback survives without content"
        );
    }

    #[test]
    fn clone_with_ids_remaps_interface_targets() {
        let m = demo_master();
        let mut map = HashMap::new();
        let mut next = 50u64;
        let copy = clone_with_ids(&SvelteNode::Element(m), &mut map, &mut next);
        match &copy {
            SvelteNode::Element(e) => {
                let it = e.interface.as_ref().unwrap();
                assert_eq!(it.prop("title").unwrap().target, *map.get(&2).unwrap());
                assert_eq!(it.prop("accent").unwrap().target, *map.get(&3).unwrap());
                assert_eq!(it.prop("note").unwrap().target, *map.get(&4).unwrap());
            }
            _ => panic!("expected element"),
        }
    }
}
