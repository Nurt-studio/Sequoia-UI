//! Editor actions for the Phase 2 behavior system: state variables, rules,
//! bindings and the CSS hover state. Mirrors `state/components.rs` — the AST
//! stays the single source of truth, these helpers only read and mutate
//! `root` (with a history push before each change).

use super::EditorState;
use crate::model::behavior::{
    self, Action, Behavior, Binding, HoverStyle, Issue, Rule, Trigger, Value, Var, VarType,
};
use crate::model::svelte_ast::{Element, SvelteNode};
use crate::state::PickerOption;

/// Everything the Interact tab renders for the selection, computed once per
/// rebuild (the body reads `root` tracked, so any edit rebuilds it).
pub(crate) struct BehaviorCtx {
    pub(crate) element_id: u64,
    pub(crate) element_name: String,
    pub(crate) is_master: bool,
    /// Instances are black boxes — their behavior belongs to the master.
    pub(crate) is_instance: bool,
    pub(crate) behavior: Behavior,
    pub(crate) binding: Option<Binding>,
    pub(crate) hover: Option<HoverStyle>,
    /// Variables visible from this element (nearest owner first).
    pub(crate) scope_vars: Vec<Var>,
    /// `true` when the selection itself owns the variable scope (a master
    /// or the document root) — only then the STATE section edits vars.
    pub(crate) var_owner_is_self: bool,
    /// Name of the element owning the variable scope (hint in the panel).
    pub(crate) var_owner_name: String,
    pub(crate) master_events: Vec<String>,
    pub(crate) targets: Vec<(u64, String, bool)>,
    pub(crate) issues: Vec<Issue>,
}

impl EditorState {
    /// Behavior context of the selected element (any element can carry rules).
    pub(crate) fn selected_behavior_ctx(&self) -> Option<BehaviorCtx> {
        let id = self.selected_layer.get_untracked()?;
        let root = self.root.get_untracked();
        let r = root.as_ref()?;
        let Some(SvelteNode::Element(e)) = crate::model::tree_ops::find(r, id) else {
            return None;
        };
        let is_instance = e.instance;
        let behavior = e.behavior.clone().unwrap_or_default();
        let binding = e.binding.clone();
        let hover = e.hover.clone();
        let scope_vars = behavior::scope_vars(r, id);
        let var_owner_id = behavior::var_owner_id(r, id);
        let var_owner_is_self = var_owner_id == id;
        let var_owner_name = crate::model::tree_ops::find(r, var_owner_id)
            .map(|n| n.name().to_string())
            .unwrap_or_default();
        let master_events = e
            .interface
            .as_ref()
            .map(|it| it.events.clone())
            .unwrap_or_default();
        let scope = behavior::scope_root(r, id);
        let targets = behavior::descendant_targets(scope);
        let issues = behavior::validate(r, id);
        Some(BehaviorCtx {
            element_id: id,
            element_name: e.name.clone(),
            is_master: e.component,
            is_instance,
            behavior,
            binding,
            hover,
            scope_vars,
            var_owner_is_self,
            var_owner_name,
            master_events,
            targets,
            issues,
        })
    }

    // ────────────────────────────────────────────── shared plumbing

    fn mutate_selected_element(&self, f: impl FnOnce(&mut Element)) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        let root = self.root.get_untracked();
        let is_element = root
            .as_ref()
            .and_then(|r| crate::model::tree_ops::find(r, id))
            .map(|n| matches!(n, SvelteNode::Element(_)))
            .unwrap_or(false);
        if !is_element {
            return;
        }
        self.push_history();
        self.root.update(|r| {
            if let Some(rr) = r {
                if let Some(SvelteNode::Element(e)) = crate::model::tree_ops::find_mut(rr, id) {
                    f(e);
                }
            }
        });
    }

    /// Mutates the behavior of the selected element, creating it when absent.
    fn with_behavior(&self, f: impl FnOnce(&mut Behavior)) {
        self.mutate_selected_element(|e| {
            let b = e.behavior.get_or_insert_with(Behavior::default);
            f(b);
            if b.is_empty() {
                e.behavior = None;
            }
        });
    }

    /// Mutates the variable scope owner (master or document root).
    fn with_var_owner(&self, f: impl FnOnce(&mut Behavior)) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        let root = self.root.get_untracked();
        let Some(r) = root.as_ref() else {
            return;
        };
        let owner = behavior::var_owner_id(r, id);
        let is_element = crate::model::tree_ops::find(r, owner)
            .map(|n| matches!(n, SvelteNode::Element(_)))
            .unwrap_or(false);
        if !is_element {
            return;
        }
        self.push_history();
        self.root.update(|r| {
            if let Some(rr) = r {
                if let Some(SvelteNode::Element(e)) = crate::model::tree_ops::find_mut(rr, owner) {
                    let b = e.behavior.get_or_insert_with(Behavior::default);
                    f(b);
                    if b.is_empty() {
                        e.behavior = None;
                    }
                }
            }
        });
    }

    fn scope_var_names(&self) -> Vec<String> {
        let Some(id) = self.selected_layer.get_untracked() else {
            return Vec::new();
        };
        self.root
            .get_untracked()
            .as_ref()
            .map(|r| {
                behavior::scope_vars(r, id)
                    .into_iter()
                    .map(|v| v.name)
                    .collect()
            })
            .unwrap_or_default()
    }

    // ─────────────────────────────────────────────────── variables

    pub(crate) fn add_var(&self) {
        let existing = self.scope_var_names();
        let mut i = 1;
        while existing.iter().any(|n| n == &format!("var{i}")) {
            i += 1;
        }
        let name = format!("var{i}");
        self.with_var_owner(|b| {
            b.vars.push(Var::new(&name, VarType::Bool));
        });
    }

    pub(crate) fn remove_var(&self, name: &str) {
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        let root = self.root.get_untracked();
        let Some(r) = root.as_ref() else {
            return;
        };
        let owner = behavior::var_owner_id(r, id);
        self.push_history();
        self.root.update(|rr| {
            let Some(rr) = rr else { return };
            let Some(start) = crate::model::tree_ops::find_mut(rr, owner) else {
                return;
            };
            remove_var_in_scope(start, owner, name);
        });
    }

    pub(crate) fn rename_var(&self, old: &str, new: &str) {
        if !behavior::is_valid_var_name(new) {
            self.toast_warn("Variable name must be a JS identifier");
            return;
        }
        if old == new {
            return;
        }
        let names = self.scope_var_names();
        if names.iter().any(|n| n == new) {
            self.toast_warn("A variable with this name already exists in scope");
            return;
        }
        // Rename the declaration on the owner and every reference in the
        // variable's scope: rules on any element (nested Delay steps
        // included) and bindings, so nothing is left dangling.
        let Some(id) = self.selected_layer.get_untracked() else {
            return;
        };
        let root = self.root.get_untracked();
        let Some(r) = root.as_ref() else {
            return;
        };
        let owner = behavior::var_owner_id(r, id);
        self.push_history();
        self.root.update(|rr| {
            let Some(rr) = rr else { return };
            let Some(start) = crate::model::tree_ops::find_mut(rr, owner) else {
                return;
            };
            rename_var_in_scope(start, owner, old, new);
        });
    }

    pub(crate) fn set_var_type(&self, name: &str, ty: VarType) {
        self.with_var_owner(|b| {
            for v in &mut b.vars {
                if v.name == name {
                    v.ty = ty;
                    v.initial = ty.default_value();
                }
            }
        });
    }

    pub(crate) fn set_var_initial_bool(&self, name: &str, val: bool) {
        self.with_var_owner(|b| {
            for v in &mut b.vars {
                if v.name == name && v.ty == VarType::Bool {
                    v.initial = Value::Bool(val);
                }
            }
        });
    }

    pub(crate) fn set_var_initial_number(&self, name: &str, val: f32) {
        self.with_var_owner(|b| {
            for v in &mut b.vars {
                if v.name == name && v.ty == VarType::Number {
                    v.initial = Value::Number(val);
                }
            }
        });
    }

    pub(crate) fn set_var_initial_str(&self, name: &str, val: String) {
        self.with_var_owner(|b| {
            for v in &mut b.vars {
                if v.name == name && v.ty == VarType::String {
                    v.initial = Value::Str(val.clone());
                }
            }
        });
    }

    // ─────────────────────────────────────────────────────── rules

    pub(crate) fn add_rule(&self, trigger: Trigger) {
        self.with_behavior(|b| b.rules.push(Rule::new(trigger)));
    }

    pub(crate) fn remove_rule(&self, index: usize) {
        self.with_behavior(|b| {
            if index < b.rules.len() {
                b.rules.remove(index);
            }
        });
    }

    pub(crate) fn set_rule_trigger(&self, index: usize, trigger: Trigger) {
        self.with_behavior(|b| {
            if let Some(r) = b.rules.get_mut(index) {
                r.trigger = trigger;
            }
        });
    }

    pub(crate) fn set_rule_key(&self, index: usize, key: String) {
        self.with_behavior(|b| {
            if let Some(r) = b.rules.get_mut(index) {
                r.trigger = Trigger::OnKey(key);
            }
        });
    }

    pub(crate) fn add_action(&self, rule: usize, action: Action) {
        self.with_behavior(|b| {
            if let Some(r) = b.rules.get_mut(rule) {
                r.actions.push(action);
            }
        });
    }

    pub(crate) fn remove_action(&self, rule: usize, index: usize) {
        self.with_behavior(|b| {
            if let Some(r) = b.rules.get_mut(rule) {
                if index < r.actions.len() {
                    r.actions.remove(index);
                }
            }
        });
    }

    pub(crate) fn replace_action(&self, rule: usize, index: usize, action: Action) {
        self.with_behavior(|b| {
            if let Some(r) = b.rules.get_mut(rule) {
                if index < r.actions.len() {
                    r.actions[index] = action;
                }
            }
        });
    }

    // ─────────────────────────────────────── delays (Phase 3)

    /// Duration of the Delay at `rule.actions[delay]`.
    pub(crate) fn set_delay_ms(&self, rule: usize, delay: usize, ms: u32) {
        self.with_behavior(|b| {
            if let Some(r) = b.rules.get_mut(rule)
                && let Some(Action::Delay(m, _)) = r.actions.get_mut(delay)
            {
                *m = ms;
            }
        });
    }

    /// Appends a step to the Delay's nested sequence.
    pub(crate) fn add_nested_action(&self, rule: usize, delay: usize, action: Action) {
        self.with_behavior(|b| {
            if let Some(r) = b.rules.get_mut(rule)
                && let Some(Action::Delay(_, nested)) = r.actions.get_mut(delay)
            {
                nested.push(action);
            }
        });
    }

    pub(crate) fn remove_nested_action(&self, rule: usize, delay: usize, index: usize) {
        self.with_behavior(|b| {
            if let Some(r) = b.rules.get_mut(rule)
                && let Some(Action::Delay(_, nested)) = r.actions.get_mut(delay)
                && index < nested.len()
            {
                nested.remove(index);
            }
        });
    }

    pub(crate) fn replace_nested_action(
        &self,
        rule: usize,
        delay: usize,
        index: usize,
        action: Action,
    ) {
        self.with_behavior(|b| {
            if let Some(r) = b.rules.get_mut(rule)
                && let Some(Action::Delay(_, nested)) = r.actions.get_mut(delay)
                && index < nested.len()
            {
                nested[index] = action;
            }
        });
    }

    // ───────────────────────────────────────────────────── binding

    pub(crate) fn set_binding_var(&self, name: Option<String>) {
        self.mutate_selected_element(|e| match name {
            None => e.binding = None,
            Some(v) => match &mut e.binding {
                Some(b) => b.var = v,
                None => {
                    e.binding = Some(Binding {
                        var: v,
                        transition: None,
                    })
                }
            },
        });
    }

    /// `kind` "" clears the transition; otherwise slide/fade/scale.
    pub(crate) fn set_binding_transition(&self, kind: &str, ms: f32) {
        let ms = ms.max(0.0) as u32;
        self.mutate_selected_element(|e| {
            if let Some(b) = &mut e.binding {
                b.transition = behavior::Transition::from_kind(kind, ms);
            }
        });
    }

    // ─────────────────────────────────────────────────────── hover

    pub(crate) fn set_hover_fill(&self, hex: Option<String>) {
        self.mutate_selected_element(|e| {
            let mut h = e.hover.take().unwrap_or_default();
            h.fill = hex;
            if h.is_empty() {
                e.hover = None;
            } else {
                e.hover = Some(h);
            }
        });
    }

    pub(crate) fn set_hover_color(&self, hex: Option<String>) {
        self.mutate_selected_element(|e| {
            let mut h = e.hover.take().unwrap_or_default();
            h.color = hex;
            if h.is_empty() {
                e.hover = None;
            } else {
                e.hover = Some(h);
            }
        });
    }

    pub(crate) fn set_hover_opacity(&self, opacity: Option<f32>) {
        self.mutate_selected_element(|e| {
            let mut h = e.hover.take().unwrap_or_default();
            h.opacity = opacity;
            if h.is_empty() {
                e.hover = None;
            } else {
                e.hover = Some(h);
            }
        });
    }

    // ─────────────────────────────────────────────── picker options

    /// Options for the variable dropdown (rules + binding).
    pub(crate) fn var_picker_options(&self) -> Vec<PickerOption> {
        self.scope_var_names()
            .into_iter()
            .map(|n| PickerOption {
                label: n.clone(),
                value: n,
            })
            .collect()
    }

    /// Options for the Show/Hide target dropdown (own component subtree).
    pub(crate) fn target_picker_options(&self) -> Vec<PickerOption> {
        let Some(id) = self.selected_layer.get_untracked() else {
            return Vec::new();
        };
        self.root
            .get_untracked()
            .as_ref()
            .map(|r| {
                let scope = behavior::scope_root(r, id);
                behavior::descendant_targets(scope)
                    .into_iter()
                    .filter(|(tid, _, _)| *tid != id)
                    .map(|(_, name, bound)| PickerOption {
                        label: if bound {
                            format!("{name} ◇")
                        } else {
                            name.clone()
                        },
                        value: name,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Options for the Emit dropdown: interface events of the nearest master.
    pub(crate) fn event_picker_options(&self) -> Vec<PickerOption> {
        let Some(id) = self.selected_layer.get_untracked() else {
            return Vec::new();
        };
        self.root
            .get_untracked()
            .as_ref()
            .and_then(|r| {
                let mut cursor = crate::model::tree_ops::find(r, id)?;
                loop {
                    if let SvelteNode::Element(e) = cursor {
                        if let Some(it) = &e.interface {
                            if !it.events.is_empty() {
                                return Some(
                                    it.events
                                        .iter()
                                        .map(|ev| PickerOption {
                                            label: ev.clone(),
                                            value: ev.clone(),
                                        })
                                        .collect(),
                                );
                            }
                        }
                    }
                    cursor = crate::model::tree_ops::parent_of(r, cursor.id())?;
                }
            })
            .unwrap_or_default()
    }
}

fn rename_in_action(a: &mut Action, old: &str, new: &str) {
    match a {
        Action::ToggleVar(v) | Action::SetVar(v, _) if v == old => *v = new.to_string(),
        Action::SetVar(_, expr) => {
            if let behavior::ValueExpr::Var(name) = expr {
                if name == old {
                    *name = new.to_string();
                }
            }
        }
        _ => {}
    }
}

/// Drops the dangling `Toggle`/`Set` actions of one rule — top level and
/// nested inside Delay alike; the (now empty) rule stays and the validation
/// flags it.
fn prune_var_actions(b: &mut Behavior, name: &str) {
    for r in &mut b.rules {
        r.actions.retain(|a| match a {
            Action::ToggleVar(v) | Action::SetVar(v, _) => v != name,
            _ => true,
        });
        for a in &mut r.actions {
            if let Action::Delay(_, nested) = a {
                nested.retain(
                    |a| !matches!(a, Action::ToggleVar(v) | Action::SetVar(v, _) if v == name),
                );
            }
        }
    }
}

/// Removes a variable from its owner and prunes every reference in the
/// scope — the owner plus all elements below it, which is exactly where the
/// variable is visible and therefore referenced.
fn remove_var_in_scope(node: &mut SvelteNode, owner: u64, name: &str) {
    let Some(SvelteNode::Element(e)) = Some(node) else {
        return;
    };
    if e.id == owner {
        let drop = if let Some(b) = &mut e.behavior {
            b.vars.retain(|v| v.name != name);
            prune_var_actions(b, name);
            b.is_empty()
        } else {
            false
        };
        if drop {
            e.behavior = None;
        }
    } else if let Some(b) = &mut e.behavior {
        prune_var_actions(b, name);
    }
    for c in e.children.iter_mut() {
        remove_var_in_scope(c, owner, name);
    }
}

/// Renames a variable on its owner and every reference in the scope: rules
/// on any element (nested Delay steps included) and bindings.
fn rename_var_in_scope(node: &mut SvelteNode, owner: u64, old: &str, new: &str) {
    let Some(SvelteNode::Element(e)) = Some(node) else {
        return;
    };
    if e.id == owner {
        if let Some(b) = &mut e.behavior {
            for v in &mut b.vars {
                if v.name == old {
                    v.name = new.to_string();
                }
            }
        }
    }
    if let Some(b) = &mut e.behavior {
        for r in &mut b.rules {
            for a in &mut r.actions {
                rename_in_action(a, old, new);
                if let Action::Delay(_, nested) = a {
                    for a in nested {
                        rename_in_action(a, old, new);
                    }
                }
            }
        }
    }
    if let Some(b) = &mut e.binding {
        if b.var == old {
            b.var = new.to_string();
        }
    }
    for c in e.children.iter_mut() {
        rename_var_in_scope(c, owner, old, new);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::behavior::ValueExpr;
    use crate::model::svelte_ast::new_frame;

    /// Page(1){ master Card(2){ Btn(3) }, Panel(4) }; selection defaults to
    /// the master (2) so the state owns its variable scope.
    fn seed(selected: u64) -> EditorState {
        let mut btn = crate::model::svelte_ast::new_button(3, 0.0, 0.0);
        if let SvelteNode::Element(e) = &mut btn {
            e.name = "Btn".into();
        }
        let mut panel = new_frame(400, 0.0, 0.0, 80.0, 40.0);
        if let SvelteNode::Element(e) = &mut panel {
            e.name = "Panel".into();
        }
        let mut card = new_frame(2, 0.0, 0.0, 100.0, 40.0);
        if let SvelteNode::Element(e) = &mut card {
            e.component = true;
            e.name = "Card".into();
            e.children = vec![btn];
        }
        let mut root = new_frame(1, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.children = vec![card, panel];
        }
        EditorState::with_seed(Some(root), Some(selected))
    }

    fn element(root: &SvelteNode, id: u64) -> Element {
        match crate::model::tree_ops::find(root, id) {
            Some(SvelteNode::Element(e)) => e.clone(),
            _ => panic!("element {id} missing"),
        }
    }

    #[test]
    fn add_var_lands_on_the_scope_owner() {
        let st = seed(3); // the button inside the master
        st.add_var();
        let root = st.root.get_untracked().unwrap();
        let card = element(&root, 2);
        assert_eq!(card.behavior.unwrap().vars.len(), 1, "master owns vars");
        let btn = element(&root, 3);
        assert!(btn.behavior.is_none(), "the trigger element stays clean");

        // The context reports the scope for the button.
        st.selected_layer.set(Some(3));
        let ctx = st.selected_behavior_ctx().unwrap();
        assert_eq!(ctx.scope_vars.len(), 1);
        assert!(!ctx.var_owner_is_self);
    }

    #[test]
    fn rule_lifecycle_updates_the_element() {
        let st = seed(3);
        st.add_rule(Trigger::OnClick);
        st.add_action(0, Action::ToggleVar("open".into()));

        // `open` does not exist yet → validation reports the error…
        let ctx = st.selected_behavior_ctx().unwrap();
        assert!(ctx.issues.iter().any(|i| i.severity == issue_severity()));

        // …adding the variable fixes it.
        st.add_var();
        st.rename_var("var1", "open");
        let ctx = st.selected_behavior_ctx().unwrap();
        assert!(
            !ctx.issues
                .iter()
                .any(|i| i.severity == crate::model::behavior::Severity::Error)
        );

        let root = st.root.get_untracked().unwrap();
        let btn = element(&root, 3);
        let b = btn.behavior.unwrap();
        assert_eq!(b.rules.len(), 1);
        assert_eq!(b.rules[0].actions, vec![Action::ToggleVar("open".into())]);
        // The variable landed on the master (scope owner), not on the button.
        assert_eq!(element(&root, 2).behavior.unwrap().vars[0].name, "open");
    }

    fn issue_severity() -> crate::model::behavior::Severity {
        crate::model::behavior::Severity::Error
    }

    #[test]
    fn set_var_type_resets_the_initial_value() {
        let st = seed(2);
        st.add_var();
        st.set_var_type("var1", VarType::Number);
        st.set_var_initial_number("var1", 4.0);
        let root = st.root.get_untracked().unwrap();
        let card = element(&root, 2);
        let v = &card.behavior.unwrap().vars[0];
        assert_eq!(v.ty, VarType::Number);
        assert_eq!(v.initial, Value::Number(4.0));
        st.set_var_type("var1", VarType::Bool);
        let root = st.root.get_untracked().unwrap();
        let v = &element(&root, 2).behavior.unwrap().vars[0];
        assert_eq!(v.initial, Value::Bool(false), "type switch resets initial");
    }

    #[test]
    fn remove_var_drops_dangling_actions() {
        let st = seed(2);
        st.add_var();
        st.rename_var("var1", "open");
        st.selected_layer.set(Some(3));
        st.add_rule(Trigger::OnClick);
        st.add_action(0, Action::ToggleVar("open".into()));
        st.add_action(0, Action::Emit("confirm".into()));

        st.remove_var("open");
        let root = st.root.get_untracked().unwrap();
        let btn = element(&root, 3);
        let actions = &btn.behavior.unwrap().rules[0].actions;
        assert_eq!(actions.len(), 1, "Toggle open is gone");
        assert_eq!(actions[0].kind_name(), "emit");
    }

    #[test]
    fn binding_and_transition_are_set_on_the_selection() {
        let st = seed(400); // the page-level panel (id 400 — button(3)
        // carries an internal label with id 4, so it must not be reused)
        st.add_var(); // lands on root (page scope)
        st.rename_var("var1", "open");
        st.set_binding_var(Some("open".into()));
        st.set_binding_transition("slide", 200.0);

        let root = st.root.get_untracked().unwrap();
        let panel = element(&root, 400);
        let b = panel.binding.expect("binding set");
        assert_eq!(b.var, "open");
        assert_eq!(
            b.transition,
            Some(crate::model::behavior::Transition::Slide { ms: 200 })
        );

        st.set_binding_var(None);
        let root = st.root.get_untracked().unwrap();
        assert!(element(&root, 400).binding.is_none());
    }

    #[test]
    fn set_value_action_replaces_in_place() {
        let st = seed(3);
        st.add_rule(Trigger::OnHoverIn);
        st.add_var();
        st.add_action(0, Action::ToggleVar("var1".into()));
        st.replace_action(
            0,
            0,
            Action::SetVar("var1".into(), ValueExpr::Value(Value::Bool(true))),
        );
        let root = st.root.get_untracked().unwrap();
        let btn = element(&root, 3);
        let actions = &btn.behavior.unwrap().rules[0].actions;
        assert_eq!(
            actions[0],
            Action::SetVar("var1".into(), ValueExpr::Value(Value::Bool(true)))
        );
        st.remove_action(0, 0);
        let root = st.root.get_untracked().unwrap();
        assert!(
            element(&root, 3).behavior.unwrap().rules[0]
                .actions
                .is_empty()
        );
    }

    #[test]
    fn hover_fields_are_independent() {
        let st = seed(2);
        st.set_hover_fill(Some("#3A3A3E".into()));
        st.set_hover_opacity(Some(0.8));
        let root = st.root.get_untracked().unwrap();
        let h = element(&root, 2).hover.unwrap();
        assert_eq!(h.fill.as_deref(), Some("#3A3A3E"));
        assert_eq!(h.opacity, Some(0.8));

        st.set_hover_fill(None);
        st.set_hover_opacity(None);
        let root = st.root.get_untracked().unwrap();
        assert!(element(&root, 2).hover.is_none(), "empty hover is removed");
    }

    #[test]
    fn delay_edits_ms_and_nested_steps() {
        let st = seed(2);
        st.add_var();
        st.rename_var("var1", "open");
        st.add_rule(Trigger::OnClick);
        st.add_action(0, Action::Delay(200, vec![]));
        st.add_action(0, Action::ToggleVar("open".into()));

        // Nested step + duration edit.
        st.add_nested_action(0, 0, Action::ToggleVar("open".into()));
        st.set_delay_ms(0, 0, 450);
        let root = st.root.get_untracked().unwrap();
        let actions = &element(&root, 2).behavior.unwrap().rules[0].actions;
        assert_eq!(
            actions[0],
            Action::Delay(450, vec![Action::ToggleVar("open".into())])
        );
        assert_eq!(actions[1], Action::ToggleVar("open".into()));

        // Replace / remove a nested step in place.
        st.replace_nested_action(0, 0, 0, Action::Emit("confirm".into()));
        st.remove_nested_action(0, 0, 0);
        let root = st.root.get_untracked().unwrap();
        let actions = &element(&root, 2).behavior.unwrap().rules[0].actions;
        assert_eq!(actions[0], Action::Delay(450, vec![]));

        // Renaming the variable also rewrites nested references…
        st.add_nested_action(0, 0, Action::ToggleVar("open".into()));
        st.rename_var("open", "visible");
        let root = st.root.get_untracked().unwrap();
        let actions = &element(&root, 2).behavior.unwrap().rules[0].actions;
        assert_eq!(
            actions[0],
            Action::Delay(450, vec![Action::ToggleVar("visible".into())])
        );
        assert_eq!(actions[1], Action::ToggleVar("visible".into()));

        // …and removing it prunes dangling actions across the whole scope —
        // the owner's own top-level toggles too (empty rules stay and the
        // validation flags them).
        st.remove_var("visible");
        let root = st.root.get_untracked().unwrap();
        let actions = &element(&root, 2).behavior.unwrap().rules[0].actions;
        assert_eq!(actions[0], Action::Delay(450, vec![]));
        assert_eq!(actions.len(), 1, "top-level toggle is pruned as well");
    }
}
