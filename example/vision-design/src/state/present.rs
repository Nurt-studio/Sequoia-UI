//! Present interpreter (Phase 4, design doc §7): the same `Behavior` that
//! codegen emits runs directly on the canvas.
//!
//! - runtime variables live per declaring element: `(owner id, name) → value`
//!   (the layout engine resolves bindings through the ancestor chain, exactly
//!   like `behavior::scope_vars`),
//! - clicks/hovers/keys fire the matching rules of the hit chain, like DOM
//!   events bubbling,
//! - `Show`/`Hide` assign the target's binding variable — the same bridge the
//!   codegen uses — and start a transition,
//! - `Emit` routes to the nearest instance ancestor's `On Event` rules (the
//!   on-canvas counterpart of `on:{event}` on the instance tag),
//! - `Delay` parks its nested steps in a timer queue drained by the paint loop,
//! - transitions animate at paint time (fade → alpha, slide → offset,
//!   scale → scale around the target's center), so no timers or threads are
//!   needed: the canvas repaints while anims or timers are alive.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::{Duration, Instant};

use sequoia_ui::prelude::*;

use super::EditorState;
use crate::core::hit_test::hit_layer;
use crate::core::layer::Layer;
use crate::model::behavior::{self, Action, Trigger, Value, ValueExpr};
use crate::model::layout_engine::PresentVars;
use crate::model::svelte_ast::{
    ProtoAction, ProtoTransition, ProtoTrigger, PrototypeLink, SvelteNode,
};
use crate::model::tree_ops;

/// One running transition. `entering == false` keeps the target rendered
/// while it animates out (the binding already flipped it off in the layout).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Anim {
    pub(crate) target: u64,
    pub(crate) kind: &'static str,
    pub(crate) ms: u32,
    pub(crate) start: Instant,
    pub(crate) entering: bool,
}

impl Anim {
    /// 0..1 progress, clamped.
    pub(crate) fn progress(&self) -> f32 {
        let el = self.start.elapsed().as_millis() as f32;
        (el / self.ms.max(1) as f32).clamp(0.0, 1.0)
    }
}

/// Screen-to-screen navigation running at paint time.
#[derive(Clone, Debug)]
pub(crate) struct NavAnim {
    pub(crate) from: Option<u64>,
    pub(crate) to: Option<u64>,
    pub(crate) kind: ProtoTransition,
    pub(crate) ms: u32,
    pub(crate) start: Instant,
}

impl NavAnim {
    fn progress(&self) -> f32 {
        let el = self.start.elapsed().as_millis() as f32;
        (el / self.ms.max(1) as f32).clamp(0.0, 1.0)
    }
}

/// Deferred `Delay` steps: they fire when the paint loop notices they are due.
struct PendingTimer {
    at: Instant,
    actions: Vec<Action>,
    element: u64,
    payload: Value,
}

/// Paint-time effect of the transitions, composed per layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Fx {
    pub(crate) alpha: f32,
    pub(crate) dx: f32,
    pub(crate) dy: f32,
    pub(crate) scale: f32,
}

impl Default for Fx {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            dx: 0.0,
            dy: 0.0,
            scale: 1.0,
        }
    }
}

#[derive(Clone)]
pub(crate) struct PresentState {
    pub(crate) on: Signal<bool>,
    pub(crate) vars: Signal<PresentVars>,
    pub(crate) anims: Signal<Vec<Anim>>,
    /// Focused screen frame (`None` = whole document).
    pub(crate) screen: Signal<Option<u64>>,
    /// Screen animating out while `screen` fades in.
    pub(crate) prev_screen: Signal<Option<u64>>,
    /// Overlay frames shown above the focused screen.
    pub(crate) overlays: Signal<Vec<u64>>,
    /// Bumped on every navigation so the canvas repaints.
    pub(crate) nav_epoch: Signal<u64>,
    nav: Rc<RefCell<Option<NavAnim>>>,
    /// Screen history for `Back` links and Esc.
    stack: Rc<RefCell<Vec<Option<u64>>>>,
    timers: Rc<RefCell<Vec<PendingTimer>>>,
    /// Element ids currently under the pointer (On Hover In/Out rules).
    pub(crate) hovered: Rc<RefCell<Vec<u64>>>,
}

impl PresentState {
    pub(crate) fn new() -> Self {
        Self {
            on: Signal::new(false),
            vars: Signal::new(PresentVars::new()),
            anims: Signal::new(Vec::new()),
            screen: Signal::new(None),
            prev_screen: Signal::new(None),
            overlays: Signal::new(Vec::new()),
            nav_epoch: Signal::new(0),
            nav: Rc::new(RefCell::new(None)),
            stack: Rc::new(RefCell::new(Vec::new())),
            timers: Rc::new(RefCell::new(Vec::new())),
            hovered: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl EditorState {
    // ───────────────────────────────────────────── session

    pub(crate) fn toggle_present(&self) {
        if self.present.on.get_untracked() {
            self.stop_present();
        } else {
            self.start_present();
        }
    }

    /// Enters Present mode: every declared variable starts at its initial
    /// value, then `On Mount` rules run (design doc §7).
    pub(crate) fn start_present(&self) {
        self.present.screen.set(None);
        self.present.prev_screen.set(None);
        self.present.overlays.set(Vec::new());
        *self.present.nav.borrow_mut() = None;
        self.present.stack.borrow_mut().clear();
        let mut vars = PresentVars::new();
        if let Some(r) = self.root.get_untracked() {
            // Multi-screen documents start on their first screen instead of
            // showing every screen stacked on the canvas.
            if let Some(first) = initial_screen(&r) {
                self.present.screen.set(Some(first));
            }
            collect_initial_vars(&r, &mut vars);
            self.present.vars.set(vars);
            self.present.hovered.borrow_mut().clear();
            self.present.timers.borrow_mut().clear();
            self.present.anims.set(Vec::new());
            // On Mount rules, document order.
            let mounts = rules_with_trigger(&r, |t| matches!(t, Trigger::OnMount));
            for (id, actions) in mounts {
                self.execute_actions(&actions, id, &Value::Str(String::new()), 0);
            }
        }
        self.present.on.set(true);
        self.toast("Present — click the canvas · Esc exits");
    }

    pub(crate) fn stop_present(&self) {
        if !self.present.on.get_untracked() {
            return;
        }
        self.present.on.set(false);
        self.present.vars.set(PresentVars::new());
        self.present.anims.set(Vec::new());
        self.present.screen.set(None);
        self.present.prev_screen.set(None);
        self.present.overlays.set(Vec::new());
        *self.present.nav.borrow_mut() = None;
        self.present.stack.borrow_mut().clear();
        self.present.timers.borrow_mut().clear();
        self.present.hovered.borrow_mut().clear();
    }

    // ───────────────────────────────────────────── input

    /// Layers that can receive Present input: the focused screen's subtree
    /// (plus overlays). Screens hidden by the focus must not win hit tests,
    /// even before the canvas recomputes its layer snapshot.
    fn present_hit_layers(&self) -> Vec<Layer> {
        let mut layers = self.layers.get_untracked();
        let focus = self.present.screen.get_untracked();
        let Some(screen) = focus else {
            return layers;
        };
        let mut ids = subtree_ids(&layers, screen);
        for ov in self.present.overlays.get_untracked() {
            ids.extend(subtree_ids(&layers, ov));
        }
        layers.retain(|l| ids.contains(&l.id));
        layers
    }

    /// A left click on the canvas in Present mode: fire the `On Click` rules
    /// of the hit chain (topmost first, bubbling like in the DOM).
    pub(crate) fn present_click(&self, wx: f32, wy: f32) {
        let layers = self.present_hit_layers();
        let Some(hit) = hit_layer(&layers, wx, wy) else {
            return;
        };
        let chain = ancestor_chain_ids(&layers, hit);
        let Some(r) = self.root.get_untracked() else {
            return;
        };
        // Prototype connections win over behavior rules: a link is the
        // presentation-level "go to this screen" gesture.
        for id in &chain {
            if let Some(SvelteNode::Element(e)) = tree_ops::find(&r, *id)
                && let Some(link) = &e.prototype
                && link.trigger == ProtoTrigger::Click
            {
                let link = link.clone();
                self.fire_prototype(&link);
                return;
            }
        }

        let mut fired: Vec<(u64, Vec<Action>)> = Vec::new();
        for id in chain {
            if let Some(SvelteNode::Element(e)) = tree_ops::find(&r, id)
                && let Some(b) = &e.behavior
            {
                for rule in &b.rules {
                    if matches!(rule.trigger, Trigger::OnClick) && !rule.actions.is_empty() {
                        fired.push((id, rule.actions.clone()));
                    }
                }
            }
        }
        for (id, actions) in fired {
            self.execute_actions(&actions, id, &Value::Str(String::new()), 0);
        }
    }

    /// Tracks the hovered chain and fires `On Hover In` / `On Hover Out`.
    pub(crate) fn present_hover(&self, wx: f32, wy: f32) {
        let layers = self.present_hit_layers();
        let now = hit_layer(&layers, wx, wy)
            .map(|h| ancestor_chain_ids(&layers, h))
            .unwrap_or_default();

        let Some(r) = self.root.get_untracked() else {
            return;
        };
        let mut entered: Vec<(u64, Vec<Action>)> = Vec::new();
        let mut left: Vec<(u64, Vec<Action>)> = Vec::new();
        {
            let prev = self.present.hovered.borrow();
            for id in &now {
                if !prev.contains(id)
                    && let Some(actions) =
                        rule_actions_of(&r, *id, |t| matches!(t, Trigger::OnHoverIn))
                {
                    entered.push((*id, actions));
                }
            }
            for id in prev.iter() {
                if !now.contains(id)
                    && let Some(actions) =
                        rule_actions_of(&r, *id, |t| matches!(t, Trigger::OnHoverOut))
                {
                    left.push((*id, actions));
                }
            }
        }
        *self.present.hovered.borrow_mut() = now.clone();

        // Hover prototype links fire for newly entered elements.
        let mut hover_link: Option<PrototypeLink> = None;
        {
            let prev = self.present.hovered.borrow();
            for id in &now {
                if prev.contains(id) {
                    continue;
                }
                if let Some(SvelteNode::Element(e)) = tree_ops::find(&r, *id)
                    && let Some(link) = &e.prototype
                    && link.trigger == ProtoTrigger::Hover
                {
                    hover_link = Some(link.clone());
                    break;
                }
            }
        }
        if let Some(link) = hover_link {
            self.fire_prototype(&link);
        }

        // Leave first (like CSS), then enter.
        for (id, actions) in left {
            self.execute_actions(&actions, id, &Value::Str(String::new()), 0);
        }
        for (id, actions) in entered {
            self.execute_actions(&actions, id, &Value::Str(String::new()), 0);
        }
    }

    /// `On Key` rules of the hovered/selected chain. `name` is the normalized
    /// key name (`Enter`, `a`, …).
    pub(crate) fn present_key(&self, name: &str) {
        let Some(r) = self.root.get_untracked() else {
            return;
        };
        let mut scope: Vec<u64> = self.present.hovered.borrow().clone();
        if scope.is_empty() {
            scope = self.selected_layers.get_untracked();
        }
        if scope.is_empty() {
            return;
        }
        let mut fired: Vec<(u64, Vec<Action>)> = Vec::new();
        for id in scope {
            if let Some(actions) =
                rule_actions_of(&r, id, |t| matches!(t, Trigger::OnKey(k) if k == name))
            {
                fired.push((id, actions));
            }
        }
        for (id, actions) in fired {
            self.execute_actions(&actions, id, &Value::Str(String::new()), 0);
        }
    }

    // ───────────────────────────────────────────── frame maintenance

    /// Called from the canvas paint loop: fires due `Delay` steps and prunes
    /// finished transitions. Returns `true` while something is still running
    /// (the canvas keeps repainting until then).
    pub(crate) fn advance_present(&self) -> bool {
        if !self.present.on.get_untracked() {
            return false;
        }
        let now = Instant::now();
        let due: Vec<PendingTimer> = {
            let mut timers = self.present.timers.borrow_mut();
            let mut rest = Vec::new();
            let mut ready = Vec::new();
            for t in timers.drain(..) {
                if t.at <= now {
                    ready.push(t);
                } else {
                    rest.push(t);
                }
            }
            *timers = rest;
            ready
        };
        for p in due {
            self.execute_actions(&p.actions, p.element, &p.payload, 0);
        }
        // Drop finished transitions (small grace so the last frame paints;
        // both directions must end, or the canvas would repaint forever).
        self.present.anims.update(|anims| {
            anims.retain(|a| (a.start.elapsed().as_millis() as u32) < a.ms.saturating_add(50))
        });
        let anims_live = !self.present.anims.get_untracked().is_empty();
        // Navigation: drop the outgoing screen once its transition is done.
        let nav_running = self
            .present
            .nav
            .borrow()
            .as_ref()
            .is_some_and(|n| (n.start.elapsed().as_millis() as u32) < n.ms.max(1));
        if !nav_running && self.present.nav.borrow().is_some() {
            *self.present.nav.borrow_mut() = None;
            self.present.prev_screen.set(None);
        }
        anims_live || nav_running || !self.present.timers.borrow().is_empty()
    }

    /// Composed paint effect per layer id (transitions of the layer and of
    /// every ancestor are folded together).
    pub(crate) fn present_fx(&self, layers: &[Layer]) -> HashMap<u64, Fx> {
        let mut out: HashMap<u64, Fx> = HashMap::new();
        if !self.present.on.get_untracked() {
            return out;
        }
        let anims = self.present.anims.get_untracked();
        for an in &anims {
            let Some(target) = layers.iter().find(|l| l.id == an.target) else {
                continue;
            };
            let p = an.progress();
            let t = if an.entering { p } else { 1.0 - p };
            let mut fx = Fx::default();
            match an.kind {
                "fade" => fx.alpha = t,
                "slide" => {
                    fx.dy = (1.0 - t) * target.h;
                    fx.alpha = t;
                }
                _ => {
                    // "scale"
                    fx.scale = 0.9 + 0.1 * t;
                    fx.alpha = t;
                }
            }
            // The target itself: scale around its own center.
            let entry = out.entry(an.target).or_default();
            let prev = *entry;
            *entry = compose_self(&prev, &fx);
            // Descendants: follow the ancestor's offset/scale, alpha multiplies.
            let ids = descendant_ids(layers, an.target);
            for id in ids {
                let Some(l) = layers.iter().find(|l| l.id == id) else {
                    continue;
                };
                let child_center = (l.x + l.w * 0.5, l.y + l.h * 0.5);
                let target_center = (target.x + target.w * 0.5, target.y + target.h * 0.5);
                let entry = out.entry(id).or_default();
                let prev = *entry;
                *entry = compose_child(&prev, &fx, child_center, target_center);
            }
        }
        // Screen navigation: the outgoing screen leaves, the incoming one
        // enters. Shared ancestors (the root) keep their effect.
        let nav = self.present.nav.borrow().clone();
        if let Some(nav) = nav {
            let e = ease_out(nav.progress());
            let from_ids = nav
                .from
                .map(|id| subtree_ids(layers, id))
                .unwrap_or_default();
            let to_ids = nav.to.map(|id| subtree_ids(layers, id)).unwrap_or_default();
            for l in layers {
                let in_from = from_ids.contains(&l.id);
                let in_to = to_ids.contains(&l.id);
                if in_from == in_to {
                    continue;
                }
                let entering = in_to;
                let (ox, oy) = nav_slide_offset(nav.kind, l.w, l.h);
                let entry = out.entry(l.id).or_default();
                if entering {
                    entry.alpha *= e;
                    entry.dx += ox * (1.0 - e);
                    entry.dy += oy * (1.0 - e);
                } else {
                    entry.alpha *= 1.0 - e;
                    entry.dx -= ox * e;
                    entry.dy -= oy * e;
                }
            }
        }
        out
    }

    /// Layers of the open overlay frames (the canvas dims the screen behind
    /// them and paints these last).
    pub(crate) fn overlay_ids(&self, layers: &[Layer]) -> HashSet<u64> {
        let mut out = HashSet::new();
        if !self.present.on.get_untracked() {
            return out;
        }
        for id in self.present.overlays.get_untracked() {
            out.extend(subtree_ids(layers, id));
        }
        out
    }

    /// Esc / Back links: close an overlay first, then walk the screen stack.
    /// Returns `false` when there is nothing left to go back to (leave Present).
    pub(crate) fn present_back(&self) -> bool {
        if !self.present.on.get_untracked() {
            return false;
        }
        {
            let overlays = self.present.overlays.get_untracked();
            if !overlays.is_empty() {
                self.present.overlays.update(|v| {
                    v.pop();
                });
                *self.present.nav.borrow_mut() = None;
                self.present.prev_screen.set(None);
                self.present.nav_epoch.update(|v| *v = v.wrapping_add(1));
                return true;
            }
        }
        let prev = self.present.stack.borrow_mut().pop();
        match prev {
            Some(to) => {
                self.goto_screen(to, ProtoTransition::Dissolve, 200, false);
                true
            }
            None => false,
        }
    }

    /// Runs one prototype connection.
    pub(crate) fn fire_prototype(&self, link: &PrototypeLink) {
        match link.action {
            ProtoAction::Navigate => {
                if let Some(target) = link.target {
                    self.goto_screen(Some(target), link.transition, link.duration_ms, true);
                }
            }
            ProtoAction::Back => {
                self.present_back();
            }
            ProtoAction::OpenOverlay => {
                if let Some(target) = link.target {
                    self.open_overlay(target, link.transition, link.duration_ms);
                }
            }
        }
    }

    /// Focuses `to` (a screen frame) with a transition.
    pub(crate) fn goto_screen(
        &self,
        to: Option<u64>,
        kind: ProtoTransition,
        ms: u32,
        remember: bool,
    ) {
        let from = self.present.screen.get_untracked();
        if to == from && self.present.overlays.get_untracked().is_empty() {
            return;
        }
        if remember {
            self.present.stack.borrow_mut().push(from);
        }
        self.present.overlays.set(Vec::new());
        if kind == ProtoTransition::Instant {
            *self.present.nav.borrow_mut() = None;
            self.present.prev_screen.set(None);
        } else {
            *self.present.nav.borrow_mut() = Some(NavAnim {
                from,
                to,
                kind,
                ms,
                start: Instant::now(),
            });
            self.present.prev_screen.set(from);
        }
        self.present.screen.set(to);
        self.present.nav_epoch.update(|v| *v = v.wrapping_add(1));
    }

    /// Shows `target` above the current screen.
    pub(crate) fn open_overlay(&self, target: u64, kind: ProtoTransition, ms: u32) {
        if self.present.overlays.get_untracked().contains(&target) {
            return;
        }
        self.present.overlays.update(|v| v.push(target));
        if kind == ProtoTransition::Instant {
            *self.present.nav.borrow_mut() = None;
        } else {
            *self.present.nav.borrow_mut() = Some(NavAnim {
                from: None,
                to: Some(target),
                kind,
                ms,
                start: Instant::now(),
            });
        }
        self.present.prev_screen.set(None);
        self.present.nav_epoch.update(|v| *v = v.wrapping_add(1));
        self.toast("Overlay — Esc closes it");
    }

    // ───────────────────────────────────────────── interpreter core

    /// Runs one rule's actions in the scope of `element`. `payload` is what
    /// `Set x = payload` reads inside `On Event` rules.
    pub(crate) fn execute_actions(
        &self,
        actions: &[Action],
        element: u64,
        payload: &Value,
        depth: usize,
    ) {
        if depth > 8 {
            return; // Emit/Delay chains are finite by construction; belt & braces
        }
        let Some(r) = self.root.get_untracked() else {
            return;
        };
        for a in actions {
            match a {
                Action::ToggleVar(name) => {
                    let owner = behavior::var_owner_id(&r, element);
                    let next = match self.runtime_value(&r, owner, name) {
                        Value::Bool(b) => Value::Bool(!b),
                        other => other,
                    };
                    self.set_runtime_var(owner, name, next);
                }
                Action::SetVar(name, expr) => {
                    let owner = behavior::var_owner_id(&r, element);
                    let v = self.eval_expr(&r, owner, expr, payload);
                    self.set_runtime_var(owner, name, v);
                }
                Action::Show { target, transition } | Action::Hide { target, transition } => {
                    let entering = matches!(a, Action::Show { .. });
                    let scope = behavior::scope_root(&r, element);
                    let Some(t) = behavior::resolve_target(scope, target) else {
                        continue;
                    };
                    let Some(var) = (match t {
                        SvelteNode::Element(te) => te.binding.as_ref().map(|b| b.var.clone()),
                        _ => None,
                    }) else {
                        continue;
                    };
                    let owner = behavior::var_owner_id(&r, t.id());
                    self.set_runtime_var(owner, &var, Value::Bool(entering));
                    let tr = transition
                        .map(|t| (t.kind_name(), t.ms()))
                        .unwrap_or(("fade", 150));
                    self.start_anim(t.id(), tr.0, tr.1, entering);
                }
                Action::Emit(ev) => {
                    // The on-canvas counterpart of `on:{ev}` on the instance
                    // tag: the nearest instance ancestor catches the event.
                    if let Some(inst) = nearest_instance_ancestor(&r, element)
                        && let Some(actions) = rule_actions_of(
                            &r,
                            inst,
                            |t| matches!(t, Trigger::OnEvent(e) if e == ev),
                        )
                    {
                        self.execute_actions(&actions, inst, payload, depth + 1);
                    }
                }
                Action::Delay(ms, nested) => {
                    self.present.timers.borrow_mut().push(PendingTimer {
                        at: Instant::now() + Duration::from_millis(*ms as u64),
                        actions: nested.clone(),
                        element,
                        payload: payload.clone(),
                    });
                }
            }
        }
    }

    fn runtime_value(&self, root: &SvelteNode, owner: u64, name: &str) -> Value {
        if let Some(v) = self.present.vars.get_untracked().get(&(owner, name.into())) {
            return v.clone();
        }
        match tree_ops::find(root, owner) {
            Some(SvelteNode::Element(e)) => e
                .behavior
                .as_ref()
                .and_then(|b| b.var(name))
                .map(|v| v.initial.clone())
                .unwrap_or(Value::Bool(false)),
            _ => Value::Bool(false),
        }
    }

    fn eval_expr(&self, root: &SvelteNode, owner: u64, expr: &ValueExpr, payload: &Value) -> Value {
        match expr {
            ValueExpr::Value(v) => v.clone(),
            ValueExpr::Var(n) => self.runtime_value(root, owner, n),
            ValueExpr::Payload => payload.clone(),
        }
    }

    fn set_runtime_var(&self, owner: u64, name: &str, value: Value) {
        self.present.vars.update(|m| {
            m.insert((owner, name.to_string()), value);
        });
    }

    fn start_anim(&self, target: u64, kind: &'static str, ms: u32, entering: bool) {
        self.present.anims.update(|anims| {
            anims.retain(|a| !(a.target == target && a.entering == entering));
            anims.push(Anim {
                target,
                kind,
                ms,
                start: Instant::now(),
                entering,
            });
        });
    }
}

// ─────────────────────────────────────────────────────── helpers

/// First screen of a document whose root is an invisible wrapper around
/// several screen frames (the prototype starter document).
fn initial_screen(root: &SvelteNode) -> Option<u64> {
    match root {
        SvelteNode::Element(e)
            if e.style.display == crate::model::svelte_ast::Display::None
                && e.children.len() > 1 =>
        {
            e.children
                .iter()
                .find(|c| matches!(c, SvelteNode::Element(_)))
                .map(|c| c.id())
        }
        _ => None,
    }
}

fn collect_initial_vars(node: &SvelteNode, out: &mut PresentVars) {
    if let SvelteNode::Element(e) = node
        && let Some(b) = &e.behavior
    {
        for v in &b.vars {
            out.insert((e.id, v.name.clone()), v.initial.clone());
        }
    }
    for c in node.children() {
        collect_initial_vars(c, out);
    }
    if let SvelteNode::Element(e) = node {
        for sc in &e.slot_content {
            for n in &sc.nodes {
                collect_initial_vars(n, out);
            }
        }
    }
}

/// All `(element, actions)` pairs whose behavior has a rule matching `pred`.
fn rules_with_trigger(
    root: &SvelteNode,
    pred: impl Fn(&Trigger) -> bool,
) -> Vec<(u64, Vec<Action>)> {
    let mut out = Vec::new();
    walk_rules(root, &pred, &mut out);
    out
}

fn walk_rules(
    node: &SvelteNode,
    pred: &impl Fn(&Trigger) -> bool,
    out: &mut Vec<(u64, Vec<Action>)>,
) {
    if let SvelteNode::Element(e) = node
        && let Some(b) = &e.behavior
    {
        for rule in &b.rules {
            if pred(&rule.trigger) && !rule.actions.is_empty() {
                out.push((e.id, rule.actions.clone()));
            }
        }
    }
    for c in node.children() {
        walk_rules(c, pred, out);
    }
}

/// Actions of the first matching rule on `element`, if any.
fn rule_actions_of(
    root: &SvelteNode,
    element: u64,
    pred: impl Fn(&Trigger) -> bool,
) -> Option<Vec<Action>> {
    match tree_ops::find(root, element) {
        Some(SvelteNode::Element(e)) => e
            .behavior
            .as_ref()?
            .rules
            .iter()
            .find(|r| pred(&r.trigger))
            .map(|r| r.actions.clone()),
        _ => None,
    }
}

/// Nearest instance ancestor-or-self of `element` — the receiver of `Emit`.
fn nearest_instance_ancestor(root: &SvelteNode, element: u64) -> Option<u64> {
    let mut cursor = tree_ops::find(root, element)?;
    loop {
        if let SvelteNode::Element(e) = cursor {
            if e.instance {
                return Some(e.id);
            }
        }
        cursor = tree_ops::parent_of(root, cursor.id())?;
    }
}

/// Hit element plus its ancestors, topmost first.
fn ancestor_chain_ids(layers: &[Layer], hit: u64) -> Vec<u64> {
    let mut out = vec![hit];
    let mut cursor = hit;
    while let Some(l) = layers.iter().find(|l| l.id == cursor) {
        match l.parent {
            Some(p) => {
                out.push(p);
                cursor = p;
            }
            None => break,
        }
    }
    out
}

/// Ids of `target` and everything below it.
fn descendant_ids(layers: &[Layer], target: u64) -> Vec<u64> {
    let mut out = vec![target];
    let mut i = 0;
    while i < out.len() {
        let cur = out[i];
        for l in layers {
            if l.parent == Some(cur) && !out.contains(&l.id) {
                out.push(l.id);
            }
        }
        i += 1;
    }
    out
}

/// Folds an ancestor transition into a descendant: offset/scale follow the
/// ancestor's center, alpha multiplies.
fn compose_child(
    prev: &Fx,
    ancestor: &Fx,
    child_center: (f32, f32),
    ancestor_center: (f32, f32),
) -> Fx {
    let pull = ancestor.scale - 1.0;
    Fx {
        alpha: prev.alpha * ancestor.alpha,
        dx: prev.dx + ancestor.dx + pull * (child_center.0 - ancestor_center.0),
        dy: prev.dy + ancestor.dy + pull * (child_center.1 - ancestor_center.1),
        scale: prev.scale * ancestor.scale,
    }
}

/// Folds a new transition into the target's own effect.
fn compose_self(prev: &Fx, next: &Fx) -> Fx {
    Fx {
        alpha: prev.alpha * next.alpha,
        dx: prev.dx + next.dx,
        dy: prev.dy + next.dy,
        scale: prev.scale * next.scale,
    }
}

/// Smooth ease-out used by screen transitions.
fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Start offset of an entering screen (the outgoing one moves the other way).
fn nav_slide_offset(kind: ProtoTransition, w: f32, h: f32) -> (f32, f32) {
    match kind {
        ProtoTransition::SlideLeft => (w, 0.0),
        ProtoTransition::SlideRight => (-w, 0.0),
        ProtoTransition::SlideUp => (0.0, h),
        ProtoTransition::SlideDown => (0.0, -h),
        ProtoTransition::Instant | ProtoTransition::Dissolve => (0.0, 0.0),
    }
}

/// Layer ids of `root` and all of its descendants.
fn subtree_ids(layers: &[Layer], root: u64) -> HashSet<u64> {
    let mut out = HashSet::new();
    for l in layers {
        let mut cur = Some(l.id);
        let mut guard = 0;
        while let Some(id) = cur {
            if id == root {
                out.insert(l.id);
                break;
            }
            guard += 1;
            if guard > 512 {
                break;
            }
            cur = layers.iter().find(|p| p.id == id).and_then(|p| p.parent);
        }
    }
    out
}

/// Ids of the elements that are animating OUT — the layout keeps them
/// rendered so the leave transition has something to draw.
pub(crate) fn leaving_targets(anims: &[Anim]) -> HashSet<u64> {
    anims
        .iter()
        .filter(|a| !a.entering)
        .map(|a| a.target)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::behavior::{Behavior, Binding, Rule, Var, VarType};
    use crate::model::svelte_ast::{Element, new_button, new_frame};

    fn el_mut(root: &mut SvelteNode, id: u64) -> &mut Element {
        match tree_ops::find_mut(root, id) {
            Some(SvelteNode::Element(e)) => e,
            _ => panic!("element {id} missing"),
        }
    }

    /// Page(1, vars: open=false) { Panel(2, binding: open, slide 200), Btn(3, click → Toggle open) }
    fn demo() -> EditorState {
        let mut panel = new_frame(2, 0.0, 0.0, 120.0, 60.0);
        if let SvelteNode::Element(e) = &mut panel {
            e.name = "Panel".into();
            e.binding = Some(Binding {
                var: "open".into(),
                transition: Some(behavior::Transition::Slide { ms: 200 }),
            });
        }
        let mut btn = new_button(3, 0.0, 0.0);
        if let SvelteNode::Element(e) = &mut btn {
            e.name = "Btn".into();
            e.style.left = Some(0.0);
            e.style.top = Some(100.0);
            e.behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![Action::ToggleVar("open".into())],
                }],
            });
        }
        let mut root = new_frame(1, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.name = "Page".into();
            e.behavior = Some(Behavior {
                vars: vec![Var::new("open", VarType::Bool)],
                rules: vec![],
            });
            e.children = vec![panel, btn];
        }
        EditorState::with_seed(Some(root), None)
    }

    #[test]
    fn prototype_links_navigate_and_go_back() {
        let state = EditorState::with_seed(Some(crate::document::demo::prototype_tree()), None);
        state.start_present();
        assert_eq!(
            state.present.screen.get_untracked(),
            Some(1),
            "Present starts on the first screen"
        );

        // Home's "Open details" button links to screen 10.
        state.present_click(60.0, 130.0);
        assert_eq!(
            state.present.screen.get_untracked(),
            Some(10),
            "clicking a link navigates to its target"
        );

        assert!(state.present_back(), "history has the previous screen");
        assert_eq!(state.present.screen.get_untracked(), Some(1));
        state.stop_present();
    }

    #[test]
    fn click_toggles_the_bound_panel() {
        let state = demo();
        assert!(!state.present.on.get_untracked());
        state.start_present();
        assert!(state.present.on.get_untracked());

        // Btn(3) sits at (0,100); the panel (binding open=false) is hidden.
        state.present_click(5.0, 105.0);
        let vars = state.present.vars.get_untracked();
        assert_eq!(
            vars.get(&(1u64, "open".to_string())),
            Some(&Value::Bool(true)),
            "Toggle open ran"
        );

        // The button's action list is a Toggle; a second click flips back.
        state.present_click(5.0, 105.0);
        let vars = state.present.vars.get_untracked();
        assert_eq!(
            vars.get(&(1u64, "open".to_string())),
            Some(&Value::Bool(false))
        );
        state.stop_present();
        assert!(state.present.vars.get_untracked().is_empty());
    }

    #[test]
    fn show_and_hide_start_transitions_and_flip_the_binding_var() {
        let state = demo();
        // Give the button a Show/Hide rule pair instead of Toggle.
        if let Some(r) = state.root.get_untracked() {
            let mut root = r;
            el_mut(&mut root, 3).behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![Action::Show {
                        target: "Panel".into(),
                        transition: Some(behavior::Transition::Fade { ms: 120 }),
                    }],
                }],
            });
            state.root.set(Some(root));
        }
        state.start_present();
        state.present_click(5.0, 105.0);
        let vars = state.present.vars.get_untracked();
        assert_eq!(vars.get(&(1u64, "open".into())), Some(&Value::Bool(true)));
        let anims = state.present.anims.get_untracked();
        assert_eq!(anims.len(), 1);
        assert!(anims[0].entering);
        assert_eq!(anims[0].target, 2);
        state.stop_present();
    }

    #[test]
    fn emit_reaches_the_instance_event_rule() {
        // master Card(10){ Confirm(11, click → Emit "confirm") },
        // instance Card(20) with On Event(confirm) → Set open = true, page vars on root(1).
        let mut confirm = new_button(11, 0.0, 0.0);
        if let SvelteNode::Element(e) = &mut confirm {
            e.name = "Confirm".into();
            e.behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![Action::Emit("confirm".into())],
                }],
            });
        }
        let mut card = new_frame(10, 0.0, 0.0, 100.0, 50.0);
        if let SvelteNode::Element(e) = &mut card {
            e.component = true;
            e.name = "Card".into();
            e.children = vec![confirm];
        }
        let master = match card {
            SvelteNode::Element(e) => e,
            _ => unreachable!(),
        };
        let mut next = 20u64;
        let mut inst = crate::model::component_ops::instantiate(&master, vec![], vec![], &mut next);
        if let SvelteNode::Element(e) = &mut inst {
            e.style.top = Some(120.0);
            e.style.left = Some(0.0);
            e.behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnEvent("confirm".into()),
                    actions: vec![Action::SetVar(
                        "open".into(),
                        ValueExpr::Value(Value::Bool(true)),
                    )],
                }],
            });
        }
        let mut root = new_frame(1, 0.0, 0.0, 400.0, 300.0);
        if let SvelteNode::Element(e) = &mut root {
            e.name = "Page".into();
            e.behavior = Some(Behavior {
                vars: vec![Var::new("open", VarType::Bool)],
                rules: vec![],
            });
            e.children = vec![SvelteNode::Element(master), inst];
        }
        let state = EditorState::with_seed(Some(root), None);
        state.start_present();

        // The master sits at (0,0) and its copy of Confirm (absolute, world
        // coords) lands there too — drawn after the master, so it wins the
        // hit test. Clicking it emits "confirm"; the instance's On Event rule
        // sets open = true.
        state.present_click(5.0, 5.0);
        let vars = state.present.vars.get_untracked();
        assert_eq!(
            vars.get(&(1u64, "open".into())),
            Some(&Value::Bool(true)),
            "Emit routed to the instance's On Event rule"
        );
    }

    #[test]
    fn delay_fires_after_the_timer_via_advance() {
        let state = demo();
        if let Some(r) = state.root.get_untracked() {
            let mut root = r;
            el_mut(&mut root, 3).behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![Action::Delay(30, vec![Action::ToggleVar("open".into())])],
                }],
            });
            state.root.set(Some(root));
        }
        state.start_present();
        state.present_click(5.0, 105.0);
        // Not yet fired — the runtime only holds the seeded initial value.
        assert_eq!(
            state
                .present
                .vars
                .get_untracked()
                .get(&(1u64, "open".into())),
            Some(&Value::Bool(false))
        );
        std::thread::sleep(Duration::from_millis(60));
        let _ = state.advance_present();
        assert_eq!(
            state
                .present
                .vars
                .get_untracked()
                .get(&(1u64, "open".into())),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn leaving_transition_keeps_the_target_rendered() {
        let state = demo();
        if let Some(r) = state.root.get_untracked() {
            let mut root = r;
            // Start with open = true so the panel is visible, then Hide.
            el_mut(&mut root, 3).behavior = Some(Behavior {
                vars: vec![],
                rules: vec![Rule {
                    trigger: Trigger::OnClick,
                    actions: vec![Action::Hide {
                        target: "Panel".into(),
                        transition: Some(behavior::Transition::Fade { ms: 100 }),
                    }],
                }],
            });
            state.root.set(Some(root));
        }
        state.start_present();
        // Simulate open = true before the click.
        state.present.vars.update(|m| {
            m.insert((1u64, "open".into()), Value::Bool(true));
        });
        state.present_click(5.0, 105.0);
        // The panel is leaving: the runtime keeps it visible for the anim.
        let keep = leaving_targets(&state.present.anims.get_untracked());
        assert!(keep.contains(&2), "panel is animating out");
        let runtime = crate::model::layout_engine::PresentRuntime {
            vars: state.present.vars.get_untracked(),
            keep_visible: keep,
            ..Default::default()
        };
        let flat = crate::model::layout_engine::compute_layout_with_runtime(
            &state.root.get_untracked(),
            &runtime,
        );
        // Panel subtree still laid out while the transition runs.
        assert!(
            flat.iter().any(|f| f.id == 2 && f.visible),
            "leaving panel stays in the flat list"
        );
    }

    #[test]
    fn hover_rules_fire_on_enter_and_leave() {
        let state = demo();
        if let Some(r) = state.root.get_untracked() {
            let mut root = r;
            el_mut(&mut root, 3).behavior = Some(Behavior {
                vars: vec![],
                rules: vec![
                    Rule {
                        trigger: Trigger::OnHoverIn,
                        actions: vec![Action::SetVar(
                            "open".into(),
                            ValueExpr::Value(Value::Bool(true)),
                        )],
                    },
                    Rule {
                        trigger: Trigger::OnHoverOut,
                        actions: vec![Action::SetVar(
                            "open".into(),
                            ValueExpr::Value(Value::Bool(false)),
                        )],
                    },
                ],
            });
            state.root.set(Some(root));
        }
        state.start_present();
        state.present_hover(5.0, 105.0);
        assert_eq!(
            state
                .present
                .vars
                .get_untracked()
                .get(&(1u64, "open".into())),
            Some(&Value::Bool(true)),
            "hover in"
        );
        state.present_hover(350.0, 250.0);
        assert_eq!(
            state
                .present
                .vars
                .get_untracked()
                .get(&(1u64, "open".into())),
            Some(&Value::Bool(false)),
            "hover out"
        );
    }
}
