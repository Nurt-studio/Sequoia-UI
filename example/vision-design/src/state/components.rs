//! Editor actions for the Phase 1 component system: instance overrides and
//! master interface access for the panels. The AST stays the single source
//! of truth — these helpers only read and mutate `root`.

use super::EditorState;
use crate::model::component_ops::{effective_value, find_master, prop_path};
use crate::model::svelte_ast::{ComponentInterface, Override, PropDef, SvelteNode};
use crate::state::PickerOption;

/// Context for the Overrides section: the master's contract plus the
/// selected instance.
pub(crate) struct InstanceCtx {
    pub(crate) master: ComponentInterface,
    pub(crate) overrides: Vec<Override>,
    pub(crate) slot_content: Vec<crate::model::svelte_ast::SlotContent>,
    pub(crate) instance_id: u64,
}

impl EditorState {
    /// Master interface + overrides of the selected instance (◈).
    pub(crate) fn selected_instance_ctx(&self) -> Option<InstanceCtx> {
        let id = self.selected_layer.get_untracked()?;
        let root = self.root.get_untracked();
        let r = root.as_ref()?;
        let node = crate::model::tree_ops::find(r, id)?;
        let SvelteNode::Element(e) = node else {
            return None;
        };
        if !e.instance {
            return None;
        }
        let master = find_master(r, &e.name)?;
        let interface = master.interface.clone()?;
        Some(InstanceCtx {
            master: interface,
            overrides: e.overrides.clone(),
            slot_content: e.slot_content.clone(),
            instance_id: e.id,
        })
    }

    /// Interface of the selected master (◇), if any.
    pub(crate) fn selected_master_interface(&self) -> Option<ComponentInterface> {
        let id = self.selected_layer.get_untracked()?;
        let root = self.root.get_untracked();
        let r = root.as_ref()?;
        match crate::model::tree_ops::find(r, id) {
            Some(SvelteNode::Element(e)) if e.component => e.interface.clone(),
            _ => None,
        }
    }

    /// Sets one prop override on the selected instance: updates the
    /// materialized copy (so the canvas follows immediately) and records the
    /// (prop, value) pair for the panel and codegen.
    pub(crate) fn set_override(&self, prop: &str, value: String) {
        let id = match self.selected_layer.get_untracked() {
            Some(id) => id,
            None => return,
        };
        let root = self.root.get_untracked();
        let Some(r) = root.as_ref() else {
            return;
        };
        let Some(SvelteNode::Element(inst)) = crate::model::tree_ops::find(r, id) else {
            return;
        };
        if !inst.instance {
            return;
        }
        let Some(master) = find_master(r, &inst.name) else {
            self.toast_warn("Master component not found for this instance");
            return;
        };
        let Some(interface) = master.interface.as_ref() else {
            return;
        };
        let Some(def) = interface.prop(prop) else {
            return;
        };
        let def = def.clone();

        // Nothing to do when the value did not actually change (submit +
        // blur can both fire for the same edit).
        let Some(SvelteNode::Element(current)) = crate::model::tree_ops::find(r, id) else {
            return;
        };
        let current_value = current
            .overrides
            .iter()
            .find(|o| o.prop == prop)
            .map(|o| o.value.clone())
            .unwrap_or_else(|| effective_value(current, master, &def));
        if current_value == value {
            return;
        }

        let Some(path) = prop_path(master, def.target) else {
            self.toast_warn("Prop target is missing in the master");
            return;
        };

        self.push_history();
        self.root.update(|r| {
            let Some(rr) = r else { return };
            let Some(node) = crate::model::tree_ops::find_mut(rr, id) else {
                return;
            };
            // 1. Apply the value to the materialized copy (canvas follows).
            crate::model::component_ops::apply_override_at(node, &path, def.kind, &value);
            // 2. Record / replace the override entry (panel + codegen).
            if let SvelteNode::Element(inst) = node {
                match inst.overrides.iter_mut().find(|o| o.prop == prop) {
                    Some(o) => o.value = value,
                    None => inst.overrides.push(Override {
                        prop: prop.to_string(),
                        value,
                    }),
                }
            }
        });
    }

    /// Prop definitions of the selected master for the read-only COMPONENT
    /// section (interface editing arrives in Phase 2).
    pub(crate) fn selected_master_props(&self) -> Vec<PropDef> {
        self.selected_master_interface()
            .map(|it| it.props)
            .unwrap_or_default()
    }

    // ─────────────────────────────── contract editing (Phase 3)

    /// Mutates the interface of the selected master, creating it when
    /// absent. No-op for non-masters.
    fn with_master_interface(&self, f: impl FnOnce(&mut ComponentInterface)) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        let root = self.root.get_untracked();
        let is_master = root
            .as_ref()
            .and_then(|r| crate::model::tree_ops::find(r, id))
            .map(|n| match n {
                SvelteNode::Element(e) => e.component,
                _ => false,
            })
            .unwrap_or(false);
        if !is_master {
            return;
        }
        self.push_history();
        self.root.update(|r| {
            if let Some(rr) = r
                && let Some(SvelteNode::Element(e)) = crate::model::tree_ops::find_mut(rr, id)
            {
                let it = e.interface.get_or_insert_with(Default::default);
                f(it);
                if it.is_empty() {
                    e.interface = None;
                }
            }
        });
    }

    /// Whole-document walk for rename propagation (instances may live
    /// anywhere; their overrides and Emit calls reference the contract).
    fn for_every_action(&self, mut f: impl FnMut(&mut crate::model::behavior::Action)) {
        self.root.update(|r| {
            if let Some(rr) = r {
                walk_actions(rr, &mut f);
            }
        });
    }

    fn for_every_override(&self, mut f: impl FnMut(&mut Override)) {
        self.root.update(|r| {
            if let Some(rr) = r {
                walk_overrides(rr, &mut f);
            }
        });
    }

    fn for_every_trigger(&self, mut f: impl FnMut(&mut crate::model::behavior::Trigger)) {
        self.root.update(|r| {
            if let Some(rr) = r {
                walk_triggers(rr, &mut f);
            }
        });
    }

    // events ──

    pub(crate) fn add_interface_event(&self) {
        let existing = self
            .selected_master_interface()
            .map(|it| it.events)
            .unwrap_or_default();
        let mut i = 1;
        while existing.iter().any(|e| e == &format!("event{i}")) {
            i += 1;
        }
        self.with_master_interface(|it| it.events.push(format!("event{i}")));
    }

    pub(crate) fn remove_interface_event(&self, name: &str) {
        self.with_master_interface(|it| it.events.retain(|e| e != name));
    }

    /// Renames the event everywhere: the contract plus every `Emit` that
    /// refers to it (they become `on{name}` callback props in codegen).
    pub(crate) fn rename_interface_event(&self, old: &str, new: &str) {
        if !crate::model::behavior::is_valid_var_name(new) {
            self.toast_warn("Event name must be a JS identifier");
            return;
        }
        if old == new {
            return;
        }
        if self
            .selected_master_interface()
            .map(|it| it.events.iter().any(|e| e == new))
            .unwrap_or(true)
        {
            self.toast_warn("An event with this name already exists");
            return;
        }
        self.with_master_interface(|it| {
            for e in &mut it.events {
                if e == old {
                    *e = new.to_string();
                }
            }
        });
        self.for_every_action(|a| {
            if let crate::model::behavior::Action::Emit(e) = a {
                if e == old {
                    *e = new.to_string();
                }
            }
        });
    }

    // props ──

    pub(crate) fn add_interface_prop(&self) {
        let (existing, first_target) = {
            let id = self.selected_layer.get_untracked();
            let root = self.root.get_untracked();
            match (id, root.as_ref()) {
                (Some(id), Some(r)) => {
                    let it = match crate::model::tree_ops::find(r, id) {
                        Some(SvelteNode::Element(e)) => e.interface.clone(),
                        _ => None,
                    };
                    // First node of the master's subtree (element or text,
                    // the master itself excluded) becomes the default target.
                    let target =
                        first_prop_target(crate::model::tree_ops::find(r, id).unwrap_or(r), id)
                            .unwrap_or(id);
                    (it, target)
                }
                _ => (None, 0),
            }
        };
        let mut i = 1;
        let base = existing
            .as_ref()
            .map(|it| it.props.clone())
            .unwrap_or_default();
        while base.iter().any(|p| p.name == format!("prop{i}")) {
            i += 1;
        }
        self.with_master_interface(move |it| {
            it.props.push(PropDef {
                name: format!("prop{i}"),
                default: String::new(),
                target: first_target,
                kind: crate::model::svelte_ast::PropKind::Text,
            });
        });
    }

    pub(crate) fn remove_interface_prop(&self, name: &str) {
        self.with_master_interface(|it| it.props.retain(|p| p.name != name));
    }

    /// Renames the prop everywhere: contract, `On Prop Changed` triggers and
    /// the overrides recorded on instances.
    pub(crate) fn rename_interface_prop(&self, old: &str, new: &str) {
        if !crate::model::behavior::is_valid_var_name(new) {
            self.toast_warn("Prop name must be a JS identifier");
            return;
        }
        if old == new {
            return;
        }
        if self
            .selected_master_interface()
            .map(|it| it.prop(new).is_some())
            .unwrap_or(true)
        {
            self.toast_warn("A prop with this name already exists");
            return;
        }
        self.with_master_interface(|it| {
            for p in &mut it.props {
                if p.name == old {
                    p.name = new.to_string();
                }
            }
        });
        // `On Prop Changed (old)` rules follow the contract.
        self.for_every_trigger(|t| {
            if let crate::model::behavior::Trigger::OnPropChanged(p) = t {
                if p == old {
                    *p = new.to_string();
                }
            }
        });
        self.for_every_override(|o| {
            if o.prop == old {
                o.prop = new.to_string();
            }
        });
    }

    pub(crate) fn set_interface_prop_kind(
        &self,
        name: &str,
        kind: crate::model::svelte_ast::PropKind,
    ) {
        self.with_master_interface(|it| {
            if let Some(p) = it.prop_mut(name) {
                p.kind = kind;
            }
        });
    }

    pub(crate) fn set_interface_prop_target(&self, name: &str, target: u64) {
        self.with_master_interface(|it| {
            if let Some(p) = it.prop_mut(name) {
                p.target = target;
            }
        });
    }

    // slots ── (Phase 4)

    /// Adds a slot to the selected master's contract. The first slot is
    /// named `default` (the plain `<slot />`); further slots get `slot2`,
    /// `slot3`, … The first element of the master's subtree becomes the
    /// default container (changeable per row in the panel).
    pub(crate) fn add_interface_slot(&self) {
        let (existing, first_target) = {
            let id = self.selected_layer.get_untracked();
            let root = self.root.get_untracked();
            match (id, root.as_ref()) {
                (Some(id), Some(r)) => {
                    let slots = match crate::model::tree_ops::find(r, id) {
                        Some(SvelteNode::Element(e)) => e
                            .interface
                            .as_ref()
                            .map(|it| it.slots.clone())
                            .unwrap_or_default(),
                        _ => Vec::new(),
                    };
                    // Slot containers must be ELEMENTS (a text node cannot
                    // host children): the default container is the first
                    // element of the master's subtree, the master root
                    // itself when it has none.
                    let target =
                        first_element_target(crate::model::tree_ops::find(r, id).unwrap_or(r), id)
                            .unwrap_or(id);
                    (slots, target)
                }
                _ => (Vec::new(), 0),
            }
        };
        let name = if existing.is_empty() {
            "default".to_string()
        } else {
            let mut i = 2;
            while existing.iter().any(|s| s.name == format!("slot{i}")) {
                i += 1;
            }
            format!("slot{i}")
        };
        self.with_master_interface(move |it| {
            it.slots.push(crate::model::svelte_ast::SlotDef {
                name,
                container: first_target,
            });
        });
    }

    pub(crate) fn remove_interface_slot(&self, name: &str) {
        self.with_master_interface(|it| it.slots.retain(|s| s.name != name));
        // Instances of this master drop the content they provided for the
        // removed slot (the fallback children take over again).
        let removed = name.to_string();
        self.root.update(|r| {
            if let Some(rr) = r {
                walk_elements_mut(rr, &mut |e: &mut crate::model::svelte_ast::Element| {
                    e.slot_content.retain(|sc| sc.slot != removed);
                });
            }
        });
    }

    pub(crate) fn set_interface_slot_container(&self, name: &str, target: u64) {
        self.with_master_interface(|it| {
            if let Some(s) = it.slots.iter_mut().find(|s| s.name == name) {
                s.container = target;
            }
        });
    }

    /// Fill candidates for a slot of the selected instance: the instance's
    /// siblings (the parent's other children, document order).
    pub(crate) fn slot_fill_options(&self) -> Vec<PickerOption> {
        let id = self.selected_layer.get_untracked();
        let root = self.root.get_untracked();
        let Some((id, r)) = id.zip(root.as_ref()) else {
            return Vec::new();
        };
        let Some(SvelteNode::Element(inst)) = crate::model::tree_ops::find(r, id) else {
            return Vec::new();
        };
        if !inst.instance {
            return Vec::new();
        }
        let Some(parent_id) = inst_parent(r, id) else {
            return Vec::new();
        };
        let Some(SvelteNode::Element(parent)) = crate::model::tree_ops::find(r, parent_id) else {
            return Vec::new();
        };
        parent
            .children
            .iter()
            .filter(|c| c.id() != id)
            .map(|c| PickerOption {
                label: c.name().to_string(),
                value: c.id().to_string(),
            })
            .collect()
    }

    /// Puts a **copy** of `source` (fresh ids) into the named slot of the
    /// selected instance. The canvas materializes it through
    /// `inject_slots`, so the change is visible immediately; the original
    /// layer stays in the page.
    pub(crate) fn set_slot_content(&self, slot: &str, source: u64) {
        let id = self.selected_layer.get_untracked();
        let root = self.root.get_untracked();
        let Some((id, r)) = id.zip(root.as_ref()) else {
            return;
        };
        let Some(SvelteNode::Element(inst)) = crate::model::tree_ops::find(r, id) else {
            return;
        };
        if !inst.instance {
            return;
        }
        let Some(src) = crate::model::tree_ops::find(r, source) else {
            return;
        };
        let mut next = self.next_node_id();
        let copy = super::document_ops::clone_with_new_ids(src, &mut next);
        let slot = slot.to_string();
        self.push_history();
        self.root.update(|r| {
            let Some(rr) = r else { return };
            let Some(SvelteNode::Element(e)) = crate::model::tree_ops::find_mut(rr, id) else {
                return;
            };
            e.slot_content.retain(|sc| sc.slot != slot);
            e.slot_content.push(crate::model::svelte_ast::SlotContent {
                slot,
                nodes: vec![copy],
            });
        });
    }

    /// Removes the content of one slot of the selected instance (the
    /// master's fallback children show again).
    pub(crate) fn clear_slot_content(&self, slot: &str) {
        let id = match self.selected_layer.get_untracked() {
            Some(id) => id,
            None => return,
        };
        let slot = slot.to_string();
        self.push_history();
        self.root.update(|r| {
            let Some(rr) = r else { return };
            let Some(SvelteNode::Element(e)) = crate::model::tree_ops::find_mut(rr, id) else {
                return;
            };
            e.slot_content.retain(|sc| sc.slot != slot);
        });
    }

    /// Options for the prop target dropdown: elements of the master
    /// subtree (`name · id` when names repeat).
    pub(crate) fn master_target_options(&self) -> Vec<PickerOption> {
        let id = self.selected_layer.get_untracked();
        let root = self.root.get_untracked();
        let Some((id, r)) = id.zip(root.as_ref()) else {
            return Vec::new();
        };
        match crate::model::tree_ops::find(r, id) {
            Some(node @ SvelteNode::Element(e)) if e.component => {
                let mut targets: Vec<(u64, String)> = Vec::new();
                collect_prop_targets(node, id, &mut targets);
                let mut seen: Vec<String> = Vec::new();
                targets
                    .into_iter()
                    .map(|(tid, name)| {
                        let dup = seen.contains(&name);
                        seen.push(name.clone());
                        PickerOption {
                            label: if dup {
                                format!("{name} · {tid}")
                            } else {
                                name.clone()
                            },
                            value: tid.to_string(),
                        }
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }
}

/// First node of the master's subtree (element or text, the master itself
/// excluded) — the default target for a new interface prop.
fn first_prop_target(node: &SvelteNode, master_id: u64) -> Option<u64> {
    if let SvelteNode::Element(e) = node {
        if e.id == master_id {
            for c in &e.children {
                if let Some(id) = first_prop_target(c, master_id) {
                    return Some(id);
                }
            }
            return None;
        }
        return Some(e.id);
    }
    Some(node.id())
}

/// First ELEMENT of the master's subtree (the master itself excluded) —
/// the default slot container (texts cannot host children).
fn first_element_target(node: &SvelteNode, master_id: u64) -> Option<u64> {
    if let SvelteNode::Element(e) = node {
        if e.id != master_id {
            return Some(e.id);
        }
        for c in &e.children {
            if let Some(id) = first_element_target(c, master_id) {
                return Some(id);
            }
        }
    }
    None
}

/// Every node of the master's subtree (elements and texts, the master
/// excluded) in document order — the prop target candidates.
fn collect_prop_targets(node: &SvelteNode, master_id: u64, out: &mut Vec<(u64, String)>) {
    if let SvelteNode::Element(e) = node {
        if e.id != master_id {
            out.push((e.id, e.name.clone()));
        }
        for c in &e.children {
            collect_prop_targets(c, master_id, out);
        }
    } else if let SvelteNode::Text(t) = node {
        out.push((t.id, t.name.clone()));
    }
}

/// Mutable access to a prop by name (the struct exposes only `prop`).
trait PropMut {
    fn prop_mut(&mut self, name: &str) -> Option<&mut PropDef>;
}

impl PropMut for crate::model::svelte_ast::ComponentInterface {
    fn prop_mut(&mut self, name: &str) -> Option<&mut PropDef> {
        self.props.iter_mut().find(|p| p.name == name)
    }
}

fn walk_actions(node: &mut SvelteNode, f: &mut impl FnMut(&mut crate::model::behavior::Action)) {
    if let SvelteNode::Element(e) = node {
        if let Some(b) = &mut e.behavior {
            for r in &mut b.rules {
                for a in &mut r.actions {
                    f(a);
                    if let crate::model::behavior::Action::Delay(_, nested) = a {
                        for a in nested {
                            f(a);
                        }
                    }
                }
            }
        }
    }
    if let Some(children) = node.children_mut() {
        for c in children {
            walk_actions(c, f);
        }
    }
}

fn walk_triggers(node: &mut SvelteNode, f: &mut impl FnMut(&mut crate::model::behavior::Trigger)) {
    if let SvelteNode::Element(e) = node {
        if let Some(b) = &mut e.behavior {
            for r in &mut b.rules {
                f(&mut r.trigger);
            }
        }
    }
    if let Some(children) = node.children_mut() {
        for c in children {
            walk_triggers(c, f);
        }
    }
}

fn walk_overrides(node: &mut SvelteNode, f: &mut impl FnMut(&mut Override)) {
    if let SvelteNode::Element(e) = node {
        for o in &mut e.overrides {
            f(o);
        }
    }
    if let Some(children) = node.children_mut() {
        for c in children {
            walk_overrides(c, f);
        }
    }
}

fn walk_elements_mut(
    node: &mut SvelteNode,
    f: &mut impl FnMut(&mut crate::model::svelte_ast::Element),
) {
    if let SvelteNode::Element(e) = node {
        f(e);
    }
    if let Some(children) = node.children_mut() {
        for c in children {
            walk_elements_mut(c, f);
        }
    }
}

/// Id of the parent element of `id` (None for the root).
fn inst_parent(root: &SvelteNode, id: u64) -> Option<u64> {
    crate::model::tree_ops::parent_of(root, id).map(|p| p.id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::svelte_ast::PropKind;

    fn seed_with_component() -> EditorState {
        // master Overlay(1) { text Title(2), div Body(3) }, instance Overlay(4)
        let mut master = crate::model::svelte_ast::new_frame(1, 0.0, 0.0, 200.0, 100.0);
        if let SvelteNode::Element(e) = &mut master {
            e.component = true;
            e.name = "Overlay".into();
            e.children = vec![crate::model::svelte_ast::new_text(2, 0.0, 0.0, "Hello")];
            e.interface = Some(ComponentInterface {
                props: vec![PropDef {
                    name: "title".into(),
                    default: "Hello".into(),
                    target: 2,
                    kind: PropKind::Text,
                }],
                ..Default::default()
            });
        }
        let master = match master {
            SvelteNode::Element(e) => e,
            _ => unreachable!(),
        };
        // Fresh ids from 4: instance root = 4, text copy = 5.
        let mut next = 4u64;
        let mut instance =
            crate::model::component_ops::instantiate(&master, vec![], vec![], &mut next);
        if let SvelteNode::Element(e) = &mut instance {
            e.style.top = Some(150.0);
            e.style.left = Some(220.0);
        }
        let mut root = crate::model::svelte_ast::new_frame(10, 0.0, 0.0, 500.0, 400.0);
        if let SvelteNode::Element(e) = &mut root {
            e.children = vec![SvelteNode::Element(master), instance];
        }
        EditorState::with_seed(Some(root), Some(4))
    }

    #[test]
    fn slot_ops_define_and_fill_a_slot() {
        let state = seed_with_component();
        // Select the master and add a slot targeting its text (id 2).
        state.selected_layer.set(Some(1));
        state.add_interface_slot();
        let it = state.selected_master_interface().unwrap();
        assert_eq!(it.slots.len(), 1);
        assert_eq!(it.slots[0].name, "default");
        // The master's only child is a text node — the master root (1)
        // becomes the container (texts cannot host children).
        assert_eq!(it.slots[0].container, 1);

        // The instance sees the slot and the sibling fill candidates.
        state.selected_layer.set(Some(4));
        let ctx = state.selected_instance_ctx().unwrap();
        assert_eq!(ctx.master.slots[0].name, "default");
        let options = state.slot_fill_options();
        // Siblings of the instance (4): only the master (1).
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].value, "1");

        // Fill: a copy of the master lands in the slot content.
        state.set_slot_content("default", 1);
        let ctx = state.selected_instance_ctx().unwrap();
        assert_eq!(ctx.slot_content.len(), 1);
        assert_eq!(ctx.slot_content[0].slot, "default");
        assert_eq!(ctx.slot_content[0].nodes.len(), 1);
        let copy_id = ctx.slot_content[0].nodes[0].id();
        assert_ne!(copy_id, 1, "the content copy gets fresh ids");

        // The canvas materializes the copy inside the instance's container.
        let root = state.root.get_untracked().unwrap();
        let flat = crate::model::layout_engine::compute_layout(&Some(root));
        assert!(
            flat.iter().any(|f| f.id == copy_id),
            "copy rendered on canvas"
        );

        // Clear brings the fallback back.
        state.clear_slot_content("default");
        let ctx = state.selected_instance_ctx().unwrap();
        assert!(ctx.slot_content.is_empty());

        // Removing the slot from the contract also drops instance content.
        state.set_slot_content("default", 1);
        state.selected_layer.set(Some(1));
        state.remove_interface_slot("default");
        state.selected_layer.set(Some(4));
        let ctx = state
            .selected_instance_ctx()
            .expect("interface still exists after removing the slot");
        assert!(ctx.master.slots.is_empty());
        assert!(
            ctx.slot_content.is_empty(),
            "stale content for the removed slot is dropped"
        );
    }

    #[test]
    fn set_override_updates_copy_and_records_entry() {
        let state = seed_with_component();
        assert!(state.selected_instance_ctx().is_some());

        state.set_override("title", "Zmieniony".into());

        let root = state.root.get_untracked().unwrap();
        let flat = crate::model::layout_engine::compute_layout(&Some(root.clone()));
        // Instance subtree: 4 (root), 5 (text copy) — title copy shows the new text.
        let texts: Vec<&crate::model::layout_engine::FlatNode> = flat
            .iter()
            .filter(|f| matches!(f.kind, crate::model::layout_engine::FlatKind::Text { .. }))
            .collect();
        let inst_text = texts
            .iter()
            .find(|f| f.id == 5)
            .expect("instance text copy laid out");
        match &inst_text.kind {
            crate::model::layout_engine::FlatKind::Text { content, .. } => {
                assert_eq!(content, "Zmieniony");
            }
            _ => panic!("expected text"),
        }
        // Override recorded on the instance element.
        match crate::model::tree_ops::find(&root, 4) {
            Some(SvelteNode::Element(e)) => {
                assert_eq!(e.overrides.len(), 1);
                assert_eq!(e.overrides[0].prop, "title");
                assert_eq!(e.overrides[0].value, "Zmieniony");
            }
            _ => panic!("instance missing"),
        }
        // Master's own text is untouched.
        match crate::model::tree_ops::find(&root, 2) {
            Some(SvelteNode::Text(t)) => assert_eq!(t.content, "Hello"),
            _ => panic!("master text missing"),
        }
    }

    #[test]
    fn set_override_twice_replaces_the_entry() {
        let state = seed_with_component();
        state.set_override("title", "A".into());
        state.set_override("title", "B".into());
        let root = state.root.get_untracked().unwrap();
        match crate::model::tree_ops::find(&root, 4) {
            Some(SvelteNode::Element(e)) => {
                assert_eq!(e.overrides.len(), 1, "no duplicates");
                assert_eq!(e.overrides[0].value, "B");
            }
            _ => panic!("instance missing"),
        }
        // Copy text follows the last value.
        match crate::model::tree_ops::find(&root, 5) {
            Some(SvelteNode::Text(t)) => assert_eq!(t.content, "B"),
            _ => panic!("instance text missing"),
        }
    }

    #[test]
    fn set_override_with_same_value_is_a_noop() {
        let state = seed_with_component();
        state.set_override("title", "Same".into());
        let history_len = state.history.borrow().len();
        state.set_override("title", "Same".into());
        assert_eq!(
            state.history.borrow().len(),
            history_len,
            "identical value must not push history"
        );
    }

    #[test]
    fn master_selection_exposes_interface() {
        let state = seed_with_component();
        state.selected_layer.set(Some(1));
        let props = state.selected_master_props();
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].name, "title");
        assert!(state.selected_instance_ctx().is_none());
    }

    #[test]
    fn contract_edits_add_rename_and_drop_events() {
        use crate::model::behavior::{Action, Trigger};
        let state = seed_with_component();
        state.selected_layer.set(Some(1));

        state.add_interface_event();
        state.rename_interface_event("event1", "confirm");
        assert_eq!(
            state.selected_master_interface().unwrap().events,
            vec!["confirm"]
        );

        // An Emit("confirm") rule on the master follows the rename.
        state.add_rule(Trigger::OnClick);
        state.add_action(0, Action::Emit("confirm".into()));
        state.rename_interface_event("confirm", "cancel");
        assert_eq!(
            state.selected_master_interface().unwrap().events,
            vec!["cancel"]
        );
        let root = state.root.get_untracked().unwrap();
        match crate::model::tree_ops::find(&root, 1) {
            Some(SvelteNode::Element(e)) => assert_eq!(
                e.behavior.as_ref().unwrap().rules[0].actions,
                vec![Action::Emit("cancel".into())],
            ),
            _ => panic!("master missing"),
        }

        state.remove_interface_event("cancel");
        assert!(state.selected_master_interface().unwrap().events.is_empty());
    }

    #[test]
    fn event_rename_rejects_bad_or_duplicate_names() {
        let state = seed_with_component();
        state.selected_layer.set(Some(1));
        state.add_interface_event(); // event1
        state.rename_interface_event("event1", "1bad");
        assert_eq!(
            state.selected_master_interface().unwrap().events,
            vec!["event1"]
        );
        state.add_interface_event(); // event2
        state.rename_interface_event("event1", "event2");
        assert_eq!(
            state.selected_master_interface().unwrap().events,
            vec!["event1", "event2"],
            "duplicate name rejected"
        );
    }

    #[test]
    fn prop_edits_cover_name_kind_target_and_propagation() {
        use crate::model::behavior::Trigger;
        use crate::model::svelte_ast::PropKind;
        let state = seed_with_component();
        state.selected_layer.set(Some(1));

        state.add_interface_prop();
        let it = state.selected_master_interface().unwrap();
        assert_eq!(it.props[1].name, "prop1");
        assert_eq!(it.props[1].target, 2, "first descendant element");
        assert_eq!(it.props[1].kind, PropKind::Text);

        // On Prop Changed + instance override follow the rename.
        state.add_rule(Trigger::OnPropChanged("prop1".into()));
        state.selected_layer.set(Some(4));
        // "Hello" is the instance's current text content — set_override
        // treats it as a no-op; use a different value so it is recorded.
        state.set_override("prop1", "Hi!".into());
        state.selected_layer.set(Some(1));
        state.rename_interface_prop("prop1", "label");
        state.set_interface_prop_kind("label", PropKind::Fill);
        state.set_interface_prop_target("label", 1);

        let it = state.selected_master_interface().unwrap();
        assert_eq!(it.props[1].name, "label");
        assert_eq!(it.props[1].kind, PropKind::Fill);
        assert_eq!(it.props[1].target, 1);
        let root = state.root.get_untracked().unwrap();
        match crate::model::tree_ops::find(&root, 1) {
            Some(SvelteNode::Element(e)) => assert_eq!(
                e.behavior.as_ref().unwrap().rules[0].trigger,
                Trigger::OnPropChanged("label".into()),
            ),
            _ => panic!("master missing"),
        }
        match crate::model::tree_ops::find(&root, 4) {
            Some(SvelteNode::Element(inst)) => {
                assert_eq!(inst.overrides[0].prop, "label", "override renamed too");
            }
            _ => panic!("instance missing"),
        }

        state.remove_interface_prop("label");
        assert_eq!(state.selected_master_interface().unwrap().props.len(), 1);
    }
}
